---
schema_version: 1
id: GOAL-20260725T220102-001-prepare-a-validated-0-3-2-patch-release
type: goal
title: Prepare a validated 0.3.2 patch release
status: active
created_at: 2026-07-25T22:01:02+09:00
updated_at: 2026-07-25T22:04:41+09:00
revision: 2
tags: []
links: []
metadata: {}
---

## Summary

Prepare version 0.3.2 for the backward-compatible Browse activation and readability fixes accumulated after 0.3.1.

## Success Criteria

- [SC-001] Cargo package metadata and lockfile consistently report version 0.3.2.
- [SC-002] The 0.3.2 scope is documented as backward-compatible Browse bug fixes and visual refinement.
- [SC-003] Rust 1.87 fmt, clippy, all-target tests, rebuild, doctor, and Playwright E2E pass against the release candidate.
- [SC-004] Tagging, GitHub Release creation, publishing, PR creation, and merge occur only after separate explicit human approval.

## Constraints

- Preserve existing CLI, storage schema, managed Markdown, Explore API, and read-only Browse contracts.
- Do not create a tag, GitHub Release, PR, publish artifact, or merge during version-bump implementation.
- Do not claim release readiness while Playwright interaction tests are unverified.

## Non-goals

- Introducing a release automation workflow.
- Publishing to crates.io or another package registry.
- Adding unrelated product changes.

## Verification

- Confirm Cargo.toml and the belay-trace package entry in Cargo.lock both use 0.3.2.
- Run Rust 1.87 fmt, clippy, all-target tests, rebuild, and doctor.
- Run Playwright E2E in CI or another browser-capable environment.
- Review the final diff and Evidence before any release action.

## Risks

- The repository currently has no remote tags or GitHub Releases, so the intended public release mechanism is Unknown.
- Local Playwright execution is unavailable in the current environment.
- Existing active Goal and AGENTS integration drift can keep doctor non-passing even if the version fields are correct.
