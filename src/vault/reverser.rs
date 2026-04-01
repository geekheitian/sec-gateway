use crate::vault::PrivacyVault;
use regex::Regex;
use std::collections::HashSet;
use std::sync::OnceLock;

static REDACTION_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();

pub struct Reverser {
    vault: PrivacyVault,
    fpe_key: [u8; 32],
}

impl Reverser {
    pub fn new(vault: PrivacyVault, fpe_key: [u8; 32]) -> Self {
        Self { vault, fpe_key }
    }

    pub fn restore_response(&self, session_id: &str, body: &str) -> Result<String, String> {
        let tokens = self.find_tokens_in_text(body);
        self.restore_response_with_allowlist(session_id, body, &tokens)
    }

    pub fn restore_response_with_allowlist(
        &self,
        session_id: &str,
        body: &str,
        allowed_tokens: &HashSet<String>,
    ) -> Result<String, String> {
        let mut result = body.to_string();

        let patterns = REDACTION_PATTERNS.get_or_init(|| {
            vec![
                Regex::new(r"\[REDACTED_CHINESE_ID_\d+\]").unwrap(),
                Regex::new(r"\[REDACTED_PHONE_NUMBER_\d+\]").unwrap(),
                Regex::new(r"\[REDACTED_CREDIT_CARD_\d+\]").unwrap(),
                Regex::new(r"\[REDACTED_EMAIL_\d+\]").unwrap(),
                Regex::new(r"\[REDACTED_IP_ADDRESS_\d+\]").unwrap(),
                Regex::new(r"\[REDACTED_DATABASE_CONNECTION_STRING_\d+\]").unwrap(),
                Regex::new(r"\[REDACTED_JWT_\d+\]").unwrap(),
                Regex::new(r"\[REDACTED_API_KEY_\d+\]").unwrap(),
                Regex::new(r"\[REDACTED_API_SECRET_\d+\]").unwrap(),
                Regex::new(r"\[REDACTED_AWS_ACCESS_KEY_\d+\]").unwrap(),
                Regex::new(r"\[REDACTED_AWS_SECRET_KEY_\d+\]").unwrap(),
                Regex::new(r"\[REDACTED_GITHUB_TOKEN_\d+\]").unwrap(),
                Regex::new(r"\[REDACTED_NER_PERSON_\d+\]").unwrap(),
                Regex::new(r"\[REDACTED_NER_LOCATION_\d+\]").unwrap(),
                Regex::new(r"\[REDACTED_NER_ORGANIZATION_\d+\]").unwrap(),
                Regex::new(r"\[HASH:[a-f0-9]{32}\]").unwrap(),
            ]
        });

        let mut all_matches: Vec<(usize, usize, String)> = Vec::new();

        for re in patterns.iter() {
            for m in re.find_iter(&result) {
                all_matches.push((m.start(), m.end(), m.as_str().to_string()));
            }
        }

        for token in allowed_tokens {
            if !token.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            let escaped = regex::escape(token);
            if let Ok(re) = Regex::new(&format!(r"\b{}\b", escaped)) {
                for m in re.find_iter(&result) {
                    all_matches.push((m.start(), m.end(), m.as_str().to_string()));
                }
            }
        }

        all_matches.sort_by_key(|&(start, _, _)| start);

        for (start, end, token) in all_matches.into_iter().rev() {
            if !allowed_tokens.contains(&token) {
                continue;
            }
            if let Some(original) = self.vault.retrieve(session_id, &token)? {
                result.replace_range(start..end, &original);
            }
        }

        Ok(result)
    }

