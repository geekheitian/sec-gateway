use axum::{
    body::Body,
    http::{Request, Response, StatusCode},
};
use reqwest::Client;
use std::time::Duration;

pub struct ProxyClient {
    client: Client,
    target_url: String,
}

impl ProxyClient {
    pub fn new(target_url: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, target_url }
    }

    pub async fn forward_request(
        &self,
        req: Request<Body>,
    ) -> Result<Response<Body>, (StatusCode, String)> {
        let (parts, body) = req.into_parts();
        
        let body_bytes = axum::body::to_bytes(body, usize::MAX)
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to read body: {}", e)))?;

        let method_str = parts.method.as_str();
        let method = reqwest::Method::from_bytes(method_str.as_bytes())
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid method: {}", e)))?;

        let mut target_req = self
            .client
            .request(method, &self.target_url)
            .body(body_bytes.to_vec());

        for (key, value) in parts.headers.iter() {
            if key.as_str().starts_with("x-") || key == "content-type" || key == "authorization" {
                if let Ok(value_str) = value.to_str() {
                    target_req = target_req.header(key.as_str(), value_str);
                }
            }
        }

        let response = target_req
            .send()
            .await
            .map_err(|e| (StatusCode::BAD_GATEWAY, format!("Forward failed: {}", e)))?;

        let status_code = response.status().as_u16();
        let body_bytes = response
            .bytes()
            .await
            .map_err(|e| (StatusCode::BAD_GATEWAY, format!("Read response failed: {}", e)))?;

        let resp = Response::builder()
            .status(status_code)
            .body(Body::from(body_bytes.to_vec()))
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Build response failed: {}", e)))?;

        Ok(resp)
    }
}
