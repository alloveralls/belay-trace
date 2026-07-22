---
schema_version: 1
id: GOAL-20260712T163956-001-maintain-intent-to-evidence-alignment-during-age
type: goal
title: Maintain intent-to-evidence alignment during agent delivery
status: draft
created_at: 2026-07-12T16:39:56+09:00
updated_at: 2026-07-12T16:39:56+09:00
revision: 1
tags: []
links: []
metadata: {}
---

## Summary

- Human intent, implementation state, and verification evidence remain aligned throughout Tier 2 and Tier 3 agent coding work.

## Success Criteria

- Tier 2 and Tier 3 work starts with an Intent Brief that exposes assumptions, unknowns, and human decisions.
- Every Goal success criterion maps to observable delivery tasks and a verification method.
- AI and humans can distinguish not started, in progress, blocked, implemented, verified, and dropped work.
- Reconciliation exposes scope changes, unimplemented criteria, and unverified work before completion.
- Completion requires relevant Evidence and human acceptance rather than implementation alone.

## Constraints

- Belay core remains deterministic and does not call an LLM.
- The workflow must not replace an issue tracker or require detailed task management for Tier 1 work.
- Productization follows dogfooding evidence rather than preceding it.

## Non-goals

- Eliminating all ambiguity from human requests.
- Encoding a comprehensive software-design knowledge base in the Belay skill.
- Treating task completion percentage as a quality score.

## Verification

- Dogfood the Agent-first MVP on five Tier 2 or Tier 3 tasks.
- Record whether Intent correction, missing implementation, missing verification, or specification drift is found before completion.
- Compare operational value with the time, token, and stale-map overhead.

## Risks

- A consistent but incorrect Intent Brief can create an orderly misunderstanding.
- Delivery Maps can become stale or ceremonial.
- Task modeling can expand Belay into project management.
