mod audit;
mod config;
mod crypto;
mod detector;
mod masker;
mod provider;
mod vault;

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header::AUTHORIZATION, HeaderValue, Method, Request, StatusCode},
    response::Response,
    routing::{any, delete, get},
    Json, Router,
};
use chrono::Utc;
use futures_util::StreamExt;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{atomic::{AtomicU64, Ordering}, Arc, Mutex},
    time::{Duration, Instant},
};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use audit::{AuditEvent, FileAppender, LogLevel, RotationStrategy};
use config::Config;
use crypto::fpe::FPECipher;
use detector::{active_detectors, detect_custom_patterns};
use masker::hash::hash_value;
use provider::{
    ProviderFactory,
    Provider,
    ProviderMessage,
    ProviderMetadata,
    ProviderRequest,
    ProviderStreamEvent,
};
use vault::{PrivacyVault, Reverser};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MaskingStrategy {
    Fpe,
    Hash,
    Replace,
}

fn masking_strategy_for(pii_type: &str, config: &config::PiiConfig) -> MaskingStrategy {
    match config
        .per_type_masking_strategies
        .get(pii_type)
        .map(|s| s.as_str())
        .unwrap_or(config.masking_strategy.as_str())
    {
        "fpe" => MaskingStrategy::Fpe,
        "hash" => MaskingStrategy::Hash,
        _ => MaskingStrategy::Replace,
    }
}

