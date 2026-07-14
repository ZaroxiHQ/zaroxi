//! AI provider abstraction — domain types for provider config, model metadata,
//! and authentication. These are pure data structures owned by the domain layer.
//!
//! Application crates implement provider connections; interface crates render
//! the config UI. No transport or rendering concerns live here.

use serde::{Deserialize, Serialize};

/// Supported AI backend providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProviderKind {
    OpenAI,
    Anthropic,
    Google,
    Xai,
    OpenRouter,
    /// User-configured local/compatible endpoint (Ollama, vLLM, etc.).
    Local,
    /// Catch-all for future providers; carries a display label.
    Custom,
}

impl ProviderKind {
    pub fn all() -> &'static [ProviderKind] {
        &[
            ProviderKind::OpenAI,
            ProviderKind::Anthropic,
            ProviderKind::Google,
            ProviderKind::Xai,
            ProviderKind::OpenRouter,
            ProviderKind::Local,
            ProviderKind::Custom,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ProviderKind::OpenAI => "OpenAI",
            ProviderKind::Anthropic => "Anthropic",
            ProviderKind::Google => "Google AI",
            ProviderKind::Xai => "xAI / Grok",
            ProviderKind::OpenRouter => "OpenRouter",
            ProviderKind::Local => "Local / Self-Hosted",
            ProviderKind::Custom => "Custom",
        }
    }

    /// Default API base URL when using the official endpoint.
    pub fn default_api_base(&self) -> Option<&'static str> {
        match self {
            ProviderKind::OpenAI => Some("https://api.openai.com/v1"),
            ProviderKind::Anthropic => Some("https://api.anthropic.com"),
            ProviderKind::Google => Some("https://generativelanguage.googleapis.com"),
            ProviderKind::Xai => Some("https://api.x.ai"),
            ProviderKind::OpenRouter => Some("https://openrouter.ai/api/v1"),
            ProviderKind::Local => None,
            ProviderKind::Custom => None,
        }
    }

    /// Account/API-key management page URL for browser-based onboarding.
    pub fn account_url(&self) -> Option<&'static str> {
        match self {
            ProviderKind::OpenAI => Some("https://platform.openai.com/api-keys"),
            ProviderKind::Anthropic => Some("https://console.anthropic.com/settings/keys"),
            ProviderKind::Google => Some("https://aistudio.google.com/app/apikey"),
            ProviderKind::Xai => Some("https://console.x.ai"),
            ProviderKind::OpenRouter => Some("https://openrouter.ai/keys"),
            ProviderKind::Local => None,
            ProviderKind::Custom => None,
        }
    }

    /// Environment variable name conventionally used for the API key.
    pub fn env_key_name(&self) -> Option<&'static str> {
        match self {
            ProviderKind::OpenAI => Some("OPENAI_API_KEY"),
            ProviderKind::Anthropic => Some("ANTHROPIC_API_KEY"),
            ProviderKind::Google => Some("GOOGLE_API_KEY"),
            ProviderKind::Xai => Some("XAI_API_KEY"),
            ProviderKind::OpenRouter => Some("OPENROUTER_API_KEY"),
            ProviderKind::Local => None,
            ProviderKind::Custom => None,
        }
    }
}

/// Authentication method for a provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthMethod {
    /// No auth needed (local providers).
    None,
    /// Bearer-token / API-key based auth.
    ApiKey {
        /// Where the key lives: env var name or inline (from settings).
        source: ApiKeySource,
    },
    /// OAuth 2.0 token (future phase).
    OAuth2 { token_url: String, client_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiKeySource {
    /// Read from an environment variable.
    EnvVar(String),
    /// Stored in local config (with appropriate warnings).
    ConfigStored,
}

/// Provider-level configuration stored per-provider.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Which provider this config applies to.
    pub kind: ProviderKind,
    /// Whether the provider is enabled.
    pub enabled: bool,
    /// Override the default API base URL.
    pub api_base: Option<String>,
    /// Per-provider API key (if stored in config — prefer env vars).
    pub api_key: Option<String>,
    /// Auth method derived from the above.
    pub auth_method: AuthMethod,
    /// Selected model identifier.
    pub selected_model: Option<String>,
    /// Whether the provider has been tested/validated.
    pub connection_verified: bool,
    /// Timestamp of last successful connection test (epoch millis).
    pub last_verified_at: Option<u64>,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            kind: ProviderKind::OpenAI,
            enabled: false,
            api_base: None,
            api_key: None,
            auth_method: AuthMethod::None,
            selected_model: None,
            connection_verified: false,
            last_verified_at: None,
        }
    }
}

