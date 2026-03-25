use super::{PIIMatch, PIIType};
use regex::Regex;

const EMAIL_PATTERN: &str = r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b";

pub fn detect_email(text: &str) -> Vec<PIIMatch> {
    let re = Regex::new(EMAIL_PATTERN).expect("Failed to compile email regex");
    re.find_iter(text)
        .map(|m| {
            PIIMatch::new(
                PIIType::Email,
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
    fn test_detect_simple_email() {
        let text = "Contact: user@example.com";
        let results = detect_email(text);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "user@example.com");
        assert_eq!(results[0].pii_type, PIIType::Email);
    }

    #[test]
    fn test_detect_multiple_emails() {
        let text = "Emails: alice@example.com, bob@test.org";
        let results = detect_email(text);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].value, "alice@example.com");
        assert_eq!(results[1].value, "bob@test.org");
    }

    #[test]
    fn test_detect_email_with_dots() {
        let text = "Email: first.last@company.co.uk";
        let results = detect_email(text);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "first.last@company.co.uk");
    }

    #[test]
    fn test_detect_email_with_numbers() {
        let text = "Support: user123@example456.com";
        let results = detect_email(text);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "user123@example456.com");
    }

    #[test]
    fn test_detect_email_with_plus() {
        let text = "Tagged: user+tag@gmail.com";
        let results = detect_email(text);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "user+tag@gmail.com");
    }

    #[test]
    fn test_reject_invalid_format() {
        let text = "Invalid: @example.com, user@, user@@example.com";
        let results = detect_email(text);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_detect_no_email() {
        let text = "No email here, just text";
        let results = detect_email(text);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_boundary_detection() {
        let text = "Valid:user@example.com Invalid:notanemail";
        let results = detect_email(text);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "user@example.com");
    }
}
