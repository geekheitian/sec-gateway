# Frontend / UI Draft

## Goal
- Build a lightweight admin/dashboard layer for the privacy gateway.
- Keep it aligned with the Rust backend lifecycle, not ahead of it.

## Recommended Stack
- Tauri
- Preact
- TypeScript
- Zustand or local store for small state
- CSS modules or a small utility-first layer

## Why this stack
- Small desktop footprint
- Easy local-first deployment
- Fits a security-sensitive tool better than a heavy web app

## Phase Boundary
- **Phase 2A/2B**: design only
- **Phase 2C**: scaffold app shell, routing, layout, empty states
- **Phase 3**: full UI, workflow polish, charts, audit views

## Information Architecture
1. Overview
2. Sessions
3. Requests
4. Rules / PII Detection
5. Audit / Metrics
6. Settings

## Page Breakdown

### 1) Overview
- service status
- active sessions
- recent detections
- error rate / latency summary

### 2) Sessions
- session list
- session details
- restore history
- TTL / cleanup state

### 3) Requests
- inbound request viewer
- masked payload diff
- upstream provider response

### 4) Rules / PII Detection
- enabled detectors
- masking strategy by PII type
- custom regex editor

### 5) Audit / Metrics
- request log timeline
- metric cards
- export/download actions

### 6) Settings
- provider config
- TLS/auth/rate limit toggles
- CORS whitelist
- FPE key status (no secret display)

## Layout Proposal
- left sidebar: navigation
- top bar: runtime status + active session
- main panel: detail content
- right drawer: request/session metadata

## Implementation Notes
- No business logic in components
- Backend API should provide DTOs for UI needs
- Keep secret material out of UI state
- Design for desktop first, browser second
