use async_trait::async_trait;
use reqwest::Url;
use std::time::Duration;

use super::{HttpTransport, Provider, ProviderError, ProviderRequest, ProviderResponse, ProviderStream, parse_sse_text};

#[derive(Debug, Clone)]
pub struct GeminiProvider {
    transport: HttpTransport,
    target_url: String,
}

impl GeminiProvider {
    pub fn new(target_url: String) -> Self {
        Self {
            transport: HttpTransport::new(Duration::from_secs(120)),
            target_url,
        }
    }

    fn request_body(&self, request: &ProviderRequest, stream: bool) -> bytes::Bytes {
        bytes::Bytes::from(
            serde_json::json!({
                "contents": request.messages.iter().map(|m| serde_json::json!({
                    "role": m.role,
                    "parts": [{ "text": m.content }],
                })).collect::<Vec<_>>(),
                "generationConfig": { "temperature": 0.2 },
                "stream": stream,
            })
            .to_string(),
        )
    }

    fn stream_url(&self) -> Result<String, ProviderError> {
        let mut url = Url::parse(&self.target_url).map_err(|e| ProviderError::InvalidRequest(e.to_string()))?;
        url.query_pairs_mut().append_pair("alt", "sse");
        Ok(url.to_string())
    }
}

#[async_trait]
impl Provider for GeminiProvider {
    async fn send(&self, request: ProviderRequest) -> Result<ProviderResponse, ProviderError> {
        let body = self.request_body(&request, false);
        let api_key = request.headers.get("x-api-key").map(String::as_str).unwrap_or("");
        let headers = if api_key.is_empty() { vec![] } else { vec![("x-goog-api-key", api_key)] };
        self.transport.send_bytes(&self.target_url, &request, body, &headers, false).await
    }

    async fn send_stream(&self, request: ProviderRequest) -> Result<ProviderStream, ProviderError> {
        let body = self.request_body(&request, true);
        let stream_url = self.stream_url()?;
        let api_key = request.headers.get("x-api-key").map(String::as_str).unwrap_or("");
        let headers = if api_key.is_empty() { vec![] } else { vec![("x-goog-api-key", api_key)] };
        let response = self.transport.send_bytes(&stream_url, &request, body, &headers, true).await?;
        let text = String::from_utf8_lossy(&response.body).to_string();
        let events = parse_sse_text(&text);
        Ok(ProviderStream {
            content_type: response.content_type,
            events: Box::pin(futures_util::stream::iter(events)),
        })
    }
}
