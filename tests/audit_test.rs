#[cfg(test)]
mod audit_tests {
    use sec_gateway::audit::{AuditEvent, FileAppender, LogLevel, RotationStrategy};
    use sec_gateway::vault::{PrivacyVault, SessionMetadata};
    use sec_gateway::crypto::key_rotation::KeyRotation;
    use chrono::Utc;
    use std::path::PathBuf;
    use tokio::fs;

    #[tokio::test]
    async fn test_file_appender_daily_rotation() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_audit.log");
        
        let appender = FileAppender::new(
            &test_file,
            100,
            RotationStrategy::Daily,
        );

        let event = AuditEvent {
            timestamp: Utc::now(),
            level: LogLevel::Info,
            session_id: "test-session".to_string(),
            client_ip: "127.0.0.1".to_string(),
            method: "POST".to_string(),
            path: "/v1/chat/completions".to_string(),
            status_code: 200,
            duration_ms: 100,
            provider: "openai".to_string(),
            pii_detected: 2,
            error: None,
        };

        appender.append(&event.to_json()).await.unwrap();

        let _ = fs::remove_file(&test_file).await;
    }

    #[test]
    fn test_session_metadata_tracking() {
        let vault = PrivacyVault::new();
        let session_id = "test-session-001";

        vault
            .store(session_id, "token1".to_string(), "value1".to_string())
            .unwrap();

        let metadata = vault.get_session_metadata(session_id).unwrap();
        assert!(metadata.is_some());
        let meta = metadata.unwrap();
        assert_eq!(meta.request_count, 1);

        vault
            .store(session_id, "token2".to_string(), "value2".to_string())
            .unwrap();

        let metadata2 = vault.get_session_metadata(session_id).unwrap().unwrap();
        assert_eq!(metadata2.request_count, 2);
    }

    #[test]
    fn test_list_all_sessions_with_metadata() {
        let vault = PrivacyVault::new();

        vault
            .store("session-1", "token1".to_string(), "value1".to_string())
            .unwrap();
        vault
            .store("session-2", "token2".to_string(), "value2".to_string())
            .unwrap();

        let sessions = vault.list_all_sessions().unwrap();
        assert_eq!(sessions.len(), 2);
        assert!(sessions.contains_key("session-1"));
        assert!(sessions.contains_key("session-2"));
    }

    #[test]
    fn test_key_rotation_should_rotate() {
        let initial_key = [0u8; 32];
        let rotation = KeyRotation::new(initial_key, 0);

        assert!(rotation.should_rotate());
    }

    #[test]
    fn test_key_rotation_rotate_key() {
        let initial_key = [1u8; 32];
        let rotation = KeyRotation::new(initial_key, 90);

        let new_key = rotation.rotate_key().unwrap();
        assert_ne!(initial_key, new_key);

        let current = rotation.current_key().unwrap();
        assert_eq!(current, new_key);
    }

    #[test]
    fn test_vault_re_encrypt() {
        let vault = PrivacyVault::new();
        let session_id = "test-session";

        vault
            .store(session_id, "token1".to_string(), "value1".to_string())
            .unwrap();
        vault
            .store(session_id, "token2".to_string(), "value2".to_string())
            .unwrap();

        let new_key = [42u8; 32];
        let count = vault.re_encrypt_with_new_key(new_key).unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_audit_event_serialization() {
        let event = AuditEvent {
            timestamp: Utc::now(),
            level: LogLevel::Warn,
            session_id: "session-123".to_string(),
            client_ip: "192.168.1.1".to_string(),
            method: "GET".to_string(),
            path: "/sessions".to_string(),
            status_code: 401,
            duration_ms: 10,
            provider: "anthropic".to_string(),
            pii_detected: 0,
            error: Some("Unauthorized".to_string()),
        };

        let json = event.to_json();
        assert!(json.contains("\"level\":\"warn\""));
        assert!(json.contains("\"session_id\":\"session-123\""));
        assert!(json.contains("\"status_code\":401"));
    }
}
