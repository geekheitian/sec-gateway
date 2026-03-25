use std::collections::HashMap;

pub fn calculate_shannon_entropy(text: &str) -> f64 {
    if text.is_empty() {
        return 0.0;
    }

    let mut freq_map: HashMap<char, usize> = HashMap::new();
    for c in text.chars() {
        *freq_map.entry(c).or_insert(0) += 1;
    }

    let len = text.len() as f64;
    let mut entropy = 0.0;

    for count in freq_map.values() {
        let probability = *count as f64 / len;
        entropy -= probability * probability.log2();
    }

    entropy
}

pub fn is_high_entropy(text: &str, threshold: f64) -> bool {
    calculate_shannon_entropy(text) > threshold
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_entropy() {
        let text = "aaaaaaa";
        let entropy = calculate_shannon_entropy(text);
        assert!(entropy < 0.1, "Expected near-zero entropy, got {}", entropy);
    }

    #[test]
    fn test_low_entropy() {
        let text = "hello";
        let entropy = calculate_shannon_entropy(text);
        assert!(entropy < 3.0, "Expected low entropy, got {}", entropy);
    }

    #[test]
    fn test_high_entropy_api_key() {
        let text = "sk-proj-AbCdEf1234567890XyZaBcDeF1234567890";
        let entropy = calculate_shannon_entropy(text);
        assert!(
            entropy > 4.5,
            "Expected high entropy for API key, got {}",
            entropy
        );
    }

    #[test]
    fn test_high_entropy_random() {
        let text = "Xy9Kq3Lm8Np2Wr5Zt7Bv4Gc6Hd1Jf0";
        let entropy = calculate_shannon_entropy(text);
        assert!(
            entropy > 4.5,
            "Expected high entropy for random string, got {}",
            entropy
        );
    }

    #[test]
    fn test_is_high_entropy_threshold() {
        let high_entropy_text = "AbCdEf1234567890XyZaBcDeF1234567890";
        let low_entropy_text = "password123";

        assert!(is_high_entropy(high_entropy_text, 4.5));
        assert!(!is_high_entropy(low_entropy_text, 4.5));
    }

    #[test]
    fn test_empty_string() {
        let text = "";
        let entropy = calculate_shannon_entropy(text);
        assert_eq!(entropy, 0.0);
    }

    #[test]
    fn test_real_openai_key_pattern() {
        let text = "sk-proj-1a2b3c4d5e6f7g8h9i0j";
        let entropy = calculate_shannon_entropy(text);
        assert!(
            entropy > 4.0,
            "OpenAI key should have entropy > 4.0, got {}",
            entropy
        );
    }

    #[test]
    fn test_real_github_token_pattern() {
        let text = "ghp_1A2b3C4d5E6f7G8h9I0j1K2l3M4n5O6p7Q8r";
        let entropy = calculate_shannon_entropy(text);
        assert!(
            entropy > 4.0,
            "GitHub token should have entropy > 4.0, got {}",
            entropy
        );
    }

    #[test]
    fn test_aws_secret_pattern() {
        let text = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let entropy = calculate_shannon_entropy(text);
        assert!(
            entropy > 4.5,
            "AWS secret should have entropy > 4.5, got {}",
            entropy
        );
    }
}
