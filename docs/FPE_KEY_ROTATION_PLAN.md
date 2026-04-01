# FPE Key Rotation Plan

**Status**: Proposed  
**Priority**: P2  
**Author**: Security Audit (2026-03-31)  
**Prerequisite**: SEC-009 documentation fix (completed)

---

## Problem Statement

Current `key_rotation.rs` only rotates the **Vault encryption key** (XOR stream cipher key used for at-rest token storage). The **FPE key** (AES-256-FF1 or SM4-128-FF1) is loaded once at startup and never rotates.

This means:
- If the FPE key is compromised, all past and future FPE-encrypted tokens are reversible
- There is no forward secrecy for FPE operations
- The module name `key_rotation` creates a false impression that FPE keys rotate

---

## Current Architecture

```
Startup:
  main.rs → get_or_generate_key() → [u8; 32] (AES) or [u8; 16] (SM4)
    → create_aes_backend(key, radix) → Arc<dyn FpeBackend>
    → stored in AppState.fpe_backend: DynFpeBackend

Runtime:
  FpeBackend.encrypt(plaintext, tweak) → ciphertext  (key baked into struct)
  FpeBackend.decrypt(ciphertext, tweak) → plaintext  (same key)

Key Rotation (Vault only):
  KeyRotation.rotate_key() → new [u8; 32]
    → re-encrypts all Vault entries
    → FPE key UNCHANGED
```

**Key insight**: FPE is deterministic — same key + same tweak + same plaintext = same ciphertext. Rotation requires tracking which key version encrypted each token.

---

## Proposed Design: Versioned FPE Keys

### Core Concept

```rust
struct VersionedFpeBackend {
    current_version: u32,
    backends: HashMap<u32, DynFpeBackend>,  // version → backend
}
```

Each masked token carries its key version:
- **Before**: `05509628502` (bare ciphertext)
- **After**: `v1:05509628502` (versioned ciphertext)

### Data Flow

```
Encrypt (always uses current version):
  plaintext → backends[current_version].encrypt() → "v{current_version}:{ciphertext}"

Decrypt (uses token's version):
  "v{N}:{ciphertext}" → backends[N].decrypt() → plaintext

Rotation:
  1. Generate new key via OsRng
  2. Create new FpeBackend with new key
  3. backends.insert(current_version + 1, new_backend)
  4. current_version += 1
  5. Old backends kept for decryption of in-flight tokens
  6. Cleanup: remove old versions after configurable retention period
```

### Implementation Steps

#### Step 1: VersionedFpeBackend wrapper (minimal invasion)

```rust
// src/crypto/versioned_fpe.rs

pub struct VersionedFpeBackend {
    current_version: Arc<RwLock<u32>>,
    backends: Arc<RwLock<HashMap<u32, DynFpeBackend>>>,
}

impl VersionedFpeBackend {
    pub fn new(initial_backend: DynFpeBackend) -> Self { /* v1 */ }
    pub fn encrypt(&self, plaintext: &str, tweak: &[u8]) -> Result<String, String> { /* v{N}:{ct} */ }
    pub fn decrypt(&self, versioned_ciphertext: &str, tweak: &[u8]) -> Result<String, String> { /* parse version, route */ }
    pub fn rotate(&self, new_backend: DynFpeBackend) -> u32 { /* returns new version */ }
    pub fn remove_version(&self, version: u32) -> bool { /* cleanup old keys */ }
}
```

#### Step 2: Update AppState

```rust
// Before:
pub fpe_backend: DynFpeBackend,

// After:
pub fpe_backend: Arc<VersionedFpeBackend>,
```

#### Step 3: Update masker to pass through versioned tokens

The masker currently stores bare ciphertext in the Vault. After this change, it stores `v{N}:{ciphertext}`. The Reverser already works with opaque token strings, so no Reverser changes needed.

#### Step 4: Background rotation task

```rust
// In main.rs, alongside existing vault key rotation task
tokio::spawn(async move {
    loop {
        tokio::time::sleep(Duration::from_secs(fpe_rotation_interval)).await;
        let new_key = generate_fpe_key();
        let new_backend = create_backend(new_key, radix);
        versioned_fpe.rotate(new_backend);
        // Optionally remove versions older than retention_period
    }
});
```

#### Step 5: Config additions

```yaml
security:
  key_rotation:
    fpe:
      enabled: false          # opt-in, default off for backward compat
      interval_days: 90
      retention_versions: 3   # keep N old versions for in-flight decryption
```

### Migration Path

1. **v1.0 (current)**: No version prefix. All tokens are bare ciphertext.
2. **v1.1 (this plan)**: Tokens gain `v1:` prefix. Bare tokens treated as v1 (backward compat).
3. **v1.2+**: Old sessions gradually expire. Eventually all tokens are versioned.

### Backward Compatibility

```rust
fn decrypt(&self, token: &str, tweak: &[u8]) -> Result<String, String> {
    if let Some((version_str, ciphertext)) = token.split_once(':') {
        if let Some(v) = version_str.strip_prefix('v') {
            let version: u32 = v.parse().map_err(|_| "invalid version")?;
            return self.backends[&version].decrypt(ciphertext, tweak);
        }
    }
    // No version prefix → legacy token, use v1
    self.backends[&1].decrypt(token, tweak)
}
```

---

## Effort Estimate

| Step | Files | Effort |
|------|-------|--------|
| VersionedFpeBackend | 1 new file (~80 lines) | 1 hour |
| AppState + main.rs update | 2 files | 30 min |
| Masker passthrough | 1 file (minor) | 15 min |
| Background task | 1 file (main.rs) | 30 min |
| Config additions | 1 file + config.rs | 30 min |
| Tests | 1 new file (~100 lines) | 1 hour |
| **Total** | | **~4 hours** |

---

## Risks and Mitigations

| Risk | Mitigation |
|------|-----------|
| Performance overhead from version parsing | Negligible — one `split_once` per decrypt |
| Key storage for multiple versions | In-memory only; keys regenerated on restart (same as current) |
| Token format change breaks existing sessions | Backward compat: bare tokens → v1 |
| Rotation during high traffic | RwLock allows concurrent reads; rotation only takes write lock briefly |

---

## Decision Required

1. **Persist FPE keys across restarts?** Current design regenerates on restart, invalidating all FPE tokens. If persistence is needed, keys must be written to encrypted key files (similar to current `.sec-gateway-aes.key`).

2. **Rotation trigger**: Time-based (background task) vs. API-triggered (`POST /admin/rotate-fpe-key`) vs. both?

3. **Priority**: Implement now or defer to Phase 4?
