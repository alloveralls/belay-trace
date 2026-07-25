---
schema_version: 1
id: WRK-20260725T202757-001-implement-explore-node-activation-arbitration
type: work
title: Implement Explore node activation arbitration
status: in-progress
created_at: 2026-07-25T20:27:57+09:00
updated_at: 2026-07-25T20:41:16+09:00
revision: 6
tags: []
links:
- relation: implements
  id: DEC-20260725T203527-001-use-cytoscape-double-click-window-for-explore-ar
- relation: implements
  id: PLN-20260725T194011-001-resolve-explore-single-and-double-click-contenti
- relation: fulfills
  id: GOAL-20260725T194008-001-make-explore-node-activation-reliable-and-unambi
metadata: {}
---

## Scope

Implement PLN-20260725T194011-001 Delivery Map tasks T-001 through T-005. T-006 remains a human acceptance gate.

## Progress

- T-001: implemented
- T-002: implemented
- T-003: implemented
- T-004: blocked
- T-005: verified
- T-006: blocked

## Assumptions and Unknowns

- Confirmed from vendored Cytoscape.js 3.34.0: `multiClickDebounceTime()` supplies the configured double-click window (250ms by default), so the single-click timer uses that value rather than duplicating a magic number.
- This is a hypothesis pending E2E execution: deferring expansion for that window and cancelling the matching node timer on `dbltap` fully resolves Issue #15 across supported pointer devices.

## Validation

- Passed: `node --check src/browse.js`
- Passed: `node --check tests/e2e/browse.spec.js`
- Passed: `cargo fmt --all -- --check`
- Passed: Rust 1.87.0 `cargo clippy --all-targets --locked -- -D warnings`
- Passed: Rust 1.87.0 `cargo test --all-targets --locked` (123 tests)
- Blocked by execution environment: Playwright Chromium and local Chrome both abort before test startup because macOS denies Mach port rendezvous. No interaction assertion ran.
- Browser plugin unavailable: the requested in-app Browser returned `Browser is not available: iab`.
- Passed: fresh-context focused review after resolving two findings (initial graph readiness race and duplicate in-flight expansion); no blocking implementation finding remains.

## Review

- REV-20260725T204017-001-review-explore-node-activation-arbitration
- `requires_human_review: true` because real-browser interaction remains unverified.
