use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::error::ProviderError;
use super::sse::ProviderStream;

#[async_trait]
pub trait Provider: Send + Sync {
    async fn send(&self, request: ProviderRequest) -> Result<ProviderResponse, ProviderError>;

    async fn send_stream(
        &self,
        request: ProviderRequest,
    ) -> Result<ProviderStream, ProviderError>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderMetadata {
    pub session_id: Option<String>,
    pub trace_id: Option<String>,
    pub streaming: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderRequest {
    pub method: String,
    pub model: String,
    pub messages: Vec<ProviderMessage>,
    pub headers: std::collections::HashMap<String, String>,
    pub metadata: ProviderMetadata,
    pub raw_body: Option<bytes::Bytes>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderResponse {
    pub status: u16,
    pub content_type: String,
    pub body: bytes::Bytes,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_request_serde_roundtrip() {
        let mut headers = std::collections::HashMap::new();
        headers.insert("authorization".to_string(), "Bearer test-token".to_string());

        let request = ProviderRequest {
            method: "POST".to_string(),
            model: "gpt-4o-mini".to_string(),
            messages: vec![ProviderMessage {
                role: "user".to_string(),
                content: "hello".to_string(),
            }],
            headers,
            metadata: ProviderMetadata {
                session_id: Some("session-1".to_string()),
                trace_id: Some("trace-1".to_string()),
                streaming: true,
            },
            raw_body: Some(bytes::Bytes::from("{\"foo\":\"bar\"}")),
        };

        let json = serde_json::to_string(&request).expect("serialize request");
        let parsed: ProviderRequest = serde_json::from_str(&json).expect("deserialize request");
        assert_eq!(parsed, request);
    }

    #[test]
    fn test_provider_response_serde_roundtrip() {
        let response = ProviderResponse {
            status: 200,
            content_type: "application/json".to_string(),
            body: bytes::Bytes::from("{\"ok\":true}"),
        };

        let json = serde_json::to_string(&response).expect("serialize response");
        let parsed: ProviderResponse = serde_json::from_str(&json).expect("deserialize response");
        assert_eq!(parsed, response);
    }
}
