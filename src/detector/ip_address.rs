use super::{PIIMatch, PIIType};
use regex::Regex;

const IPV4_PATTERN: &str =
    r"\b(?:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\.){3}(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\b";

pub fn detect_ip_address(text: &str) -> Vec<PIIMatch> {
    let re = Regex::new(IPV4_PATTERN).expect("Failed to compile IPv4 regex");
    re.find_iter(text)
        .map(|m| {
            PIIMatch::new(
                PIIType::IPAddress,
                m.as_str().to_string(),
                m.start(),
                m.end(),
                0.95,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_valid_ipv4() {
        let text = "Server IP: 192.168.1.1";
        let results = detect_ip_address(text);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "192.168.1.1");
        assert_eq!(results[0].pii_type, PIIType::IPAddress);
    }

    #[test]
    fn test_reject_invalid_ipv4() {
        let text = "Bad IP: 256.100.100.100";
        let results = detect_ip_address(text);
        assert!(results.is_empty());
    }

    #[test]
    fn test_detect_multiple_ipv4() {
        let text = "IPs: 10.0.0.1 and 8.8.8.8";
        let results = detect_ip_address(text);
        assert_eq!(results.len(), 2);
    }
}
