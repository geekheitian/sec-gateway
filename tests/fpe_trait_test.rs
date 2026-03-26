#[cfg(test)]
mod fpe_trait_tests {
    use sec_gateway::crypto::fpe_trait::{create_aes_backend, DynFpeBackend, FpeBackend};
    use sec_gateway::crypto::sm4_fpe::Sm4FpeCipher;
    use std::sync::Arc;

    #[test]
    fn test_aes_backend_via_trait() {
        let key = [0x42; 32];
        let backend: DynFpeBackend = create_aes_backend(&key, 10).unwrap();
        let tweak = b"test-session-001";
        let plaintext = "1234567890";

        let ciphertext = backend.encrypt(plaintext, tweak).unwrap();
        let decrypted = backend.decrypt(&ciphertext, tweak).unwrap();

        assert_eq!(plaintext, decrypted);
        assert_eq!(backend.backend_name(), "AES-FF1");
        assert_ne!(plaintext, ciphertext, "Ciphertext should differ");
        assert_eq!(
            plaintext.len(),
            ciphertext.len(),
            "Length should be preserved"
        );
    }

    #[test]
    fn test_sm4_backend_via_trait() {
        let key = [0x42; 16];
        let cipher = Sm4FpeCipher::new(&key, 10).unwrap();
        let backend: Arc<dyn FpeBackend> = Arc::new(cipher);
        let tweak = b"test-session-002";
        let plaintext = "9876543210";

        let ciphertext = backend.encrypt(plaintext, tweak).unwrap();
        let decrypted = backend.decrypt(&ciphertext, tweak).unwrap();

        assert_eq!(plaintext, decrypted);
        assert_eq!(backend.backend_name(), "SM4-FF1");
        assert_ne!(plaintext, ciphertext, "Ciphertext should differ");
        assert_eq!(
            plaintext.len(),
            ciphertext.len(),
            "Length should be preserved"
        );
    }

    #[test]
    fn test_runtime_backend_switching() {
        let aes_key = [0x11; 32];
        let sm4_key = [0x22; 16];
        let tweak = b"shared-tweak";
        let plaintext = "1234567890";

        let backends: Vec<DynFpeBackend> = vec![
            create_aes_backend(&aes_key, 10).unwrap(),
            Arc::new(Sm4FpeCipher::new(&sm4_key, 10).unwrap()),
        ];

        for backend in backends {
            let ciphertext = backend.encrypt(plaintext, tweak).unwrap();
            let decrypted = backend.decrypt(&ciphertext, tweak).unwrap();

            assert_eq!(plaintext, decrypted, "Backend: {}", backend.backend_name());
            assert_ne!(
                plaintext,
                ciphertext,
                "Ciphertext should differ from plaintext for {}",
                backend.backend_name()
            );
            assert_eq!(
                plaintext.len(),
                ciphertext.len(),
                "Length should be preserved for {}",
                backend.backend_name()
            );
        }
    }

    #[test]
    fn test_different_backends_produce_different_ciphertexts() {
        let aes_key = [0x42; 32];
        let sm4_key = [0x42; 16];
        let tweak = b"same-tweak";
        let plaintext = "1234567890";

        let aes_backend = create_aes_backend(&aes_key, 10).unwrap();
        let sm4_backend: Arc<dyn FpeBackend> = Arc::new(Sm4FpeCipher::new(&sm4_key, 10).unwrap());

        let aes_ciphertext = aes_backend.encrypt(plaintext, tweak).unwrap();
        let sm4_ciphertext = sm4_backend.encrypt(plaintext, tweak).unwrap();

        assert_ne!(
            aes_ciphertext, sm4_ciphertext,
            "Different backends should produce different ciphertexts"
        );

        let aes_decrypted = aes_backend.decrypt(&aes_ciphertext, tweak).unwrap();
        let sm4_decrypted = sm4_backend.decrypt(&sm4_ciphertext, tweak).unwrap();

        assert_eq!(aes_decrypted, plaintext);
        assert_eq!(sm4_decrypted, plaintext);
    }

    #[test]
    fn test_tweak_sensitivity_via_trait() {
        let key = [0x99; 16];
        let cipher = Sm4FpeCipher::new(&key, 10).unwrap();
        let backend: Arc<dyn FpeBackend> = Arc::new(cipher);
        let plaintext = "1234567890";

        let ciphertext1 = backend.encrypt(plaintext, b"tweak-001").unwrap();
        let ciphertext2 = backend.encrypt(plaintext, b"tweak-002").unwrap();

        assert_ne!(
            ciphertext1, ciphertext2,
            "Different tweaks should produce different ciphertexts"
        );

        let decrypted1 = backend.decrypt(&ciphertext1, b"tweak-001").unwrap();
        let decrypted2 = backend.decrypt(&ciphertext2, b"tweak-002").unwrap();

        assert_eq!(decrypted1, plaintext);
        assert_eq!(decrypted2, plaintext);
    }
}
