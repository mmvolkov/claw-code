use std::future::Future;
use std::pin::Pin;

use crate::error::ApiError;
use crate::types::{MessageRequest, MessageResponse};

pub mod anthropic;
pub mod openai_compat;

pub const DEFAULT_ANTHROPIC_MODEL: &str = "claude-opus-4-6";
pub const DEFAULT_XAI_MODEL: &str = "grok-3";
pub const DEFAULT_GEMINI_MODEL: &str = "gemini-3-flash-preview";
pub const DEFAULT_DEEPSEEK_MODEL: &str = "deepseek-chat";
pub const DEFAULT_PERPLEXITY_MODEL: &str = "sonar-pro";

#[allow(dead_code)]
pub type ProviderFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, ApiError>> + Send + 'a>>;

#[allow(dead_code)]
pub trait Provider {
    type Stream;

    fn send_message<'a>(
        &'a self,
        request: &'a MessageRequest,
    ) -> ProviderFuture<'a, MessageResponse>;

    fn stream_message<'a>(
        &'a self,
        request: &'a MessageRequest,
    ) -> ProviderFuture<'a, Self::Stream>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    Anthropic,
    Xai,
    OpenAi,
    Gemini,
    DeepSeek,
    Perplexity,
}

impl ProviderKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::Xai => "xai",
            Self::OpenAi => "openai-compatible",
            Self::Gemini => "gemini",
            Self::DeepSeek => "deepseek",
            Self::Perplexity => "perplexity",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderSelection {
    Auto,
    Anthropic,
    Xai,
    OpenAiCompatible,
    Gemini,
    DeepSeek,
    Perplexity,
}

impl ProviderSelection {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "anthropic" => Ok(Self::Anthropic),
            "xai" => Ok(Self::Xai),
            "openai" | "openai-compatible" => Ok(Self::OpenAiCompatible),
            "gemini" => Ok(Self::Gemini),
            "deepseek" => Ok(Self::DeepSeek),
            "perplexity" => Ok(Self::Perplexity),
            other => Err(format!(
                "unsupported value for --provider: {other} (expected auto, anthropic, openai-compatible, xai, gemini, deepseek, or perplexity)"
            )),
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Anthropic => "anthropic",
            Self::Xai => "xai",
            Self::OpenAiCompatible => "openai-compatible",
            Self::Gemini => "gemini",
            Self::DeepSeek => "deepseek",
            Self::Perplexity => "perplexity",
        }
    }

    #[must_use]
    pub fn resolve_kind(self, model: &str) -> ProviderKind {
        match self {
            Self::Auto => detect_provider_kind(model),
            Self::Anthropic => ProviderKind::Anthropic,
            Self::Xai => ProviderKind::Xai,
            Self::OpenAiCompatible => ProviderKind::OpenAi,
            Self::Gemini => ProviderKind::Gemini,
            Self::DeepSeek => ProviderKind::DeepSeek,
            Self::Perplexity => ProviderKind::Perplexity,
        }
    }
}

