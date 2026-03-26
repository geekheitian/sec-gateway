#[cfg(test)]
mod sm4_fpe_tests {
    use sec_gateway::crypto::sm4_fpe::Sm4FpeCipher;

    #[test]
    fn test_basic_encrypt_decrypt_roundtrip() {
        let key = [
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54,
            0x32, 0x10,
        ];
        let tweak = b"test-session-001";
        let radix = 10;

        let cipher = Sm4FpeCipher::new(&key, radix).expect("Failed to create SM4 FPE cipher");

        let plaintext = "1234567890";
        let ciphertext = cipher.encrypt(plaintext, tweak).expect("Encryption failed");

        assert_ne!(
            plaintext, ciphertext,
            "Ciphertext should differ from plaintext"
        );
        assert_eq!(
            plaintext.len(),
            ciphertext.len(),
            "Length should be preserved"
        );

        let decrypted = cipher
            .decrypt(&ciphertext, tweak)
            .expect("Decryption failed");

        assert_eq!(
            plaintext, decrypted,
            "Decrypted text should match original plaintext"
        );
    }

    #[test]
    fn test_min_length_validation_encrypt() {
        let key = [0u8; 16];
        let tweak = b"test-tweak";
        let radix = 10;

        let cipher = Sm4FpeCipher::new(&key, radix).expect("Failed to create SM4 FPE cipher");

        let short_inputs = vec!["12345", "1234", "123", "12", "1", ""];

        for input in short_inputs {
            let result = cipher.encrypt(input, tweak);
            assert!(
                result.is_err(),
                "Should reject input shorter than 6 chars: '{}'",
                input
            );
            assert!(
                result.unwrap_err().contains("too short"),
                "Error should mention length constraint"
            );
        }
    }

    #[test]
    fn test_min_length_validation_decrypt() {
        let key = [0u8; 16];
        let tweak = b"test-tweak";
        let radix = 10;

        let cipher = Sm4FpeCipher::new(&key, radix).expect("Failed to create SM4 FPE cipher");

        let short_inputs = vec!["12345", "1234", "123"];

        for input in short_inputs {
            let result = cipher.decrypt(input, tweak);
            assert!(
                result.is_err(),
                "Should reject ciphertext shorter than 6 chars: '{}'",
                input
            );
        }
    }

    #[test]
    fn test_deterministic_encryption() {
        let key = [
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54,
            0x32, 0x10,
        ];
        let tweak = b"test-session-002";
        let radix = 10;

        let cipher = Sm4FpeCipher::new(&key, radix).expect("Failed to create SM4 FPE cipher");

        let plaintext = "9876543210";

        let ciphertext1 = cipher
            .encrypt(plaintext, tweak)
            .expect("First encryption failed");
        let ciphertext2 = cipher
            .encrypt(plaintext, tweak)
            .expect("Second encryption failed");
        let ciphertext3 = cipher
            .encrypt(plaintext, tweak)
            .expect("Third encryption failed");

        assert_eq!(
            ciphertext1, ciphertext2,
            "Same plaintext should produce same ciphertext (1 vs 2)"
        );
        assert_eq!(
            ciphertext2, ciphertext3,
            "Same plaintext should produce same ciphertext (2 vs 3)"
        );
        assert_eq!(
            ciphertext1, ciphertext3,
            "Same plaintext should produce same ciphertext (1 vs 3)"
        );
    }

    #[test]
    fn test_different_tweaks_produce_different_ciphertexts() {
        let key = [0x42; 16];
        let radix = 10;
        let plaintext = "1234567890";

        let cipher = Sm4FpeCipher::new(&key, radix).expect("Failed to create cipher");

        let ciphertext1 = cipher
            .encrypt(plaintext, b"tweak-001")
            .expect("Encryption 1 failed");
        let ciphertext2 = cipher
            .encrypt(plaintext, b"tweak-002")
            .expect("Encryption 2 failed");

        assert_ne!(
            ciphertext1, ciphertext2,
            "Different tweaks should produce different ciphertexts"
        );
    }

