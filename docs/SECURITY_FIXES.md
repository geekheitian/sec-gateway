# Security Audit Report

**Project**: sec-gateway  
**Date**: 2026-03-31  
**Scope**: Full codebase security review  
**Commit**: `8e0b079` (all fixes applied)  
**Test Status**: 289/289 passing  

---

## Summary

| Severity | Count | Fixed |
|----------|-------|-------|
| Critical | 3     | 3     |
| High     | 4     | 4     |
| Medium   | 2     | 2     |
| **Total** | **9** | **9** |

---

## Critical

### SEC-001: Reverser Offset Accumulation Bug

**File**: `src/vault/reverser.rs`  
**Risk**: PII restoration produces corrupted output when multiple masked tokens exist  
**Root Cause**: Reverse-order iteration accumulated an offset variable that shifted subsequent replacement positions incorrectly. The offset was only correct for forward iteration.  
**Fix**: Remove offset accumulation entirely — reverse iteration uses direct `start..end` range replacement, which is self-consistent because earlier ranges are unaffected by later replacements.  
**Regression Test**: `test_reverse_multiple_tokens` added.

### SEC-002: SM4 Mutex Poisoned Panic

**File**: `src/crypto/sm4_fpe.rs` (lines 141, 188)  
**Risk**: A poisoned mutex causes `unwrap()` panic, crashing the service  
**Root Cause**: `self.sm4.lock().unwrap()` propagates panic from any previous thread failure.  
**Fix**: Replace `unwrap()` with `map_err(|e| Sm4FpeError::LockError(e.to_string()))` and return `Result`.

### SEC-003: Transport/Provider Init Panics

**Files**: `src/provider/transport.rs`, `factory.rs`, `openai.rs`, `anthropic.rs`, `gemini.rs`  
**Risk**: Invalid configuration (e.g., TLS error, bad URL) panics the entire process via `expect()`  
**Root Cause**: `HttpTransport::new()` and all provider constructors used `expect()` for reqwest client creation.  
**Fix**: Full Result-based chain — `HttpTransport::new() -> Result<Self, ProviderError>`, propagated through all providers and `ProviderFactory::build()`. Server startup now handles init errors gracefully.

---

## High

### SEC-004: Request Body DoS (Unbounded Read)

**File**: `src/handlers/proxy.rs`  
**Risk**: Attacker sends multi-GB request body, exhausting server memory  
**Root Cause**: Body size limit was `usize::MAX` (effectively unlimited).  
**Fix**: Set limit to `10 * 1024 * 1024` (10 MB). Sufficient for LLM API payloads, prevents memory exhaustion.

### SEC-005: Async Hot-Path with std::sync::Mutex

**Files**: `src/app_state.rs`, `src/middleware.rs`, `src/main.rs`, `src/handlers/proxy.rs`  
**Risk**: `std::sync::Mutex` on the rate limiter blocks the tokio runtime thread pool under high concurrency, causing cascading latency spikes  
**Root Cause**: Rate limiter `HashMap` wrapped in `std::sync::Mutex`, held across `.await` points.  
**Fix**: Replace with `tokio::sync::Mutex`. All call sites updated to `.lock().await`.

### SEC-006: Constant-Time Comparison Side-Channel

**File**: `src/middleware.rs` (line 40-48)  
**Risk**: Token length leakage via timing — `min(len)` reveals whether the shorter string is the secret  
**Root Cause**: Comparison loop used `min(a.len(), b.len())`, making shorter-input comparisons faster.  
**Fix**: Use `max(a.len(), b.len())` with bounds-safe access via `.get(i).copied().unwrap_or(0)`. Both strings are always fully traversed regardless of length difference.

### SEC-007: Session ID Injection

**File**: `src/middleware.rs` (lines 212-219)  
**Risk**: Arbitrary session IDs enable path traversal in log filenames, cache key pollution, or log injection  
**Root Cause**: No validation on the `x-session-id` header value.  
**Fix**: Three-layer validation:
1. Length limit: 128 characters max
2. Character whitelist: `[A-Za-z0-9._-]` only
3. Empty check: fallback to UUID v4

---

## Medium

### SEC-008: reqwest 0.11 (End-of-Life)

**File**: `Cargo.toml`  
**Risk**: No security patches for known vulnerabilities in hyper 0.14 / h2 dependency chain  
**Root Cause**: Project pinned to reqwest 0.11, which depends on hyper 0.14 (EOL).  
**Fix**: Upgrade to reqwest 0.12, aligned with axum 0.7 / hyper 1.x ecosystem.

### SEC-009: Key Rotation Scope Mismatch (Documentation)

**File**: `src/crypto/key_rotation.rs`  
**Risk**: Operators may believe FPE keys rotate, when only Vault encryption keys do. False sense of security.  
**Root Cause**: Module named `key_rotation` implies all keys rotate. In reality, FPE keys are static for the process lifetime.  
**Fix**: Documented in this report. Implementation plan in `FPE_KEY_ROTATION_PLAN.md`.  
**Status**: P1 (docs clarification done), P2 (versioned FPE key rotation — planned).

---

## Verification Evidence

```
$ cargo test
test result: ok. 289 passed; 0 failed; 0 ignored

$ cargo clippy
0 warnings

$ cargo build --release
Finished release target(s)
```

---

## Recommendations

1. **Integrate `cargo clippy` and `cargo test` into CI** — prevent regressions
2. **Add `cargo audit` to CI** — catch dependency vulnerabilities automatically
3. **Implement FPE key rotation** — see `FPE_KEY_ROTATION_PLAN.md`
4. **Consider rate-limit persistence** — current in-memory rate limiter resets on restart
5. **Add request body size limit to config** — currently hardcoded at 10 MB
