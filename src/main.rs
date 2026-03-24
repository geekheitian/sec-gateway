mod proxy;
mod detector;
mod masker;
mod crypto;
mod config;

use axum::{
    routing::{any, get},
    Router,
    Json,
    body::Body,
    http::{Request, StatusCode},
    extract::State,
};
use serde_json::{json, Value};
use std::{net::SocketAddr, sync::Arc};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use proxy::ProxyClient;
use detector::chinese_id::detect_chinese_id;
use masker::replace::mask_with_placeholder;
use config::Config;

#[derive(Clone)]
struct AppState {
    proxy_client: Arc<ProxyClient>,
    config: Arc<Config>,
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

    let config = Config::load_default()
        .unwrap_or_else(|e| {
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

    let proxy_client = Arc::new(ProxyClient::new(config.proxy.target_url.clone()));
    let state = AppState { 
        proxy_client,
        config: Arc::new(config.clone()),
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
    
    let body_bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to read body: {}", e)))?;

    let body_str = String::from_utf8_lossy(&body_bytes);
    
    let detections = detect_chinese_id(&body_str);
    
    if !detections.is_empty() {
        tracing::info!("Detected {} Chinese ID(s) in request", detections.len());
        let masked_body = mask_with_placeholder(&body_str, &detections, "[REDACTED_ID", "]");
        
        req = Request::from_parts(parts, Body::from(masked_body));
    } else {
        req = Request::from_parts(parts, Body::from(body_bytes.to_vec()));
    }

    tracing::debug!("Proxying request to target");
    state.proxy_client.forward_request(req).await
}
