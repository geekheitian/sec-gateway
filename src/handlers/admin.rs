use axum::{
    body::Body,
    extract::{Path, State},
    http::{Request, StatusCode},
    response::Response,
    Json,
};
use serde_json::{json, Value};

use crate::{app_state::AppState, middleware};

pub async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": "sec-gateway",
        "version": "0.1.0"
    }))
}

pub async fn session_list_handler(State(state): State<AppState>) -> Json<Value> {
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

pub async fn session_delete_handler(
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

pub async fn session_list_handler_protected(
    State(state): State<AppState>,
    req: Request<Body>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (parts, _) = req.into_parts();
    middleware::enforce_endpoint_auth(&parts, &state.config)?;
    Ok(session_list_handler(State(state)).await)
}

pub async fn session_delete_handler_protected(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    req: Request<Body>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let (parts, _) = req.into_parts();
    middleware::enforce_endpoint_auth(&parts, &state.config)?;
    session_delete_handler(State(state), Path(session_id)).await
}

pub async fn metrics_handler(State(state): State<AppState>) -> Response {
    use std::sync::atomic::Ordering;
    
    let total = state.metrics.total_requests.load(Ordering::Relaxed);
    let blocked = state.metrics.blocked_requests.load(Ordering::Relaxed);
    let pii = state
        .metrics
        .pii_detected_requests
        .load(Ordering::Relaxed);
    let fpe_backend = state.fpe_cipher.backend_name();

    let body = format!(
        "# HELP sec_gateway_requests_total Total requests\n# TYPE sec_gateway_requests_total counter\nsec_gateway_requests_total {}\n# HELP sec_gateway_blocked_requests_total Blocked requests\n# TYPE sec_gateway_blocked_requests_total counter\nsec_gateway_blocked_requests_total {}\n# HELP sec_gateway_pii_detected_requests_total Requests with detected PII\n# TYPE sec_gateway_pii_detected_requests_total counter\nsec_gateway_pii_detected_requests_total {}\n# HELP sec_gateway_fpe_backend Current FPE backend\n# TYPE sec_gateway_fpe_backend gauge\nsec_gateway_fpe_backend{{backend=\"{}\"}} 1\n",
        total, blocked, pii, fpe_backend
    );

    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/plain; version=0.0.4")
        .body(Body::from(body))
        .unwrap_or_else(|_| Response::new(Body::from("metrics unavailable")))
}

pub async fn metrics_handler_protected(
    State(state): State<AppState>,
    req: Request<Body>,
) -> Result<Response, (StatusCode, String)> {
    let (parts, _) = req.into_parts();
    middleware::enforce_endpoint_auth(&parts, &state.config)?;
    Ok(metrics_handler(State(state)).await)
}