    pub fn find_tokens_in_text(&self, text: &str) -> HashSet<String> {
        let mut tokens = HashSet::new();

        let patterns = REDACTION_PATTERNS.get_or_init(|| {
            vec![
                Regex::new(r"\[REDACTED_CHINESE_ID_\d+\]").unwrap(),
                Regex::new(r"\[REDACTED_PHONE_NUMBER_\d+\]").unwrap(),
                Regex::new(r"\[REDACTED_CREDIT_CARD_\d+\]").unwrap(),
                Regex::new(r"\[REDACTED_EMAIL_\d+\]").unwrap(),
                Regex::new(r"\[REDACTED_IP_ADDRESS_\d+\]").unwrap(),
                Regex::new(r"\[REDACTED_DATABASE_CONNECTION_STRING_\d+\]").unwrap(),
                Regex::new(r"\[REDACTED_JWT_\d+\]").unwrap(),
                Regex::new(r"\[REDACTED_API_KEY_\d+\]").unwrap(),
                Regex::new(r"\[REDACTED_API_SECRET_\d+\]").unwrap(),
                Regex::new(r"\[REDACTED_AWS_ACCESS_KEY_\d+\]").unwrap(),
                Regex::new(r"\[REDACTED_AWS_SECRET_KEY_\d+\]").unwrap(),
                Regex::new(r"\[REDACTED_GITHUB_TOKEN_\d+\]").unwrap(),
                Regex::new(r"\[REDACTED_NER_PERSON_\d+\]").unwrap(),
                Regex::new(r"\[REDACTED_NER_LOCATION_\d+\]").unwrap(),
                Regex::new(r"\[REDACTED_NER_ORGANIZATION_\d+\]").unwrap(),
                Regex::new(r"\[HASH:[a-f0-9]{32}\]").unwrap(),
            ]
        });

        for re in patterns.iter() {
            for m in re.find_iter(text) {
                tokens.insert(m.as_str().to_string());
            }
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
            if let Some(original) = self.vault.retrieve(session_id, token)? {
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
    fn test_find_tokens_does_not_match_raw_fpe_numbers() {
        let vault = PrivacyVault::new();
        let reverser = Reverser::new(vault, [0u8; 32]);

        let text = "Raw FPE numbers: 165455343746803619 and 284759384726584920";
        let tokens = reverser.find_tokens_in_text(text);

        assert!(!tokens.contains("165455343746803619"));
        assert!(!tokens.contains("284759384726584920"));
    }

    #[test]
    fn test_find_tokens_hash() {
        let vault = PrivacyVault::new();
        let reverser = Reverser::new(vault, [0u8; 32]);

        let text =
            "Hashed values: [HASH:e0b7453469e42b48e0b7453469e42b48] and [REDACTED_EMAIL_001]";
        let tokens = reverser.find_tokens_in_text(text);

        assert!(tokens.contains("[HASH:e0b7453469e42b48e0b7453469e42b48]"));
        assert!(tokens.contains("[REDACTED_EMAIL_001]"));
    }

    #[test]
    fn test_restore_response_with_allowlist_fpe() {
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
        let allowlist: HashSet<String> =
            vec!["165455343746803619".to_string()].into_iter().collect();

        let restored = reverser
            .restore_response_with_allowlist("session1", body, &allowlist)
            .unwrap();

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
    fn test_restore_response_multiple_tokens_with_allowlist() {
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
        let allowlist: HashSet<String> = vec![
            "165455343746803619".to_string(),
            "[REDACTED_EMAIL_001]".to_string(),
        ]
        .into_iter()
        .collect();

        let restored = reverser
            .restore_response_with_allowlist("session1", body, &allowlist)
            .unwrap();

        assert!(restored.contains("110101199001011234"));
        assert!(restored.contains("test@example.com"));
    }

    #[test]
    fn test_restore_response_with_allowlist_only_restores_listed_token() {
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
                "284759384726584920".to_string(),
                "320101198001011234".to_string(),
            )
            .unwrap();

        let reverser = Reverser::new(vault, [0u8; 32]);
        let body = r#"{"a":"165455343746803619","b":"284759384726584920"}"#;
        let allowlist: HashSet<String> =
            vec!["165455343746803619".to_string()].into_iter().collect();

        let restored = reverser
            .restore_response_with_allowlist("session1", body, &allowlist)
            .unwrap();

        assert!(restored.contains("110101199001011234"));
        assert!(restored.contains("284759384726584920"));
        assert!(!restored.contains("320101198001011234"));
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

    #[test]
    fn test_restore_response_with_allowlist_multiple_reverse_replacement_lengths() {
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
                "alice.very.long@example.com".to_string(),
            )
            .unwrap();

        let reverser = Reverser::new(vault, [0u8; 32]);
        let body = r#"{"id":"165455343746803619","email":"[REDACTED_EMAIL_001]"}"#;
        let allowlist: HashSet<String> = vec![
            "165455343746803619".to_string(),
            "[REDACTED_EMAIL_001]".to_string(),
        ]
        .into_iter()
        .collect();

        let restored = reverser
            .restore_response_with_allowlist("session1", body, &allowlist)
            .unwrap();

        assert!(restored.contains("110101199001011234"));
        assert!(restored.contains("alice.very.long@example.com"));
    }
}
