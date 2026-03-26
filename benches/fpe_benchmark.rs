use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sec_gateway::crypto::fpe_trait::{create_aes_backend, FpeBackend};
use sec_gateway::crypto::sm4_fpe::Sm4FpeCipher;
use std::sync::Arc;

fn benchmark_aes_encrypt(c: &mut Criterion) {
    let aes_key = [0x42; 32];
    let backend = create_aes_backend(&aes_key, 10).expect("Failed to create AES backend");
    let tweak = b"benchmark-session";

    let mut group = c.benchmark_group("aes_encrypt");

    for size in [6, 11, 18, 36].iter() {
        let plaintext = "1234567890".repeat(*size / 10 + 1)[..*size].to_string();
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &plaintext, |b, text| {
            b.iter(|| {
                backend
                    .encrypt(black_box(text), black_box(tweak))
                    .expect("Encryption failed")
            });
        });
    }
    group.finish();
}

fn benchmark_sm4_encrypt(c: &mut Criterion) {
    let sm4_key = [0x42; 16];
    let cipher = Sm4FpeCipher::new(&sm4_key, 10).expect("Failed to create SM4 cipher");
    let backend: Arc<dyn FpeBackend> = Arc::new(cipher);
    let tweak = b"benchmark-session";

    let mut group = c.benchmark_group("sm4_encrypt");

    for size in [6, 11, 18, 36].iter() {
        let plaintext = "1234567890".repeat(*size / 10 + 1)[..*size].to_string();
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &plaintext, |b, text| {
            b.iter(|| {
                backend
                    .encrypt(black_box(text), black_box(tweak))
                    .expect("Encryption failed")
            });
        });
    }
    group.finish();
}

fn benchmark_aes_decrypt(c: &mut Criterion) {
    let aes_key = [0x42; 32];
    let backend = create_aes_backend(&aes_key, 10).expect("Failed to create AES backend");
    let tweak = b"benchmark-session";

    let mut group = c.benchmark_group("aes_decrypt");

    for size in [6, 11, 18, 36].iter() {
        let plaintext = "1234567890".repeat(*size / 10 + 1)[..*size].to_string();
        let ciphertext = backend
            .encrypt(&plaintext, tweak)
            .expect("Encryption failed");
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &ciphertext, |b, text| {
            b.iter(|| {
                backend
                    .decrypt(black_box(text), black_box(tweak))
                    .expect("Decryption failed")
            });
        });
    }
    group.finish();
}

fn benchmark_sm4_decrypt(c: &mut Criterion) {
    let sm4_key = [0x42; 16];
    let cipher = Sm4FpeCipher::new(&sm4_key, 10).expect("Failed to create SM4 cipher");
    let backend: Arc<dyn FpeBackend> = Arc::new(cipher);
    let tweak = b"benchmark-session";

    let mut group = c.benchmark_group("sm4_decrypt");

    for size in [6, 11, 18, 36].iter() {
        let plaintext = "1234567890".repeat(*size / 10 + 1)[..*size].to_string();
        let ciphertext = backend
            .encrypt(&plaintext, tweak)
            .expect("Encryption failed");
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &ciphertext, |b, text| {
            b.iter(|| {
                backend
                    .decrypt(black_box(text), black_box(tweak))
                    .expect("Decryption failed")
            });
        });
    }
    group.finish();
}

fn benchmark_backend_comparison(c: &mut Criterion) {
    let aes_key = [0x42; 32];
    let sm4_key = [0x42; 16];
    let aes_backend = create_aes_backend(&aes_key, 10).expect("Failed to create AES backend");
    let sm4_cipher = Sm4FpeCipher::new(&sm4_key, 10).expect("Failed to create SM4 cipher");
    let sm4_backend: Arc<dyn FpeBackend> = Arc::new(sm4_cipher);
    let tweak = b"benchmark-session";

    let mut group = c.benchmark_group("backend_comparison");
    let phone_number = "13812345678";

    group.bench_function("aes_phone", |b| {
        b.iter(|| {
            aes_backend
                .encrypt(black_box(phone_number), black_box(tweak))
                .expect("AES encryption failed")
        });
    });

    group.bench_function("sm4_phone", |b| {
        b.iter(|| {
            sm4_backend
                .encrypt(black_box(phone_number), black_box(tweak))
                .expect("SM4 encryption failed")
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_aes_encrypt,
    benchmark_sm4_encrypt,
    benchmark_aes_decrypt,
    benchmark_sm4_decrypt,
    benchmark_backend_comparison
);
criterion_main!(benches);
