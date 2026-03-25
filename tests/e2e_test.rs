use sec_gateway::crypto::fpe::FPECipher;
use sec_gateway::detector::{
    api_key::detect_api_keys, chinese_id::detect_chinese_id, email::detect_email,
    phone::detect_phone_number, PIIMatch, PIIType,
};
use sec_gateway::masker::hash::hash_value;
use sec_gateway::vault::PrivacyVault;

#[test]
fn test_e2e_pii_masking_flow_chinese_id() {
    let fpe_key = [0u8; 32];
    let cipher = FPECipher::new(&fpe_key, 10).unwrap();
    let vault = PrivacyVault::new();

    let body = r#"{"messages":[{"role":"user","content":"My ID is 110101199001011234"}]}"#;
    let session_id = "test-session-e2e-001";

    let detections: Vec<PIIMatch> = detect_chinese_id(body)
        .into_iter()
        .map(|(start, end, value)| PIIMatch::new(PIIType::ChineseID, value, start, end, 1.0))
        .collect();

    assert_eq!(detections.len(), 1, "Should detect 1 Chinese ID");
    assert_eq!(detections[0].value, "110101199001011234");

    let mut masked_body = body.to_string();
    let mut offset: i64 = 0;

    for (idx, pii) in detections.iter().enumerate() {
        let token = cipher.encrypt(&pii.value, session_id.as_bytes()).unwrap();
        vault
            .store(session_id, token.clone(), pii.value.clone())
            .unwrap();

        let start = (pii.start as i64 + offset) as usize;
        let end = (pii.end as i64 + offset) as usize;
        masked_body.replace_range(start..end, &token);
        offset += token.len() as i64 - (pii.end - pii.start) as i64;

        assert!(
            !masked_body.contains(&pii.value),
            "Original PII should NOT be in masked body"
        );
        assert_ne!(
            token, "110101199001011234",
            "Encrypted ID should differ from original"
        );
        assert_eq!(token.len(), 18, "FPE preserves format length");
    }

    let retrieved = vault
        .retrieve(
            session_id,
            &cipher
                .encrypt("110101199001011234", session_id.as_bytes())
                .unwrap(),
        )
        .unwrap();
    assert_eq!(retrieved, Some("110101199001011234".to_string()));
}

#[test]
fn test_e2e_pii_masking_flow_phone() {
    let fpe_key = [0u8; 32];
    let cipher = FPECipher::new(&fpe_key, 10).unwrap();
    let vault = PrivacyVault::new();

    let body = r#"{"content":"Call me at 13812345678"}"#;
    let session_id = "test-session-e2e-002";

    let detections = detect_phone_number(body);
    assert_eq!(detections.len(), 1);
    assert_eq!(detections[0].value, "13812345678");

    let mut masked_body = body.to_string();
    let mut offset: i64 = 0;

    for pii in &detections {
        let token = cipher.encrypt(&pii.value, session_id.as_bytes()).unwrap();
        vault
            .store(session_id, token.clone(), pii.value.clone())
            .unwrap();

        let start = (pii.start as i64 + offset) as usize;
        let end = (pii.end as i64 + offset) as usize;
        masked_body.replace_range(start..end, &token);
        offset += token.len() as i64 - (pii.end - pii.start) as i64;

        assert!(
            !masked_body.contains("13812345678"),
            "Original phone should NOT be in masked body"
        );
    }

    let retrieved = vault
        .retrieve(
            session_id,
            &cipher
                .encrypt("13812345678", session_id.as_bytes())
                .unwrap(),
        )
        .unwrap();
    assert_eq!(retrieved, Some("13812345678".to_string()));
}

#[test]
fn test_e2e_pii_masking_flow_api_key() {
    let vault = PrivacyVault::new();

    let body = r#"{"api_key":"sk-proj-AbCdEf1234567890XyZ"}"#;
    let session_id = "test-session-e2e-003";

    let detections = detect_api_keys(body);
    assert!(!detections.is_empty());
    assert_eq!(detections[0].pii_type, PIIType::APIKey);

    let mut masked_body = body.to_string();
    let mut offset: i64 = 0;

    for pii in &detections {
        let token = hash_value(&pii.value);
        vault
            .store(session_id, token.clone(), pii.value.clone())
            .unwrap();

        let start = (pii.start as i64 + offset) as usize;
        let end = (pii.end as i64 + offset) as usize;
        masked_body.replace_range(start..end, &token);
        offset += token.len() as i64 - (pii.end - pii.start) as i64;

        assert!(
            !masked_body.contains("sk-proj-AbCdEf1234567890XyZ"),
            "Original API key should NOT be in masked body"
        );
    }

    assert!(
        masked_body.contains("[HASH:"),
        "Should contain hash placeholder"
    );

    let hash_token = hash_value("sk-proj-AbCdEf1234567890XyZ");
    let retrieved = vault.retrieve(session_id, &hash_token).unwrap();
    assert_eq!(retrieved, Some("sk-proj-AbCdEf1234567890XyZ".to_string()));
}

