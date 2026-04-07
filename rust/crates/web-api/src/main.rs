use std::collections::{BTreeMap, BTreeSet};
use std::convert::Infallible;
use std::env;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;

use api::{
    default_model_for_provider_selection, max_tokens_for_model, resolve_model_alias,
    resolve_startup_auth_source, AnthropicClient, AuthSource, ContentBlockDelta, InputContentBlock,
    InputMessage, MessageRequest, MessageResponse, OutputContentBlock, PromptCache, ProviderClient,
    ProviderKind, ProviderSelection, StreamEvent as ApiStreamEvent, ToolChoice,
    ToolResultContentBlock,
};
use axum::extract::{Path as AxumPath, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Local;
use plugins::{PluginHooks, PluginManager, PluginManagerConfig, PluginRegistry};
use runtime::permission_enforcer::PermissionEnforcer;
use runtime::session_control::{
    create_managed_session_handle_for, list_managed_sessions_for, load_managed_session_for,
};
use runtime::{
    bind_oauth_callback_listener, clear_oauth_credentials, compact_session, complete_oauth_login,
    credentials_path, format_usd, generate_pkce_pair, generate_state, load_dotenv_for,
    load_oauth_credentials, write_oauth_callback_response, ApiClient, ApiRequest, AssistantEvent,
    CompactionConfig, ConfigLoader, ContentBlock, ConversationMessage, ConversationRuntime,
    MessageRole, OAuthAuthorizationRequest, OAuthCallbackParams, OAuthConfig, PermissionMode,
    PermissionPolicy, PromptCacheEvent, ResolvedPermissionMode, RuntimeError, Session, TokenUsage,
    ToolError, ToolExecutor, UsageTracker,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::net::TcpListener as TokioTcpListener;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_stream::StreamExt;
use tools::GlobalToolRegistry;

const DEFAULT_MODEL: &str = "claude-opus-4-6";
const DEFAULT_PORT: u16 = 8787;
const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_CONTAINER_HOST: &str = "0.0.0.0";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    let config = ServerConfig::from_args(&args)?;
    load_dotenv_for(&config.workspace_root)?;
    let listener = TokioTcpListener::bind(config.socket_addr()?).await?;
    let state = AppState::new(config.clone());

    println!(
        "Claw Web listening on http://{}:{}",
        config.display_host(),
        config.port
    );
    println!("Workspace root: {}", config.workspace_root.display());
    if let Some(public_base_url) = &config.public_base_url {
        println!("Legacy public base URL hint: {public_base_url}");
    }

    let app = Router::new()
        .route("/", get(index))
        .route("/app.js", get(app_js))
        .route("/styles.css", get(styles_css))
        .route("/api/health", get(health))
        .route("/api/auth/status", get(auth_status))
        .route("/api/auth/login/start", post(auth_login_start))
        .route("/api/auth/callback", get(auth_callback))
        .route("/api/auth/logout", post(auth_logout))
        .route("/api/sessions", get(list_sessions))
        .route("/api/sessions/:reference", get(get_session))
        .route(
            "/api/sessions/:reference/compact",
            post(compact_session_route),
        )
        .route("/api/chat/stream", post(chat_stream))
        .route("/api/chat", post(chat))
        .with_state(state);

    axum::serve(listener, app).await?;
    Ok(())
}

#[derive(Debug, Clone)]
struct ServerConfig {
    host: String,
    port: u16,
    workspace_root: PathBuf,
    public_base_url: Option<String>,
}

impl ServerConfig {
    fn from_args(args: &[String]) -> Result<Self, AppError> {
        let mut host = default_bind_host().to_string();
        let mut port = DEFAULT_PORT;
        let mut workspace_root = env::current_dir().map_err(|error| {
            AppError::internal(format!("failed to read current directory: {error}"))
        })?;
        let mut public_base_url = env::var("CLAW_WEB_BASE_URL")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--host" => {
                    let value = args
                        .get(index + 1)
                        .ok_or_else(|| AppError::bad_request("missing value for --host"))?;
                    host.clone_from(value);
                    index += 2;
                }
                "--port" => {
                    let value = args
                        .get(index + 1)
                        .ok_or_else(|| AppError::bad_request("missing value for --port"))?;
                    port = value.parse::<u16>().map_err(|error| {
                        AppError::bad_request(format!("invalid --port value `{value}`: {error}"))
                    })?;
                    index += 2;
                }
                "--cwd" => {
                    let value = args
                        .get(index + 1)
                        .ok_or_else(|| AppError::bad_request("missing value for --cwd"))?;
                    workspace_root = PathBuf::from(value);
                    index += 2;
                }
                "--public-base-url" => {
                    public_base_url = Some(
                        args.get(index + 1)
                            .ok_or_else(|| {
                                AppError::bad_request("missing value for --public-base-url")
                            })?
                            .trim()
                            .trim_end_matches('/')
                            .to_string(),
                    );
                    index += 2;
                }
                "--help" | "-h" => {
                    print_help();
                    std::process::exit(0);
                }
                other => {
                    return Err(AppError::bad_request(format!(
                        "unknown option `{other}`. Use --host, --port, --cwd, or --public-base-url."
                    )))
                }
            }
        }

        Ok(Self {
            host,
            port,
            workspace_root,
            public_base_url,
        })
    }

    fn socket_addr(&self) -> Result<SocketAddr, AppError> {
        format!("{}:{}", self.host, self.port)
            .to_socket_addrs()
            .map_err(|error| {
                AppError::bad_request(format!(
                    "failed to resolve bind address {}:{}: {error}",
                    self.host, self.port
                ))
            })?
            .next()
            .ok_or_else(|| AppError::bad_request("resolved bind address list was empty"))
    }

    fn display_host(&self) -> &str {
        if self.host == "0.0.0.0" {
            "localhost"
        } else {
            &self.host
        }
    }
}

fn print_help() {
    println!("claw-web");
    println!();
    println!("Usage:");
    println!("  claw-web [--host HOST] [--port PORT] [--cwd PATH] [--public-base-url URL]");
    println!();
    println!("Options:");
    println!(
        "  --host HOST             Bind host (default: {DEFAULT_HOST}; auto-switches to {DEFAULT_CONTAINER_HOST} in containers)"
    );
    println!("  --port PORT             Bind port (default: {DEFAULT_PORT})");
    println!("  --cwd PATH              Workspace root used for config, sessions, and tools");
    println!("  --public-base-url URL   Legacy external base URL hint; OAuth now always uses localhost loopback callbacks");
}

#[derive(Clone)]
struct AppState {
    shared: Arc<SharedState>,
}

struct SharedState {
    config: ServerConfig,
    pending_oauth: Mutex<BTreeMap<String, PendingOAuthFlow>>,
    oauth_listener: OnceLock<WebOAuthListener>,
}

impl AppState {
    fn new(config: ServerConfig) -> Self {
        Self {
            shared: Arc::new(SharedState {
                config,
                pending_oauth: Mutex::new(BTreeMap::new()),
                oauth_listener: OnceLock::new(),
            }),
        }
    }

    fn workspace_root(&self) -> &Path {
        &self.shared.config.workspace_root
    }
}

#[derive(Debug, Clone)]
struct PendingOAuthFlow {
    oauth: OAuthConfig,
    redirect_uri: String,
    verifier: String,
}

#[derive(Debug)]
struct WebOAuthListener {
    callback_port: u16,
}

#[derive(Debug)]
struct AppError {
    status: StatusCode,
    message: String,
}

impl AppError {
    fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }

    fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, message)
    }

    fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, message)
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(json!({
                "error": self.message,
            })),
        )
            .into_response()
    }
}

type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
    workspace_root: String,
    date: String,
    anthropic_default_model: String,
    openai_default_model: Option<String>,
    xai_default_model: String,
    gemini_default_model: String,
    deepseek_default_model: String,
    perplexity_default_model: String,
    default_system_prompt: Option<String>,
}