impl ProviderConfig {
    pub fn for_kind(kind: ProviderKind) -> Self {
        Self { kind, ..Default::default() }
    }

    /// Resolve the effective API base URL.
    pub fn effective_api_base(&self) -> Option<String> {
        self.api_base.clone().or_else(|| self.kind.default_api_base().map(String::from))
    }

    /// Resolve the effective auth method.
    pub fn resolve_auth(&mut self) {
        if let Some(env_name) = self.kind.env_key_name()
            && std::env::var(env_name).is_ok()
        {
            self.auth_method =
                AuthMethod::ApiKey { source: ApiKeySource::EnvVar(env_name.to_string()) };
            return;
        }
        if self.api_key.is_some() {
            self.auth_method = AuthMethod::ApiKey { source: ApiKeySource::ConfigStored };
        } else {
            self.auth_method = AuthMethod::None;
        }
    }

    /// Whether this provider is ready to make API calls.
    pub fn is_ready(&self) -> bool {
        self.enabled
            && self.selected_model.is_some()
            && matches!(self.auth_method, AuthMethod::ApiKey { .. })
            && self.connection_verified
    }

    /// Mask the API key for display (shows last 4 chars).
    pub fn masked_key(&self) -> Option<String> {
        self.api_key.as_ref().map(|k| {
            if k.len() <= 4 { "****".to_string() } else { format!("****{}", &k[k.len() - 4..]) }
        })
    }
}

/// Model metadata surfaced for the model picker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelMetadata {
    pub provider: ProviderKind,
    pub model_id: String,
    pub display_name: String,
    pub context_window: Option<usize>,
    pub supports_streaming: bool,
    pub supports_tools: bool,
    pub supports_vision: bool,
}

