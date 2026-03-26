use async_trait::async_trait;
use std::time::Duration;

use super::{HttpTransport, Provider, ProviderError, ProviderRequest, ProviderResponse, ProviderStream, parse_sse_text};

#[derive(Debug, Clone)]
pub struct AnthropicProvider {
    transport: HttpTransport,
    target_url: String,
}

impl AnthropicProvider {
    pub fn new(target_url: String) -> Self {
        Self {
            transport: HttpTransport::new(Duration::from_secs(120)),
            target_url,
        }
    }

    fn transform_request(&self, request: &ProviderRequest, stream: bool) -> serde_json::Value {
        serde_json::json!({
            "model": request.model,
            "messages": request.messages.iter().map(|m| serde_json::json!({
                "role": m.role,
                "content": m.content,
            })).collect::<Vec<_>>(),
            "stream": stream,
        })
    }

    fn request_body(&self, request: &ProviderRequest, stream: bool) -> bytes::Bytes {
        bytes::Bytes::from(self.transform_request(request, stream).to_string())
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    async fn send(&self, request: ProviderRequest) -> Result<ProviderResponse, ProviderError> {
        let body = self.request_body(&request, false);
        self.transport
            .send_bytes(
                &self.target_url,
                &request,
                body,
                &[("anthropic-version", "2023-06-01")],
                false,
            )
            .await
    }

    async fn send_stream(&self, request: ProviderRequest) -> Result<ProviderStream, ProviderError> {
        let body = self.request_body(&request, true);
        let response = self
            .transport
            .send_bytes(
                &self.target_url,
                &request,
                body,
                &[("anthropic-version", "2023-06-01")],
                true,
            )
            .await?;
        let text = String::from_utf8_lossy(&response.body).to_string();
        let events = parse_sse_text(&text);
        Ok(ProviderStream { content_type: response.content_type, events: Box::pin(futures_util::stream::iter(events)) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{ProviderMessage, ProviderMetadata};

    fn sample_request() -> ProviderRequest {
        ProviderRequest {
            method: "POST".to_string(),
            model: "claude-3-5-sonnet".to_string(),
            messages: vec![ProviderMessage {
                role: "user".to_string(),
                content: "hello".to_string(),
            }],
            headers: std::collections::HashMap::new(),
            metadata: ProviderMetadata {
                session_id: Some("s1".to_string()),
                trace_id: Some("t1".to_string()),
                streaming: false,
            },
            raw_body: None,
        }
    }

    #[test]
    fn test_transform_request_non_stream() {
        let provider = AnthropicProvider::new("https://api.anthropic.com/v1/messages".to_string());
        let body = provider.transform_request(&sample_request(), false);

        assert_eq!(body["model"], serde_json::json!("claude-3-5-sonnet"));
        assert_eq!(body["stream"], serde_json::json!(false));
        assert_eq!(body["messages"][0]["role"], serde_json::json!("user"));
        assert_eq!(body["messages"][0]["content"], serde_json::json!("hello"));
    }

    #[test]
    fn test_request_body_stream_flag() {
        let provider = AnthropicProvider::new("https://api.anthropic.com/v1/messages".to_string());
        let request = sample_request();

        let body = provider.request_body(&request, true);
        let parsed: serde_json::Value = serde_json::from_slice(&body).expect("valid json body");
        assert_eq!(parsed["stream"], serde_json::json!(true));
    }
}