fn redaction_placeholder(pii_type: &str, index: usize) -> String {
    format!("[REDACTED_{}_{}]", pii_type.to_uppercase(), index + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;
    use axum::http::HeaderMap;

    fn default_auth_config() -> config::AuthConfig {
        config::AuthConfig {
            enabled: true,
            bearer_token: Some("token123".to_string()),
            header_name: Some("x-gateway-key".to_string()),
            header_value: Some("k123".to_string()),
        }
    }

    #[test]
    fn test_masking_strategy_for_per_type_override() {
        let mut config = config::PiiConfig {
            types: vec![],
            masking_strategy: "replace".to_string(),
            per_type_masking_strategies: std::collections::HashMap::new(),
            detectors: std::collections::HashMap::new(),
        };
        config
            .per_type_masking_strategies
            .insert("credit_card".to_string(), "hash".to_string());

        assert_eq!(masking_strategy_for("credit_card", &config), MaskingStrategy::Hash);
        assert_eq!(masking_strategy_for("email", &config), MaskingStrategy::Replace);
    }

    #[test]
    fn test_is_authorized_with_bearer() {
        let (mut parts, _) = Request::new(()).into_parts();
        parts.headers = HeaderMap::new();
        parts
            .headers
            .insert(AUTHORIZATION, HeaderValue::from_static("Bearer token123"));

        assert!(is_authorized(&parts, &default_auth_config()));
    }

    #[test]
    fn test_is_authorized_with_custom_header() {
        let (mut parts, _) = Request::new(()).into_parts();
        parts.headers = HeaderMap::new();
        parts
            .headers
            .insert("x-gateway-key", HeaderValue::from_static("k123"));

        assert!(is_authorized(&parts, &default_auth_config()));
    }

    #[test]
    fn test_build_cors_layer_enabled() {
        let cfg = config::CorsConfig {
            enabled: true,
            allowed_origins: vec!["http://localhost:3000".to_string()],
        };
        assert!(build_cors_layer(&cfg).is_some());
    }

    #[test]
    fn test_enforce_tls_requirement() {
        let (mut parts, _) = Request::new(()).into_parts();
        parts.headers = HeaderMap::new();

        let cfg = config::TlsConfig {
            enabled: false,
            cert_path: None,
            key_path: None,
            require_forwarded_https: true,
        };
        assert!(enforce_tls_requirement(&parts, &cfg).is_err());

        parts
            .headers
            .insert("x-forwarded-proto", HeaderValue::from_static("https"));
        assert!(enforce_tls_requirement(&parts, &cfg).is_ok());
    }
}

#[derive(Clone)]
struct AppState {
    provider: Arc<dyn Provider>,
    config: Arc<Config>,
    vault: PrivacyVault,
    fpe_cipher: Arc<FPECipher>,
    reverser: Reverser,
    rate_limiter: Arc<Mutex<HashMap<String, (u32, Instant)>>>,
    metrics: Arc<MetricsState>,
    audit_appender: Option<Arc<FileAppender>>,
}

#[derive(Default)]
struct MetricsState {
    total_requests: AtomicU64,
    blocked_requests: AtomicU64,
    pii_detected_requests: AtomicU64,
}

fn is_authorized(parts: &axum::http::request::Parts, config: &config::AuthConfig) -> bool {
    if !config.enabled {
        return true;
    }

    let bearer_ok = if let Some(expected) = &config.bearer_token {
        parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .map(|v| constant_time_eq(v.as_bytes(), format!("Bearer {}", expected).as_bytes()))
            .unwrap_or(false)
    } else {
        false
    };

    let header_ok = match (&config.header_name, &config.header_value) {
        (Some(name), Some(value)) => parts
            .headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(|v| constant_time_eq(v.as_bytes(), value.as_bytes()))
            .unwrap_or(false),
        _ => false,
    };

    bearer_ok || header_ok
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for i in 0..a.len() {
        diff |= a[i] ^ b[i];
    }
    diff == 0
}

fn client_id_from_parts(parts: &axum::http::request::Parts, config: &Config) -> String {
    if config.security.proxy.trust_forwarded_headers {
        if let Some(forwarded) = parts
            .headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
        {
            let first = forwarded.split(',').next().map(str::trim).unwrap_or("anonymous");
            return first.to_string();
        }
    }

    parts
        .headers
        .get("x-session-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("anonymous")
        .to_string()
}

fn check_rate_limit(
    state: &AppState,
    client_id: &str,
) -> Result<(), (StatusCode, String)> {
    let now = Instant::now();
    let window = Duration::from_secs(60);
    let limit = state.config.security.rate_limit.requests_per_minute;
    let burst = state.config.security.rate_limit.burst_size;
    let max_allowed = limit.saturating_add(burst);

    let mut limiter = state
        .rate_limiter
        .lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Rate limiter lock poisoned".to_string()))?;

    let entry = limiter.entry(client_id.to_string()).or_insert((0, now));
    if now.duration_since(entry.1) >= window {
        *entry = (0, now);
    }
    if entry.0 >= max_allowed {
        return Err((StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded".to_string()));
    }
    entry.0 += 1;
    Ok(())
}

fn enforce_tls_requirement(parts: &axum::http::request::Parts, config: &config::TlsConfig) -> Result<(), (StatusCode, String)> {
    if !config.require_forwarded_https {
        return Ok(());
    }
    let forwarded_proto = parts
        .headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if forwarded_proto.eq_ignore_ascii_case("https") {
        Ok(())
    } else {
        Err((
            StatusCode::UPGRADE_REQUIRED,
            "HTTPS required (x-forwarded-proto=https)".to_string(),
        ))
    }
}

fn audit_request(parts: &axum::http::request::Parts, config: &Config, session_id: &str, pii_count: usize) {
    if !config.security.audit.enabled {
        return;
    }

    let client_ip = if config.security.proxy.trust_forwarded_headers {
        parts
            .headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
    } else {
        "untrusted-forwarded-header"
    };

    if config.security.audit.log_headers {
        let mut sanitized_headers = parts.headers.clone();
        sanitized_headers.insert(AUTHORIZATION, HeaderValue::from_static("[REDACTED]"));
        if let Some(name) = &config.security.auth.header_name {
            if let Ok(header_name) = axum::http::header::HeaderName::from_bytes(name.as_bytes()) {
                sanitized_headers.insert(header_name, HeaderValue::from_static("[REDACTED]"));
            }
        }
        tracing::info!(
            method = %parts.method,
            uri = %parts.uri,
            session_id = %session_id,
            pii_count = pii_count,
            client_ip = %client_ip,
            headers = ?sanitized_headers,
            "audit request"
        );
    } else {
        tracing::info!(
            method = %parts.method,
            uri = %parts.uri,
            session_id = %session_id,
            pii_count = pii_count,
            client_ip = %client_ip,
            "audit request"
        );
    }
}

fn build_cors_layer(config: &config::CorsConfig) -> Option<CorsLayer> {
    if !config.enabled {
        return None;
    }

    let mut layer = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any);

    if config.allowed_origins.iter().any(|origin| origin == "*") {
        layer = layer.allow_origin(Any);
    } else {
        let mut origins = Vec::new();
        for origin in &config.allowed_origins {
            if let Ok(value) = HeaderValue::from_str(origin) {
                origins.push(value);
            }
        }
        layer = layer.allow_origin(origins);
    }

    Some(layer)
}

async fn metrics_handler(State(state): State<AppState>) -> Response {
    let total = state.metrics.total_requests.load(Ordering::Relaxed);
    let blocked = state.metrics.blocked_requests.load(Ordering::Relaxed);
    let pii = state
        .metrics
        .pii_detected_requests
        .load(Ordering::Relaxed);

    let body = format!(
        "# HELP sec_gateway_requests_total Total requests\n# TYPE sec_gateway_requests_total counter\nsec_gateway_requests_total {}\n# HELP sec_gateway_blocked_requests_total Blocked requests\n# TYPE sec_gateway_blocked_requests_total counter\nsec_gateway_blocked_requests_total {}\n# HELP sec_gateway_pii_detected_requests_total Requests with detected PII\n# TYPE sec_gateway_pii_detected_requests_total counter\nsec_gateway_pii_detected_requests_total {}\n",
        total, blocked, pii
    );

    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/plain; version=0.0.4")
        .body(Body::from(body))
        .unwrap_or_else(|_| Response::new(Body::from("metrics unavailable")))
}

fn enforce_endpoint_auth(parts: &axum::http::request::Parts, config: &Config) -> Result<(), (StatusCode, String)> {
    if is_authorized(parts, &config.security.auth) {
        Ok(())
    } else {
        Err((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()))
    }
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
            provider: config::ProviderConfig {
                kind: config::ProviderKind::OpenAI,
                model: "gpt-3.5-turbo".to_string(),
                target_url: std::env::var("TARGET_URL")
                    .unwrap_or_else(|_| "https://api.openai.com/v1/chat/completions".to_string()),
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

    let reverser = Reverser::new(vault.clone(), fpe_key);
    let provider: Arc<dyn Provider> = ProviderFactory::build(config.provider.kind.clone(), config.provider.target_url.clone()).into();
    
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
        .route("/health", get(health_check))
        .route("/v1/chat/completions", any(proxy_handler))
        .route("/sessions", get(session_list_handler_protected))
        .route("/sessions/:id", delete(session_delete_handler_protected));

    if state.config.security.metrics.enabled {
        app = app.route(
            state.config.security.metrics.path.as_str(),
            get(metrics_handler_protected),
        );
    }

    if let Some(cors_layer) = build_cors_layer(&state.config.security.cors) {
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

async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": "sec-gateway",
        "version": "0.1.0"
    }))
}

async fn session_list_handler(State(state): State<AppState>) -> Json<Value> {
    let sessions_with_metadata = state.vault.list_all_sessions().unwrap_or_default();
    let sessions: Vec<Value> = sessions_with_metadata
        .into_iter()
        .map(|(id, metadata)| {
            json!({
                "id": id,
                "created_at": metadata.created_at.to_rfc3339(),
                "last_accessed": metadata.last_accessed.to_rfc3339(),
                "request_count": metadata.request_count,
            })
        })
        .collect();
    
    Json(json!({
        "session_count": sessions.len(),
        "sessions": sessions,
    }))
}

async fn session_delete_handler(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    uuid::Uuid::parse_str(session_id.as_str())
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid session id".to_string()))?;

    state
        .vault
        .clear_session(session_id.as_str())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(json!({
        "deleted": true,
        "session_id": session_id,
    })))
}

async fn proxy_handler(
    State(state): State<AppState>,
    req: Request<Body>,
) -> Result<axum::response::Response, (StatusCode, String)> {
    let request_start = Instant::now();
    state.metrics.total_requests.fetch_add(1, Ordering::Relaxed);
    let (parts, body) = req.into_parts();
    
    let session_id = extract_session_id(&parts);
    let client_ip = client_id_from_parts(&parts, &state.config);
    let method = parts.method.as_str().to_string();
    let path = parts.uri.path().to_string();

    enforce_tls_requirement(&parts, &state.config.security.tls).inspect_err(|_| {
        state
            .metrics
            .blocked_requests
            .fetch_add(1, Ordering::Relaxed);
    })?;

    if !is_authorized(&parts, &state.config.security.auth) {
        state
            .metrics
            .blocked_requests
            .fetch_add(1, Ordering::Relaxed);
        
        let audit_event = AuditEvent {
            timestamp: Utc::now(),
            level: LogLevel::Warn,
            session_id: session_id.clone(),
            client_ip: client_ip.clone(),
            method: method.clone(),
            path: path.clone(),
            status_code: StatusCode::UNAUTHORIZED.as_u16(),
            duration_ms: request_start.elapsed().as_millis() as u64,
            provider: state.config.provider.kind.clone().into(),
            pii_detected: 0,
            error: Some("Unauthorized".to_string()),
        };
        if let Some(appender) = &state.audit_appender {
            let _ = appender.append(&audit_event.to_json()).await;
        }
        
        return Err((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()));
    }

    let client_id = client_id_from_parts(&parts, &state.config);

    check_rate_limit(&state, &client_id).inspect_err(|_| {
        state
            .metrics
            .blocked_requests
            .fetch_add(1, Ordering::Relaxed);
    })?;

    let body_bytes = axum::body::to_bytes(body, usize::MAX).await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Failed to read body: {}", e),
        )
    })?;

    let body_str = String::from_utf8_lossy(&body_bytes);

    let mut all_matches = Vec::new();

    for detect_fn in active_detectors(&state.config.pii.types) {
        all_matches.extend(detect_fn(&body_str));
    }
    all_matches.extend(detect_custom_patterns(
        &body_str,
        &state.config.pii.detectors,
    ));

    if all_matches.is_empty() {
        let session_id = extract_session_id(&parts);
        audit_request(&parts, &state.config, &session_id, 0);
        tracing::debug!("No PII detected, forwarding original request");
        let provider_request = ProviderRequest {
            method: parts.method.as_str().to_string(),
            model: state.config.provider.model.clone(),
            messages: vec![ProviderMessage {
                role: "user".to_string(),
                content: body_str.to_string(),
            }],
            headers: parts
                .headers
                .iter()
                .filter_map(|(key, value)| value.to_str().ok().map(|v| (key.as_str().to_string(), v.to_string())))
                .collect(),
            metadata: ProviderMetadata {
                session_id: extract_session_id(&parts).into(),
                trace_id: parts
                    .headers
                    .get("x-trace-id")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string()),
                streaming: false,
            },
            raw_body: Some(body_bytes.clone().into()),
        };
        let response = state.provider.send(provider_request).await.map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Provider forwarding failed: {}", e),
            )
        })?;

        let response = Response::builder()
            .status(response.status)
            .body(Body::from(response.body.to_vec()))
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Build response failed: {}", e),
                )
            })?;

        return Ok(response);
    }

    all_matches.sort_by_key(|m| m.start);

    let session_id = extract_session_id(&parts);
    state
        .metrics
        .pii_detected_requests
        .fetch_add(1, Ordering::Relaxed);
    audit_request(&parts, &state.config, &session_id, all_matches.len());
    tracing::info!(
        "Detected {} PII item(s) in session {}",
        all_matches.len(),
        session_id
    );

    let mut masked_body = body_str.to_string();
    let mut offset: i64 = 0;
    let mut issued_tokens = std::collections::HashSet::new();

    for (idx, pii_match) in all_matches.iter().enumerate() {
        let original_value = &pii_match.value;
        let token = match masking_strategy_for(pii_match.pii_type.as_str(), &state.config.pii) {
            MaskingStrategy::Fpe => {
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
            MaskingStrategy::Hash => hash_value(original_value),
            MaskingStrategy::Replace => redaction_placeholder(pii_match.pii_type.as_str(), idx),
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
        issued_tokens.insert(token.clone());

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

    let provider_request = ProviderRequest {
        method: parts.method.as_str().to_string(),
        model: state.config.provider.model.clone(),
        messages: vec![ProviderMessage {
            role: "user".to_string(),
            content: masked_body.clone(),
        }],
        headers: parts
            .headers
            .iter()
            .filter_map(|(key, value)| value.to_str().ok().map(|v| (key.as_str().to_string(), v.to_string())))
            .collect(),
        metadata: ProviderMetadata {
            session_id: Some(session_id.clone()),
            trace_id: parts
                .headers
                .get("x-trace-id")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string()),
            streaming: false,
        },
        raw_body: Some(masked_body.into()),
    };

    let is_streaming = parts
        .headers
        .get("accept")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.contains("text/event-stream"))
        .unwrap_or(false);

    if is_streaming {
        let stream = state.provider.send_stream(provider_request).await.map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Provider streaming failed: {}", e),
            )
        })?;

        let mut body_chunks = Vec::new();
        let mut events = stream.events;
        while let Some(event) = events.next().await {
            match event {
                Ok(ProviderStreamEvent::Data(text)) => body_chunks.push(format!("data: {}\n\n", text)),
                Ok(ProviderStreamEvent::JsonDelta(value)) => body_chunks.push(format!("data: {}\n\n", value)),
                Ok(ProviderStreamEvent::Comment(comment)) => body_chunks.push(format!(":{}\n\n", comment)),
                Ok(ProviderStreamEvent::Retry(ms)) => body_chunks.push(format!("retry: {}\n\n", ms)),
                Ok(ProviderStreamEvent::Done) => body_chunks.push("data: [DONE]\n\n".to_string()),
                Err(e) => {
                    return Err((
                        StatusCode::BAD_GATEWAY,
                        format!("Stream event failed: {}", e),
                    ));
                }
            }
        }

        let mut response = Response::builder()
            .status(StatusCode::OK)
            .body(Body::from_stream(futures_util::stream::iter(
                body_chunks
                    .into_iter()
                    .map(|chunk| Ok::<_, std::convert::Infallible>(bytes::Bytes::from(chunk))),
            )))
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Build streaming response failed: {}", e),
                )
            })?;

        response.headers_mut().insert(
            "content-type",
            axum::http::HeaderValue::from_str(&stream.content_type)
                .unwrap_or(axum::http::HeaderValue::from_static("text/event-stream")),
        );

        return Ok(response);
    }

    tracing::debug!("Proxying request through provider");
    let provider_response = state.provider.send(provider_request).await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("Provider forwarding failed: {}", e),
        )
    })?;

    let body_str = String::from_utf8_lossy(&provider_response.body);

    let restored_body = state
        .reverser
        .restore_response_with_allowlist(&session_id, &body_str, &issued_tokens)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Response restoration failed: {}", e),
            )
        })?;

    tracing::debug!("Response restored for session {}", session_id);

    let audit_event = AuditEvent {
        timestamp: Utc::now(),
        level: LogLevel::Info,
        session_id: session_id.clone(),
        client_ip: client_ip.clone(),
        method,
        path,
        status_code: provider_response.status,
        duration_ms: request_start.elapsed().as_millis() as u64,
        provider: state.config.provider.kind.clone().into(),
        pii_detected: all_matches.len(),
        error: None,
    };
    if let Some(appender) = &state.audit_appender {
        let _ = appender.append(&audit_event.to_json()).await;
    }

    let response = Response::builder()
        .status(provider_response.status)
        .body(Body::from(restored_body))
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Build response failed: {}", e),
            )
        })?;
    Ok(response)
}

async fn session_list_handler_protected(
    State(state): State<AppState>,
    req: Request<Body>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (parts, _) = req.into_parts();
    enforce_endpoint_auth(&parts, &state.config)?;
    Ok(session_list_handler(State(state)).await)
}

async fn session_delete_handler_protected(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    req: Request<Body>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (parts, _) = req.into_parts();
    enforce_endpoint_auth(&parts, &state.config)?;
    session_delete_handler(State(state), Path(session_id)).await
}

async fn metrics_handler_protected(
    State(state): State<AppState>,
    req: Request<Body>,
) -> Result<Response, (StatusCode, String)> {
    let (parts, _) = req.into_parts();
    enforce_endpoint_auth(&parts, &state.config)?;
    Ok(metrics_handler(State(state)).await)
}

fn extract_session_id(parts: &axum::http::request::Parts) -> String {
    parts
        .headers
        .get("x-session-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
}