#[derive(Debug, Serialize)]
#[allow(clippy::struct_excessive_bools)]
struct AuthStatusResponse {
    authenticated: bool,
    inference_ready: bool,
    active_source: &'static str,
    env_api_key: bool,
    env_bearer_token: bool,
    env_openai_api_key: bool,
    env_xai_api_key: bool,
    env_gemini_api_key: bool,
    env_deepseek_api_key: bool,
    env_perplexity_api_key: bool,
    anthropic_inference_ready: bool,
    openai_inference_ready: bool,
    xai_inference_ready: bool,
    gemini_inference_ready: bool,
    deepseek_inference_ready: bool,
    perplexity_inference_ready: bool,
    saved_oauth: bool,
    saved_oauth_expired: bool,
    expires_at: Option<u64>,
    scopes: Vec<String>,
    credentials_path: Option<String>,
    warning: Option<String>,
}

#[derive(Debug, Serialize)]
struct LoginStartResponse {
    authorize_url: String,
    redirect_uri: String,
}

#[derive(Debug, Serialize)]
struct SessionSummaryResponse {
    id: String,
    path: String,
    modified_epoch_millis: u128,
    message_count: usize,
    parent_session_id: Option<String>,
    branch_name: Option<String>,
}

#[derive(Debug, Serialize)]
struct SessionResponse {
    id: String,
    path: String,
    message_count: usize,
    turns: u32,
    usage: UsageResponse,
    compaction: Option<CompactionResponse>,
    messages: Vec<MessageResponseDto>,
}

#[derive(Debug, Serialize)]
struct CompactionResponse {
    count: u32,
    removed_message_count: usize,
    summary: String,
}

#[derive(Debug, Serialize)]
struct UsageResponse {
    input_tokens: u32,
    output_tokens: u32,
    cache_creation_input_tokens: u32,
    cache_read_input_tokens: u32,
    total_tokens: u32,
    estimated_cost_usd: String,
}

#[derive(Debug, Serialize)]
struct MessageResponseDto {
    role: &'static str,
    blocks: Vec<MessageBlockResponse>,
    preview: String,
    usage: Option<UsageResponse>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum MessageBlockResponse {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: String,
    },
    ToolResult {
        tool_use_id: String,
        tool_name: String,
        output: String,
        is_error: bool,
    },
}

#[derive(Debug, Deserialize)]
struct ChatRequest {
    prompt: String,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    provider: Option<String>,
    #[serde(default)]
    permission_mode: Option<String>,
    #[serde(default)]
    allowed_tools: Vec<String>,
    #[serde(default)]
    enable_tools: Option<bool>,
    #[serde(default)]
    system_prompt: Option<String>,
}

#[derive(Debug, Serialize)]
struct ChatResponse {
    session: SessionResponse,
    assistant_text: String,
    iterations: usize,
    usage: UsageResponse,
    auto_compacted: Option<usize>,
    prompt_cache_events: Vec<PromptCacheEventResponse>,
}

#[derive(Debug, Serialize)]
struct PromptCacheEventResponse {
    unexpected: bool,
    reason: String,
    previous_cache_read_input_tokens: u32,
    current_cache_read_input_tokens: u32,
    token_drop: u32,
}

#[derive(Debug, Serialize)]
struct StreamTurnStarted {
    session_id: String,
    model: String,
    provider: String,
    permission_mode: String,
    enable_tools: bool,
}

#[derive(Debug, Serialize)]
struct AssistantTextDeltaResponse {
    text: String,
}

#[derive(Debug, Serialize)]
struct AssistantToolUseResponse {
    id: String,
    name: String,
    input: String,
}

#[derive(Debug, Serialize)]
struct ToolExecutionStartedResponse {
    tool_name: String,
    input: String,
}

#[derive(Debug, Serialize)]
struct ToolExecutionFinishedResponse {
    tool_name: String,
    output: String,
    is_error: bool,
}

#[derive(Debug, Serialize)]
struct StreamErrorResponse {
    message: String,
}

#[derive(Debug)]
struct StreamEnvelope {
    event: String,
    data: String,
}

#[derive(Clone)]
struct StreamEventSink {
    sender: mpsc::UnboundedSender<StreamEnvelope>,
}

impl StreamEventSink {
    fn new(sender: mpsc::UnboundedSender<StreamEnvelope>) -> Self {
        Self { sender }
    }

    fn send<T: Serialize>(&self, event: &str, payload: &T) {
        let Ok(data) = serde_json::to_string(payload) else {
            return;
        };
        let _ = self.sender.send(StreamEnvelope {
            event: event.to_string(),
            data,
        });
    }

    fn turn_started(
        &self,
        session_id: &str,
        model: &str,
        provider: ProviderKind,
        permission_mode: PermissionMode,
        enable_tools: bool,
    ) {
        self.send(
            "turn_started",
            &StreamTurnStarted {
                session_id: session_id.to_string(),
                model: model.to_string(),
                provider: provider.as_str().to_string(),
                permission_mode: permission_mode.as_str().to_string(),
                enable_tools,
            },
        );
    }

    fn assistant_text_delta(&self, text: &str) {
        self.send(
            "assistant_text_delta",
            &AssistantTextDeltaResponse {
                text: text.to_string(),
            },
        );
    }

    fn assistant_tool_use(&self, id: &str, name: &str, input: &str) {
        self.send(
            "assistant_tool_use",
            &AssistantToolUseResponse {
                id: id.to_string(),
                name: name.to_string(),
                input: input.to_string(),
            },
        );
    }

    fn assistant_usage(&self, usage: &UsageResponse) {
        self.send("assistant_usage", usage);
    }

    fn prompt_cache(&self, event: &PromptCacheEventResponse) {
        self.send("prompt_cache", event);
    }

    fn tool_execution_started(&self, tool_name: &str, input: &str) {
        self.send(
            "tool_execution_started",
            &ToolExecutionStartedResponse {
                tool_name: tool_name.to_string(),
                input: input.to_string(),
            },
        );
    }

    fn tool_execution_finished(&self, tool_name: &str, output: &str, is_error: bool) {
        self.send(
            "tool_execution_finished",
            &ToolExecutionFinishedResponse {
                tool_name: tool_name.to_string(),
                output: output.to_string(),
                is_error,
            },
        );
    }

    fn done(&self, response: &ChatResponse) {
        self.send("done", response);
    }

    fn error_message(&self, message: impl Into<String>) {
        self.send(
            "error",
            &StreamErrorResponse {
                message: message.into(),
            },
        );
    }
}

#[derive(Debug, Deserialize)]
struct ToolSearchRequest {
    query: String,
    max_results: Option<usize>,
}

#[derive(Debug, Clone)]
struct WebRuntimePluginState {
    feature_config: runtime::RuntimeFeatureConfig,
    tool_registry: GlobalToolRegistry,
    plugin_registry: PluginRegistry,
}

struct WebRuntimeClient {
    runtime: tokio::runtime::Runtime,
    client: ProviderClient,
    model: String,
    enable_tools: bool,
    allowed_tools: Option<AllowedToolSet>,
    tool_registry: GlobalToolRegistry,
    stream_sink: Option<StreamEventSink>,
}

type AllowedToolSet = BTreeSet<String>;

impl WebRuntimeClient {
    #[allow(clippy::too_many_arguments)]
    fn new(
        cwd: &Path,
        session_id: &str,
        model: String,
        provider_selection: ProviderSelection,
        enable_tools: bool,
        allowed_tools: Option<AllowedToolSet>,
        tool_registry: GlobalToolRegistry,
        stream_sink: Option<StreamEventSink>,
    ) -> Result<Self, AppError> {
        Ok(Self {
            runtime: tokio::runtime::Runtime::new().map_err(|error| {
                AppError::internal(format!("failed to create tokio runtime: {error}"))
            })?,
            client: resolve_web_provider_client(cwd, &model, provider_selection)?
                .with_prompt_cache(PromptCache::new(session_id)),
            model,
            enable_tools,
            allowed_tools,
            tool_registry,
            stream_sink,
        })
    }
}

impl ApiClient for WebRuntimeClient {
    #[allow(clippy::too_many_lines)]
    fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
        let message_request = MessageRequest {
            model: self.model.clone(),
            max_tokens: max_tokens_for_model(&self.model),
            messages: convert_messages(&request.messages),
            system: (!request.system_prompt.is_empty()).then(|| request.system_prompt.join("\n\n")),
            tools: self
                .enable_tools
                .then(|| self.tool_registry.definitions(self.allowed_tools.as_ref())),
            tool_choice: self.enable_tools.then_some(ToolChoice::Auto),
            stream: true,
        };

