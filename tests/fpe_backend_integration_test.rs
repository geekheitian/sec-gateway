use sec_gateway::crypto::fpe_trait::{create_aes_backend, FpeBackend};
use sec_gateway::crypto::sm4_fpe::Sm4FpeCipher;
use sec_gateway::vault::PrivacyVault;
use std::sync::Arc;

#[test]
fn test_aes_backend_end_to_end() {
    let aes_key = [0x42; 32];
    let backend = create_aes_backend(&aes_key, 10).unwrap();
    let vault = PrivacyVault::new();
    let session_id = "aes-test-session";
    let tweak = session_id.as_bytes();

    let original_id = "110101199001011234";
    let encrypted = backend.encrypt(original_id, tweak).unwrap();

    assert_ne!(encrypted, original_id, "Encrypted should differ");
    assert_eq!(
        encrypted.len(),
        original_id.len(),
        "Length should be preserved"
    );
    assert!(
        encrypted.chars().all(|c| c.is_digit(10)),
        "Format should be preserved (digits only)"
    );

    vault
        .store(session_id, encrypted.clone(), original_id.to_string())
        .unwrap();

    let decrypted = backend.decrypt(&encrypted, tweak).unwrap();
    assert_eq!(decrypted, original_id, "Roundtrip should recover original");

    let retrieved = vault.retrieve(session_id, &encrypted).unwrap();
    assert_eq!(
        retrieved,
        Some(original_id.to_string()),
        "Vault should return original"
    );
}

#[test]
fn test_sm4_backend_end_to_end() {
    let sm4_key = [0x42; 16];
    let cipher = Sm4FpeCipher::new(&sm4_key, 10).unwrap();
    let backend: Arc<dyn FpeBackend> = Arc::new(cipher);
    let vault = PrivacyVault::new();
    let session_id = "sm4-test-session";
    let tweak = session_id.as_bytes();

    let original_phone = "13812345678";
    let encrypted = backend.encrypt(original_phone, tweak).unwrap();

    assert_ne!(encrypted, original_phone, "Encrypted should differ");
    assert_eq!(
        encrypted.len(),
        original_phone.len(),
        "Length should be preserved"
    );
    assert!(
        encrypted.chars().all(|c| c.is_digit(10)),
        "Format should be preserved (digits only)"
    );

    vault
        .store(session_id, encrypted.clone(), original_phone.to_string())
        .unwrap();

    let decrypted = backend.decrypt(&encrypted, tweak).unwrap();
    assert_eq!(
        decrypted, original_phone,
        "Roundtrip should recover original"
    );

    let retrieved = vault.retrieve(session_id, &encrypted).unwrap();
    assert_eq!(
        retrieved,
        Some(original_phone.to_string()),
        "Vault should return original"
    );
}

#[test]
fn test_backend_switching_produces_different_ciphertexts() {
    let aes_key = [0x55; 32];
    let sm4_key = [0x55; 16];

    let aes_backend = create_aes_backend(&aes_key, 10).unwrap();
    let sm4_backend: Arc<dyn FpeBackend> = Arc::new(Sm4FpeCipher::new(&sm4_key, 10).unwrap());

    let plaintext = "1234567890";
    let tweak = b"shared-session";

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
fn test_aes_backend_with_multiple_sessions() {
    let aes_key = [0x99; 32];
    let backend = create_aes_backend(&aes_key, 10).unwrap();
    let vault = PrivacyVault::new();

    let test_cases = vec![
        ("session-001", "13800001111"),
        ("session-002", "13900002222"),
        ("session-003", "15000003333"),
    ];

    for (session_id, phone) in &test_cases {
        let tweak = session_id.as_bytes();
        let encrypted = backend.encrypt(phone, tweak).unwrap();

        vault
            .store(session_id, encrypted.clone(), phone.to_string())
            .unwrap();

        let decrypted = backend.decrypt(&encrypted, tweak).unwrap();
        assert_eq!(&decrypted, phone);
    }

    for (session_id, phone) in &test_cases {
        let tweak = session_id.as_bytes();
        let re_encrypted = backend.encrypt(phone, tweak).unwrap();
        let retrieved = vault.retrieve(session_id, &re_encrypted).unwrap();
        assert_eq!(retrieved, Some(phone.to_string()));
    }
}

#[test]
fn test_sm4_backend_with_multiple_sessions() {
    let sm4_key = [0x99; 16];
    let cipher = Sm4FpeCipher::new(&sm4_key, 10).unwrap();
    let backend: Arc<dyn FpeBackend> = Arc::new(cipher);
    let vault = PrivacyVault::new();

    let test_cases = vec![
        ("session-101", "110101199001011234"),
        ("session-102", "110101199101011234"),
        ("session-103", "110101199201011234"),
    ];

    for (session_id, id_number) in &test_cases {
        let tweak = session_id.as_bytes();
        let encrypted = backend.encrypt(id_number, tweak).unwrap();

        vault
            .store(session_id, encrypted.clone(), id_number.to_string())
            .unwrap();

        let decrypted = backend.decrypt(&encrypted, tweak).unwrap();
        assert_eq!(&decrypted, id_number);
    }

    for (session_id, id_number) in &test_cases {
        let tweak = session_id.as_bytes();
        let re_encrypted = backend.encrypt(id_number, tweak).unwrap();
        let retrieved = vault.retrieve(session_id, &re_encrypted).unwrap();
        assert_eq!(retrieved, Some(id_number.to_string()));
    }
}

#[test]
fn test_backend_name_identification() {
    let aes_key = [0xaa; 32];
    let sm4_key = [0xbb; 16];

    let aes_backend = create_aes_backend(&aes_key, 10).unwrap();
    let sm4_cipher = Sm4FpeCipher::new(&sm4_key, 10).unwrap();
    let sm4_backend: Arc<dyn FpeBackend> = Arc::new(sm4_cipher);

    assert_eq!(aes_backend.backend_name(), "AES-FF1");
    assert_eq!(sm4_backend.backend_name(), "SM4-FF1");
}

#[test]
fn test_aes_backend_preserves_radix() {
    let aes_key = [0x12; 32];
    let backend = create_aes_backend(&aes_key, 10).unwrap();
    let tweak = b"radix-test";

    let numeric_input = "9876543210";
    let encrypted = backend.encrypt(numeric_input, tweak).unwrap();

    assert!(
        encrypted.chars().all(|c| c.is_digit(10)),
        "Radix 10: should only contain digits 0-9"
    );

    let decrypted = backend.decrypt(&encrypted, tweak).unwrap();
    assert_eq!(decrypted, numeric_input);
}

#[test]
fn test_sm4_backend_preserves_radix() {
    let sm4_key = [0x12; 16];
    let cipher = Sm4FpeCipher::new(&sm4_key, 10).unwrap();
    let backend: Arc<dyn FpeBackend> = Arc::new(cipher);
    let tweak = b"radix-test";

    let numeric_input = "9876543210";
    let encrypted = backend.encrypt(numeric_input, tweak).unwrap();

    assert!(
        encrypted.chars().all(|c| c.is_digit(10)),
        "Radix 10: should only contain digits 0-9"
    );

    let decrypted = backend.decrypt(&encrypted, tweak).unwrap();
    assert_eq!(decrypted, numeric_input);
}
