use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    response::Response,
};
use chrono::Utc;
use futures_util::StreamExt;
use std::{
    collections::HashSet,
    sync::atomic::Ordering,
    time::Instant,
};

use crate::{
    app_state::AppState,
    audit::{AuditEvent, LogLevel},
    detector::{active_detectors, detect_custom_patterns},
    masker::hash::hash_value,
    middleware,
    provider::{ProviderMessage, ProviderMetadata, ProviderRequest, ProviderStreamEvent},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MaskingStrategy {
    Fpe,
    Hash,
    Replace,
}

fn masking_strategy_for(pii_type: &str, config: &crate::config::PiiConfig) -> MaskingStrategy {
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

pub async fn proxy_handler(
    State(state): State<AppState>,
    req: Request<Body>,
) -> Result<Response, (StatusCode, String)> {
    let request_start = Instant::now();
    state.metrics.total_requests.fetch_add(1, Ordering::Relaxed);
    let (parts, body) = req.into_parts();
    
    let session_id = middleware::extract_session_id(&parts);
    let client_ip = middleware::client_id_from_parts(&parts, &state.config);
    let method = parts.method.as_str().to_string();
    let path = parts.uri.path().to_string();

    middleware::enforce_tls_requirement(&parts, &state.config.security.tls).inspect_err(|_| {
        state
            .metrics
            .blocked_requests
            .fetch_add(1, Ordering::Relaxed);
    })?;

    if !middleware::is_authorized(&parts, &state.config.security.auth) {
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

    let client_id = middleware::client_id_from_parts(&parts, &state.config);

    middleware::check_rate_limit(
        &state.rate_limiter,
        &client_id,
        state.config.security.rate_limit.requests_per_minute,
        state.config.security.rate_limit.burst_size,
    ).inspect_err(|_| {
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
        middleware::audit_request(&parts, &state.config, &session_id, 0);
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
                session_id: Some(session_id.clone()),
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
    state
        .metrics
        .pii_detected_requests
        .fetch_add(1, Ordering::Relaxed);
    middleware::audit_request(&parts, &state.config, &session_id, all_matches.len());
    tracing::info!(
        "Detected {} PII item(s) in session {}",
        all_matches.len(),
        session_id
    );

    let mut masked_body = body_str.to_string();
    let mut offset: i64 = 0;
    let mut issued_tokens = HashSet::new();

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
        let mut accumulated = String::new();
        let mut events = stream.events;
        while let Some(event) = events.next().await {
            match event {
                Ok(ProviderStreamEvent::Data(text)) => {
                    accumulated.push_str(&text);
                    body_chunks.push(format!("data: {}\n\n", text));
                }
                Ok(ProviderStreamEvent::JsonDelta(value)) => {
                    let value_str = value.to_string();
                    accumulated.push_str(&value_str);
                    body_chunks.push(format!("data: {}\n\n", value_str));
                }
                Ok(ProviderStreamEvent::Comment(comment)) => body_chunks.push(format!(":{}\n\n", comment)),
                Ok(ProviderStreamEvent::Retry(ms)) => body_chunks.push(format!("retry: {}\n\n", ms)),
                Ok(ProviderStreamEvent::Done) => {
                    let restored = state
                        .reverser
                        .restore_response_with_allowlist(&session_id, &accumulated, &issued_tokens)
                        .unwrap_or(accumulated.clone());
                    let final_data = format!("data: {}\n\n", restored);
                    if let Some(last_idx) = body_chunks.iter().rposition(|c| c.starts_with("data: ")) {
                        body_chunks[last_idx] = final_data;
                    }
                    body_chunks.push("data: [DONE]\n\n".to_string());
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;

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
}
