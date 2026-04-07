use std::ffi::OsString;
use std::sync::{Mutex, OnceLock};

use api::{
    read_deepseek_base_url, read_gemini_base_url, read_openai_base_url, read_perplexity_base_url,
    read_xai_base_url, ApiError, AuthSource, ProviderClient, ProviderKind, ProviderSelection,
};

#[test]
fn provider_client_routes_grok_aliases_through_xai() {
    let _lock = env_lock();
    let _xai_api_key = EnvVarGuard::set("XAI_API_KEY", Some("xai-test-key"));

    let client = ProviderClient::from_model("grok-mini").expect("grok alias should resolve");

    assert_eq!(client.provider_kind(), ProviderKind::Xai);
}

#[test]
fn provider_client_uses_explicit_openai_selection_for_custom_models() {
    let _lock = env_lock();
    let _anthropic_api_key = EnvVarGuard::set("ANTHROPIC_API_KEY", None);
    let _anthropic_auth_token = EnvVarGuard::set("ANTHROPIC_AUTH_TOKEN", None);
    let _openai_api_key = EnvVarGuard::set("OPENAI_API_KEY", Some("openai-test-key"));

    let client = ProviderClient::from_model_with_selection(
        "local-qwen",
        ProviderSelection::OpenAiCompatible,
        None,
    )
    .expect("explicit OpenAI-compatible selection should build a client");

    assert_eq!(client.provider_kind(), ProviderKind::OpenAi);
}

#[test]
fn provider_client_reports_missing_xai_credentials_for_grok_models() {
    let _lock = env_lock();
    let _xai_api_key = EnvVarGuard::set("XAI_API_KEY", None);

    let error = ProviderClient::from_model("grok-3")
        .expect_err("grok requests without XAI_API_KEY should fail fast");

    match error {
        ApiError::MissingCredentials { provider, env_vars } => {
            assert_eq!(provider, "xAI");
            assert_eq!(env_vars, &["XAI_API_KEY"]);
        }
        other => panic!("expected missing xAI credentials, got {other:?}"),
    }
}

#[test]
fn provider_client_uses_explicit_anthropic_auth_without_env_lookup() {
    let _lock = env_lock();
    let _anthropic_api_key = EnvVarGuard::set("ANTHROPIC_API_KEY", None);
    let _anthropic_auth_token = EnvVarGuard::set("ANTHROPIC_AUTH_TOKEN", None);

    let client = ProviderClient::from_model_with_anthropic_auth(
        "claude-sonnet-4-6",
        Some(AuthSource::ApiKey("anthropic-test-key".to_string())),
    )
    .expect("explicit anthropic auth should avoid env lookup");

    assert_eq!(client.provider_kind(), ProviderKind::Anthropic);
}

#[test]
fn read_xai_base_url_prefers_env_override() {
    let _lock = env_lock();
    let _xai_base_url = EnvVarGuard::set("XAI_BASE_URL", Some("https://example.xai.test/v1"));

    assert_eq!(read_xai_base_url(), "https://example.xai.test/v1");
}

#[test]
fn read_openai_base_url_prefers_env_override() {
    let _lock = env_lock();
    let _openai_base_url =
        EnvVarGuard::set("OPENAI_BASE_URL", Some("https://example-openai.local/v1"));

    assert_eq!(read_openai_base_url(), "https://example-openai.local/v1");
}

#[test]
fn explicit_provider_selection_supports_gemini_deepseek_and_perplexity() {
    let _lock = env_lock();
    let _gemini_api_key = EnvVarGuard::set("GEMINI_API_KEY", Some("gemini-test-key"));
    let _deepseek_api_key = EnvVarGuard::set("DEEPSEEK_API_KEY", Some("deepseek-test-key"));
    let _perplexity_api_key = EnvVarGuard::set("PERPLEXITY_API_KEY", Some("perplexity-test-key"));

    let gemini = ProviderClient::from_model_with_selection(
        "gemini-3-flash-preview",
        ProviderSelection::Gemini,
        None,
    )
    .expect("gemini provider should build");
    let deepseek = ProviderClient::from_model_with_selection(
        "deepseek-chat",
        ProviderSelection::DeepSeek,
        None,
    )
    .expect("deepseek provider should build");
    let perplexity =
        ProviderClient::from_model_with_selection("sonar-pro", ProviderSelection::Perplexity, None)
            .expect("perplexity provider should build");

    assert_eq!(gemini.provider_kind(), ProviderKind::Gemini);
    assert_eq!(deepseek.provider_kind(), ProviderKind::DeepSeek);
    assert_eq!(perplexity.provider_kind(), ProviderKind::Perplexity);
}

#[test]
fn provider_specific_base_urls_prefer_env_override() {
    let _lock = env_lock();
    let _gemini_base_url = EnvVarGuard::set(
        "GEMINI_BASE_URL",
        Some("https://example-gemini.local/openai"),
    );
    let _deepseek_base_url =
        EnvVarGuard::set("DEEPSEEK_BASE_URL", Some("https://example-deepseek.local"));
    let _perplexity_base_url = EnvVarGuard::set(
        "PERPLEXITY_BASE_URL",
        Some("https://example-perplexity.local"),
    );

    assert_eq!(
        read_gemini_base_url(),
        "https://example-gemini.local/openai"
    );
    assert_eq!(read_deepseek_base_url(), "https://example-deepseek.local");
    assert_eq!(
        read_perplexity_base_url(),
        "https://example-perplexity.local"
    );
}

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

struct EnvVarGuard {
    key: &'static str,
    original: Option<OsString>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: Option<&str>) -> Self {
        let original = std::env::var_os(key);
        match value {
            Some(value) => std::env::set_var(key, value),
            None => std::env::remove_var(key),
        }
        Self { key, original }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(value) => std::env::set_var(self.key, value),
            None => std::env::remove_var(self.key),
        }
    }
}
