use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;

pub mod reverser;

pub use reverser::Reverser;

pub struct PrivacyVault {
    storage: Arc<RwLock<HashMap<String, (HashMap<String, String>, Instant)>>>,
}

impl PrivacyVault {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn store(&self, session_id: &str, token: String, original_value: String) -> Result<(), String> {
        let mut storage = self.storage.write()
            .map_err(|e| format!("Failed to acquire write lock: {}", e))?;
        
        let session = storage.entry(session_id.to_string()).or_insert_with(|| (HashMap::new(), Instant::now()));
        session.0.insert(token, original_value);
        
        Ok(())
    }

    pub fn retrieve(&self, session_id: &str, token: &str) -> Result<Option<String>, String> {
        let storage = self.storage.read()
            .map_err(|e| format!("Failed to acquire read lock: {}", e))?;
        
        Ok(storage.get(session_id)
            .and_then(|session| session.0.get(token))
            .cloned())
    }

    pub fn clear_session(&self, session_id: &str) -> Result<(), String> {
        let mut storage = self.storage.write()
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
        
        let stale: Vec<String> = storage.iter()
            .filter(|(_, (_, created))| *created < cutoff)
            .map(|(sid, _)| sid.clone())
            .collect();
        
        let count = stale.len();
        for sid in stale {
            storage.remove(&sid);
        }
        count
    }

    pub fn session_count(&self) -> usize {
        self.storage.read().map(|s| s.len()).unwrap_or(0)
    }

    pub fn token_count(&self, session_id: &str) -> usize {
        self.storage.read()
            .ok()
            .and_then(|s| s.get(session_id).map(|session| session.0.len()))
            .unwrap_or(0)
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

        vault.store(session, token.to_string(), value.to_string()).unwrap();
        let retrieved = vault.retrieve(session, token).unwrap();

        assert_eq!(retrieved, Some(value.to_string()));
    }

    #[test]
    fn test_session_isolation() {
        let vault = PrivacyVault::new();
        let session1 = "session_001";
        let session2 = "session_002";
        let token = "[REDACTED_ID_001]";

        vault.store(session1, token.to_string(), "value1".to_string()).unwrap();
        vault.store(session2, token.to_string(), "value2".to_string()).unwrap();

        assert_eq!(vault.retrieve(session1, token).unwrap(), Some("value1".to_string()));
        assert_eq!(vault.retrieve(session2, token).unwrap(), Some("value2".to_string()));
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

        vault.store(session, token.to_string(), "value".to_string()).unwrap();
        assert_eq!(vault.session_count(), 1);

        vault.clear_session(session).unwrap();
        assert_eq!(vault.session_count(), 0);
        assert_eq!(vault.retrieve(session, token).unwrap(), None);
    }

    #[test]
    fn test_multiple_tokens_per_session() {
        let vault = PrivacyVault::new();
        let session = "session_001";

        vault.store(session, "[TOKEN_1]".to_string(), "value1".to_string()).unwrap();
        vault.store(session, "[TOKEN_2]".to_string(), "value2".to_string()).unwrap();
        vault.store(session, "[TOKEN_3]".to_string(), "value3".to_string()).unwrap();

        assert_eq!(vault.token_count(session), 3);
        assert_eq!(vault.retrieve(session, "[TOKEN_1]").unwrap(), Some("value1".to_string()));
        assert_eq!(vault.retrieve(session, "[TOKEN_2]").unwrap(), Some("value2".to_string()));
        assert_eq!(vault.retrieve(session, "[TOKEN_3]").unwrap(), Some("value3".to_string()));
    }

    #[test]
    fn test_vault_clone_shares_storage() {
        let vault1 = PrivacyVault::new();
        let vault2 = vault1.clone();
        let session = "session_001";
        let token = "[TOKEN]";

        vault1.store(session, token.to_string(), "value".to_string()).unwrap();
        
        let retrieved = vault2.retrieve(session, token).unwrap();
        assert_eq!(retrieved, Some("value".to_string()));
    }

    #[test]
    fn test_concurrent_access() {
        use std::thread;

        let vault = PrivacyVault::new();
        let vault_clone = vault.clone();

        let handle = thread::spawn(move || {
            vault_clone.store("session_001", "[TOKEN_1]".to_string(), "value1".to_string()).unwrap();
        });

        vault.store("session_002", "[TOKEN_2]".to_string(), "value2".to_string()).unwrap();

        handle.join().unwrap();

        assert_eq!(vault.session_count(), 2);
    }

    #[test]
    fn test_token_count() {
        let vault = PrivacyVault::new();
        assert_eq!(vault.token_count("nonexistent"), 0);

        vault.store("session", "[TOKEN]".to_string(), "value".to_string()).unwrap();
        assert_eq!(vault.token_count("session"), 1);
    }
}
