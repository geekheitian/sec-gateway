use chrono::{DateTime, Utc};
use rand::{rngs::OsRng, RngCore};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use sha2::{Digest, Sha256};

pub mod reverser;

pub use reverser::Reverser;

#[derive(Debug, Clone)]
pub struct SessionMetadata {
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub request_count: u64,
}

type SessionData = (HashMap<String, Vec<u8>>, Instant, SessionMetadata);

pub struct PrivacyVault {
    storage: Arc<RwLock<HashMap<String, SessionData>>>,
    encryption_key: Arc<RwLock<[u8; 32]>>,
}

impl PrivacyVault {
    pub fn new() -> Self {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);

        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
            encryption_key: Arc::new(RwLock::new(key)),
        }
    }

    pub fn with_key(key: [u8; 32]) -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
            encryption_key: Arc::new(RwLock::new(key)),
        }
    }

    pub fn get_encryption_key(&self) -> [u8; 32] {
        *self.encryption_key.read().unwrap()
    }

    fn derive_nonce(session_id: &str, token: &str) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(session_id.as_bytes());
        hasher.update(token.as_bytes());
        let digest = hasher.finalize();
        let mut nonce = [0u8; 32];
        nonce.copy_from_slice(&digest[..32]);
        nonce
    }

    fn xor_crypt(&self, session_id: &str, token: &str, data: &[u8]) -> Vec<u8> {
        let nonce = Self::derive_nonce(session_id, token);
        let key = *self.encryption_key.read().unwrap();
        let mut out = Vec::with_capacity(data.len());
        let mut counter: u64 = 0;

        while out.len() < data.len() {
            let mut hasher = Sha256::new();
            hasher.update(key);
            hasher.update(nonce);
            hasher.update(counter.to_le_bytes());
            let block = hasher.finalize();
            for b in block {
                if out.len() >= data.len() {
                    break;
                }
                out.push(data[out.len()] ^ b);
            }
            counter = counter.saturating_add(1);
        }

        out
    }

    pub fn store(
        &self,
        session_id: &str,
        token: String,
        original_value: String,
    ) -> Result<(), String> {
        let mut storage = self
            .storage
            .write()
            .map_err(|e| format!("Failed to acquire write lock: {}", e))?;

        let session = storage.entry(session_id.to_string()).or_insert_with(|| {
            let now = Utc::now();
            (
                HashMap::new(),
                Instant::now(),
                SessionMetadata {
                    created_at: now,
                    last_accessed: now,
                    request_count: 0,
                },
            )
        });

        session.2.last_accessed = Utc::now();
        session.2.request_count += 1;

        let ciphertext = self.xor_crypt(session_id, token.as_str(), original_value.as_bytes());
        session.0.insert(token, ciphertext);

        Ok(())
    }

    pub fn retrieve(&self, session_id: &str, token: &str) -> Result<Option<String>, String> {
        let storage = self
            .storage
            .read()
            .map_err(|e| format!("Failed to acquire read lock: {}", e))?;

        Ok(storage
            .get(session_id)
            .and_then(|session| session.0.get(token))
            .map(|ciphertext| self.xor_crypt(session_id, token, ciphertext))
            .map(|plaintext| String::from_utf8(plaintext).map_err(|e| e.to_string()))
            .transpose()?)
    }

    pub fn clear_session(&self, session_id: &str) -> Result<(), String> {
        let mut storage = self
            .storage
            .write()
            .map_err(|e| format!("Failed to acquire write lock: {}", e))?;

        storage.remove(session_id);
        Ok(())
    }

    pub fn cleanup_stale_sessions(&self, max_age_secs: u64) -> usize {
        let cutoff = Instant::now() - std::time::Duration::from_secs(max_age_secs);
        let mut storage = match self.storage.write() {
            Ok(s) => s,
            Err(_) => return 0,
        };

        let stale: Vec<String> = storage
            .iter()
            .filter(|(_, (_, created, _))| *created < cutoff)
            .map(|(sid, _)| sid.clone())
            .collect();

        let count = stale.len();
        for sid in stale {
            storage.remove(&sid);
        }
        count
    }

    pub fn get_session_metadata(
        &self,
        session_id: &str,
    ) -> Result<Option<SessionMetadata>, String> {
        let storage = self
            .storage
            .read()
            .map_err(|e| format!("Failed to acquire read lock: {}", e))?;

        Ok(storage
            .get(session_id)
            .map(|(_, _, metadata)| metadata.clone()))
    }

    pub fn list_all_sessions(&self) -> Result<HashMap<String, SessionMetadata>, String> {
        let storage = self
            .storage
            .read()
            .map_err(|e| format!("Failed to acquire read lock: {}", e))?;

        Ok(storage
            .iter()
            .map(|(sid, (_, _, metadata))| (sid.clone(), metadata.clone()))
            .collect())
    }

    pub fn session_count(&self) -> usize {
        self.storage.read().map(|s| s.len()).unwrap_or(0)
    }

    pub fn session_ids(&self) -> Vec<String> {
        self.storage
            .read()
            .map(|s| s.keys().cloned().collect())
            .unwrap_or_default()
    }

    pub fn token_count(&self, session_id: &str) -> usize {
        self.storage
            .read()
            .ok()
            .and_then(|s| s.get(session_id).map(|session| session.0.len()))
            .unwrap_or(0)
    }

    pub fn re_encrypt_with_new_key(&self, new_key: [u8; 32]) -> Result<usize, String> {
        let mut storage = self
            .storage
            .write()
            .map_err(|e| format!("Failed to acquire write lock: {}", e))?;

        let old_key = *self.encryption_key.read().unwrap();
        let mut total_re_encrypted = 0;

        let old_vault = PrivacyVault::with_key(old_key);
        let new_vault = PrivacyVault::with_key(new_key);

        let session_ids: Vec<String> = storage.keys().cloned().collect();

        for session_id in session_ids {
            if let Some((tokens, _created, _metadata)) = storage.get_mut(&session_id) {
                let token_keys: Vec<String> = tokens.keys().cloned().collect();

                for token in token_keys {
                    if let Some(old_ciphertext) = tokens.get(&token).cloned() {
                        let plaintext = old_vault.xor_crypt(&session_id, &token, &old_ciphertext);
                        let new_ciphertext = new_vault.xor_crypt(&session_id, &token, &plaintext);
                        tokens.insert(token.clone(), new_ciphertext);
                        total_re_encrypted += 1;
                    }
                }
            }
        }

        *self.encryption_key.write().unwrap() = new_key;

        Ok(total_re_encrypted)
    }
}