        self.runtime.block_on(async {
            let mut stream = self
                .client
                .stream_message(&message_request)
                .await
                .map_err(|error| RuntimeError::new(error.to_string()))?;
            let mut events = Vec::new();
            let mut pending_tool: Option<(String, String, String)> = None;
            let mut saw_stop = false;

            while let Some(event) = stream
                .next_event()
                .await
                .map_err(|error| RuntimeError::new(error.to_string()))?
            {
                match event {
                    ApiStreamEvent::MessageStart(start) => {
                        for block in start.message.content {
                            push_output_block(
                                block,
                                &mut events,
                                &mut pending_tool,
                                true,
                                self.stream_sink.as_ref(),
                            );
                        }
                    }
                    ApiStreamEvent::ContentBlockStart(start) => {
                        push_output_block(
                            start.content_block,
                            &mut events,
                            &mut pending_tool,
                            true,
                            self.stream_sink.as_ref(),
                        );
                    }
                    ApiStreamEvent::ContentBlockDelta(delta) => match delta.delta {
                        ContentBlockDelta::TextDelta { text } => {
                            if !text.is_empty() {
                                if let Some(stream_sink) = &self.stream_sink {
                                    stream_sink.assistant_text_delta(&text);
                                }
                                events.push(AssistantEvent::TextDelta(text));
                            }
                        }
                        ContentBlockDelta::InputJsonDelta { partial_json } => {
                            if let Some((_, _, input)) = &mut pending_tool {
                                input.push_str(&partial_json);
                            }
                        }
                        ContentBlockDelta::ThinkingDelta { .. }
                        | ContentBlockDelta::SignatureDelta { .. } => {}
                    },
                    ApiStreamEvent::ContentBlockStop(_) => {
                        if let Some((id, name, input)) = pending_tool.take() {
                            if let Some(stream_sink) = &self.stream_sink {
                                stream_sink.assistant_tool_use(&id, &name, &input);
                            }
                            events.push(AssistantEvent::ToolUse { id, name, input });
                        }
                    }
                    ApiStreamEvent::MessageDelta(delta) => {
                        let usage = delta.usage.token_usage();
                        if let Some(stream_sink) = &self.stream_sink {
                            stream_sink
                                .assistant_usage(&usage_to_response(usage, Some(&self.model)));
                        }
                        events.push(AssistantEvent::Usage(usage));
                    }
                    ApiStreamEvent::MessageStop(_) => {
                        saw_stop = true;
                        events.push(AssistantEvent::MessageStop);
                    }
                }
            }

            push_prompt_cache_record(&self.client, &mut events, self.stream_sink.as_ref());

            if !saw_stop
                && events.iter().any(|event| {
                    matches!(event, AssistantEvent::TextDelta(text) if !text.is_empty())
                        || matches!(event, AssistantEvent::ToolUse { .. })
                })
            {
                events.push(AssistantEvent::MessageStop);
            }

            if events
                .iter()
                .any(|event| matches!(event, AssistantEvent::MessageStop))
            {
                return Ok(events);
            }

            let response = self
                .client
                .send_message(&MessageRequest {
                    stream: false,
                    ..message_request.clone()
                })
                .await
                .map_err(|error| RuntimeError::new(error.to_string()))?;
            let mut events =
                response_to_events(response, self.stream_sink.as_ref(), Some(&self.model));
            push_prompt_cache_record(&self.client, &mut events, self.stream_sink.as_ref());
            Ok(events)
        })
    }
}

struct WebToolExecutor {
    allowed_tools: Option<AllowedToolSet>,
    tool_registry: GlobalToolRegistry,
    stream_sink: Option<StreamEventSink>,
}

impl WebToolExecutor {
    fn new(
        allowed_tools: Option<AllowedToolSet>,
        tool_registry: GlobalToolRegistry,
        stream_sink: Option<StreamEventSink>,
    ) -> Self {
        Self {
            allowed_tools,
            tool_registry,
            stream_sink,
        }
    }

    fn execute_search_tool(&self, value: Value) -> Result<String, ToolError> {
        let input: ToolSearchRequest = serde_json::from_value(value)
            .map_err(|error| ToolError::new(format!("invalid tool input JSON: {error}")))?;
        serde_json::to_string_pretty(&self.tool_registry.search(
            &input.query,
            input.max_results.unwrap_or(5),
            None,
            None,
        ))
        .map_err(|error| ToolError::new(error.to_string()))
    }
}

impl ToolExecutor for WebToolExecutor {
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, ToolError> {
        if self
            .allowed_tools
            .as_ref()
            .is_some_and(|allowed| !allowed.contains(tool_name))
        {
            return Err(ToolError::new(format!(
                "tool `{tool_name}` is not enabled by the current allowed tools setting"
            )));
        }

        if let Some(stream_sink) = &self.stream_sink {
            stream_sink.tool_execution_started(tool_name, input);
        }

        let result = (|| {
            let value = serde_json::from_str::<Value>(input)
                .map_err(|error| ToolError::new(format!("invalid tool input JSON: {error}")))?;
            if tool_name == "ToolSearch" {
                return self.execute_search_tool(value);
            }
            if self.tool_registry.has_runtime_tool(tool_name) {
                return Err(ToolError::new(format!(
                    "runtime tool `{tool_name}` is not available in the web API yet"
                )));
            }

            self.tool_registry
                .execute(tool_name, &value)
                .map_err(ToolError::new)
        })();

        if let Some(stream_sink) = &self.stream_sink {
            match &result {
                Ok(output) => stream_sink.tool_execution_finished(tool_name, output, false),
                Err(error) => {
                    stream_sink.tool_execution_finished(tool_name, &error.to_string(), true);
                }
            }
        }

        result
    }
}

async fn index() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}

async fn app_js() -> impl IntoResponse {
    (
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/javascript; charset=utf-8"),
        )],
        include_str!("../static/app.js"),
    )
}

async fn styles_css() -> impl IntoResponse {
    (
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/css; charset=utf-8"),
        )],
        include_str!("../static/styles.css"),
    )
}

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        workspace_root: state.workspace_root().display().to_string(),
        date: current_date_string(),
        anthropic_default_model: default_model_for_provider_selection(ProviderSelection::Anthropic)
            .unwrap_or_else(|| DEFAULT_MODEL.to_string()),
        openai_default_model: default_model_for_provider_selection(
            ProviderSelection::OpenAiCompatible,
        ),
        xai_default_model: default_model_for_provider_selection(ProviderSelection::Xai)
            .unwrap_or_else(|| "grok-3".to_string()),
        gemini_default_model: default_model_for_provider_selection(ProviderSelection::Gemini)
            .unwrap_or_else(|| "gemini-3-flash-preview".to_string()),
        deepseek_default_model: default_model_for_provider_selection(ProviderSelection::DeepSeek)
            .unwrap_or_else(|| "deepseek-chat".to_string()),
        perplexity_default_model: default_model_for_provider_selection(
            ProviderSelection::Perplexity,
        )
        .unwrap_or_else(|| "sonar-pro".to_string()),
        default_system_prompt: env::var("CLAW_SYSTEM_PROMPT")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
    })
}

async fn auth_status() -> AppResult<Json<AuthStatusResponse>> {
    Ok(Json(read_auth_status()?))
}

fn ensure_web_oauth_listener(state: &AppState, callback_port: u16) -> AppResult<()> {
    if let Some(existing) = state.shared.oauth_listener.get() {
        if existing.callback_port != callback_port {
            return Err(AppError::internal(format!(
                "web OAuth listener is already bound to port {}; restart claw-web to switch callback ports",
                existing.callback_port
            )));
        }
        return Ok(());
    }

    let listener = bind_oauth_callback_listener(callback_port).map_err(|error| {
        AppError::internal(format!(
            "failed to bind OAuth callback listener on http://localhost:{callback_port}/callback: {error}"
        ))
    })?;

    if state
        .shared
        .oauth_listener
        .set(WebOAuthListener { callback_port })
        .is_ok()
    {
        let shared = Arc::clone(&state.shared);
        thread::Builder::new()
            .name(format!("claw-web-oauth-{callback_port}"))
            .spawn(move || run_web_oauth_listener(listener, shared))
            .map_err(|error| {
                AppError::internal(format!(
                    "failed to start OAuth callback listener thread: {error}"
                ))
            })?;
        return Ok(());
    }

    drop(listener);
    let existing = state.shared.oauth_listener.get().ok_or_else(|| {
        AppError::internal("OAuth callback listener state became unavailable".to_string())
    })?;
    if existing.callback_port != callback_port {
        return Err(AppError::internal(format!(
            "web OAuth listener is already bound to port {}; restart claw-web to switch callback ports",
            existing.callback_port
        )));
    }
    Ok(())
}