fn env_non_empty(name: &str) -> Option<String> {
    std::env::var(name).ok().and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

#[must_use]
pub fn default_model_for_provider_selection(selection: ProviderSelection) -> Option<String> {
    match selection {
        ProviderSelection::Auto => env_non_empty("CLAW_MODEL")
            .or_else(|| env_non_empty("ANTHROPIC_MODEL"))
            .or_else(|| Some(DEFAULT_ANTHROPIC_MODEL.to_string())),
        ProviderSelection::Anthropic => {
            env_non_empty("ANTHROPIC_MODEL").or_else(|| Some(DEFAULT_ANTHROPIC_MODEL.to_string()))
        }
        ProviderSelection::Xai => {
            env_non_empty("XAI_MODEL").or_else(|| Some(DEFAULT_XAI_MODEL.to_string()))
        }
        ProviderSelection::OpenAiCompatible => env_non_empty("OPENAI_MODEL"),
        ProviderSelection::Gemini => {
            env_non_empty("GEMINI_MODEL").or_else(|| Some(DEFAULT_GEMINI_MODEL.to_string()))
        }
        ProviderSelection::DeepSeek => {
            env_non_empty("DEEPSEEK_MODEL").or_else(|| Some(DEFAULT_DEEPSEEK_MODEL.to_string()))
        }
        ProviderSelection::Perplexity => {
            env_non_empty("PERPLEXITY_MODEL").or_else(|| Some(DEFAULT_PERPLEXITY_MODEL.to_string()))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderMetadata {
    pub provider: ProviderKind,
    pub auth_env: &'static str,
    pub base_url_env: &'static str,
    pub default_base_url: &'static str,
}

const MODEL_REGISTRY: &[(&str, ProviderMetadata)] = &[
    (
        "opus",
        ProviderMetadata {
            provider: ProviderKind::Anthropic,
            auth_env: "ANTHROPIC_API_KEY",
            base_url_env: "ANTHROPIC_BASE_URL",
            default_base_url: anthropic::DEFAULT_BASE_URL,
        },
    ),
    (
        "sonnet",
        ProviderMetadata {
            provider: ProviderKind::Anthropic,
            auth_env: "ANTHROPIC_API_KEY",
            base_url_env: "ANTHROPIC_BASE_URL",
            default_base_url: anthropic::DEFAULT_BASE_URL,
        },
    ),
    (
        "haiku",
        ProviderMetadata {
            provider: ProviderKind::Anthropic,
            auth_env: "ANTHROPIC_API_KEY",
            base_url_env: "ANTHROPIC_BASE_URL",
            default_base_url: anthropic::DEFAULT_BASE_URL,
        },
    ),
    (
        "grok",
        ProviderMetadata {
            provider: ProviderKind::Xai,
            auth_env: "XAI_API_KEY",
            base_url_env: "XAI_BASE_URL",
            default_base_url: openai_compat::DEFAULT_XAI_BASE_URL,
        },
    ),
    (
        "grok-3",
        ProviderMetadata {
            provider: ProviderKind::Xai,
            auth_env: "XAI_API_KEY",
            base_url_env: "XAI_BASE_URL",
            default_base_url: openai_compat::DEFAULT_XAI_BASE_URL,
        },
    ),
    (
        "grok-mini",
        ProviderMetadata {
            provider: ProviderKind::Xai,
            auth_env: "XAI_API_KEY",
            base_url_env: "XAI_BASE_URL",
            default_base_url: openai_compat::DEFAULT_XAI_BASE_URL,
        },
    ),
    (
        "grok-3-mini",
        ProviderMetadata {
            provider: ProviderKind::Xai,
            auth_env: "XAI_API_KEY",
            base_url_env: "XAI_BASE_URL",
            default_base_url: openai_compat::DEFAULT_XAI_BASE_URL,
        },
    ),
    (
        "grok-2",
        ProviderMetadata {
            provider: ProviderKind::Xai,
            auth_env: "XAI_API_KEY",
            base_url_env: "XAI_BASE_URL",
            default_base_url: openai_compat::DEFAULT_XAI_BASE_URL,
        },
    ),
    (
        "gemini",
        ProviderMetadata {
            provider: ProviderKind::Gemini,
            auth_env: "GEMINI_API_KEY",
            base_url_env: "GEMINI_BASE_URL",
            default_base_url: openai_compat::DEFAULT_GEMINI_BASE_URL,
        },
    ),
    (
        "deepseek",
        ProviderMetadata {
            provider: ProviderKind::DeepSeek,
            auth_env: "DEEPSEEK_API_KEY",
            base_url_env: "DEEPSEEK_BASE_URL",
            default_base_url: openai_compat::DEFAULT_DEEPSEEK_BASE_URL,
        },
    ),
    (
        "sonar",
        ProviderMetadata {
            provider: ProviderKind::Perplexity,
            auth_env: "PERPLEXITY_API_KEY",
            base_url_env: "PERPLEXITY_BASE_URL",
            default_base_url: openai_compat::DEFAULT_PERPLEXITY_BASE_URL,
        },
    ),
    (
        "perplexity",
        ProviderMetadata {
            provider: ProviderKind::Perplexity,
            auth_env: "PERPLEXITY_API_KEY",
            base_url_env: "PERPLEXITY_BASE_URL",
            default_base_url: openai_compat::DEFAULT_PERPLEXITY_BASE_URL,
        },
    ),
];

#[must_use]
pub fn resolve_model_alias(model: &str) -> String {
    let trimmed = model.trim();
    let lower = trimmed.to_ascii_lowercase();
    MODEL_REGISTRY
        .iter()
        .find_map(|(alias, metadata)| {
            (*alias == lower).then_some(match metadata.provider {
                ProviderKind::Anthropic => match *alias {
                    "opus" => "claude-opus-4-6",
                    "sonnet" => "claude-sonnet-4-6",
                    "haiku" => "claude-haiku-4-5-20251213",
                    _ => trimmed,
                },
                ProviderKind::Xai => match *alias {
                    "grok" | "grok-3" => "grok-3",
                    "grok-mini" | "grok-3-mini" => "grok-3-mini",
                    "grok-2" => "grok-2",
                    _ => trimmed,
                },
                ProviderKind::Gemini => DEFAULT_GEMINI_MODEL,
                ProviderKind::DeepSeek => DEFAULT_DEEPSEEK_MODEL,
                ProviderKind::Perplexity => DEFAULT_PERPLEXITY_MODEL,
                ProviderKind::OpenAi => trimmed,
            })
        })
        .map_or_else(|| trimmed.to_string(), ToOwned::to_owned)
}

#[must_use]
pub fn metadata_for_model(model: &str) -> Option<ProviderMetadata> {
    let canonical = resolve_model_alias(model);
    if canonical.starts_with("claude") {
        return Some(ProviderMetadata {
            provider: ProviderKind::Anthropic,
            auth_env: "ANTHROPIC_API_KEY",
            base_url_env: "ANTHROPIC_BASE_URL",
            default_base_url: anthropic::DEFAULT_BASE_URL,
        });
    }
    if canonical.starts_with("grok") {
        return Some(ProviderMetadata {
            provider: ProviderKind::Xai,
            auth_env: "XAI_API_KEY",
            base_url_env: "XAI_BASE_URL",
            default_base_url: openai_compat::DEFAULT_XAI_BASE_URL,
        });
    }
    if canonical.starts_with("gemini") {
        return Some(ProviderMetadata {
            provider: ProviderKind::Gemini,
            auth_env: "GEMINI_API_KEY",
            base_url_env: "GEMINI_BASE_URL",
            default_base_url: openai_compat::DEFAULT_GEMINI_BASE_URL,
        });
    }
    if canonical.starts_with("deepseek") {
        return Some(ProviderMetadata {
            provider: ProviderKind::DeepSeek,
            auth_env: "DEEPSEEK_API_KEY",
            base_url_env: "DEEPSEEK_BASE_URL",
            default_base_url: openai_compat::DEFAULT_DEEPSEEK_BASE_URL,
        });
    }
    if canonical.starts_with("sonar") {
        return Some(ProviderMetadata {
            provider: ProviderKind::Perplexity,
            auth_env: "PERPLEXITY_API_KEY",
            base_url_env: "PERPLEXITY_BASE_URL",
            default_base_url: openai_compat::DEFAULT_PERPLEXITY_BASE_URL,
        });
    }
    None
}

#[must_use]
pub fn detect_provider_kind(model: &str) -> ProviderKind {
    if let Some(metadata) = metadata_for_model(model) {
        return metadata.provider;
    }
    if anthropic::has_auth_from_env_or_saved().unwrap_or(false) {
        return ProviderKind::Anthropic;
    }
    if openai_compat::has_api_key("OPENAI_API_KEY") {
        return ProviderKind::OpenAi;
    }
    if openai_compat::has_api_key("XAI_API_KEY") {
        return ProviderKind::Xai;
    }
    if openai_compat::has_api_key("GEMINI_API_KEY") {
        return ProviderKind::Gemini;
    }
    if openai_compat::has_api_key("DEEPSEEK_API_KEY") {
        return ProviderKind::DeepSeek;
    }
    if openai_compat::has_api_key("PERPLEXITY_API_KEY") {
        return ProviderKind::Perplexity;
    }
    ProviderKind::Anthropic
}

#[must_use]
pub fn max_tokens_for_model(model: &str) -> u32 {
    let canonical = resolve_model_alias(model);
    if canonical.contains("opus") {
        32_000
    } else {
        64_000
    }
}

#[cfg(test)]
mod tests {
    use super::{
        default_model_for_provider_selection, detect_provider_kind, max_tokens_for_model,
        resolve_model_alias, ProviderKind, ProviderSelection, DEFAULT_DEEPSEEK_MODEL,
        DEFAULT_GEMINI_MODEL, DEFAULT_PERPLEXITY_MODEL,
    };
    use std::ffi::OsString;
    use std::sync::{Mutex, OnceLock};

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

    #[test]
    fn resolves_grok_aliases() {
        assert_eq!(resolve_model_alias("grok"), "grok-3");
        assert_eq!(resolve_model_alias("grok-mini"), "grok-3-mini");
        assert_eq!(resolve_model_alias("grok-2"), "grok-2");
    }

    #[test]
    fn detects_provider_from_model_name_first() {
        assert_eq!(detect_provider_kind("grok"), ProviderKind::Xai);
        assert_eq!(
            detect_provider_kind("claude-sonnet-4-6"),
            ProviderKind::Anthropic
        );
    }

    #[test]
    fn keeps_existing_max_token_heuristic() {
        assert_eq!(max_tokens_for_model("opus"), 32_000);
        assert_eq!(max_tokens_for_model("grok-3"), 64_000);
    }

    #[test]
    fn parses_provider_selection_variants() {
        assert_eq!(
            ProviderSelection::parse("openai-compatible").expect("openai-compatible should parse"),
            ProviderSelection::OpenAiCompatible
        );
        assert_eq!(
            ProviderSelection::parse("openai").expect("openai alias should parse"),
            ProviderSelection::OpenAiCompatible
        );
        assert_eq!(
            ProviderSelection::parse("xai").expect("xai should parse"),
            ProviderSelection::Xai
        );
        assert_eq!(
            ProviderSelection::parse("auto").expect("auto should parse"),
            ProviderSelection::Auto
        );
        assert_eq!(
            ProviderSelection::parse("gemini").expect("gemini should parse"),
            ProviderSelection::Gemini
        );
        assert_eq!(
            ProviderSelection::parse("deepseek").expect("deepseek should parse"),
            ProviderSelection::DeepSeek
        );
        assert_eq!(
            ProviderSelection::parse("perplexity").expect("perplexity should parse"),
            ProviderSelection::Perplexity
        );
    }

    #[test]
    fn openai_provider_default_model_reads_openai_model_env() {
        let _lock = env_lock();
        let _openai_model = EnvVarGuard::set("OPENAI_MODEL", Some("gpt-oss-120b"));
        assert_eq!(
            default_model_for_provider_selection(ProviderSelection::OpenAiCompatible).as_deref(),
            Some("gpt-oss-120b")
        );
    }

    #[test]
    fn provider_specific_defaults_use_env_and_builtin_values() {
        let _lock = env_lock();
        let _gemini_model = EnvVarGuard::set("GEMINI_MODEL", None);
        let _deepseek_model = EnvVarGuard::set("DEEPSEEK_MODEL", None);
        let _perplexity_model = EnvVarGuard::set("PERPLEXITY_MODEL", None);

        assert_eq!(
            default_model_for_provider_selection(ProviderSelection::Gemini).as_deref(),
            Some(DEFAULT_GEMINI_MODEL)
        );
        assert_eq!(
            default_model_for_provider_selection(ProviderSelection::DeepSeek).as_deref(),
            Some(DEFAULT_DEEPSEEK_MODEL)
        );
        assert_eq!(
            default_model_for_provider_selection(ProviderSelection::Perplexity).as_deref(),
            Some(DEFAULT_PERPLEXITY_MODEL)
        );
    }
}
