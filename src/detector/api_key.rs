use regex::Regex;
use super::{PIIMatch, PIIType, entropy};

const OPENAI_KEY_PATTERN: &str = r"\bsk-[A-Za-z0-9\-]{20,}\b";
const GITHUB_TOKEN_PATTERN: &str = r"\bghp_[A-Za-z0-9]{36,}\b";
const AWS_ACCESS_KEY_PATTERN: &str = r"\bAKIA[0-9A-Z]{16}\b";
const AWS_SECRET_KEY_PATTERN: &str = r"\b[A-Za-z0-9/+=]{40}\b";

const ENTROPY_THRESHOLD: f64 = 4.5;

pub fn detect_api_keys(text: &str) -> Vec<PIIMatch> {
    let mut matches = Vec::new();

    matches.extend(detect_openai_keys(text));
    matches.extend(detect_github_tokens(text));
    matches.extend(detect_aws_access_keys(text));
    matches.extend(detect_aws_secret_keys(text));

    matches
}

fn detect_openai_keys(text: &str) -> Vec<PIIMatch> {
    let re = Regex::new(OPENAI_KEY_PATTERN).expect("Failed to compile OpenAI key regex");
    re.find_iter(text)
        .map(|m| {
            PIIMatch::new(
                PIIType::APIKey,
                m.as_str().to_string(),
                m.start(),
                m.end(),
                1.0,
            )
        })
        .collect()
}

fn detect_github_tokens(text: &str) -> Vec<PIIMatch> {
    let re = Regex::new(GITHUB_TOKEN_PATTERN).expect("Failed to compile GitHub token regex");
    re.find_iter(text)
        .map(|m| {
            PIIMatch::new(
                PIIType::GitHubToken,
                m.as_str().to_string(),
                m.start(),
                m.end(),
                1.0,
            )
        })
        .collect()
}

fn detect_aws_access_keys(text: &str) -> Vec<PIIMatch> {
    let re = Regex::new(AWS_ACCESS_KEY_PATTERN).expect("Failed to compile AWS access key regex");
    re.find_iter(text)
        .map(|m| {
            PIIMatch::new(
                PIIType::AWSAccessKey,
                m.as_str().to_string(),
                m.start(),
                m.end(),
                1.0,
            )
        })
        .collect()
}

fn detect_aws_secret_keys(text: &str) -> Vec<PIIMatch> {
    let re = Regex::new(AWS_SECRET_KEY_PATTERN).expect("Failed to compile AWS secret key regex");
    re.find_iter(text)
        .filter(|m| entropy::is_high_entropy(m.as_str(), ENTROPY_THRESHOLD))
        .map(|m| {
            PIIMatch::new(
                PIIType::AWSSecretKey,
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
    fn test_detect_openai_key() {
        let text = "API Key: sk-proj-AbCdEf1234567890XyZ";
        let results = detect_api_keys(text);
        assert!(results.iter().any(|m| m.pii_type == PIIType::APIKey));
        assert_eq!(results.iter().filter(|m| m.pii_type == PIIType::APIKey).count(), 1);
    }

    #[test]
    fn test_detect_github_token() {
        let text = "Token: ghp_1A2b3C4d5E6f7G8h9I0j1K2l3M4n5O6p7Q8r";
        let results = detect_api_keys(text);
        assert!(results.iter().any(|m| m.pii_type == PIIType::GitHubToken));
    }

    #[test]
    fn test_detect_aws_access_key() {
        let text = "Access: AKIAIOSFODNN7EXAMPLE";
        let results = detect_api_keys(text);
        assert!(results.iter().any(|m| m.pii_type == PIIType::AWSAccessKey));
    }

    #[test]
    fn test_detect_aws_secret_key() {
        let text = "Secret: wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let results = detect_api_keys(text);
        assert!(results.iter().any(|m| m.pii_type == PIIType::AWSSecretKey));
    }

    #[test]
    fn test_detect_multiple_keys() {
        let text = "OpenAI: sk-proj-AbCdEf1234567890XyZ, GitHub: ghp_1A2b3C4d5E6f7G8h9I0j1K2l3M4n5O6p7Q8r";
        let results = detect_api_keys(text);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_no_false_positive_short_string() {
        let text = "This is just normal text with sk- prefix but nothing more";
        let results = detect_api_keys(text);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_aws_secret_entropy_filter() {
        let low_entropy = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let results = detect_aws_secret_keys(low_entropy);
        assert_eq!(results.len(), 0, "Low entropy string should not be detected");
    }

    #[test]
    fn test_boundary_detection() {
        let text = "Valid:sk-proj-AbCdEf1234567890XyZ Invalid:sk-short";
        let results = detect_api_keys(text);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_real_openai_format() {
        let text = "OPENAI_API_KEY=sk-proj-1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p";
        let results = detect_api_keys(text);
        assert!(results.iter().any(|m| m.pii_type == PIIType::APIKey));
    }

    #[test]
    fn test_real_github_format() {
        let text = "GITHUB_TOKEN=ghp_A1b2C3d4E5f6G7h8I9j0K1l2M3n4O5p6Q7r8";
        let results = detect_api_keys(text);
        assert!(results.iter().any(|m| m.pii_type == PIIType::GitHubToken));
    }
}