#[allow(clippy::needless_pass_by_value)]
fn run_web_oauth_listener(listener: std::net::TcpListener, shared: Arc<SharedState>) {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("claw-web OAuth listener failed to create tokio runtime: {error}");
            return;
        }
    };

    loop {
        match runtime::accept_oauth_callback(&listener) {
            Ok((mut stream, callback)) => {
                let result = complete_web_oauth_callback(&shared, &runtime, callback);
                let (success, message) = match result {
                    Ok(()) => (true, "Claude OAuth login complete.".to_string()),
                    Err(message) => (false, message),
                };
                if let Err(error) = write_oauth_callback_response(
                    &mut stream,
                    "text/html; charset=utf-8",
                    &auth_callback_page(success, &message),
                ) {
                    eprintln!("claw-web OAuth listener failed to write callback response: {error}");
                }
            }
            Err(error) => {
                eprintln!("claw-web OAuth listener failed to read callback request: {error}");
            }
        }
    }
}

fn complete_web_oauth_callback(
    shared: &SharedState,
    runtime: &tokio::runtime::Runtime,
    callback: OAuthCallbackParams,
) -> Result<(), String> {
    let client = AnthropicClient::from_auth(AuthSource::None).with_base_url(api::read_base_url());
    complete_web_oauth_callback_with_exchange(shared, callback, |oauth, exchange_request| {
        runtime
            .block_on(client.exchange_oauth_code(oauth, exchange_request))
            .map(|token_set| runtime::OAuthTokenSet {
                access_token: token_set.access_token,
                refresh_token: token_set.refresh_token,
                expires_at: token_set.expires_at,
                scopes: token_set.scopes,
            })
            .map_err(|error: api::ApiError| error.to_string())
    })?;
    Ok(())
}

fn complete_web_oauth_callback_with_exchange<F, E>(
    shared: &SharedState,
    callback: OAuthCallbackParams,
    exchange: F,
) -> Result<(), String>
where
    F: FnOnce(
        &OAuthConfig,
        &runtime::OAuthTokenExchangeRequest,
    ) -> Result<runtime::OAuthTokenSet, E>,
    E: std::fmt::Display,
{
    let state_token = callback
        .state
        .clone()
        .ok_or_else(|| "callback did not include state".to_string())?;
    let pending = shared
        .pending_oauth
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .remove(&state_token)
        .ok_or_else(|| "unknown or expired OAuth state".to_string())?;
    let oauth = pending.oauth;

    complete_oauth_login(
        &oauth,
        callback,
        &state_token,
        pending.verifier,
        pending.redirect_uri,
        |exchange_request| exchange(&oauth, exchange_request),
    )?;
    Ok(())
}

async fn auth_login_start(State(state): State<AppState>) -> AppResult<Json<LoginStartResponse>> {
    let loader = ConfigLoader::default_for(state.workspace_root());
    let runtime_config = loader
        .load()
        .map_err(|error| AppError::internal(format!("failed to load runtime config: {error}")))?;
    let oauth = runtime_config
        .oauth()
        .cloned()
        .unwrap_or_else(default_oauth_config);
    let callback_port = runtime::oauth_callback_port(&oauth);
    ensure_web_oauth_listener(&state, callback_port)?;
    let redirect_uri = runtime::loopback_redirect_uri_for_config(&oauth);
    let pkce = generate_pkce_pair()
        .map_err(|error| AppError::internal(format!("failed to generate PKCE pair: {error}")))?;
    let state_token = generate_state()
        .map_err(|error| AppError::internal(format!("failed to generate OAuth state: {error}")))?;
    let authorize_url = OAuthAuthorizationRequest::from_config(
        &oauth,
        redirect_uri.clone(),
        state_token.clone(),
        &pkce,
    )
    .build_url();

    state
        .shared
        .pending_oauth
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(
            state_token,
            PendingOAuthFlow {
                oauth,
                redirect_uri: redirect_uri.clone(),
                verifier: pkce.verifier,
            },
        );

    Ok(Json(LoginStartResponse {
        authorize_url,
        redirect_uri,
    }))
}

async fn auth_callback() -> Html<String> {
    Html(auth_callback_page(
        false,
        "Claw Web no longer completes OAuth on this port. Start login from the app and allow the browser to return to http://localhost:4545/callback. In Docker, publish -p 4545:4545.",
    ))
}

async fn auth_logout() -> AppResult<Json<Value>> {
    clear_oauth_credentials().map_err(|error| {
        AppError::internal(format!("failed to clear OAuth credentials: {error}"))
    })?;
    Ok(Json(json!({ "ok": true })))
}

