use thiserror::Error;

#[allow(dead_code)]
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ProviderError {
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("unauthorized: {0}")]
    Unauthorized(String),
    #[error("rate limited: {0}")]
    RateLimited(String),
    #[error("upstream error ({0}): {1}")]
    Upstream(u16, String),
    #[error("timeout: {0}")]
    Timeout(String),
    #[error("transport error: {0}")]
    Transport(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_error_display_messages() {
        assert_eq!(
            ProviderError::InvalidRequest("bad payload".to_string()).to_string(),
            "invalid request: bad payload"
        );
        assert_eq!(
            ProviderError::Unauthorized("missing token".to_string()).to_string(),
            "unauthorized: missing token"
        );
        assert_eq!(
            ProviderError::RateLimited("too many requests".to_string()).to_string(),
            "rate limited: too many requests"
        );
        assert_eq!(
            ProviderError::Upstream(502, "bad gateway".to_string()).to_string(),
            "upstream error (502): bad gateway"
        );
        assert_eq!(
            ProviderError::Timeout("deadline exceeded".to_string()).to_string(),
            "timeout: deadline exceeded"
        );
        assert_eq!(
            ProviderError::Transport("socket closed".to_string()).to_string(),
            "transport error: socket closed"
        );
    }
}
