mod app_state;
mod audit;
mod config;
mod crypto;
mod detector;
mod handlers;
mod masker;
mod middleware;
mod provider;
mod vault;

use axum::{
    routing::{any, delete, get},
    Router,
};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use app_state::{AppState, MetricsState};
use audit::{FileAppender, RotationStrategy};
use config::Config;
use provider::ProviderFactory;
use vault::{PrivacyVault, Reverser};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sec_gateway=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::load_default().unwrap_or_else(|e| {
        tracing::warn!("Failed to load config file: {}, using defaults", e);
        Config {
            server: config::ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                log_level: "debug".to_string(),
            },
            provider: config::ProviderConfig {
                kind: config::ProviderKind::OpenAI,
                model: "gpt-3.5-turbo".to_string(),
                target_url: std::env::var("TARGET_URL")
                    .unwrap_or_else(|_| "https://api.openai.com/v1/chat/completions".to_string()),
            },
            crypto: config::CryptoConfig {
                fpe: config::FpeConfig {
                    backend: "aes".to_string(),
                    radix: 10,
                },
            },
            pii: config::PiiConfig {
                types: vec!["chinese_id".to_string()],
                masking_strategy: "replace".to_string(),
                per_type_masking_strategies: std::collections::HashMap::new(),
                detectors: std::collections::HashMap::new(),
            },
            security: config::SecurityConfig {
                tls: config::TlsConfig {
                    enabled: false,
                    cert_path: None,
                    key_path: None,
                    require_forwarded_https: false,
                },
                auth: config::AuthConfig {
                    enabled: false,
                    bearer_token: None,
                    header_name: None,
                    header_value: None,
                },
                proxy: config::ProxyConfig {
                    trust_forwarded_headers: false,
                },
                rate_limit: config::RateLimitConfig {
                    requests_per_minute: 60,
                    burst_size: 10,
                },
                cors: config::CorsConfig {
                    enabled: true,
                    allowed_origins: vec!["http://localhost:3000".to_string()],
                },
                audit: config::AuditConfig {
                    enabled: true,
                    log_headers: false,
                    file_path: None,
                    max_file_size_mb: 100,
                    rotation_strategy: "daily".to_string(),
                },
                metrics: config::MetricsConfig {
                    enabled: true,
                    path: "/metrics".to_string(),
                },
                session: config::SessionConfig {
                    cleanup_interval_seconds: 300,
                    ttl_seconds: 1800,
                },
                key_rotation: config::KeyRotationConfig {
                    enabled: false,
                    interval_days: 90,
                    auto_rotate: false,
                },
            },
        }
    });

    tracing::info!("Target URL: {}", config.provider.target_url);
    tracing::info!("PII types enabled: {:?}", config.pii.types);
    tracing::info!("FPE backend: {}", config.crypto.fpe.backend);

    fn get_or_generate_key<const N: usize>(env_var: &str, key_type: &str) -> [u8; N] {
        match std::env::var(env_var) {
            Ok(key_str) => {
                let key_bytes = key_str.as_bytes();
                if key_bytes.len() != N * 2 {
                    panic!("{} must be {} hex characters ({} bytes)", env_var, N * 2, N);
                }
                let mut key = [0u8; N];
                for (i, chunk) in key_bytes.chunks(2).enumerate() {
                    key[i] = u8::from_str_radix(std::str::from_utf8(chunk).unwrap(), 16)
                        .expect("Key must be hex encoded");
                }
                key
            }
            Err(_) => {
                use rand::{rngs::OsRng, RngCore};
                
                // Use cryptographically secure random source
                let mut key = [0u8; N];
                OsRng.fill_bytes(&mut key);
                
                let hex_key = key.iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<String>();
                
                // Write key to local file (secure alternative to logging)
                let key_file = format!(".sec-gateway-{}.key", key_type.to_lowercase());
                match std::fs::write(&key_file, &hex_key) {
                    Ok(_) => {
                        // Set file permissions to 600 (owner read/write only)
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            let _ = std::fs::set_permissions(&key_file, std::fs::Permissions::from_mode(0o600));
                        }
                        tracing::warn!("{} not set, auto-generated and saved to: {}", env_var, key_file);
                        tracing::warn!("IMPORTANT: Save this key file securely! Set {}={} or keep the .key file", env_var, hex_key);
                    }
                    Err(e) => {
                        tracing::error!("Failed to write key file {}: {}", key_file, e);
                        tracing::warn!("{} not set, auto-generated. Set {}={}", env_var, env_var, hex_key);
                        tracing::warn!("IMPORTANT: Save this key! It cannot be recovered if lost.");
                    }
                }
                
                key
            }
        }
    }

    let (fpe_cipher, vault_key): (crypto::fpe_trait::DynFpeBackend, [u8; 32]) = match config.crypto.fpe.backend.as_str() {
        "sm4" => {
            let sm4_key = get_or_generate_key::<16>("SM4_FPE_KEY", "SM4");
            use crypto::fpe_trait::create_sm4_backend;
            let cipher = create_sm4_backend(&sm4_key, config.crypto.fpe.radix)
                .expect("Failed to initialize SM4-FF1 backend");
            let mut vault_key = [0u8; 32];
            vault_key[..16].copy_from_slice(&sm4_key);
            vault_key[16..].copy_from_slice(&sm4_key);
            (cipher, vault_key)
        }
        "aes" | _ => {
            let aes_key = get_or_generate_key::<32>("FPE_KEY", "AES");
            use crypto::fpe_trait::create_aes_backend;
            let cipher = create_aes_backend(&aes_key, config.crypto.fpe.radix)
                .expect("Failed to initialize AES-FF1 backend");
            (cipher, aes_key)
        }
    };

    tracing::info!("FPE backend initialized: {}", fpe_cipher.backend_name());

    let vault = PrivacyVault::new();

    let vault_cleanup = vault.clone();
    let cleanup_interval_seconds = config.security.session.cleanup_interval_seconds;
    let session_ttl_seconds = config.security.session.ttl_seconds;
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(cleanup_interval_seconds));
        loop {
            interval.tick().await;
            let removed = vault_cleanup.cleanup_stale_sessions(session_ttl_seconds);
            if removed > 0 {
                tracing::info!("Cleaned up {} stale session(s)", removed);
            }
        }
    });

    let reverser = Reverser::new(vault.clone(), vault_key);
    let provider: Arc<dyn provider::Provider> = ProviderFactory::build(config.provider.kind.clone(), config.provider.target_url.clone()).into();
    
    if config.security.key_rotation.enabled && config.security.key_rotation.auto_rotate {
        use crypto::key_rotation::KeyRotation;
        let vault_for_rotation = vault.clone();
        let rotation_interval_days = config.security.key_rotation.interval_days;
        
        let current_vault_key = vault_for_rotation.get_encryption_key();
        let key_rotation = Arc::new(KeyRotation::new(current_vault_key, rotation_interval_days));
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(86400));
            loop {
                interval.tick().await;
                if key_rotation.should_rotate() {
                    match key_rotation.rotate_key() {
                        Ok(new_key) => {
                            match vault_for_rotation.re_encrypt_with_new_key(new_key) {
                                Ok(count) => {
                                    tracing::info!("Key rotation completed: re-encrypted {} tokens", count);
                                }
                                Err(e) => {
                                    tracing::error!("Key rotation failed during re-encryption: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Key rotation failed: {}", e);
                        }
                    }
                }
            }
        });
    }
    
    let audit_appender = if let Some(file_path) = &config.security.audit.file_path {
        let strategy = match config.security.audit.rotation_strategy.as_str() {
            "size" => RotationStrategy::Size,
            _ => RotationStrategy::Daily,
        };
        Some(Arc::new(FileAppender::new(
            file_path,
            config.security.audit.max_file_size_mb,
            strategy,
        )))
    } else {
        None
    };
    
    let state = AppState {
        provider,
        config: Arc::new(config.clone()),
        vault,
        fpe_cipher,
        reverser,
        rate_limiter: Arc::new(Mutex::new(HashMap::new())),
        metrics: Arc::new(MetricsState::default()),
        audit_appender,
    };

    let mut app = Router::new()
        .route("/health", get(handlers::admin::health_check))
        .route("/v1/chat/completions", any(handlers::proxy::proxy_handler))
        .route("/sessions", get(handlers::admin::session_list_handler_protected))
        .route("/sessions/:id", delete(handlers::admin::session_delete_handler_protected));

    if state.config.security.metrics.enabled {
        app = app.route(
            state.config.security.metrics.path.as_str(),
            get(handlers::admin::metrics_handler_protected),
        );
    }

    if let Some(cors_layer) = middleware::build_cors_layer(&state.config.security.cors) {
        app = app.layer(cors_layer);
    }

    let app = app.with_state(state.clone());

    let addr = SocketAddr::from(([0, 0, 0, 0], state.config.server.port));
    tracing::info!("Privacy Gateway listening on {}", addr);

    if state.config.security.tls.enabled {
        tracing::warn!(
            "TLS is enabled in config, but direct TLS termination is not wired in this binary; run behind an HTTPS reverse proxy and set x-forwarded-proto=https"
        );
    }

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