async fn list_sessions(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<SessionSummaryResponse>>> {
    let sessions =
        list_managed_sessions_for(state.workspace_root()).map_err(session_control_error)?;
    Ok(Json(
        sessions
            .into_iter()
            .map(|session| SessionSummaryResponse {
                id: session.id,
                path: session.path.display().to_string(),
                modified_epoch_millis: session.modified_epoch_millis,
                message_count: session.message_count,
                parent_session_id: session.parent_session_id,
                branch_name: session.branch_name,
            })
            .collect(),
    ))
}

async fn get_session(
    State(state): State<AppState>,
    AxumPath(reference): AxumPath<String>,
) -> AppResult<Json<SessionResponse>> {
    let loaded = load_managed_session_for(state.workspace_root(), &reference)
        .map_err(session_control_error)?;
    Ok(Json(session_to_response(
        &loaded.handle.path,
        &loaded.session,
    )))
}

async fn compact_session_route(
    State(state): State<AppState>,
    AxumPath(reference): AxumPath<String>,
) -> AppResult<Json<SessionResponse>> {
    let loaded = load_managed_session_for(state.workspace_root(), &reference)
        .map_err(session_control_error)?;
    let result = compact_session(
        &loaded.session,
        CompactionConfig {
            max_estimated_tokens: 0,
            ..CompactionConfig::default()
        },
    );
    result
        .compacted_session
        .save_to_path(&loaded.handle.path)
        .map_err(|error| {
            AppError::internal(format!("failed to save compacted session: {error}"))
        })?;
    Ok(Json(session_to_response(
        &loaded.handle.path,
        &result.compacted_session,
    )))
}

async fn chat(
    State(state): State<AppState>,
    Json(request): Json<ChatRequest>,
) -> AppResult<Json<ChatResponse>> {
    let workspace_root = state.workspace_root().to_path_buf();
    let response =
        tokio::task::spawn_blocking(move || execute_chat_request(&workspace_root, request, None))
            .await
            .map_err(|error| AppError::internal(format!("chat worker task failed: {error}")))??;
    Ok(Json(response))
}

async fn chat_stream(
    State(state): State<AppState>,
    Json(request): Json<ChatRequest>,
) -> AppResult<Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>>> {
    let (sender, receiver) = mpsc::unbounded_channel();
    let stream_sink = StreamEventSink::new(sender);
    let workspace_root = state.workspace_root().to_path_buf();

    tokio::spawn(async move {
        let worker_sink = stream_sink.clone();
        let result = tokio::task::spawn_blocking(move || {
            execute_chat_request(&workspace_root, request, Some(worker_sink.clone()))
        })
        .await;

        match result {
            Ok(Ok(response)) => stream_sink.done(&response),
            Ok(Err(error)) => stream_sink.error_message(error.to_string()),
            Err(error) => {
                stream_sink.error_message(format!("chat stream worker task failed: {error}"));
            }
        }
    });

    let stream = UnboundedReceiverStream::new(receiver).map(|envelope| {
        Ok::<_, Infallible>(Event::default().event(envelope.event).data(envelope.data))
    });
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

fn execute_chat_request(
    workspace_root: &Path,
    request: ChatRequest,
    stream_sink: Option<StreamEventSink>,
) -> AppResult<ChatResponse> {
    if request.prompt.trim().is_empty() {
        return Err(AppError::bad_request("prompt must not be empty"));
    }

    let loader = ConfigLoader::default_for(workspace_root);
    let runtime_config = loader
        .load()
        .map_err(|error| AppError::internal(format!("failed to load runtime config: {error}")))?;
    let runtime_plugin_state =
        build_runtime_plugin_state_with_loader(workspace_root, &loader, &runtime_config)?;
    runtime_plugin_state
        .plugin_registry
        .initialize()
        .map_err(|error| AppError::internal(format!("failed to initialize plugins: {error}")))?;

    let result = execute_chat_request_with_plugins(
        workspace_root,
        &runtime_config,
        request,
        runtime_plugin_state.clone(),
        stream_sink,
    );
    let shutdown_result = runtime_plugin_state
        .plugin_registry
        .shutdown()
        .map_err(|error| AppError::internal(format!("failed to shut down plugins: {error}")));

    match result {
        Ok(response) => {
            shutdown_result?;
            Ok(response)
        }
        Err(error) => Err(error),
    }
}

fn execute_chat_request_with_plugins(
    workspace_root: &Path,
    runtime_config: &runtime::RuntimeConfig,
    request: ChatRequest,
    mut runtime_plugin_state: WebRuntimePluginState,
    stream_sink: Option<StreamEventSink>,
) -> AppResult<ChatResponse> {
    let permission_mode =
        requested_permission_mode(runtime_config, request.permission_mode.as_deref())?;
    let allowed_tools = runtime_plugin_state
        .tool_registry
        .normalize_allowed_tools(&request.allowed_tools)
        .map_err(AppError::bad_request)?;
    let policy = permission_policy(
        permission_mode,
        &runtime_plugin_state.feature_config,
        &runtime_plugin_state.tool_registry,
    )
    .map_err(AppError::internal)?;
    runtime_plugin_state
        .tool_registry
        .set_enforcer(PermissionEnforcer::new(policy.clone()));

    let (handle, session, _created) =
        load_or_create_session(workspace_root, request.session_id.as_deref())?;
    let session_id = session.session_id.clone();
    let provider_selection = parse_provider_selection(request.provider.as_deref())?;
    let model = resolve_requested_model(request.model.as_deref(), provider_selection)?;
    let provider_kind = provider_selection.resolve_kind(&model);
    let enable_tools = request.enable_tools.unwrap_or(true);
    if let Some(stream_sink) = &stream_sink {
        stream_sink.turn_started(
            &session_id,
            &model,
            provider_kind,
            permission_mode,
            enable_tools,
        );
    }
    let system_prompt = runtime::load_system_prompt(
        workspace_root.to_path_buf(),
        current_date_string(),
        env::consts::OS,
        "unknown",
    )
    .map_err(|error| AppError::internal(format!("failed to build system prompt: {error}")))?;
    let system_prompt = extend_system_prompt(system_prompt, request.system_prompt.as_deref());

    let mut runtime = ConversationRuntime::new_with_features(
        session,
        WebRuntimeClient::new(
            workspace_root,
            &session_id,
            model.clone(),
            provider_selection,
            enable_tools,
            allowed_tools.clone(),
            runtime_plugin_state.tool_registry.clone(),
            stream_sink.clone(),
        )?,
        WebToolExecutor::new(
            allowed_tools,
            runtime_plugin_state.tool_registry,
            stream_sink,
        ),
        policy,
        system_prompt,
        &runtime_plugin_state.feature_config,
    );

    let turn_result = runtime.run_turn(request.prompt, None);
    let session = runtime.into_session();
    session
        .save_to_path(&handle.path)
        .map_err(|error| AppError::internal(format!("failed to save session: {error}")))?;

    let summary = turn_result.map_err(classify_runtime_error)?;
    Ok(ChatResponse {
        assistant_text: final_assistant_text(&summary),
        iterations: summary.iterations,
        usage: usage_to_response(summary.usage, Some(&model)),
        auto_compacted: summary
            .auto_compaction
            .map(|event| event.removed_message_count),
        prompt_cache_events: summary
            .prompt_cache_events
            .into_iter()
            .map(|event| PromptCacheEventResponse {
                unexpected: event.unexpected,
                reason: event.reason,
                previous_cache_read_input_tokens: event.previous_cache_read_input_tokens,
                current_cache_read_input_tokens: event.current_cache_read_input_tokens,
                token_drop: event.token_drop,
            })
            .collect(),
        session: session_to_response(&handle.path, &session),
    })
}

fn parse_provider_selection(value: Option<&str>) -> AppResult<ProviderSelection> {
    value
        .map(ProviderSelection::parse)
        .transpose()
        .map_err(AppError::bad_request)?
        .map_or(Ok(ProviderSelection::Auto), Ok)
}

fn resolve_requested_model(
    requested_model: Option<&str>,
    provider_selection: ProviderSelection,
) -> AppResult<String> {
    if let Some(model) = requested_model
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Ok(resolve_model_alias(model));
    }

    if let Some(default_model) = default_model_for_provider_selection(provider_selection) {
        return Ok(resolve_model_alias(&default_model));
    }

    match provider_selection {
        ProviderSelection::OpenAiCompatible => Err(AppError::bad_request(
            "no model specified for openai-compatible provider; set OPENAI_MODEL or enter a model id in the UI",
        )),
        ProviderSelection::Xai => Err(AppError::bad_request(
            "no model specified for xai provider; set XAI_MODEL or enter a model id in the UI",
        )),
        ProviderSelection::Gemini => Err(AppError::bad_request(
            "no model specified for gemini provider; set GEMINI_MODEL or enter a model id in the UI",
        )),
        ProviderSelection::DeepSeek => Err(AppError::bad_request(
            "no model specified for deepseek provider; set DEEPSEEK_MODEL or enter a model id in the UI",
        )),
        ProviderSelection::Perplexity => Err(AppError::bad_request(
            "no model specified for perplexity provider; set PERPLEXITY_MODEL or enter a model id in the UI",
        )),
        ProviderSelection::Auto | ProviderSelection::Anthropic => Ok(DEFAULT_MODEL.to_string()),
    }
}

fn extend_system_prompt(
    mut base_prompt: Vec<String>,
    custom_system_prompt: Option<&str>,
) -> Vec<String> {
    if let Some(custom) = custom_system_prompt
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        base_prompt.push(custom.to_string());
    }
    base_prompt
}

fn load_or_create_session(
    workspace_root: &Path,
    reference: Option<&str>,
) -> AppResult<(runtime::session_control::SessionHandle, Session, bool)> {
    let reference = reference.map(str::trim).filter(|value| !value.is_empty());
    if let Some(reference) = reference {
        let loaded =
            load_managed_session_for(workspace_root, reference).map_err(session_control_error)?;
        return Ok((loaded.handle, loaded.session, false));
    }

    let session = Session::new();
    let handle = create_managed_session_handle_for(workspace_root, &session.session_id)
        .map_err(session_control_error)?;
    let session = session.with_persistence_path(handle.path.clone());
    session
        .save_to_path(&handle.path)
        .map_err(|error| AppError::internal(format!("failed to save new session: {error}")))?;
    Ok((handle, session, true))
}

fn build_runtime_plugin_state_with_loader(
    cwd: &Path,
    loader: &ConfigLoader,
    runtime_config: &runtime::RuntimeConfig,
) -> AppResult<WebRuntimePluginState> {
    let plugin_manager = build_plugin_manager(cwd, loader, runtime_config);
    let plugin_registry = plugin_manager
        .plugin_registry()
        .map_err(|error| AppError::internal(format!("failed to build plugin registry: {error}")))?;
    let plugin_hook_config =
        runtime_hook_config_from_plugin_hooks(plugin_registry.aggregated_hooks().map_err(
            |error| AppError::internal(format!("failed to load plugin hooks: {error}")),
        )?);
    let feature_config = runtime_config
        .feature_config()
        .clone()
        .with_hooks(runtime_config.hooks().merged(&plugin_hook_config));
    let tool_registry =
        GlobalToolRegistry::with_plugin_tools(plugin_registry.aggregated_tools().map_err(
            |error| AppError::internal(format!("failed to load plugin tools: {error}")),
        )?)
        .map_err(AppError::internal)?;

    Ok(WebRuntimePluginState {
        feature_config,
        tool_registry,
        plugin_registry,
    })
}

