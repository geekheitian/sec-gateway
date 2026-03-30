use sha2::{Digest, Sha256};

pub fn hash_value(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    let result = hasher.finalize();
    format!("[HASH:{}]", hex::encode(&result[..16]))
}

mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_consistency() {
        let value = "sk-proj-AbCdEf1234567890XyZ";
        let hash1 = hash_value(value);
        let hash2 = hash_value(value);
        assert_eq!(hash1, hash2, "Same input should produce same hash");
    }

    #[test]
    fn test_hash_irreversibility() {
        let value = "sk-proj-AbCdEf1234567890XyZ";
        let hashed = hash_value(value);
        assert!(!hashed.contains("sk-proj"));
        assert!(hashed.starts_with("[HASH:"));
        assert!(hashed.ends_with("]"));
    }

    #[test]
    fn test_different_inputs_different_hashes() {
        let value1 = "sk-proj-AbCdEf1234567890XyZ";
        let value2 = "sk-proj-DIFFERENT1234567890";
        let hash1 = hash_value(value1);
        let hash2 = hash_value(value2);
        assert_ne!(
            hash1, hash2,
            "Different inputs should produce different hashes"
        );
    }

    #[test]
    fn test_hash_format() {
        let value = "test-api-key";
        let hashed = hash_value(value);
        assert!(hashed.starts_with("[HASH:"));
        assert!(hashed.ends_with("]"));
        let hex_part = &hashed[6..hashed.len() - 1];
        assert_eq!(hex_part.len(), 32);
        assert!(hex_part.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_empty_string() {
        let value = "";
        let hashed = hash_value(value);
        assert!(hashed.starts_with("[HASH:"));
    }

    #[test]
    fn test_special_characters() {
        let value = "key/with+special=chars";
        let hashed = hash_value(value);
        assert!(hashed.starts_with("[HASH:"));
        assert!(!hashed.contains("/"));
        assert!(!hashed.contains("+"));
    }

    #[test]
    fn test_unicode_handling() {
        let value = "密钥包含中文";
        let hashed = hash_value(value);
        assert!(hashed.starts_with("[HASH:"));
    }

    #[test]
    fn test_long_input() {
        let value = "a".repeat(1000);
        let hashed = hash_value(&value);
        assert!(hashed.starts_with("[HASH:"));
        let hex_part = &hashed[6..hashed.len() - 1];
        assert_eq!(hex_part.len(), 32);
    }
}