    #[test]
    fn test_length_preservation() {
        let key = [0x99; 16];
        let tweak = b"length-test";
        let radix = 10;

        let cipher = Sm4FpeCipher::new(&key, radix).expect("Failed to create SM4 FPE cipher");

        let min_length_input = "123456";
        let medium_length_input = "1234567890";
        let long_length_input = "12345678901234567890";

        let test_cases = vec![min_length_input, medium_length_input, long_length_input];

        for plaintext in test_cases {
            let ciphertext = cipher
                .encrypt(plaintext, tweak)
                .expect(&format!("Encryption failed for '{}'", plaintext));

            assert_eq!(
                plaintext.len(),
                ciphertext.len(),
                "Length should be preserved: plaintext='{}' ({} chars), ciphertext='{}' ({} chars)",
                plaintext,
                plaintext.len(),
                ciphertext,
                ciphertext.len()
            );
        }
    }

    #[test]
    fn test_invalid_radix() {
        let key = [0u8; 16];

        let result_low = Sm4FpeCipher::new(&key, 1);
        assert!(result_low.is_err(), "Radix 1 should be rejected");

        let result_high = Sm4FpeCipher::new(&key, 37);
        assert!(result_high.is_err(), "Radix 37 should be rejected");

        let result_valid = Sm4FpeCipher::new(&key, 10);
        assert!(result_valid.is_ok(), "Radix 10 should be accepted");
    }

    #[test]
    fn test_alphabet_validation() {
        let key = [0xaa; 16];
        let tweak = b"alphabet-test";
        let radix = 10;

        let cipher = Sm4FpeCipher::new(&key, radix).expect("Failed to create SM4 FPE cipher");

        let result = cipher.encrypt("123abc", tweak);
        assert!(
            result.is_err(),
            "Should reject characters outside decimal alphabet"
        );
        assert!(
            result.unwrap_err().contains("alphabet"),
            "Error should mention alphabet constraint"
        );
    }

    #[test]
    fn test_chinese_phone_number_encryption() {
        let key = [
            0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66,
            0x77, 0x88,
        ];
        let tweak = b"phone-session";
        let radix = 10;

        let cipher = Sm4FpeCipher::new(&key, radix).expect("Failed to create SM4 FPE cipher");

        let phone = "13812345678";
        let encrypted = cipher
            .encrypt(phone, tweak)
            .expect("Failed to encrypt phone number");

        assert_ne!(
            phone, encrypted,
            "Encrypted phone should differ from original"
        );
        assert_eq!(phone.len(), encrypted.len(), "Length should be preserved");
        assert!(
            encrypted.chars().all(|c| c.is_digit(10)),
            "Encrypted phone should contain only digits"
        );

        let decrypted = cipher
            .decrypt(&encrypted, tweak)
            .expect("Failed to decrypt phone number");
        assert_eq!(phone, decrypted, "Decrypted phone should match original");
    }

    #[test]
    fn test_chinese_id_number_encryption() {
        let key = [0xff; 16];
        let tweak = b"id-session";
        let radix = 10;

        let cipher = Sm4FpeCipher::new(&key, radix).expect("Failed to create SM4 FPE cipher");

        let id_number = "110101199001011234";
        let encrypted = cipher
            .encrypt(id_number, tweak)
            .expect("Failed to encrypt ID number");

        assert_ne!(
            id_number, encrypted,
            "Encrypted ID should differ from original"
        );
        assert_eq!(
            id_number.len(),
            encrypted.len(),
            "Length should be preserved"
        );

        let decrypted = cipher
            .decrypt(&encrypted, tweak)
            .expect("Failed to decrypt ID number");
        assert_eq!(id_number, decrypted, "Decrypted ID should match original");
    }
}
