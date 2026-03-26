use super::{PIIMatch, PIIType};
use regex::Regex;

const JWT_PATTERN: &str = r"\b[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{2,}\.[A-Za-z0-9_-]{10,}\b";

pub fn detect_jwt(text: &str) -> Vec<PIIMatch> {
    let re = Regex::new(JWT_PATTERN).expect("Failed to compile JWT regex");
    re.find_iter(text)
        .map(|m| {
            PIIMatch::new(
                PIIType::JWT,
                m.as_str().to_string(),
                m.start(),
                m.end(),
                0.92,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_jwt() {
        let text = "token=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.e30.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let results = detect_jwt(text);
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].value,
            "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.e30.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c"
        );
        assert_eq!(results[0].pii_type, PIIType::JWT);
    }

    #[test]
    fn test_reject_invalid_jwt() {
        let text = "token=not.a.jwt.token";
        let results = detect_jwt(text);
        assert!(results.is_empty());
    }

    #[test]
    fn test_detect_realistic_jwt() {
        let text = "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.e30.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let results = detect_jwt(text);
        assert_eq!(results.len(), 1);
    }
}
