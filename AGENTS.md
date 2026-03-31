# AGENTS.md

This file provides guidance to Codex (Codex.ai/code) when working with code in this repository.

## Project Overview

**sec-gateway** is a privacy-preserving AI gateway written in Rust. It acts as a local reverse proxy that intercepts requests to LLM providers (OpenAI, Anthropic, Gemini), detects and de-identifies PII before forwarding, then restores original values in the response — ensuring sensitive data never leaves the local environment.

## Commands

### Backend (Rust)

```bash
cargo build --release        # Production build
cargo run                    # Development run
cargo test                   # Run unit tests
cargo clippy                 # Lint
cargo test <test_name>       # Run a single test by name
```

### Integration Tests

```bash
make test-full               # Full suite: setup → start service → test → stop
make test-quick              # Quick tests (requires service already running)
make test-start              # Start service only
make test-stop               # Stop service only
make test-clean              # Clean up artifacts
```

### Dashboard (Frontend)

```bash
cd dashboard && npm install && npm run dev   # Dev server
cd dashboard && npm run build               # Production build
```

### Docker

```bash
docker build -t sec-gateway:latest .
docker run -p 8080:8080 sec-gateway:latest
```

### Verify deployment

```bash
curl http://localhost:8080/health
```

## Architecture

### Request Flow

```
Client → proxy_handler (src/main.rs)
    → Auth check (is_authorized)
    → Rate limiting (check_rate_limit)
    → PII detection (active_detectors + detect_custom_patterns)
    → Masking: FPE | Hash | Replace
    → Vault storage (token → original mapping, encrypted at rest)
    → Forward to Provider (OpenAI/Anthropic/Gemini)
    → Response: restore original values via Reverser (allowlist-only)
```

### Key Modules

| Module | Path | Responsibility |
|--------|------|----------------|
| `main.rs` | `src/main.rs` | Axum server, routing, initialization (272 lines after refactoring) |
| `app_state` | `src/app_state.rs` | AppState and MetricsState definitions |
| `middleware` | `src/middleware.rs` | Auth, rate limiting, CORS, TLS, audit, session management |
| `handlers` | `src/handlers/` | Request handlers (admin.rs: mgmt APIs; proxy.rs: proxy logic) |
| `config` | `src/config.rs` | YAML config + env var loading |
| `detector` | `src/detector/` | PII detection: 8 built-in types + custom regex |
| `masker` | `src/masker/` | Three strategies: FPE, SHA-256 hash (16-byte), placeholder |
| `crypto` | `src/crypto/` | AES-256-FF1 and SM4-128-FF1 FPE; Key rotation (OsRng) |
| `vault` | `src/vault/` | In-memory token→original store (XOR-encrypted w/ OsRng); Reverser |
| `provider` | `src/provider/` | OpenAI, Anthropic, Gemini adapters + SSE streaming |
| `audit` | `src/audit/` | File-based audit log with daily/size rotation |

### PII Types and Default Strategies

Configured in `config/default.yaml`. Built-in types: `chinese_id`, `phone_number`, `email`, `credit_card`, `jwt`, `api_key`, `ip_address`, `database_connection_string`. Default strategies:
- FPE (format-preserving): `chinese_id`, `phone_number`
- Hash (SHA-256): `credit_card`, `jwt`, `api_key`, `database_connection_string`
- Replace (placeholder): `email`, `ip_address`

### Configuration

Primary config: `config/default.yaml`. Key environment variable overrides:
- `FPE_KEY` — 64-char hex AES-256 key (auto-generated to `.sec-gateway-aes.key` if unset)
- `SM4_FPE_KEY` — 32-char hex SM4-128 key (auto-generated to `.sec-gateway-sm4.key` when `crypto.fpe.backend: "sm4"`)
- `TARGET_URL` — override the LLM endpoint
- `SERVER_PORT` — override listen port

**Key Generation Security**: Uses cryptographically secure `OsRng` (based on `/dev/urandom`). Key files are created with `600` permissions (owner read/write only) and excluded from version control via `.gitignore`.

### Session Management

Sessions are identified via the `x-session-id` request header (auto-generated UUID if absent). The Vault stores PII mappings per session. Stale sessions are cleaned up on a configurable interval (`security.session.cleanup_interval_seconds`).

### Security Endpoints

- `GET /health` — unauthenticated health check
- `POST /v1/chat/completions` — main proxy (auth enforced if enabled)
- `GET /sessions` — session list (requires auth)
- `DELETE /sessions/:id` — delete session (requires auth)
- `GET /metrics` — Prometheus metrics (requires auth)

Auth supports Bearer token (`Authorization: Bearer <token>`) or custom header, using constant-time comparison.

### Provider Abstraction

`src/provider/factory.rs` builds the correct provider from config. All providers implement the `Provider` trait (`src/provider/mod.rs`) with `send()` and `send_stream()` methods. SSE streaming is handled in `src/provider/sse.rs`.

### FPE Crypto

Two backends selectable at runtime via `crypto.fpe.backend`:
- `"aes"` (default): AES-256-FF1 via the `fpe` crate
- `"sm4"`: SM4-128-FF1 (Chinese national standard)

Keys are 32 bytes (AES) or 16 bytes (SM4), hex-encoded in env vars. Auto-generated on first startup with a warning to save the key.

## Recent Changes

### Code Refactoring (Commit 6acaa79)

**Main Goals**: Reduce `main.rs` bloat (1034 lines → 272 lines, -74%), improve code organization, fix bugs discovered during refactoring.

