use async_trait::async_trait;
use std::time::Duration;

use super::{HttpTransport, Provider, ProviderError, ProviderRequest, ProviderResponse, ProviderStream, parse_sse_text};

#[derive(Debug, Clone)]
pub struct OpenAIProvider {
    transport: HttpTransport,
    target_url: String,
}

impl OpenAIProvider {
    pub fn new(target_url: String) -> Self {
        Self {
            transport: HttpTransport::new(Duration::from_secs(120)),
            target_url,
        }
    }

    fn request_body(&self, request: &ProviderRequest, stream: bool) -> bytes::Bytes {
        let body = if let Some(raw_body) = &request.raw_body {
            raw_body.clone()
        } else {
            bytes::Bytes::from(
                serde_json::to_string(&serde_json::json!({
                    "model": request.model,
                    "messages": request.messages,
                    "metadata": request.metadata,
                    "stream": stream,
                }))
                .unwrap_or_else(|_| "{}".to_string()),
            )
        };
        body
    }
}

#[async_trait]
impl Provider for OpenAIProvider {
    async fn send(&self, request: ProviderRequest) -> Result<ProviderResponse, ProviderError> {
        let body = self.request_body(&request, false);
        self.transport
            .send_bytes(&self.target_url, &request, body, &[], false)
            .await
    }

    async fn send_stream(&self, request: ProviderRequest) -> Result<ProviderStream, ProviderError> {
        let body = self.request_body(&request, true);
        let response = self
            .transport
            .send_bytes(&self.target_url, &request, body, &[], true)
            .await?;
        let text = String::from_utf8_lossy(&response.body).to_string();
        let events = parse_sse_text(&text);
        Ok(ProviderStream { content_type: response.content_type, events: Box::pin(futures_util::stream::iter(events)) })
    }
}
