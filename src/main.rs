mod config;
mod crypto;
mod detector;
mod masker;
mod proxy;
mod vault;

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    response::Response,
    routing::{any, get},
    Json, Router,
};
use serde_json::{json, Value};
use std::{net::SocketAddr, sync::Arc};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use config::Config;
use crypto::fpe::FPECipher;
use detector::{
    api_key::detect_api_keys, chinese_id::detect_chinese_id, email::detect_email,
    phone::detect_phone_number, PIIMatch, PIIType,
};
use masker::hash::hash_value;
use proxy::ProxyClient;
use vault::{PrivacyVault, Reverser};

#[derive(Clone)]
struct AppState {
    proxy_client: Arc<ProxyClient>,
    config: Arc<Config>,
    vault: PrivacyVault,
    fpe_cipher: Arc<FPECipher>,
    reverser: Reverser,
}

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
            proxy: config::ProxyConfig {
                target_url: std::env::var("TARGET_URL")
                    .unwrap_or_else(|_| "https://api.openai.com/v1/chat/completions".to_string()),
                timeout_seconds: 30,
                max_retries: 3,
            },
            pii: config::PiiConfig {
                types: vec!["chinese_id".to_string()],
                masking_strategy: "replace".to_string(),
                detectors: std::collections::HashMap::new(),
            },
            security: config::SecurityConfig {
                rate_limit: config::RateLimitConfig {
                    requests_per_minute: 60,
                    burst_size: 10,
                },
                cors: config::CorsConfig {
                    enabled: true,
                    allowed_origins: vec!["*".to_string()],
                },
            },
        }
    });

    tracing::info!("Target URL: {}", config.proxy.target_url);
    tracing::info!("PII types enabled: {:?}", config.pii.types);

    let fpe_key = std::env::var("FPE_KEY")
        .map(|key_str| {
            let key_bytes: &[u8] = key_str.as_bytes();
            let mut key = [0u8; 32];
            if key_bytes.len() != 64 {
                panic!("FPE_KEY must be 64 hex characters (32 bytes)");
            }
            for (i, chunk) in key_bytes.chunks(2).enumerate() {
                key[i] = u8::from_str_radix(std::str::from_utf8(chunk).unwrap(), 16)
                    .expect("FPE_KEY must be hex encoded");
            }
            key
        })
        .unwrap_or_else(|_| {
            panic!("FPE_KEY environment variable must be set");
        });
    let fpe_cipher =
        Arc::new(FPECipher::new(&fpe_key, 10).expect("Failed to initialize FPE cipher"));
    let vault = PrivacyVault::new();

    let vault_cleanup = vault.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
        loop {
            interval.tick().await;
            let removed = vault_cleanup.cleanup_stale_sessions(1800);
            if removed > 0 {
                tracing::info!("Cleaned up {} stale session(s)", removed);
            }
        }
    });

    let reverser = Reverser::new(vault.clone(), fpe_key);
    let proxy_client = Arc::new(ProxyClient::new(config.proxy.target_url.clone()));
    let state = AppState {
        proxy_client,
        config: Arc::new(config.clone()),
        vault,
        fpe_cipher,
        reverser,
    };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/v1/chat/completions", any(proxy_handler))
        .with_state(state.clone());

    let addr = SocketAddr::from(([0, 0, 0, 0], state.config.server.port));
    tracing::info!("Privacy Gateway listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": "sec-gateway",
        "version": "0.1.0"
    }))
}

async fn proxy_handler(
    State(state): State<AppState>,
    mut req: Request<Body>,
) -> Result<axum::response::Response, (StatusCode, String)> {
    let (parts, body) = req.into_parts();

    let body_bytes = axum::body::to_bytes(body, usize::MAX).await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Failed to read body: {}", e),
        )
    })?;

    let body_str = String::from_utf8_lossy(&body_bytes);

    let mut all_matches = Vec::new();

    let chinese_id_detections = detect_chinese_id(&body_str);
    all_matches.extend(
        chinese_id_detections
            .into_iter()
            .map(|(start, end, value)| PIIMatch::new(PIIType::ChineseID, value, start, end, 1.0)),
    );

    all_matches.extend(detect_phone_number(&body_str));
    all_matches.extend(detect_email(&body_str));
    all_matches.extend(detect_api_keys(&body_str));

    if all_matches.is_empty() {
        tracing::debug!("No PII detected, forwarding original request");
        req = Request::from_parts(parts, Body::from(body_bytes.to_vec()));
        return state.proxy_client.forward_request(req).await;
    }

    all_matches.sort_by_key(|m| m.start);

    let session_id = extract_session_id(&parts);
    tracing::info!(
        "Detected {} PII item(s) in session {}",
        all_matches.len(),
        session_id
    );

    let mut masked_body = body_str.to_string();
    let mut offset: i64 = 0;

    for (idx, pii_match) in all_matches.iter().enumerate() {
        let original_value = &pii_match.value;
        let token = match pii_match.pii_type {
            PIIType::ChineseID | PIIType::PhoneNumber => {
                let encrypted = state
                    .fpe_cipher
                    .encrypt(original_value, session_id.as_bytes())
                    .map_err(|e| {
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("FPE encryption failed: {}", e),
                        )
                    })?;
                encrypted
            }
            PIIType::APIKey
            | PIIType::APISecret
            | PIIType::AWSAccessKey
            | PIIType::AWSSecretKey
            | PIIType::GitHubToken => hash_value(original_value),
            PIIType::Email => {
                format!("[REDACTED_EMAIL_{:03}]", idx + 1)
            }
        };

        state
            .vault
            .store(&session_id, token.clone(), original_value.clone())
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Vault storage failed: {}", e),
                )
            })?;

        let start = (pii_match.start as i64 + offset) as usize;
        let end = (pii_match.end as i64 + offset) as usize;

        masked_body.replace_range(start..end, &token);
        offset += token.len() as i64 - (pii_match.end - pii_match.start) as i64;

        tracing::debug!(
            "Masked {} at position {}-{} with token: {}",
            pii_match.pii_type.as_str(),
            pii_match.start,
            pii_match.end,
            token
        );
    }

    tracing::info!(
        "Masked body preview: {}",
        masked_body.chars().take(200).collect::<String>()
    );

    req = Request::from_parts(parts, Body::from(masked_body));

    tracing::debug!("Proxying request to target");
    let response = state.proxy_client.forward_request(req).await?;

    let (parts, body) = response.into_parts();
    let body_bytes = axum::body::to_bytes(body, usize::MAX).await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("Read response body failed: {}", e),
        )
    })?;
    let body_str = String::from_utf8_lossy(&body_bytes);

    let restored_body = state
        .reverser
        .restore_response(&session_id, &body_str)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Response restoration failed: {}", e),
            )
        })?;

    tracing::debug!("Response restored for session {}", session_id);

    let response = Response::from_parts(parts, restored_body.into());
    Ok(response)
}

fn extract_session_id(parts: &axum::http::request::Parts) -> String {
    parts
        .headers
        .get("x-session-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
}
