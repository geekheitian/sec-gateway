use reqwest::Client;
use std::time::Duration;

use super::{ProviderError, ProviderRequest, ProviderResponse};

#[derive(Debug, Clone)]
pub struct HttpTransport {
    client: Client,
}

impl HttpTransport {
    pub fn new(timeout: Duration) -> Result<Self, ProviderError> {
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| ProviderError::Transport(format!("failed to create reqwest client: {}", e)))?;
        Ok(Self { client })
    }

    pub async fn send_bytes(
        &self,
        url: &str,
        request: &ProviderRequest,
        body: bytes::Bytes,
        extra_headers: &[(&str, &str)],
        stream: bool,
    ) -> Result<ProviderResponse, ProviderError> {
        let method = reqwest::Method::from_bytes(request.method.as_bytes())
            .map_err(|e| ProviderError::InvalidRequest(e.to_string()))?;

        let mut builder = self.client.request(method, url).body(body.to_vec());
        for (key, value) in request.headers.iter() {
            builder = builder.header(key, value);
        }
        for (key, value) in extra_headers.iter() {
            builder = builder.header(*key, *value);
        }
        if stream {
            builder = builder.header("accept", "text/event-stream");
        }

        let response = builder
            .send()
            .await
            .map_err(|e| ProviderError::Transport(e.to_string()))?;

        let status = response.status().as_u16();
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or(if stream { "text/event-stream" } else { "application/json" })
            .to_string();
        let body = response
            .bytes()
            .await
            .map_err(|e| ProviderError::Transport(e.to_string()))?;

        Ok(ProviderResponse { status, content_type, body })
    }
}
