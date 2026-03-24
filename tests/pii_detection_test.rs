use sec_gateway::detector::{
    chinese_id::detect_chinese_id,
    phone::detect_phone_number,
    email::detect_email,
    api_key::detect_api_keys,
    PIIType,
};

#[test]
fn test_chinese_id_detection_accuracy() {
    let valid_ids = vec![
        "110101199001011234",
        "320101198001011234",
        "440101199501011234",
    ];
    
    for id in valid_ids {
        let text = format!("ID: {}", id);
        let results = detect_chinese_id(&text);
        assert_eq!(results.len(), 1, "Failed to detect valid ID: {}", id);
        assert_eq!(results[0].2, id);
    }
}

#[test]
fn test_chinese_id_no_false_positives() {
    let invalid_ids = vec![
        "123456789012345678",
        "11010119900101123",
        "110101199001011235X",
    ];
    
    for id in invalid_ids {
        let text = format!("ID: {}", id);
        let results = detect_chinese_id(&text);
        assert_eq!(results.len(), 0, "False positive for: {}", id);
    }
}

#[test]
fn test_phone_number_detection_accuracy() {
    let valid_phones = vec![
        "13812345678",
        "15912345678",
        "18812345678",
    ];
    
    for phone in valid_phones {
        let text = format!("Phone: {}", phone);
        let results = detect_phone_number(&text);
        assert_eq!(results.len(), 1, "Failed to detect valid phone: {}", phone);
        assert_eq!(results[0].value, phone);
    }
}

#[test]
fn test_phone_number_no_false_positives() {
    let invalid_phones = vec![
        "12812345678",
        "1381234567",
        "138123456789",
    ];
    
    for phone in invalid_phones {
        let text = format!("Phone: {}", phone);
        let results = detect_phone_number(&text);
        assert_eq!(results.len(), 0, "False positive for phone: {}", phone);
    }
}

#[test]
fn test_email_detection_accuracy() {
    let valid_emails = vec![
        "test@example.com",
        "user.name@domain.co.uk",
        "admin+tag@company.org",
    ];
    
    for email in valid_emails {
        let text = format!("Email: {}", email);
        let results = detect_email(&text);
        assert_eq!(results.len(), 1, "Failed to detect valid email: {}", email);
        assert_eq!(results[0].value, email);
    }
}

#[test]
fn test_email_no_false_positives() {
    let invalid_emails = vec![
        "notanemail",
        "@example.com",
        "user@",
    ];
    
    for email in invalid_emails {
        let text = format!("Email: {}", email);
        let results = detect_email(&text);
        assert_eq!(results.len(), 0, "False positive for email: {}", email);
    }
}

#[test]
fn test_api_key_openai_detection() {
    let valid_keys = vec![
        "sk-proj-AbCdEf1234567890XyZ",
        "sk-1234567890abcdefghij",
    ];
    
    for key in valid_keys {
        let text = format!("Key: {}", key);
        let results = detect_api_keys(&text);
        assert!(!results.is_empty(), "Failed to detect OpenAI key: {}", key);
        assert_eq!(results[0].pii_type, PIIType::APIKey);
    }
}

#[test]
fn test_api_key_github_detection() {
    let valid_tokens = vec![
        "ghp_1234567890abcdefghijklmnopqrstuvwxyz",
    ];
    
    for token in valid_tokens {
        let text = format!("Token: {}", token);
        let results = detect_api_keys(&text);
        assert!(!results.is_empty(), "Failed to detect GitHub token: {}", token);
        assert_eq!(results[0].pii_type, PIIType::GitHubToken);
    }
}

#[test]
fn test_api_key_aws_access_detection() {
    let valid_keys = vec![
        "AKIAIOSFODNN7EXAMPLE",
    ];
    
    for key in valid_keys {
        let text = format!("AWS Key: {}", key);
        let results = detect_api_keys(&text);
        assert!(!results.is_empty(), "Failed to detect AWS access key: {}", key);
        assert_eq!(results[0].pii_type, PIIType::AWSAccessKey);
    }
}