fn build_plugin_manager(
    cwd: &Path,
    loader: &ConfigLoader,
    runtime_config: &runtime::RuntimeConfig,
) -> PluginManager {
    let plugin_settings = runtime_config.plugins();
    let mut plugin_config = PluginManagerConfig::new(loader.config_home().to_path_buf());
    plugin_config.enabled_plugins = plugin_settings.enabled_plugins().clone();
    plugin_config.external_dirs = plugin_settings
        .external_directories()
        .iter()
        .map(|path| resolve_plugin_path(cwd, loader.config_home(), path))
        .collect();
    plugin_config.install_root = plugin_settings
        .install_root()
        .map(|path| resolve_plugin_path(cwd, loader.config_home(), path));
    plugin_config.registry_path = plugin_settings
        .registry_path()
        .map(|path| resolve_plugin_path(cwd, loader.config_home(), path));
    plugin_config.bundled_root = plugin_settings
        .bundled_root()
        .map(|path| resolve_plugin_path(cwd, loader.config_home(), path));
    PluginManager::new(plugin_config)
}

fn resolve_plugin_path(cwd: &Path, config_home: &Path, value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else if value.starts_with('.') {
        cwd.join(path)
    } else {
        config_home.join(path)
    }
}

fn runtime_hook_config_from_plugin_hooks(hooks: PluginHooks) -> runtime::RuntimeHookConfig {
    runtime::RuntimeHookConfig::new(
        hooks.pre_tool_use,
        hooks.post_tool_use,
        hooks.post_tool_use_failure,
    )
}

fn default_oauth_config() -> OAuthConfig {
    OAuthConfig {
        client_id: String::from("9d1c250a-e61b-44d9-88ed-5944d1962f5e"),
        authorize_url: String::from("https://platform.claude.com/oauth/authorize"),
        token_url: String::from("https://platform.claude.com/v1/oauth/token"),
        callback_port: None,
        manual_redirect_url: None,
        scopes: vec![
            String::from("user:profile"),
            String::from("user:inference"),
            String::from("user:sessions:claude_code"),
        ],
    }
}

fn read_auth_status() -> AppResult<AuthStatusResponse> {
    let env_api_key = env_non_empty("ANTHROPIC_API_KEY");
    let env_bearer_token = env_non_empty("ANTHROPIC_AUTH_TOKEN");
    let env_openai_api_key = env_non_empty("OPENAI_API_KEY");
    let env_xai_api_key = env_non_empty("XAI_API_KEY");
    let env_gemini_api_key = env_non_empty("GEMINI_API_KEY");
    let env_deepseek_api_key = env_non_empty("DEEPSEEK_API_KEY");
    let env_perplexity_api_key = env_non_empty("PERPLEXITY_API_KEY");
    let base_url = api::read_base_url();
    let saved_oauth = load_oauth_credentials().map_err(|error| {
        AppError::internal(format!("failed to inspect OAuth credentials: {error}"))
    })?;
    let warning = api::saved_oauth_direct_api_warning_for_base_url(&base_url)
        .map_err(|error| AppError::internal(format!("failed to inspect auth support: {error}")))?;
    let saved_oauth_expired = saved_oauth
        .as_ref()
        .and_then(|value| value.expires_at)
        .is_some_and(|expires_at| expires_at <= current_unix_timestamp());
    let anthropic_inference_ready = env_api_key
        || env_bearer_token
        || (saved_oauth.is_some() && !saved_oauth_expired && warning.is_none());
    let openai_inference_ready = env_openai_api_key;
    let xai_inference_ready = env_xai_api_key;
    let gemini_inference_ready = env_gemini_api_key;
    let deepseek_inference_ready = env_deepseek_api_key;
    let perplexity_inference_ready = env_perplexity_api_key;
    let active_source = match (
        env_api_key,
        env_bearer_token,
        env_openai_api_key,
        env_xai_api_key,
        env_gemini_api_key,
        env_deepseek_api_key,
        env_perplexity_api_key,
        saved_oauth.is_some(),
    ) {
        (true, true, _, _, _, _, _, _) => "api_key_and_bearer",
        (true, false, _, _, _, _, _, _) => "api_key",
        (false, true, _, _, _, _, _, _) => "bearer",
        (false, false, true, _, _, _, _, _) => "openai_api_key",
        (false, false, false, true, _, _, _, _) => "xai_api_key",
        (false, false, false, false, true, _, _, _) => "gemini_api_key",
        (false, false, false, false, false, true, _, _) => "deepseek_api_key",
        (false, false, false, false, false, false, true, _) => "perplexity_api_key",
        (false, false, false, false, false, false, false, true) => "oauth",
        (false, false, false, false, false, false, false, false) => "none",
    };

    Ok(AuthStatusResponse {
        authenticated: env_api_key
            || env_bearer_token
            || env_openai_api_key
            || env_xai_api_key
            || env_gemini_api_key
            || env_deepseek_api_key
            || env_perplexity_api_key
            || saved_oauth.is_some(),
        inference_ready: anthropic_inference_ready
            || openai_inference_ready
            || xai_inference_ready
            || gemini_inference_ready
            || deepseek_inference_ready
            || perplexity_inference_ready,
        active_source,
        env_api_key,
        env_bearer_token,
        env_openai_api_key,
        env_xai_api_key,
        env_gemini_api_key,
        env_deepseek_api_key,
        env_perplexity_api_key,
        anthropic_inference_ready,
        openai_inference_ready,
        xai_inference_ready,
        gemini_inference_ready,
        deepseek_inference_ready,
        perplexity_inference_ready,
        saved_oauth: saved_oauth.is_some(),
        saved_oauth_expired,
        expires_at: saved_oauth.as_ref().and_then(|value| value.expires_at),
        scopes: saved_oauth.map_or_else(Vec::new, |value| value.scopes),
        credentials_path: credentials_path()
            .ok()
            .map(|path| path.display().to_string()),
        warning,
    })
}

fn resolve_web_provider_client(
    cwd: &Path,
    model: &str,
    provider_selection: ProviderSelection,
) -> AppResult<ProviderClient> {
    let anthropic_auth = (provider_selection.resolve_kind(model) == ProviderKind::Anthropic)
        .then(|| resolve_web_auth_source(cwd))
        .transpose()?;
    ProviderClient::from_model_with_selection(model, provider_selection, anthropic_auth)
        .map_err(|error| AppError::unauthorized(error.to_string()))
}

fn resolve_web_auth_source(cwd: &Path) -> AppResult<AuthSource> {
    if let Some(message) = api::saved_oauth_direct_api_warning_for_base_url(&api::read_base_url())
        .map_err(|error| {
        AppError::internal(format!("failed to inspect auth support: {error}"))
    })? {
        return Err(AppError::unauthorized(message));
    }
    resolve_startup_auth_source(|| {
        let config = ConfigLoader::default_for(cwd).load().map_err(|error| {
            api::ApiError::Auth(format!("failed to load runtime OAuth config: {error}"))
        })?;
        Ok(Some(
            config.oauth().cloned().unwrap_or_else(default_oauth_config),
        ))
    })
    .map_err(|error| AppError::unauthorized(error.to_string()))
}

fn requested_permission_mode(
    runtime_config: &runtime::RuntimeConfig,
    value: Option<&str>,
) -> AppResult<PermissionMode> {
    if let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) {
        return parse_permission_mode(value);
    }

    Ok(runtime_config.permission_mode().map_or(
        PermissionMode::DangerFullAccess,
        permission_mode_from_resolved,
    ))
}

fn parse_permission_mode(value: &str) -> AppResult<PermissionMode> {
    match value {
        "read-only" => Ok(PermissionMode::ReadOnly),
        "workspace-write" => Ok(PermissionMode::WorkspaceWrite),
        "danger-full-access" => Ok(PermissionMode::DangerFullAccess),
        other => Err(AppError::bad_request(format!(
            "unsupported permission mode `{other}`. Use read-only, workspace-write, or danger-full-access."
        ))),
    }
}

fn permission_mode_from_resolved(mode: ResolvedPermissionMode) -> PermissionMode {
    match mode {
        ResolvedPermissionMode::ReadOnly => PermissionMode::ReadOnly,
        ResolvedPermissionMode::WorkspaceWrite => PermissionMode::WorkspaceWrite,
        ResolvedPermissionMode::DangerFullAccess => PermissionMode::DangerFullAccess,
    }
}