/// Well-known models per provider for phase-1 scaffolding.
pub fn known_models(provider: ProviderKind) -> Vec<ModelMetadata> {
    match provider {
        ProviderKind::OpenAI => vec![
            ModelMetadata {
                provider,
                model_id: "gpt-4o".into(),
                display_name: "GPT-4o".into(),
                context_window: Some(128_000),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
            ModelMetadata {
                provider,
                model_id: "gpt-4o-mini".into(),
                display_name: "GPT-4o Mini".into(),
                context_window: Some(128_000),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
        ],
        ProviderKind::Anthropic => vec![ModelMetadata {
            provider,
            model_id: "claude-sonnet-4-20250514".into(),
            display_name: "Claude Sonnet 4".into(),
            context_window: Some(200_000),
            supports_streaming: true,
            supports_tools: true,
            supports_vision: true,
        }],
        ProviderKind::Google => vec![ModelMetadata {
            provider,
            model_id: "gemini-2.5-flash".into(),
            display_name: "Gemini 2.5 Flash".into(),
            context_window: Some(1_000_000),
            supports_streaming: true,
            supports_tools: true,
            supports_vision: true,
        }],
        ProviderKind::Xai => vec![ModelMetadata {
            provider,
            model_id: "grok-3-beta".into(),
            display_name: "Grok 3 Beta".into(),
            context_window: Some(1_000_000),
            supports_streaming: true,
            supports_tools: false,
            supports_vision: false,
        }],
        ProviderKind::OpenRouter => vec![ModelMetadata {
            provider,
            model_id: "openai/gpt-4o".into(),
            display_name: "GPT-4o (via OpenRouter)".into(),
            context_window: Some(128_000),
            supports_streaming: true,
            supports_tools: true,
            supports_vision: true,
        }],
        ProviderKind::Local => vec![ModelMetadata {
            provider,
            model_id: "local-model".into(),
            display_name: "Local Model".into(),
            context_window: None,
            supports_streaming: true,
            supports_tools: false,
            supports_vision: false,
        }],
        ProviderKind::Custom => vec![],
    }
}

/// Aggregate AI provider settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiProviderSettings {
    /// Per-provider configuration entries.
    pub providers: Vec<ProviderConfig>,
    /// Which provider is currently active.
    pub active_provider: ProviderKind,
}

impl Default for AiProviderSettings {
    fn default() -> Self {
        Self {
            providers: ProviderKind::all().iter().map(|&k| ProviderConfig::for_kind(k)).collect(),
            active_provider: ProviderKind::OpenAI,
        }
    }
}

impl AiProviderSettings {
    pub fn get_provider(&self, kind: ProviderKind) -> Option<&ProviderConfig> {
        self.providers.iter().find(|p| p.kind == kind)
    }

    pub fn get_provider_mut(&mut self, kind: ProviderKind) -> Option<&mut ProviderConfig> {
        self.providers.iter_mut().find(|p| p.kind == kind)
    }

    pub fn active_config(&self) -> Option<&ProviderConfig> {
        self.get_provider(self.active_provider)
    }

    /// The first ready provider, or the active provider even if not ready.
    pub fn effective_provider(&self) -> Option<&ProviderConfig> {
        self.active_config()
            .filter(|c| c.is_ready())
            .or_else(|| self.providers.iter().find(|c| c.is_ready()))
    }

    pub fn has_any_ready(&self) -> bool {
        self.providers.iter().any(|c| c.is_ready())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_kind_all_covers_every_variant() {
        assert_eq!(ProviderKind::all().len(), 7);
    }

    #[test]
    fn default_config_is_not_ready() {
        let cfg = ProviderConfig::default();
        assert!(!cfg.is_ready());
    }

    #[test]
    fn ready_provider_requires_enabled_model_and_auth() {
        let mut cfg = ProviderConfig {
            kind: ProviderKind::OpenAI,
            enabled: true,
            selected_model: Some("gpt-4o".into()),
            api_key: Some("sk-test".into()),
            connection_verified: true,
            ..Default::default()
        };
        cfg.resolve_auth();
        assert!(cfg.is_ready());
    }

    #[test]
    fn masked_key_hides_all_but_last_4() {
        let cfg =
            ProviderConfig { api_key: Some("sk-1234567890abcdef".into()), ..Default::default() };
        assert_eq!(cfg.masked_key(), Some("****cdef".into()));
    }

    #[test]
    fn short_key_is_fully_masked() {
        let cfg = ProviderConfig { api_key: Some("abc".into()), ..Default::default() };
        assert_eq!(cfg.masked_key(), Some("****".into()));
    }

    #[test]
    fn resolve_auth_from_config_key() {
        let mut cfg = ProviderConfig { kind: ProviderKind::OpenAI, ..Default::default() };
        cfg.resolve_auth();
        assert!(
            matches!(cfg.auth_method, AuthMethod::None),
            "expected None, got {:?}",
            cfg.auth_method
        );

        // With API key set directly
        let mut cfg2 = ProviderConfig {
            kind: ProviderKind::OpenAI,
            api_key: Some("sk-test-key".to_string()),
            ..Default::default()
        };
        cfg2.resolve_auth();
        let is_api_key = matches!(&cfg2.auth_method, AuthMethod::ApiKey { .. });
        assert!(is_api_key, "expected ApiKey, got {:?}", cfg2.auth_method);
        assert_eq!(cfg2.masked_key(), Some("****-key".to_string()));
    }

    #[test]
    fn ai_provider_settings_default_has_all_providers() {
        let s = AiProviderSettings::default();
        assert_eq!(s.providers.len(), 7);
        assert_eq!(s.active_provider, ProviderKind::OpenAI);
    }

    #[test]
    fn known_models_returns_models_for_each_provider() {
        for &k in ProviderKind::all() {
            let models = known_models(k);
            if k == ProviderKind::Custom {
                assert!(models.is_empty());
            } else {
                assert!(!models.is_empty(), "no models for {k:?}");
            }
        }
    }
}
