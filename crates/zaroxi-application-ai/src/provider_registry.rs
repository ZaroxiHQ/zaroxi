use std::sync::Mutex;
use zaroxi_domain_ai::provider::{AiProviderSettings, ProviderConfig, ProviderKind, known_models};

/// Trait for validating a provider connection.
pub trait ProviderConnectionTest {
    fn test_connection(&self) -> Result<bool, String>;
}

/// Thread-safe registry of AI provider configurations.
pub struct ProviderRegistry {
    settings: Mutex<AiProviderSettings>,
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self { settings: Mutex::new(AiProviderSettings::default()) }
    }

    pub fn get_active_provider(&self) -> ProviderKind {
        self.settings.lock().unwrap().active_provider
    }

    pub fn set_active_provider(&self, kind: ProviderKind) {
        self.settings.lock().unwrap().active_provider = kind;
    }

    pub fn get_config(&self, kind: ProviderKind) -> Option<ProviderConfig> {
        self.settings.lock().unwrap().get_provider(kind).cloned()
    }

    pub fn set_api_key(&self, kind: ProviderKind, key: Option<String>) {
        let mut settings = self.settings.lock().unwrap();
        if let Some(provider) = settings.get_provider_mut(kind) {
            provider.api_key = key;
            provider.resolve_auth();
        }
    }

    pub fn set_selected_model(&self, kind: ProviderKind, model: Option<String>) {
        let mut settings = self.settings.lock().unwrap();
        if let Some(provider) = settings.get_provider_mut(kind) {
            provider.selected_model = model;
        }
    }

    pub fn set_enabled(&self, kind: ProviderKind, enabled: bool) {
        let mut settings = self.settings.lock().unwrap();
        if let Some(provider) = settings.get_provider_mut(kind) {
            provider.enabled = enabled;
        }
    }

    pub fn test_connection(&self, kind: ProviderKind) -> Result<bool, String> {
        let _models = known_models(kind);
        Ok(true)
    }

    pub fn list_ready_providers(&self) -> Vec<ProviderConfig> {
        self.settings.lock().unwrap().providers.iter().filter(|c| c.is_ready()).cloned().collect()
    }

    pub fn disconnect_all(&self) {
        let mut settings = self.settings.lock().unwrap();
        for provider in &mut settings.providers {
            provider.connection_verified = false;
            provider.last_verified_at = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_default_active_provider() {
        let reg = ProviderRegistry::new();
        assert_eq!(reg.get_active_provider(), ProviderKind::OpenAI);
    }

    #[test]
    fn set_and_get_active_provider() {
        let reg = ProviderRegistry::new();
        reg.set_active_provider(ProviderKind::Anthropic);
        assert_eq!(reg.get_active_provider(), ProviderKind::Anthropic);
    }

    #[test]
    fn set_api_key_resolves_auth() {
        let reg = ProviderRegistry::new();
        reg.set_api_key(ProviderKind::OpenAI, Some("sk-test".into()));
        let cfg = reg.get_config(ProviderKind::OpenAI).unwrap();
        assert!(cfg.api_key.is_some());
        assert!(matches!(cfg.auth_method, zaroxi_domain_ai::provider::AuthMethod::ApiKey { .. }));
    }

    #[test]
    fn set_selected_model_persists() {
        let reg = ProviderRegistry::new();
        reg.set_selected_model(ProviderKind::OpenAI, Some("gpt-4o".into()));
        let cfg = reg.get_config(ProviderKind::OpenAI).unwrap();
        assert_eq!(cfg.selected_model, Some("gpt-4o".into()));
    }

    #[test]
    fn set_enabled_toggles() {
        let reg = ProviderRegistry::new();
        reg.set_enabled(ProviderKind::OpenAI, true);
        assert!(reg.get_config(ProviderKind::OpenAI).unwrap().enabled);
        reg.set_enabled(ProviderKind::OpenAI, false);
        assert!(!reg.get_config(ProviderKind::OpenAI).unwrap().enabled);
    }

    #[test]
    fn test_connection_stub() {
        let reg = ProviderRegistry::new();
        assert_eq!(reg.test_connection(ProviderKind::OpenAI), Ok(true));
    }

    #[test]
    fn list_ready_providers_none_by_default() {
        let reg = ProviderRegistry::new();
        assert!(reg.list_ready_providers().is_empty());
    }

    #[test]
    fn list_ready_providers_after_setup() {
        let reg = ProviderRegistry::new();
        reg.set_enabled(ProviderKind::OpenAI, true);
        reg.set_selected_model(ProviderKind::OpenAI, Some("gpt-4o".into()));
        reg.set_api_key(ProviderKind::OpenAI, Some("sk-test".into()));
        let mut cfg = reg.get_config(ProviderKind::OpenAI).unwrap();
        cfg.connection_verified = true;
        reg.settings
            .lock()
            .unwrap()
            .get_provider_mut(ProviderKind::OpenAI)
            .unwrap()
            .connection_verified = true;
        let ready = reg.list_ready_providers();
        assert_eq!(ready.len(), 1);
    }

    #[test]
    fn disconnect_all_resets_verification() {
        let reg = ProviderRegistry::new();
        reg.set_api_key(ProviderKind::OpenAI, Some("sk-test".into()));
        reg.settings
            .lock()
            .unwrap()
            .get_provider_mut(ProviderKind::OpenAI)
            .unwrap()
            .connection_verified = true;
        reg.disconnect_all();
        let cfg = reg.get_config(ProviderKind::OpenAI).unwrap();
        assert!(!cfg.connection_verified);
        assert!(cfg.last_verified_at.is_none());
    }
}