fn permission_policy(
    mode: PermissionMode,
    feature_config: &runtime::RuntimeFeatureConfig,
    tool_registry: &GlobalToolRegistry,
) -> Result<PermissionPolicy, String> {
    Ok(tool_registry.permission_specs(None)?.into_iter().fold(
        PermissionPolicy::new(mode).with_permission_rules(feature_config.permission_rules()),
        |policy, (name, required_permission)| {
            policy.with_tool_requirement(name, required_permission)
        },
    ))
}

fn response_to_events(
    response: MessageResponse,
    stream_sink: Option<&StreamEventSink>,
    model: Option<&str>,
) -> Vec<AssistantEvent> {
    let mut events = Vec::new();
    let mut pending_tool = None;

    for block in response.content {
        push_output_block(block, &mut events, &mut pending_tool, false, stream_sink);
        if let Some((id, name, input)) = pending_tool.take() {
            if let Some(stream_sink) = stream_sink {
                stream_sink.assistant_tool_use(&id, &name, &input);
            }
            events.push(AssistantEvent::ToolUse { id, name, input });
        }
    }

    let usage = response.usage.token_usage();
    if let Some(stream_sink) = stream_sink {
        stream_sink.assistant_usage(&usage_to_response(usage, model));
    }
    events.push(AssistantEvent::Usage(usage));
    events.push(AssistantEvent::MessageStop);
    events
}

fn push_output_block(
    block: OutputContentBlock,
    events: &mut Vec<AssistantEvent>,
    pending_tool: &mut Option<(String, String, String)>,
    streaming_tool_input: bool,
    stream_sink: Option<&StreamEventSink>,
) {
    match block {
        OutputContentBlock::Text { text } => {
            if !text.is_empty() {
                if let Some(stream_sink) = stream_sink {
                    stream_sink.assistant_text_delta(&text);
                }
                events.push(AssistantEvent::TextDelta(text));
            }
        }
        OutputContentBlock::ToolUse { id, name, input } => {
            let initial_input = if streaming_tool_input
                && input.is_object()
                && input.as_object().is_some_and(serde_json::Map::is_empty)
            {
                String::new()
            } else {
                input.to_string()
            };
            *pending_tool = Some((id, name, initial_input));
        }
        OutputContentBlock::Thinking { .. } | OutputContentBlock::RedactedThinking { .. } => {}
    }
}

fn push_prompt_cache_record(
    client: &ProviderClient,
    events: &mut Vec<AssistantEvent>,
    stream_sink: Option<&StreamEventSink>,
) {
    if let Some(record) = client.take_last_prompt_cache_record() {
        if let Some(cache_break) = record.cache_break {
            let event = PromptCacheEvent {
                unexpected: cache_break.unexpected,
                reason: cache_break.reason,
                previous_cache_read_input_tokens: cache_break.previous_cache_read_input_tokens,
                current_cache_read_input_tokens: cache_break.current_cache_read_input_tokens,
                token_drop: cache_break.token_drop,
            };
            if let Some(stream_sink) = stream_sink {
                stream_sink.prompt_cache(&PromptCacheEventResponse {
                    unexpected: event.unexpected,
                    reason: event.reason.clone(),
                    previous_cache_read_input_tokens: event.previous_cache_read_input_tokens,
                    current_cache_read_input_tokens: event.current_cache_read_input_tokens,
                    token_drop: event.token_drop,
                });
            }
            events.push(AssistantEvent::PromptCache(event));
        }
    }
}

fn convert_messages(messages: &[ConversationMessage]) -> Vec<InputMessage> {
    messages
        .iter()
        .filter_map(|message| {
            let role = match message.role {
                MessageRole::System | MessageRole::User | MessageRole::Tool => "user",
                MessageRole::Assistant => "assistant",
            };
            let content = message
                .blocks
                .iter()
                .map(|block| match block {
                    ContentBlock::Text { text } => InputContentBlock::Text { text: text.clone() },
                    ContentBlock::ToolUse { id, name, input } => InputContentBlock::ToolUse {
                        id: id.clone(),
                        name: name.clone(),
                        input: serde_json::from_str(input)
                            .unwrap_or_else(|_| json!({ "raw": input })),
                    },
                    ContentBlock::ToolResult {
                        tool_use_id,
                        output,
                        is_error,
                        ..
                    } => InputContentBlock::ToolResult {
                        tool_use_id: tool_use_id.clone(),
                        content: vec![ToolResultContentBlock::Text {
                            text: output.clone(),
                        }],
                        is_error: *is_error,
                    },
                })
                .collect::<Vec<_>>();
            (!content.is_empty()).then(|| InputMessage {
                role: role.to_string(),
                content,
            })
        })
        .collect()
}

