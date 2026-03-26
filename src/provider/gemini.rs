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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{ProviderMessage, ProviderMetadata};

    fn sample_request() -> ProviderRequest {
        let mut headers = std::collections::HashMap::new();
        headers.insert("x-api-key".to_string(), "test-key".to_string());

        ProviderRequest {
            method: "POST".to_string(),
            model: "gemini-1.5-flash".to_string(),
            messages: vec![ProviderMessage {
                role: "user".to_string(),
                content: "hello gemini".to_string(),
            }],
            headers,
            metadata: ProviderMetadata {
                session_id: Some("s1".to_string()),
                trace_id: Some("t1".to_string()),
                streaming: false,
            },
            raw_body: None,
        }
    }

    #[test]
    fn test_request_body_contains_contents_and_stream_flag() {
        let provider = GeminiProvider::new("https://generativelanguage.googleapis.com/v1beta/models/gemini:generateContent".to_string());
        let body = provider.request_body(&sample_request(), true);
        let parsed: serde_json::Value = serde_json::from_slice(&body).expect("valid json body");

        assert_eq!(parsed["stream"], serde_json::json!(true));
        assert_eq!(parsed["contents"][0]["role"], serde_json::json!("user"));
        assert_eq!(parsed["contents"][0]["parts"][0]["text"], serde_json::json!("hello gemini"));
    }

    #[test]
    fn test_stream_url_appends_alt_sse() {
        let provider = GeminiProvider::new(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini:streamGenerateContent?key=abc"
                .to_string(),
        );
        let url = provider.stream_url().expect("valid stream url");

        assert!(url.contains("alt=sse"));
        assert!(url.contains("key=abc"));
    }
}
