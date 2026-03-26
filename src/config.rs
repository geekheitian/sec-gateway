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

impl From<ProviderKind> for String {
    fn from(kind: ProviderKind) -> Self {
        match kind {
            ProviderKind::OpenAI => "openai".to_string(),
            ProviderKind::Anthropic => "anthropic".to_string(),
            ProviderKind::Gemini => "gemini".to_string(),
        }
    }
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
    #[serde(default)]
    pub per_type_masking_strategies: std::collections::HashMap<String, String>,
    pub detectors: std::collections::HashMap<String, DetectorConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectorConfig {
    pub pattern: String,
    pub confidence_threshold: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub tls: TlsConfig,
    pub auth: AuthConfig,
    pub proxy: ProxyConfig,
    pub rate_limit: RateLimitConfig,
    pub cors: CorsConfig,
    pub audit: AuditConfig,
    pub metrics: MetricsConfig,
    pub session: SessionConfig,
    pub key_rotation: KeyRotationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyRotationConfig {
    pub enabled: bool,
    #[serde(default = "default_rotation_interval_days")]
    pub interval_days: u64,
    pub auto_rotate: bool,
}

fn default_rotation_interval_days() -> u64 {
    90
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub trust_forwarded_headers: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub enabled: bool,
    pub cert_path: Option<String>,
    pub key_path: Option<String>,
    #[serde(default)]
    pub require_forwarded_https: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub enabled: bool,
    pub bearer_token: Option<String>,
    pub header_name: Option<String>,
    pub header_value: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    pub enabled: bool,
    pub log_headers: bool,
    pub file_path: Option<String>,
    #[serde(default = "default_max_file_size_mb")]
    pub max_file_size_mb: u64,
    #[serde(default = "default_rotation_strategy")]
    pub rotation_strategy: String,
}

fn default_max_file_size_mb() -> u64 {
    100
}

fn default_rotation_strategy() -> String {
    "daily".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub cleanup_interval_seconds: u64,
    pub ttl_seconds: u64,
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
        if let Ok(token) = std::env::var("API_AUTH_TOKEN") {
            self.security.auth.bearer_token = Some(token);
            self.security.auth.enabled = true;
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
        assert!(!config.security.proxy.trust_forwarded_headers);
        assert!(config.security.metrics.enabled);
        assert_eq!(config.security.metrics.path, "/metrics");
        assert_eq!(config.security.cors.allowed_origins.len(), 2);
        assert!(config.security.audit.enabled);
        assert_eq!(config.security.session.ttl_seconds, 1800);
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

    #[test]
    fn test_per_type_masking_strategies_loaded() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("SERVER_PORT");
        std::env::remove_var("SERVER_HOST");
        std::env::remove_var("TARGET_URL");
        std::env::remove_var("PROVIDER_KIND");
        std::env::remove_var("PROVIDER_MODEL");
        std::env::remove_var("PROVIDER_TARGET_URL");
        std::env::remove_var("RUST_LOG");

        let config = Config::load_default().unwrap();

        assert_eq!(
            config.pii.per_type_masking_strategies.get("credit_card"),
            Some(&"hash".to_string())
        );
        assert_eq!(
            config.pii.per_type_masking_strategies.get("jwt"),
            Some(&"hash".to_string())
        );
        assert_eq!(
            config.pii.per_type_masking_strategies.get("email"),
            Some(&"replace".to_string())
        );
        assert_eq!(
            config.pii.per_type_masking_strategies.get("ip_address"),
            Some(&"replace".to_string())
        );
        assert_eq!(
            config
                .pii
                .per_type_masking_strategies
                .get("database_connection_string"),
            Some(&"hash".to_string())
        );
    }
}
