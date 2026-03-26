use sec_gateway::crypto::fpe_trait::{create_aes_backend, FpeBackend};
use sec_gateway::crypto::sm4_fpe::Sm4FpeCipher;
use std::sync::Arc;
use std::thread;

#[test]
fn test_aes_backend_concurrent_100_threads() {
    let aes_key = [0x42; 32];
    let backend = create_aes_backend(&aes_key, 10).expect("Failed to create AES backend");

    let handles: Vec<_> = (0..100)
        .map(|i| {
            let backend_clone = Arc::clone(&backend);
            thread::spawn(move || {
                let session_id = format!("concurrent-session-{}", i);
                let tweak = session_id.as_bytes();
                let plaintext = format!("1234567890{:02}", i);

                let encrypted = backend_clone
                    .encrypt(&plaintext, tweak)
                    .expect("Encryption failed");

                assert_ne!(encrypted, plaintext);
                assert_eq!(encrypted.len(), plaintext.len());

                let decrypted = backend_clone
                    .decrypt(&encrypted, tweak)
                    .expect("Decryption failed");

                assert_eq!(decrypted, plaintext);

                (encrypted, decrypted)
            })
        })
        .collect();

    let results: Vec<_> = handles
        .into_iter()
        .map(|h| h.join().expect("Thread panicked"))
        .collect();

    assert_eq!(results.len(), 100);

    for i in 0..100 {
        let expected_plaintext = format!("1234567890{:02}", i);
        assert_eq!(results[i].1, expected_plaintext);
    }
}

#[test]
fn test_sm4_backend_concurrent_100_threads() {
    let sm4_key = [0x42; 16];
    let cipher = Sm4FpeCipher::new(&sm4_key, 10).expect("Failed to create SM4 cipher");
    let backend: Arc<dyn FpeBackend> = Arc::new(cipher);

    let handles: Vec<_> = (0..100)
        .map(|i| {
            let backend_clone = Arc::clone(&backend);
            thread::spawn(move || {
                let session_id = format!("concurrent-sm4-session-{}", i);
                let tweak = session_id.as_bytes();
                let plaintext = format!("1234567890{:02}", i);

                let encrypted = backend_clone
                    .encrypt(&plaintext, tweak)
                    .expect("Encryption failed");

                assert_ne!(encrypted, plaintext);
                assert_eq!(encrypted.len(), plaintext.len());

                let decrypted = backend_clone
                    .decrypt(&encrypted, tweak)
                    .expect("Decryption failed");

                assert_eq!(decrypted, plaintext);

                (encrypted, decrypted)
            })
        })
        .collect();

    let results: Vec<_> = handles
        .into_iter()
        .map(|h| h.join().expect("Thread panicked"))
        .collect();

    assert_eq!(results.len(), 100);

    for i in 0..100 {
        let expected_plaintext = format!("1234567890{:02}", i);
        assert_eq!(results[i].1, expected_plaintext);
    }
}

#[test]
fn test_mixed_backends_concurrent_stress() {
    let aes_key = [0x55; 32];
    let sm4_key = [0x55; 16];
    let aes_backend = create_aes_backend(&aes_key, 10).expect("Failed to create AES backend");
    let sm4_cipher = Sm4FpeCipher::new(&sm4_key, 10).expect("Failed to create SM4 cipher");
    let sm4_backend: Arc<dyn FpeBackend> = Arc::new(sm4_cipher);

    let mut handles = vec![];

    for i in 0..50 {
        let aes_clone = Arc::clone(&aes_backend);
        handles.push(thread::spawn(move || {
            let tweak = format!("aes-stress-{}", i).into_bytes();
            let plaintext = "13812345678";
            aes_clone.encrypt(plaintext, &tweak).expect("AES failed")
        }));

        let sm4_clone = Arc::clone(&sm4_backend);
        handles.push(thread::spawn(move || {
            let tweak = format!("sm4-stress-{}", i).into_bytes();
            let plaintext = "13912345678";
            sm4_clone.encrypt(plaintext, &tweak).expect("SM4 failed")
        }));
    }

    let results: Vec<_> = handles
        .into_iter()
        .map(|h| h.join().expect("Thread panicked"))
        .collect();

    assert_eq!(results.len(), 100);

    for result in results {
        assert_eq!(result.len(), 11);
        assert!(result.chars().all(|c| c.is_digit(10)));
    }
}

#[test]
fn test_high_concurrency_1000_threads() {
    let aes_key = [0xAA; 32];
    let backend = create_aes_backend(&aes_key, 10).expect("Failed to create AES backend");

    let handles: Vec<_> = (0..1000)
        .map(|i| {
            let backend_clone = Arc::clone(&backend);
            thread::spawn(move || {
                let tweak = format!("high-load-{}", i).into_bytes();
                let plaintext = format!("{:010}", i % 1_000_000_000);

                backend_clone
                    .encrypt(&plaintext, &tweak)
                    .expect("Encryption failed")
            })
        })
        .collect();

    let results: Vec<_> = handles
        .into_iter()
        .map(|h| h.join().expect("Thread panicked"))
        .collect();

    assert_eq!(results.len(), 1000);

    for result in results {
        assert_eq!(result.len(), 10);
        assert!(result.chars().all(|c| c.is_digit(10)));
    }
}
