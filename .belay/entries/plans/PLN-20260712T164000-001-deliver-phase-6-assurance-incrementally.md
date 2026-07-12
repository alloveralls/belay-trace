---
schema_version: 1
id: PLN-20260712T164000-001-deliver-phase-6-assurance-incrementally
type: plan
title: Deliver Phase 6 assurance incrementally
status: draft
created_at: 2026-07-12T16:40:00+09:00
updated_at: 2026-07-12T16:40:43+09:00
revision: 3
tags: []
links:
- relation: fulfills
  id: GOAL-20260712T163956-001-maintain-intent-to-evidence-alignment-during-age
metadata: {}
---

## Intent Brief

### Problem

- Agent coding work can lose its current position, omit requested behavior, or confuse implementation with verified Goal achievement.
- Human requests may not initially contain enough specificity for a correct implementation plan.

### Desired Outcome

- AI and humans share a live map from interpreted intent through Goal criteria, delivery tasks, and Evidence.

### Success Signals

- Intent assumptions and unknowns are visible before implementation.
- Every Goal criterion has an observable task and verification path.
- Checkpoints report the same current state to AI and humans.

### Constraints

- Start with the agent skill and Plan body convention before adding core commands or schema.
- Keep Tier 1 lightweight.

### Non-goals

- Implementing Phase 6 features as part of this planning change.
- Replacing issue tracking.

### Assumptions

- Intent Brief and Delivery Map can be validated through five dogfooding tasks before first-class Task modeling is needed.

### Unknowns / Decisions Needed

- Whether reconciliation becomes a new command or an extension of detailed coverage.
- Whether Task should ever become a first-class object.

## Delivery Map

| ID | Goal item | Outcome / Task | Actor | State | Verification / Evidence |
| --- | --- | --- | --- | --- | --- |
| T-1 | SC-1 | Specify Intent Brief and Delivery Map conventions | AI + Human | implemented | human review pending; `docs/design/phase6.md` |
| T-2 | SC-1..SC-4 | Build Agent-first MVP integration | AI | not-started | integration tests + dogfooding |
| T-3 | SC-1..SC-4 | Dogfood five Tier 2 or Tier 3 tasks | AI + Human | not-started | recorded findings |
| T-4 | SC-2..SC-4 | Add deterministic Plan lint if justified | AI | not-started | unit and CLI tests |
| T-5 | SC-3..SC-4 | Add reconciliation report if justified | AI | not-started | fresh-session handoff test |
| T-6 | SC-5 | Add opt-in completion gate if justified | AI + Human | not-started | prevented premature completion |

## Roadmap

- The durable design and phase gates are defined in `docs/design/phase6.md`.
