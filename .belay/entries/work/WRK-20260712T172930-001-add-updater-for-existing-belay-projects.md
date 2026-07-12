---
schema_version: 1
id: WRK-20260712T172930-001-add-updater-for-existing-belay-projects
type: work
title: Add updater for existing belay projects
status: in-progress
created_at: 2026-07-12T17:29:30+09:00
updated_at: 2026-07-12T17:29:39+09:00
revision: 3
tags: []
links:
- relation: implements
  id: PLN-20260712T164000-001-deliver-phase-6-assurance-incrementally
- relation: fulfills
  id: GOAL-20260712T163956-001-maintain-intent-to-evidence-alignment-during-age
metadata: {}
---

Added a guarded multi-project update script that builds this checkout, refreshes generated assets, preserves existing AGENTS/Codex/Claude activation, optionally enables integrations or rebuilds state, and runs doctor. Added integration tests for activation preservation and explicit initialization.