#[test]
fn test_e2e_multiple_pii_types_single_request() {
    let fpe_key = [0u8; 32];
    let cipher = FPECipher::new(&fpe_key, 10).unwrap();
    let vault = PrivacyVault::new();

    let body = r#"{"messages":[{"role":"user","content":"ID:110101199001011234 Phone:13812345678 Email:test@example.com Key:sk-proj-Test1234567890abcdef"}]}"#;
    let session_id = "test-session-e2e-004";

    let mut all_matches: Vec<PIIMatch> = Vec::new();
    all_matches.extend(
        detect_chinese_id(body)
            .into_iter()
            .map(|(s, e, v)| PIIMatch::new(PIIType::ChineseID, v, s, e, 1.0)),
    );
    all_matches.extend(detect_phone_number(body));
    all_matches.extend(detect_email(body));
    all_matches.extend(detect_api_keys(body));

    assert_eq!(all_matches.len(), 4, "Should detect all 4 PII types");

    all_matches.sort_by_key(|m| m.start);

    let mut masked_body = body.to_string();
    let mut offset: i64 = 0;

    for pii in &all_matches {
        let token = match pii.pii_type {
            PIIType::ChineseID | PIIType::PhoneNumber => {
                cipher.encrypt(&pii.value, session_id.as_bytes()).unwrap()
            }
            PIIType::APIKey
            | PIIType::GitHubToken
            | PIIType::AWSAccessKey
            | PIIType::AWSSecretKey
            | PIIType::APISecret => hash_value(&pii.value),
            PIIType::Email => format!("[REDACTED_EMAIL]"),
        };

        vault
            .store(session_id, token.clone(), pii.value.clone())
            .unwrap();

        let start = (pii.start as i64 + offset) as usize;
        let end = (pii.end as i64 + offset) as usize;
        masked_body.replace_range(start..end, &token);
        offset += token.len() as i64 - (pii.end - pii.start) as i64;
    }

    assert!(
        !masked_body.contains("110101199001011234"),
        "Original ID should NOT be in masked body"
    );
    assert!(
        !masked_body.contains("13812345678"),
        "Original phone should NOT be in masked body"
    );
    assert!(
        !masked_body.contains("test@example.com"),
        "Original email should NOT be in masked body"
    );
    assert!(
        !masked_body.contains("sk-proj-Test1234567890abcdef"),
        "Original API key should NOT be in masked body"
    );

    assert_eq!(vault.token_count(session_id), 4);
}

#[test]
fn test_e2e_session_isolation() {
    let vault = PrivacyVault::new();

    let session1 = "session-001";
    let session2 = "session-002";

    vault
        .store(session1, "token1".to_string(), "value1".to_string())
        .unwrap();
    vault
        .store(session2, "token1".to_string(), "value2".to_string())
        .unwrap();

    assert_eq!(
        vault.retrieve(session1, "token1").unwrap(),
        Some("value1".to_string())
    );
    assert_eq!(
        vault.retrieve(session2, "token1").unwrap(),
        Some("value2".to_string())
    );
    assert_eq!(vault.retrieve(session1, "token2").unwrap(), None);
}

#[test]
fn test_e2e_zero_data_leakage() {
    let fpe_key = [0u8; 32];
    let cipher = FPECipher::new(&fpe_key, 10).unwrap();

    let original_pii_values = vec![
        "110101199001011234",
        "13812345678",
        "test@example.com",
        "sk-proj-AbCdEf1234567890XyZ",
        "ghp_abcdefghijklmnopqrstuvwxyz1234567890",
    ];

    for pii_value in original_pii_values {
        let body = format!(r#"{{"sensitive":"{}"}}"#, pii_value);

        let mut all_matches: Vec<PIIMatch> = Vec::new();
        all_matches.extend(
            detect_chinese_id(&body)
                .into_iter()
                .map(|(s, e, v)| PIIMatch::new(PIIType::ChineseID, v, s, e, 1.0)),
        );
        all_matches.extend(detect_phone_number(&body));
        all_matches.extend(detect_email(&body));
        all_matches.extend(detect_api_keys(&body));

        if !all_matches.is_empty() {
            let mut masked_body = body.clone();
            let mut offset: i64 = 0;

            for pii in &all_matches {
                let token = match pii.pii_type {
                    PIIType::ChineseID | PIIType::PhoneNumber => {
                        cipher.encrypt(&pii.value, b"test-session").unwrap()
                    }
                    _ => hash_value(&pii.value),
                };

                let start = (pii.start as i64 + offset) as usize;
                let end = (pii.end as i64 + offset) as usize;
                masked_body.replace_range(start..end, &token);
                offset += token.len() as i64 - (pii.end - pii.start) as i64;
            }

            assert!(
                !masked_body.contains(pii_value),
                "PII '{}' should NOT appear in masked body",
                pii_value
            );
        }
    }
}

#[test]
fn test_e2e_fpe_deterministic_encryption() {
    let fpe_key = [0u8; 32];
    let cipher = FPECipher::new(&fpe_key, 10).unwrap();

    let id = "110101199001011234";

    let encrypted1 = cipher.encrypt(id, b"test-session").unwrap();
    let encrypted2 = cipher.encrypt(id, b"test-session").unwrap();

    assert_eq!(encrypted1, encrypted2, "FPE should be deterministic");
    assert_ne!(encrypted1, id, "Encrypted should differ from original");
    assert_eq!(encrypted1.len(), id.len(), "FPE preserves format");
}

#[test]
fn test_e2e_vault_bulk_operations() {
    let vault = PrivacyVault::new();
    let session_id = "test-bulk-session";

    for i in 0..100 {
        let token = format!("[TOKEN_{:03}]", i);
        let value = format!("original_value_{}", i);
        vault
            .store(session_id, token.clone(), value.clone())
            .unwrap();
    }

    assert_eq!(vault.token_count(session_id), 100);

    for i in 0..100 {
        let token = format!("[TOKEN_{:03}]", i);
        let expected_value = format!("original_value_{}", i);
        assert_eq!(
            vault.retrieve(session_id, &token).unwrap(),
            Some(expected_value)
        );
    }
}
