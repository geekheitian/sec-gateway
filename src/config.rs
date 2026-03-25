use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub provider: ProviderConfig,
    pub pii: PiiConfig,
    pub security: SecurityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProviderKind {
    #[serde(rename = "openai")]
    OpenAI,
    Anthropic,
    Gemini,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub kind: ProviderKind,
    pub model: String,
    pub target_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiiConfig {
    pub types: Vec<String>,
    pub masking_strategy: String,
    pub detectors: std::collections::HashMap<String, DetectorConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectorConfig {
    pub pattern: String,
    pub confidence_threshold: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub rate_limit: RateLimitConfig,
    pub cors: CorsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub burst_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    pub enabled: bool,
    pub allowed_origins: Vec<String>,
}

impl Config {
    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let mut config: Config = serde_yaml::from_str(&content)?;
        config.apply_env_overrides();
        Ok(config)
    }

    pub fn load_default() -> Result<Self, Box<dyn std::error::Error>> {
        Self::load_from_file("config/default.yaml")
    }

    fn apply_env_overrides(&mut self) {
        if let Ok(host) = std::env::var("SERVER_HOST") {
            self.server.host = host;
        }
        if let Ok(port) = std::env::var("SERVER_PORT") {
            if let Ok(port_num) = port.parse() {
                self.server.port = port_num;
            }
        }
        if let Ok(target) = std::env::var("TARGET_URL") {
            self.provider.target_url = target;
        }
        if let Ok(kind) = std::env::var("PROVIDER_KIND") {
            self.provider.kind = match kind.to_lowercase().as_str() {
                "openai" => ProviderKind::OpenAI,
                "anthropic" => ProviderKind::Anthropic,
                "gemini" => ProviderKind::Gemini,
                _ => self.provider.kind.clone(),
            };
        }
        if let Ok(model) = std::env::var("PROVIDER_MODEL") {
            self.provider.model = model;
        }
        if let Ok(target) = std::env::var("PROVIDER_TARGET_URL") {
            self.provider.target_url = target;
        }
        if let Ok(log_level) = std::env::var("RUST_LOG") {
            self.server.log_level = log_level;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_load_default_config() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        std::env::remove_var("SERVER_PORT");
        std::env::remove_var("SERVER_HOST");
        std::env::remove_var("TARGET_URL");
        std::env::remove_var("PROVIDER_KIND");
        std::env::remove_var("PROVIDER_MODEL");
        std::env::remove_var("PROVIDER_TARGET_URL");
        std::env::remove_var("RUST_LOG");

        let config = Config::load_default();
        assert!(config.is_ok());

        let config = config.unwrap();
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 8080);
        assert!(config.pii.types.contains(&"chinese_id".to_string()));
        assert_eq!(config.provider.kind, ProviderKind::OpenAI);
    }

    #[test]
    fn test_env_override() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        std::env::remove_var("SERVER_PORT");
        std::env::remove_var("TARGET_URL");
        std::env::remove_var("PROVIDER_KIND");
        std::env::remove_var("PROVIDER_MODEL");
        std::env::remove_var("PROVIDER_TARGET_URL");

        std::env::set_var("SERVER_PORT", "9090");
        std::env::set_var("TARGET_URL", "http://test.example.com");
        std::env::set_var("PROVIDER_KIND", "anthropic");
        std::env::set_var("PROVIDER_MODEL", "claude-3-5-sonnet");
        std::env::set_var("PROVIDER_TARGET_URL", "http://provider.example.com");

        let config = Config::load_default().unwrap();

        assert_eq!(config.server.port, 9090);
        assert_eq!(config.provider.kind, ProviderKind::Anthropic);
        assert_eq!(config.provider.model, "claude-3-5-sonnet");
        assert_eq!(config.provider.target_url, "http://provider.example.com");

        std::env::remove_var("SERVER_PORT");
        std::env::remove_var("TARGET_URL");
        std::env::remove_var("PROVIDER_KIND");
        std::env::remove_var("PROVIDER_MODEL");
        std::env::remove_var("PROVIDER_TARGET_URL");
    }
}
