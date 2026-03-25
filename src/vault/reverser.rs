use crate::detector::PIIType;
use crate::vault::PrivacyVault;
use regex::Regex;
use std::collections::HashSet;

pub struct Reverser {
    vault: PrivacyVault,
    fpe_key: [u8; 32],
}

impl Reverser {
    pub fn new(vault: PrivacyVault, fpe_key: [u8; 32]) -> Self {
        Self { vault, fpe_key }
    }

    pub fn restore_response(&self, session_id: &str, body: &str) -> Result<String, String> {
        let mut result = body.to_string();
        let mut offset: i64 = 0;

        let patterns = self.get_restore_patterns();

        for pattern in patterns {
            let re = Regex::new(&pattern.0).map_err(|e| format!("Invalid regex: {}", e))?;

            let matches: Vec<_> = re
                .find_iter(&result)
                .map(|m| (m.start(), m.end(), m.as_str().to_string()))
                .collect();

            for (start, end, token) in matches.into_iter().rev() {
                if let Some(original) = self.vault.retrieve(session_id, &token).map_err(|e| e)? {
                    let adj_start = (start as i64 + offset) as usize;
                    let adj_end = (end as i64 + offset) as usize;
                    result.replace_range(adj_start..adj_end, &original);
                    offset += original.len() as i64 - (end - start) as i64;
                }
            }
        }

        Ok(result)
    }

    fn get_restore_patterns(&self) -> Vec<(String, PIIType)> {
        vec![
            (r"\b\d{18}\b".to_string(), PIIType::ChineseID),
            (r"\b1[3-9]\d{9}\b".to_string(), PIIType::PhoneNumber),
            (r"\[REDACTED_EMAIL_\d+\]".to_string(), PIIType::Email),
            (r"\[HASH:[a-f0-9]{16}\]".to_string(), PIIType::APIKey),
        ]
    }

    pub fn find_tokens_in_text(&self, text: &str) -> HashSet<String> {
        let mut tokens = HashSet::new();

        let fpe_pattern = Regex::new(r"\b\d{18}\b").unwrap();
        for m in fpe_pattern.find_iter(text) {
            tokens.insert(m.as_str().to_string());
        }

        let email_pattern = Regex::new(r"\[REDACTED_EMAIL_\d+\]").unwrap();
        for m in email_pattern.find_iter(text) {
            tokens.insert(m.as_str().to_string());
        }

        let hash_pattern = Regex::new(r"\[HASH:[a-f0-9]{16}\]").unwrap();
        for m in hash_pattern.find_iter(text) {
            tokens.insert(m.as_str().to_string());
        }

        tokens
    }

    pub fn restore_tokens(
        &self,
        session_id: &str,
        tokens: &HashSet<String>,
    ) -> Result<HashSet<String>, String> {
        let mut restored = HashSet::new();

        for token in tokens {
            if let Some(original) = self.vault.retrieve(session_id, token).map_err(|e| e)? {
                restored.insert(original);
            }
        }

        Ok(restored)
    }
}

impl Clone for Reverser {
    fn clone(&self) -> Self {
        Self {
            vault: self.vault.clone(),
            fpe_key: self.fpe_key,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_tokens_fpe() {
        let vault = PrivacyVault::new();
        let reverser = Reverser::new(vault, [0u8; 32]);

        let text = "Encrypted ID: 165455343746803619 and another: 284759384726584920";
        let tokens = reverser.find_tokens_in_text(text);

        assert!(tokens.contains("165455343746803619"));
        assert!(tokens.contains("284759384726584920"));
    }

    #[test]
    fn test_find_tokens_hash() {
        let vault = PrivacyVault::new();
        let reverser = Reverser::new(vault, [0u8; 32]);

        let text = "Hashed values: [HASH:e0b7453469e42b48] and [REDACTED_EMAIL_001]";
        let tokens = reverser.find_tokens_in_text(text);

        assert!(tokens.contains("[HASH:e0b7453469e42b48]"));
        assert!(tokens.contains("[REDACTED_EMAIL_001]"));
    }

    #[test]
    fn test_restore_response_basic() {
        let vault = PrivacyVault::new();
        vault
            .store(
                "session1",
                "165455343746803619".to_string(),
                "110101199001011234".to_string(),
            )
            .unwrap();

        let reverser = Reverser::new(vault, [0u8; 32]);
        let body = r#"{"id":"165455343746803619","type":"user"}"#;

        let restored = reverser.restore_response("session1", body).unwrap();

        assert!(restored.contains("110101199001011234"));
        assert!(!restored.contains("165455343746803619"));
    }

    #[test]
    fn test_restore_response_email() {
        let vault = PrivacyVault::new();
        vault
            .store(
                "session1",
                "[REDACTED_EMAIL_001]".to_string(),
                "test@example.com".to_string(),
            )
            .unwrap();

        let reverser = Reverser::new(vault, [0u8; 32]);
        let body = r#"{"email":"[REDACTED_EMAIL_001]"}"#;

        let restored = reverser.restore_response("session1", body).unwrap();

        assert!(restored.contains("test@example.com"));
        assert!(!restored.contains("[REDACTED_EMAIL_001]"));
    }

    #[test]
    fn test_restore_response_multiple_tokens() {
        let vault = PrivacyVault::new();
        vault
            .store(
                "session1",
                "165455343746803619".to_string(),
                "110101199001011234".to_string(),
            )
            .unwrap();
        vault
            .store(
                "session1",
                "[REDACTED_EMAIL_001]".to_string(),
                "test@example.com".to_string(),
            )
            .unwrap();

        let reverser = Reverser::new(vault, [0u8; 32]);
        let body = r#"{"id":"165455343746803619","email":"[REDACTED_EMAIL_001]"}"#;

        let restored = reverser.restore_response("session1", body).unwrap();

        assert!(restored.contains("110101199001011234"));
        assert!(restored.contains("test@example.com"));
    }

    #[test]
    fn test_restore_tokens_bulk() {
        let vault = PrivacyVault::new();
        vault
            .store(
                "session1",
                "165455343746803619".to_string(),
                "value1".to_string(),
            )
            .unwrap();
        vault
            .store(
                "session1",
                "[REDACTED_EMAIL_001]".to_string(),
                "value2".to_string(),
            )
            .unwrap();
        vault
            .store(
                "session1",
                "[HASH:abcdef1234567890]".to_string(),
                "value3".to_string(),
            )
            .unwrap();

        let reverser = Reverser::new(vault, [0u8; 32]);
        let tokens: HashSet<String> = vec![
            "165455343746803619".to_string(),
            "[REDACTED_EMAIL_001]".to_string(),
            "[HASH:abcdef1234567890]".to_string(),
        ]
        .into_iter()
        .collect();

        let restored = reverser.restore_tokens("session1", &tokens).unwrap();

        assert!(restored.contains("value1"));
        assert!(restored.contains("value2"));
        assert!(restored.contains("value3"));
        assert_eq!(restored.len(), 3);
    }
}
