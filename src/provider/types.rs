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
