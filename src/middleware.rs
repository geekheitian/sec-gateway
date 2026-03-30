use axum::http::{header::AUTHORIZATION, HeaderValue, Method, StatusCode};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tower_http::cors::{Any, CorsLayer};

use crate::config::{AuthConfig, Config, CorsConfig, TlsConfig};

pub fn is_authorized(parts: &axum::http::request::Parts, config: &AuthConfig) -> bool {
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

pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    let len = std::cmp::min(a.len(), b.len());
    let mut diff = (a.len() != b.len()) as u8;

    for i in 0..len {
        diff |= a[i] ^ b[i];
    }

    diff == 0
}

pub fn client_id_from_parts(parts: &axum::http::request::Parts, config: &Config) -> String {
    if config.security.proxy.trust_forwarded_headers {
        if let Some(forwarded) = parts
            .headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
        {
            let first = forwarded
                .split(',')
                .next()
                .map(str::trim)
                .unwrap_or("anonymous");
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

pub fn check_rate_limit(
    rate_limiter: &Arc<Mutex<HashMap<String, (u32, Instant)>>>,
    client_id: &str,
    requests_per_minute: u32,
    burst_size: u32,
) -> Result<(), (StatusCode, String)> {
    let now = Instant::now();
    let window = Duration::from_secs(60);
    let max_allowed = requests_per_minute.saturating_add(burst_size);

    let mut limiter = rate_limiter.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Rate limiter lock poisoned".to_string(),
        )
    })?;

    let entry = limiter.entry(client_id.to_string()).or_insert((0, now));
    if now.duration_since(entry.1) >= window {
        *entry = (0, now);
    }
    if entry.0 >= max_allowed {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            "Rate limit exceeded".to_string(),
        ));
    }
    entry.0 += 1;
    Ok(())
}

pub fn enforce_tls_requirement(
    parts: &axum::http::request::Parts,
    config: &TlsConfig,
) -> Result<(), (StatusCode, String)> {
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

pub fn audit_request(
    parts: &axum::http::request::Parts,
    config: &Config,
    session_id: &str,
    pii_count: usize,
) {
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

pub fn build_cors_layer(config: &CorsConfig) -> Option<CorsLayer> {
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

pub fn enforce_endpoint_auth(
    parts: &axum::http::request::Parts,
    config: &Config,
) -> Result<(), (StatusCode, String)> {
    if is_authorized(parts, &config.security.auth) {
        Ok(())
    } else {
        Err((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()))
    }
}

pub fn extract_session_id(parts: &axum::http::request::Parts) -> String {
    parts
        .headers
        .get("x-session-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;
    use axum::http::{HeaderMap, Request};

    fn default_auth_config() -> AuthConfig {
        AuthConfig {
            enabled: true,
            bearer_token: Some("token123".to_string()),
            header_name: Some("x-gateway-key".to_string()),
            header_value: Some("k123".to_string()),
        }
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
        let cfg = CorsConfig {
            enabled: true,
            allowed_origins: vec!["http://localhost:3000".to_string()],
        };
        assert!(build_cors_layer(&cfg).is_some());
    }

    #[test]
    fn test_enforce_tls_requirement() {
        let (mut parts, _) = Request::new(()).into_parts();
        parts.headers = HeaderMap::new();

        let cfg = TlsConfig {
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