impl Default for PrivacyVault {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for PrivacyVault {
    fn clone(&self) -> Self {
        Self {
            storage: Arc::clone(&self.storage),
            encryption_key: Arc::clone(&self.encryption_key),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_and_retrieve() {
        let vault = PrivacyVault::new();
        let session = "session_001";
        let token = "[REDACTED_ID_001]";
        let value = "110101199001011234";

        vault
            .store(session, token.to_string(), value.to_string())
            .unwrap();
        let retrieved = vault.retrieve(session, token).unwrap();

        assert_eq!(retrieved, Some(value.to_string()));
    }

    #[test]
    fn test_session_isolation() {
        let vault = PrivacyVault::new();
        let session1 = "session_001";
        let session2 = "session_002";
        let token = "[REDACTED_ID_001]";

        vault
            .store(session1, token.to_string(), "value1".to_string())
            .unwrap();
        vault
            .store(session2, token.to_string(), "value2".to_string())
            .unwrap();

        assert_eq!(
            vault.retrieve(session1, token).unwrap(),
            Some("value1".to_string())
        );
        assert_eq!(
            vault.retrieve(session2, token).unwrap(),
            Some("value2".to_string())
        );
    }

    #[test]
    fn test_retrieve_nonexistent() {
        let vault = PrivacyVault::new();
        let retrieved = vault.retrieve("nonexistent_session", "[TOKEN]").unwrap();

        assert_eq!(retrieved, None);
    }

    #[test]
    fn test_clear_session() {
        let vault = PrivacyVault::new();
        let session = "session_001";
        let token = "[REDACTED_ID_001]";

        vault
            .store(session, token.to_string(), "value".to_string())
            .unwrap();
        assert_eq!(vault.session_count(), 1);

        vault.clear_session(session).unwrap();
        assert_eq!(vault.session_count(), 0);
        assert_eq!(vault.retrieve(session, token).unwrap(), None);
    }

    #[test]
    fn test_multiple_tokens_per_session() {
        let vault = PrivacyVault::new();
        let session = "session_001";

        vault
            .store(session, "[TOKEN_1]".to_string(), "value1".to_string())
            .unwrap();
        vault
            .store(session, "[TOKEN_2]".to_string(), "value2".to_string())
            .unwrap();
        vault
            .store(session, "[TOKEN_3]".to_string(), "value3".to_string())
            .unwrap();

        assert_eq!(vault.token_count(session), 3);
        assert_eq!(
            vault.retrieve(session, "[TOKEN_1]").unwrap(),
            Some("value1".to_string())
        );
        assert_eq!(
            vault.retrieve(session, "[TOKEN_2]").unwrap(),
            Some("value2".to_string())
        );
        assert_eq!(
            vault.retrieve(session, "[TOKEN_3]").unwrap(),
            Some("value3".to_string())
        );
    }

    #[test]
    fn test_vault_clone_shares_storage() {
        let vault1 = PrivacyVault::new();
        let vault2 = vault1.clone();
        let session = "session_001";
        let token = "[TOKEN]";

        vault1
            .store(session, token.to_string(), "value".to_string())
            .unwrap();

        let retrieved = vault2.retrieve(session, token).unwrap();
        assert_eq!(retrieved, Some("value".to_string()));
    }

    #[test]
    fn test_concurrent_access() {
        use std::thread;

        let vault = PrivacyVault::new();
        let vault_clone = vault.clone();

        let handle = thread::spawn(move || {
            vault_clone
                .store("session_001", "[TOKEN_1]".to_string(), "value1".to_string())
                .unwrap();
        });

        vault
            .store("session_002", "[TOKEN_2]".to_string(), "value2".to_string())
            .unwrap();

        handle.join().unwrap();

        assert_eq!(vault.session_count(), 2);
    }

    #[test]
    fn test_token_count() {
        let vault = PrivacyVault::new();
        assert_eq!(vault.token_count("nonexistent"), 0);

        vault
            .store("session", "[TOKEN]".to_string(), "value".to_string())
            .unwrap();
        assert_eq!(vault.token_count("session"), 1);
    }

    #[test]
    fn test_vault_does_not_store_plaintext() {
        let vault = PrivacyVault::new();
        let session = "session1";
        let token = "[TOKEN]";
        let value = "sensitive_value";

        vault
            .store(session, token.to_string(), value.to_string())
            .unwrap();

        let storage = vault.storage.read().unwrap();
        let ciphertext = storage
            .get(session)
            .and_then(|s| s.0.get(token))
            .cloned()
            .unwrap();
        assert_ne!(ciphertext, value.as_bytes().to_vec());
    }
}
