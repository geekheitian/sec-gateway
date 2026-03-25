use aes::Aes256;
use fpe::ff1::{FlexibleNumeralString, FF1};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fpe_chinese_id_encryption() {
        let key = [0u8; 32];
        let radix = 10;
        let tweak = b"";

        let ff1 = FF1::<Aes256>::new(&key, radix).unwrap();

        let id = "110101199001011234";

        let plaintext: Vec<u16> = id.chars().map(|c| c.to_digit(10).unwrap() as u16).collect();

        let plaintext_numeral = FlexibleNumeralString::from(plaintext.clone());
        let ciphertext = ff1.encrypt(tweak, &plaintext_numeral).unwrap();

        let decrypted = ff1.decrypt(tweak, &ciphertext).unwrap();

        let decrypted_vec: Vec<u16> = decrypted.into();

        assert_eq!(plaintext, decrypted_vec);

        let cipher_vec: Vec<u16> = ciphertext.into();
        assert_ne!(plaintext, cipher_vec);
        assert_eq!(plaintext.len(), cipher_vec.len());
    }

    #[test]
    fn test_fpe_preserves_format() {
        let key = [0u8; 32];
        let radix = 10;
        let tweak = b"";

        let ff1 = FF1::<Aes256>::new(&key, radix).unwrap();

        let id = "110101199001011234";
        let plaintext: Vec<u16> = id.chars().map(|c| c.to_digit(10).unwrap() as u16).collect();

        let plaintext_numeral = FlexibleNumeralString::from(plaintext);
        let ciphertext = ff1.encrypt(tweak, &plaintext_numeral).unwrap();

        let cipher_vec: Vec<u16> = ciphertext.into();
        assert_eq!(cipher_vec.len(), 18);
        assert!(cipher_vec.iter().all(|&d| d < 10));

        let encrypted_str: String = cipher_vec
            .iter()
            .map(|&d| ((d as u8) + b'0') as char)
            .collect();
        println!("Original: {}", id);
        println!("Encrypted: {}", encrypted_str);
    }

    #[test]
    fn test_fpe_different_inputs() {
        let key = [0u8; 32];
        let radix = 10;
        let tweak = b"";

        let ff1 = FF1::<Aes256>::new(&key, radix).unwrap();

        let id1 = "110101199001011234";
        let id2 = "110101199001011235";

        let plaintext1: Vec<u16> = id1
            .chars()
            .map(|c| c.to_digit(10).unwrap() as u16)
            .collect();
        let plaintext2: Vec<u16> = id2
            .chars()
            .map(|c| c.to_digit(10).unwrap() as u16)
            .collect();

        let ciphertext1 = ff1
            .encrypt(tweak, &FlexibleNumeralString::from(plaintext1))
            .unwrap();
        let ciphertext2 = ff1
            .encrypt(tweak, &FlexibleNumeralString::from(plaintext2))
            .unwrap();

        let cipher_vec1: Vec<u16> = ciphertext1.into();
        let cipher_vec2: Vec<u16> = ciphertext2.into();

        assert_ne!(cipher_vec1, cipher_vec2);
    }
}
