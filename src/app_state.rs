use std::{
    collections::HashMap,
    sync::{atomic::AtomicU64, Arc},
    time::Instant,
};
use tokio::sync::Mutex;

use crate::{
    audit::FileAppender,
    config::Config,
    crypto::versioned_fpe::VersionedFpeBackend,
    provider::Provider,
    vault::{PrivacyVault, Reverser},
};

#[derive(Clone)]
pub struct AppState {
    pub provider: Arc<dyn Provider>,
    pub config: Arc<Config>,
    pub vault: PrivacyVault,
    pub fpe_cipher: Arc<VersionedFpeBackend>,
    pub reverser: Reverser,
    pub rate_limiter: Arc<Mutex<HashMap<String, (u32, Instant)>>>,
    pub metrics: Arc<MetricsState>,
    pub audit_appender: Option<Arc<FileAppender>>,
}

#[derive(Default)]
pub struct MetricsState {
    pub total_requests: AtomicU64,
    pub blocked_requests: AtomicU64,
    pub pii_detected_requests: AtomicU64,
}
