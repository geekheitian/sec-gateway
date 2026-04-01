use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::crypto::fpe_trait::{DynFpeBackend, FpeBackend};

pub struct VersionedFpeBackend {
    current_version: Arc<RwLock<u32>>,
    backends: Arc<RwLock<HashMap<u32, DynFpeBackend>>>,
}

impl VersionedFpeBackend {
    pub fn new(initial_backend: DynFpeBackend) -> Self {
        let mut backends = HashMap::new();
        backends.insert(1, initial_backend);
        Self {
            current_version: Arc::new(RwLock::new(1)),
            backends: Arc::new(RwLock::new(backends)),
        }
    }

    pub fn versioned_name(&self) -> String {
        let v = *self.current_version.read().unwrap();
        let backends = self.backends.read().unwrap();
        if let Some(backend) = backends.get(&v) {
            format!("{}-v{}", backend.backend_name(), v)
        } else {
            "VersionedFPE-unknown".to_string()
        }
    }

    pub fn encrypt(&self, plaintext: &str, tweak: &[u8]) -> Result<String, String> {
        let v = *self.current_version.read().unwrap();
        let backends = self.backends.read().unwrap();
        let backend = backends
            .get(&v)
            .ok_or_else(|| format!("FPE backend version {} not found", v))?;
        let ciphertext = backend.encrypt(plaintext, tweak)?;
        Ok(format!("v{}:{}", v, ciphertext))
    }

    pub fn decrypt(&self, versioned_ciphertext: &str, tweak: &[u8]) -> Result<String, String> {
        let (version, ciphertext) = match versioned_ciphertext.split_once(':') {
            Some((prefix, ct)) if prefix.starts_with('v') => {
                let v: u32 = prefix[1..].parse().map_err(|_| {
                    format!("invalid version prefix in token: {}", versioned_ciphertext)
                })?;
                (v, ct)
            }
            _ => {
                // backward compat: bare tokens are v1
                (1, versioned_ciphertext)
            }
        };

        let backends = self.backends.read().unwrap();
        let backend = backends
            .get(&version)
            .ok_or_else(|| format!("FPE backend version {} not found", version))?;
        backend.decrypt(ciphertext, tweak)
    }

    pub fn rotate(&self, new_backend: DynFpeBackend) -> u32 {
        let mut backends = self.backends.write().unwrap();
        let mut current = self.current_version.write().unwrap();
        *current += 1;
        backends.insert(*current, new_backend);
        *current
    }

    pub fn current_version(&self) -> u32 {
        *self.current_version.read().unwrap()
    }

    pub fn remove_version(&self, version: u32) -> bool {
        let mut backends = self.backends.write().unwrap();
        if version >= *self.current_version.read().unwrap() {
            return false;
        }
        backends.remove(&version);
        true
    }

    pub fn retention_versions(&self) -> usize {
        self.backends.read().unwrap().len()
    }
}

impl FpeBackend for VersionedFpeBackend {
    fn encrypt(&self, plaintext: &str, tweak: &[u8]) -> Result<String, String> {
        self.encrypt(plaintext, tweak)
    }

    fn decrypt(&self, ciphertext: &str, tweak: &[u8]) -> Result<String, String> {
        self.decrypt(ciphertext, tweak)
    }

    fn backend_name(&self) -> &'static str {
        "VersionedFPE"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::fpe_trait::create_aes_backend;

    fn test_backend() -> VersionedFpeBackend {
        let key = [0u8; 32];
        let backend = create_aes_backend(&key, 10).unwrap();
        VersionedFpeBackend::new(backend)
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let vfb = test_backend();
        let plaintext = "13812345678";

        let encrypted = vfb.encrypt(plaintext, b"session1").unwrap();
        assert!(encrypted.starts_with("v1:"));
        let ciphertext_only = encrypted.strip_prefix("v1:").unwrap();

        let decrypted = vfb.decrypt(&encrypted, b"session1").unwrap();
        assert_eq!(decrypted, plaintext);
        assert_ne!(decrypted, ciphertext_only);
    }

    #[test]
    fn test_version_prefix_format() {
        let vfb = test_backend();
        let encrypted = vfb.encrypt("110101199001011234", b"").unwrap();

        let parts: Vec<&str> = encrypted.split(':').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "v1");
        assert_eq!(parts[1].len(), 18);
    }

    #[test]
    fn test_backward_compat_bare_token() {
        let vfb = test_backend();
        let key = [0u8; 32];
        let bare_backend = create_aes_backend(&key, 10).unwrap();
        let ciphertext = bare_backend.encrypt("13812345678", b"session1").unwrap();

        // bare token (no version prefix) should decrypt as v1
        let decrypted = vfb.decrypt(&ciphertext, b"session1").unwrap();
        assert_eq!(decrypted, "13812345678");
    }

    #[test]
    fn test_rotate_increments_version() {
        let vfb = test_backend();
        assert_eq!(vfb.current_version(), 1);

        let key2 = [1u8; 32];
        let backend2 = create_aes_backend(&key2, 10).unwrap();
        let new_v = vfb.rotate(backend2);

        assert_eq!(new_v, 2);
        assert_eq!(vfb.current_version(), 2);
        assert_eq!(vfb.retention_versions(), 2);
    }

    #[test]
    fn test_rotate_new_key_different_ciphertext() {
        let vfb = test_backend();
        let plaintext = "13812345678";

        let encrypted_v1 = vfb.encrypt(plaintext, b"session1").unwrap();

        let key2 = [1u8; 32];
        let backend2 = create_aes_backend(&key2, 10).unwrap();
        vfb.rotate(backend2);

        let encrypted_v2 = vfb.encrypt(plaintext, b"session1").unwrap();

        // same plaintext, different key → different ciphertext
        let ct_v1 = encrypted_v1.strip_prefix("v1:").unwrap();
        let ct_v2 = encrypted_v2.strip_prefix("v2:").unwrap();
        assert_ne!(ct_v1, ct_v2);
    }

    #[test]
    fn test_decrypt_old_version_after_rotation() {
        let vfb = test_backend();
        let plaintext = "110101199001011234";

        let encrypted_v1 = vfb.encrypt(plaintext, b"").unwrap();
        let ciphertext_v1 = encrypted_v1.strip_prefix("v1:").unwrap();

        // rotate to v2
        let key2 = [2u8; 32];
        let backend2 = create_aes_backend(&key2, 10).unwrap();
        vfb.rotate(backend2);

        // v1 token should still decrypt
        let decrypted = vfb.decrypt(ciphertext_v1, b"").unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_remove_version_keeps_current() {
        let vfb = test_backend();

        let key2 = [1u8; 32];
        let backend2 = create_aes_backend(&key2, 10).unwrap();
        vfb.rotate(backend2);

        let key3 = [2u8; 32];
        let backend3 = create_aes_backend(&key3, 10).unwrap();
        vfb.rotate(backend3);

        assert_eq!(vfb.current_version(), 3);
        assert_eq!(vfb.retention_versions(), 3);

        // cannot remove current version
        assert!(!vfb.remove_version(3));

        // can remove old versions
        assert!(vfb.remove_version(1));
        assert_eq!(vfb.retention_versions(), 2);
        assert!(vfb.remove_version(2));
        assert_eq!(vfb.retention_versions(), 1);
    }

    #[test]
    fn test_decrypt_unknown_version_fails() {
        let vfb = test_backend();
        let result = vfb.decrypt("v99:abcdef", b"");
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_invalid_version_string_falls_back_to_v1() {
        let vfb = test_backend();
        let result = vfb.decrypt("vx:abcdef", b"");
        assert!(result.is_err());
    }
}