fn final_assistant_text(summary: &runtime::TurnSummary) -> String {
    summary
        .assistant_messages
        .last()
        .map(|message| {
            message
                .blocks
                .iter()
                .filter_map(|block| match block {
                    ContentBlock::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default()
}

fn session_to_response(path: &Path, session: &Session) -> SessionResponse {
    let tracker = UsageTracker::from_session(session);
    SessionResponse {
        id: session.session_id.clone(),
        path: path.display().to_string(),
        message_count: session.messages.len(),
        turns: tracker.turns(),
        usage: usage_to_response(tracker.cumulative_usage(), None),
        compaction: session
            .compaction
            .as_ref()
            .map(|compaction| CompactionResponse {
                count: compaction.count,
                removed_message_count: compaction.removed_message_count,
                summary: compaction.summary.clone(),
            }),
        messages: session.messages.iter().map(message_to_response).collect(),
    }
}

fn message_to_response(message: &ConversationMessage) -> MessageResponseDto {
    let blocks = message
        .blocks
        .iter()
        .map(|block| match block {
            ContentBlock::Text { text } => MessageBlockResponse::Text { text: text.clone() },
            ContentBlock::ToolUse { id, name, input } => MessageBlockResponse::ToolUse {
                id: id.clone(),
                name: name.clone(),
                input: input.clone(),
            },
            ContentBlock::ToolResult {
                tool_use_id,
                tool_name,
                output,
                is_error,
            } => MessageBlockResponse::ToolResult {
                tool_use_id: tool_use_id.clone(),
                tool_name: tool_name.clone(),
                output: output.clone(),
                is_error: *is_error,
            },
        })
        .collect::<Vec<_>>();

    MessageResponseDto {
        role: match message.role {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool",
        },
        preview: message_preview(message),
        blocks,
        usage: message.usage.map(|usage| usage_to_response(usage, None)),
    }
}

fn message_preview(message: &ConversationMessage) -> String {
    let preview = message
        .blocks
        .iter()
        .map(|block| match block {
            ContentBlock::Text { text } => text.trim().to_string(),
            ContentBlock::ToolUse { name, .. } => format!("[tool use] {name}"),
            ContentBlock::ToolResult {
                tool_name,
                is_error,
                output,
                ..
            } => format!(
                "[tool result:{}] {} {}",
                if *is_error { "error" } else { "ok" },
                tool_name,
                output.lines().next().unwrap_or_default()
            ),
        })
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    truncate_for_preview(&preview, 160)
}

fn usage_to_response(usage: TokenUsage, model: Option<&str>) -> UsageResponse {
    let estimate = model.and_then(runtime::pricing_for_model).map_or_else(
        || usage.estimate_cost_usd(),
        |pricing| usage.estimate_cost_usd_with_pricing(pricing),
    );
    UsageResponse {
        input_tokens: usage.input_tokens,
        output_tokens: usage.output_tokens,
        cache_creation_input_tokens: usage.cache_creation_input_tokens,
        cache_read_input_tokens: usage.cache_read_input_tokens,
        total_tokens: usage.total_tokens(),
        estimated_cost_usd: format_usd(estimate.total_cost_usd()),
    }
}

fn truncate_for_preview(value: &str, max_chars: usize) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= max_chars {
        trimmed.to_string()
    } else {
        let truncated = trimmed.chars().take(max_chars).collect::<String>();
        format!("{truncated}...")
    }
}

#[allow(clippy::needless_pass_by_value)]
fn session_control_error(error: runtime::session_control::SessionControlError) -> AppError {
    AppError::bad_request(error.to_string())
}

#[allow(clippy::needless_pass_by_value)]
fn classify_runtime_error(error: RuntimeError) -> AppError {
    let message = error.to_string();
    if message.contains("401")
        || message.contains("authentication_error")
        || message.contains("missing credentials")
    {
        AppError::unauthorized(message)
    } else {
        AppError::internal(message)
    }
}

fn current_date_string() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

fn default_bind_host() -> &'static str {
    if runtime::detect_container_environment().in_container {
        DEFAULT_CONTAINER_HOST
    } else {
        DEFAULT_HOST
    }
}

fn current_unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn env_non_empty(key: &str) -> bool {
    env::var(key)
        .ok()
        .is_some_and(|value| !value.trim().is_empty())
}

fn auth_callback_page(success: bool, message: &str) -> String {
    let payload = serde_json::to_string(&json!({
        "type": "claw-auth-complete",
        "ok": success,
        "message": message,
    }))
    .unwrap_or_else(|_| "{\"type\":\"claw-auth-complete\",\"ok\":false}".to_string());
    let title = if success {
        "Claude OAuth Login Complete"
    } else {
        "Claude OAuth Login Failed"
    };
    let escaped_title = html_escape(title);
    let escaped_message = html_escape(message);
    format!(
        r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>{escaped_title}</title>
    <style>
      body {{
        margin: 0;
        font-family: "Avenir Next", "Segoe UI", sans-serif;
        background: linear-gradient(135deg, #f8f3e7, #efe2c1);
        color: #1b2c3d;
        min-height: 100vh;
        display: grid;
        place-items: center;
      }}
      .card {{
        max-width: 40rem;
        padding: 2rem;
        border-radius: 1.25rem;
        background: rgba(255, 255, 255, 0.88);
        box-shadow: 0 20px 40px rgba(27, 44, 61, 0.18);
      }}
      h1 {{
        margin-top: 0;
        font-family: "Iowan Old Style", "Palatino Linotype", serif;
      }}
      p {{
        line-height: 1.6;
      }}
    </style>
  </head>
  <body>
    <div class="card">
      <h1>{escaped_title}</h1>
      <p>{escaped_message}</p>
      <p>You can close this window and return to Claw Web.</p>
    </div>
    <script>
      const payload = {payload};
      if (window.opener && window.opener !== window) {{
        window.opener.postMessage(payload, "*");
        window.close();
      }}
    </script>
  </body>
</html>"#
    )
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
        LOCK.get_or_init(|| std::sync::Mutex::new(()))
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn temp_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "claw-web-test-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ))
    }

    fn pick_unused_port() -> u16 {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("bind port probe");
        listener.local_addr().expect("probe addr").port()
    }

    fn write_workspace_oauth_config(workspace_root: &Path, callback_port: u16) {
        let config_dir = workspace_root.join(".claw");
        fs::create_dir_all(&config_dir).expect("create workspace config dir");
        fs::write(
            config_dir.join("settings.local.json"),
            format!(
                r#"{{
  "oauth": {{
    "clientId": "runtime-client",
    "authorizeUrl": "https://console.test/oauth/authorize",
    "tokenUrl": "https://console.test/oauth/token",
    "callbackPort": {callback_port},
    "scopes": ["org:read", "user:write"]
  }}
}}"#
            ),
        )
        .expect("write workspace oauth config");
    }

    fn test_server_config(workspace_root: PathBuf) -> ServerConfig {
        ServerConfig {
            host: DEFAULT_HOST.to_string(),
            port: DEFAULT_PORT,
            workspace_root,
            public_base_url: Some("https://ignored.example.test".to_string()),
        }
    }

    #[test]
    fn auth_login_start_uses_loopback_redirect_uri() {
        let _guard = env_lock();
        let config_home = temp_dir("config-home");
        let workspace_root = temp_dir("workspace");
        fs::create_dir_all(&config_home).expect("create config home");
        fs::create_dir_all(&workspace_root).expect("create workspace root");

        let callback_port = pick_unused_port();
        write_workspace_oauth_config(&workspace_root, callback_port);
        std::env::set_var("CLAW_CONFIG_HOME", &config_home);

        let state = AppState::new(test_server_config(workspace_root.clone()));
        let runtime = tokio::runtime::Runtime::new().expect("create tokio runtime");
        let Json(response) = runtime
            .block_on(auth_login_start(State(state.clone())))
            .expect("start oauth login");

        assert_eq!(
            response.redirect_uri,
            format!("http://localhost:{callback_port}/callback")
        );
        assert!(response
            .authorize_url
            .contains("redirect_uri=http%3A%2F%2Flocalhost%3A"));
        assert!(!response.authorize_url.contains("/api/auth/callback"));

        let pending = state
            .shared
            .pending_oauth
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert_eq!(pending.len(), 1);
        let flow = pending.values().next().expect("pending oauth flow");
        assert_eq!(flow.redirect_uri, response.redirect_uri);

        std::env::remove_var("CLAW_CONFIG_HOME");
        fs::remove_dir_all(config_home).expect("cleanup config home");
        fs::remove_dir_all(workspace_root).expect("cleanup workspace root");
    }

    #[test]
    fn complete_web_oauth_callback_flow_saves_credentials() {
        let _guard = env_lock();
        let config_home = temp_dir("config-home");
        let workspace_root = temp_dir("workspace");
        fs::create_dir_all(&config_home).expect("create config home");
        fs::create_dir_all(&workspace_root).expect("create workspace root");
        std::env::set_var("CLAW_CONFIG_HOME", &config_home);

        let shared = SharedState {
            config: test_server_config(workspace_root.clone()),
            pending_oauth: Mutex::new(BTreeMap::from([(
                "state-123".to_string(),
                PendingOAuthFlow {
                    oauth: default_oauth_config(),
                    redirect_uri: runtime::loopback_redirect_uri(
                        runtime::DEFAULT_OAUTH_CALLBACK_PORT,
                    ),
                    verifier: "verifier-123".to_string(),
                },
            )])),
            oauth_listener: OnceLock::new(),
        };

        complete_web_oauth_callback_with_exchange(
            &shared,
            OAuthCallbackParams {
                code: Some("auth-code".to_string()),
                state: Some("state-123".to_string()),
                error: None,
                error_description: None,
            },
            |_oauth, _exchange_request| {
                Ok::<_, String>(runtime::OAuthTokenSet {
                    access_token: "access-token".to_string(),
                    refresh_token: Some("refresh-token".to_string()),
                    expires_at: Some(current_unix_timestamp() + 60),
                    scopes: vec!["user:profile".to_string()],
                })
            },
        )
        .expect("complete oauth callback");

        assert!(load_oauth_credentials()
            .expect("load saved oauth")
            .is_some());
        assert!(shared
            .pending_oauth
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_empty());

        std::env::remove_var("CLAW_CONFIG_HOME");
        fs::remove_dir_all(config_home).expect("cleanup config home");
        fs::remove_dir_all(workspace_root).expect("cleanup workspace root");
    }

    #[test]
    fn auth_status_marks_saved_oauth_as_not_inference_ready_for_direct_api() {
        let _guard = env_lock();
        let config_home = temp_dir("config-home");
        fs::create_dir_all(&config_home).expect("create config home");
        std::env::set_var("CLAW_CONFIG_HOME", &config_home);
        std::env::remove_var("ANTHROPIC_API_KEY");
        std::env::remove_var("ANTHROPIC_AUTH_TOKEN");
        std::env::remove_var("ANTHROPIC_BASE_URL");

        runtime::save_oauth_credentials(&runtime::OAuthTokenSet {
            access_token: "access-token".to_string(),
            refresh_token: Some("refresh-token".to_string()),
            expires_at: Some(current_unix_timestamp() + 60),
            scopes: vec!["user:profile".to_string()],
        })
        .expect("save oauth credentials");

        let status = read_auth_status().expect("read auth status");
        assert!(status.authenticated);
        assert!(!status.inference_ready);
        assert_eq!(status.active_source, "oauth");
        assert!(status.warning.is_some());

        clear_oauth_credentials().expect("clear credentials");
        std::env::remove_var("CLAW_CONFIG_HOME");
        fs::remove_dir_all(config_home).expect("cleanup config home");
    }

    #[tokio::test]
    async fn legacy_auth_callback_route_returns_loopback_guidance() {
        let Html(page) = auth_callback().await;
        assert!(page.contains("http://localhost:4545/callback"));
        assert!(page.contains("postMessage(payload, \"*\")"));
    }
}
