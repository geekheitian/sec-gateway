use super::{PIIMatch, PIIType};
use regex::Regex;

const PHONE_PATTERN: &str = r"\b1[3-9]\d{9}\b";

pub fn detect_phone_number(text: &str) -> Vec<PIIMatch> {
    let re = Regex::new(PHONE_PATTERN).expect("Failed to compile phone regex");
    re.find_iter(text)
        .map(|m| {
            PIIMatch::new(
                PIIType::PhoneNumber,
                m.as_str().to_string(),
                m.start(),
                m.end(),
                1.0,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_valid_mobile() {
        let text = "Contact: 13812345678";
        let results = detect_phone_number(text);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "13812345678");
        assert_eq!(results[0].pii_type, PIIType::PhoneNumber);
    }

    #[test]
    fn test_detect_multiple_phones() {
        let text = "Phone1: 13812345678, Phone2: 18612345678";
        let results = detect_phone_number(text);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].value, "13812345678");
        assert_eq!(results[1].value, "18612345678");
    }

    #[test]
    fn test_detect_all_prefixes() {
        let prefixes = vec!["13", "14", "15", "16", "17", "18", "19"];
        for prefix in prefixes {
            let phone = format!("{}012345678", prefix);
            let results = detect_phone_number(&phone);
            assert_eq!(
                results.len(),
                1,
                "Failed to detect phone starting with {}",
                prefix
            );
            assert_eq!(results[0].value, phone);
        }
    }

    #[test]
    fn test_reject_invalid_prefix() {
        let text = "Invalid: 12012345678";
        let results = detect_phone_number(text);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_reject_wrong_length() {
        let text = "Too short: 1381234567, Too long: 138123456789";
        let results = detect_phone_number(text);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_detect_no_phone() {
        let text = "No phone here, just numbers 123456";
        let results = detect_phone_number(text);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_boundary_detection() {
        let text = "Valid:13812345678 Invalid:213812345678";
        let results = detect_phone_number(text);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "13812345678");
    }
}