#[test]
fn test_multiple_pii_types_in_single_text() {
    let text = "Contact: 13812345678, ID: 110101199001011234, Email: test@example.com";
    
    let mut all_detections = Vec::new();
    
    let id_results = detect_chinese_id(text);
    all_detections.extend(id_results.into_iter().map(|(start, end, value)| (PIIType::ChineseID, start, end, value)));
    
    let phone_results = detect_phone_number(text);
    all_detections.extend(phone_results.into_iter().map(|m| (m.pii_type, m.start, m.end, m.value)));
    
    let email_results = detect_email(text);
    all_detections.extend(email_results.into_iter().map(|m| (m.pii_type, m.start, m.end, m.value)));
    
    assert_eq!(all_detections.len(), 3, "Should detect all 3 PII types");
    assert!(all_detections.iter().any(|(t, _, _, _)| *t == PIIType::PhoneNumber));
    assert!(all_detections.iter().any(|(t, _, _, _)| *t == PIIType::ChineseID));
    assert!(all_detections.iter().any(|(t, _, _, _)| *t == PIIType::Email));
}

#[test]
fn test_overlapping_detection_priority() {
    let text = "Key: sk-proj-1234567890abcdefghij and another sk-test-xyz";
    let results = detect_api_keys(text);
    
    assert!(results.len() >= 1, "Should detect at least one API key");
}

#[test]
fn test_detection_with_chinese_text() {
    let text = "我的邮箱是 user@example.com，手机号是 13812345678";
    
    let email_results = detect_email(text);
    let phone_results = detect_phone_number(text);
    
    assert_eq!(email_results.len(), 1, "Should detect email in Chinese text");
    assert_eq!(phone_results.len(), 1, "Should detect phone in Chinese text");
}

#[test]
fn test_detection_in_json_payload() {
    let json = r#"{"user":{"phone":"13812345678","email":"test@example.com","id":"110101199001011234"}}"#;
    
    let phone_results = detect_phone_number(json);
    let email_results = detect_email(json);
    let id_results = detect_chinese_id(json);
    
    assert_eq!(phone_results.len(), 1, "Should detect phone in JSON");
    assert_eq!(email_results.len(), 1, "Should detect email in JSON");
    assert_eq!(id_results.len(), 1, "Should detect ID in JSON");
}

#[test]
fn test_empty_string() {
    let text = "";
    
    assert_eq!(detect_chinese_id(text).len(), 0);
    assert_eq!(detect_phone_number(text).len(), 0);
    assert_eq!(detect_email(text).len(), 0);
    assert_eq!(detect_api_keys(text).len(), 0);
}

#[test]
fn test_no_pii_in_normal_text() {
    let text = "This is a normal sentence without any PII information.";
    
    assert_eq!(detect_chinese_id(text).len(), 0);
    assert_eq!(detect_phone_number(text).len(), 0);
    assert_eq!(detect_email(text).len(), 0);
    assert_eq!(detect_api_keys(text).len(), 0);
}

#[test]
fn test_detection_accuracy_rate() {
    let test_cases = vec![
        ("Phone: 13812345678", vec![PIIType::PhoneNumber]),
        ("ID: 110101199001011234", vec![PIIType::ChineseID]),
        ("Email: test@example.com", vec![PIIType::Email]),
        ("Key: sk-proj-AbCdEf1234567890XyZ", vec![PIIType::APIKey]),
        ("No PII here", vec![]),
    ];
    
    let mut correct = 0;
    let total = test_cases.len();
    
    for (text, expected_types) in test_cases {
        let mut detected_types = Vec::new();
        
        if !detect_chinese_id(text).is_empty() {
            detected_types.push(PIIType::ChineseID);
        }
        if !detect_phone_number(text).is_empty() {
            detected_types.push(PIIType::PhoneNumber);
        }
        if !detect_email(text).is_empty() {
            detected_types.push(PIIType::Email);
        }
        let api_results = detect_api_keys(text);
        if !api_results.is_empty() {
            for result in api_results {
                if !detected_types.contains(&result.pii_type) {
                    detected_types.push(result.pii_type);
                }
            }
        }
        
        if detected_types.len() == expected_types.len() 
            && expected_types.iter().all(|t| detected_types.contains(t)) {
            correct += 1;
        }
    }
    
    let accuracy = (correct as f64 / total as f64) * 100.0;
    assert!(accuracy >= 95.0, "Accuracy {} is below 95%", accuracy);
}