**Changes**:
1. **Created `src/app_state.rs` (32 lines)**: Extracted `AppState` and `MetricsState` structs from handlers, fixing dependency structure
2. **Created `src/middleware.rs` (285 lines)**: Extracted 9 middleware functions (auth, rate limiting, CORS, TLS, audit, session management) + 4 unit tests
3. **Created `src/handlers/` directory**:
   - `mod.rs` (2 lines): Module declarations
   - `admin.rs` (106 lines): 7 management API handlers (health, sessions, metrics)
   - `proxy.rs` (408 lines): Core proxy_handler logic with masking/restoration
4. **Refactored `src/main.rs` (272 lines)**: Removed all migrated functions, retained only config loading, FPE initialization, background tasks, router setup, server startup

**Bugs Fixed**:
- **session_id triple extraction**: `proxy_handler` was calling `extract_session_id()` three times (lines 56, 128, 144), causing audit logs and actual requests to use different UUIDs when no PII was detected. Fixed by extracting once at function start.
- **AppState misplacement**: Moved from `handlers/admin.rs` to dedicated `src/app_state.rs`, fixing circular dependency
- **Import clutter**: Replaced verbose `std::sync::Arc::new` with clean `Arc::new` imports

**Verification**: All 287 tests passing, cargo build successful.

### Security Hardening (Current Commit)

**Main Goals**: Strengthen cryptographic security across key rotation, hash output, and vault encryption.

**Changes**:
1. **Key Rotation (`src/crypto/key_rotation.rs`)**:
   - **Before**: Used weak pseudo-random key generation via `SHA256(process_id || timestamp || thread_id)`
   - **After**: Use cryptographically secure `OsRng` from `rand` crate for key generation
   - **Impact**: Eliminates predictability in rotated keys, prevents potential key recovery attacks

2. **Hash Length Extension (`src/masker/hash.rs`)**:
   - **Before**: Truncated SHA-256 output to 8 bytes (16 hex chars), enabling collision attacks
   - **After**: Extended to 16 bytes (32 hex chars), maintaining hash uniqueness
   - **Impact**: Reduces collision probability from 1/2^64 to 1/2^128, industry-standard length
   - **Format Change**: `[HASH:e0b7453469e42b48]` → `[HASH:e0b7453469e42b48a1c2d3e4f5a6b7c8]`

3. **Vault Encryption Key Strengthening (`src/vault/mod.rs`)**:
   - **Before**: Weak key derivation via `SHA256(process_id || elapsed_nanos)`, predictable seed
   - **After**: Use `OsRng` for vault encryption key generation, wrapped in `Arc<RwLock<>>` for rotation support
   - **Added**: `PrivacyVault::with_key()` constructor for testing with explicit keys
   - **Impact**: Prevents vault encryption key prediction, enables future key rotation for vault encryption

**Verification**: All 287 tests passing, cargo build successful.

**Related Documentation**: See `PHASE2C_VERIFICATION_REPORT.md` for detailed security analysis.

### FPE Key Generation Hardening (Commit f3027b1)

**Main Goals**: Eliminate weak random source and prevent key material leakage to logs.

**Changes**:
1. **Random Source Upgrade (`src/main.rs:get_or_generate_key`)**:
   - **Before**: Used `rand::random::<u8>()` (thread-local PRNG)
   - **After**: Use cryptographically secure `OsRng` from `rand` crate
   - **Impact**: Eliminates predictability risks in containerized/virtualized environments

2. **Key Distribution Strategy (`src/main.rs:get_or_generate_key`)**:
   - **Before**: Printed key to `tracing::warn!()` logs (risk: log aggregation services)
   - **After**: Write key to local file (`.sec-gateway-aes.key` / `.sec-gateway-sm4.key`) with `600` permissions
   - **Fallback**: If file write fails, log key with explicit warning (development-only)
   - **Impact**: Prevents key leakage to centralized logging systems (ELK, Splunk, CloudWatch)

**Verification**: All 287 tests passing, committed and pushed to `origin/dev`.

### Session TTL Fix (Current Commit)

**Main Goals**: Fix session expiry logic to use sliding-window TTL instead of absolute expiry.

**Problem**: Sessions were unconditionally cleaned up after `ttl_seconds` from creation, even if actively used.

**Changes**:
1. **SessionData Type Refactoring (`src/vault/mod.rs:20`)**:
   - **Before**: `type SessionData = (HashMap<String, Vec<u8>>, Instant, SessionMetadata);` (3 fields)
   - **After**: `type SessionData = (HashMap<String, Vec<u8>>, SessionMetadata);` (2 fields)
   - **Removed**: Redundant `Instant` field, removed unused `use std::time::Instant;` import
   - **Impact**: Eliminates data redundancy, uses `DateTime<Utc>` for consistent time tracking

2. **Retrieve Method Updates (`src/vault/mod.rs:116-137`)**:
   - **Before**: Used read-only lock, never updated `last_accessed` timestamp
   - **After**: Uses write lock to update `session.1.last_accessed = Utc::now()` on every retrieval
   - **Impact**: Sessions now track actual usage, enabling proper sliding-window TTL

3. **Cleanup Logic Fix (`src/vault/mod.rs:140-158`)**:
   - **Before**: Compared `Instant` creation time → absolute expiry
   - **After**: Compares `metadata.last_accessed` → idle expiry (sliding window)
   - **Parameter Rename**: `max_age_secs` → `max_idle_secs` (semantic clarity)
   - **Impact**: Active sessions no longer expire, only idle sessions cleaned up

**Verification**: All 287 tests passing, cargo build successful.
