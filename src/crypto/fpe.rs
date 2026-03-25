use aes::Aes256;
use fpe::ff1::{FlexibleNumeralString, FF1};

pub struct FPECipher {
    ff1: FF1<Aes256>,
    radix: u32,
}

impl FPECipher {
    pub fn new(key: &[u8; 32], radix: u32) -> Result<Self, String> {
        let ff1 = FF1::<Aes256>::new(key, radix)
            .map_err(|e| format!("Failed to create FF1 cipher: {:?}", e))?;
        Ok(Self { ff1, radix })
    }

    pub fn encrypt(&self, plaintext: &str, tweak: &[u8]) -> Result<String, String> {
        let digits: Vec<u16> = plaintext
            .chars()
            .map(|c| c.to_digit(self.radix).ok_or("Invalid digit"))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Invalid plaintext: {}", e))?
            .into_iter()
            .map(|d| d as u16)
            .collect();

        let plaintext_numeral = FlexibleNumeralString::from(digits);
        let ciphertext = self
            .ff1
            .encrypt(tweak, &plaintext_numeral)
            .map_err(|e| format!("Encryption failed: {:?}", e))?;

        let cipher_vec: Vec<u16> = ciphertext.into();
        Ok(cipher_vec
            .iter()
            .map(|&d| char::from_digit(d as u32, self.radix).unwrap())
            .collect())
    }

    pub fn decrypt(&self, ciphertext: &str, tweak: &[u8]) -> Result<String, String> {
        let digits: Vec<u16> = ciphertext
            .chars()
            .map(|c| c.to_digit(self.radix).ok_or("Invalid digit"))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Invalid ciphertext: {}", e))?
            .into_iter()
            .map(|d| d as u16)
            .collect();

        let ciphertext_numeral = FlexibleNumeralString::from(digits);
        let plaintext = self
            .ff1
            .decrypt(tweak, &ciphertext_numeral)
            .map_err(|e| format!("Decryption failed: {:?}", e))?;

        let plain_vec: Vec<u16> = plaintext.into();
        Ok(plain_vec
            .iter()
            .map(|&d| char::from_digit(d as u32, self.radix).unwrap())
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fpe_encrypt_decrypt_chinese_id() {
        let key = [0u8; 32];
        let cipher = FPECipher::new(&key, 10).unwrap();

        let id = "110101199001011234";
        let encrypted = cipher.encrypt(id, b"").unwrap();
        let decrypted = cipher.decrypt(&encrypted, b"").unwrap();

        assert_eq!(id, decrypted);
        assert_ne!(id, encrypted);
        assert_eq!(id.len(), encrypted.len());
    }

    #[test]
    fn test_fpe_encrypt_decrypt_phone() {
        let key = [1u8; 32];
        let cipher = FPECipher::new(&key, 10).unwrap();

        let phone = "13812345678";
        let encrypted = cipher.encrypt(phone, b"").unwrap();
        let decrypted = cipher.decrypt(&encrypted, b"").unwrap();

        assert_eq!(phone, decrypted);
        assert_ne!(phone, encrypted);
    }

    #[test]
    fn test_fpe_format_preservation() {
        let key = [2u8; 32];
        let cipher = FPECipher::new(&key, 10).unwrap();

        let input = "123456";
        let encrypted = cipher.encrypt(input, b"").unwrap();

        assert_eq!(input.len(), encrypted.len());
        assert!(encrypted.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_fpe_deterministic() {
        let key = [3u8; 32];
        let cipher = FPECipher::new(&key, 10).unwrap();

        let input = "1234567890";
        let encrypted1 = cipher.encrypt(input, b"").unwrap();
        let encrypted2 = cipher.encrypt(input, b"").unwrap();

        assert_eq!(encrypted1, encrypted2);
    }

    #[test]
    fn test_fpe_different_keys_different_output() {
        let key1 = [4u8; 32];
        let key2 = [5u8; 32];
        let cipher1 = FPECipher::new(&key1, 10).unwrap();
        let cipher2 = FPECipher::new(&key2, 10).unwrap();

        let input = "1234567890";
        let encrypted1 = cipher1.encrypt(input, b"").unwrap();
        let encrypted2 = cipher2.encrypt(input, b"").unwrap();

        assert_ne!(encrypted1, encrypted2);
    }

    #[test]
    fn test_fpe_invalid_input() {
        let key = [6u8; 32];
        let cipher = FPECipher::new(&key, 10).unwrap();

        let invalid = "abc123";
        let result = cipher.encrypt(invalid, b"");

        assert!(result.is_err());
    }

    #[test]
    fn test_fpe_empty_string() {
        let key = [7u8; 32];
        let cipher = FPECipher::new(&key, 10).unwrap();

        let empty = "";
        let result = cipher.encrypt(empty, b"");

        assert!(result.is_err() || result.unwrap().is_empty());
    }

    #[test]
    fn test_fpe_long_input() {
        let key = [8u8; 32];
        let cipher = FPECipher::new(&key, 10).unwrap();

        let long_input = "1234567890".repeat(10);
        let encrypted = cipher.encrypt(&long_input, b"").unwrap();
        let decrypted = cipher.decrypt(&encrypted, b"").unwrap();

        assert_eq!(long_input, decrypted);
    }
}
