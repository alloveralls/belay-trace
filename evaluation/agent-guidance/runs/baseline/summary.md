# Baseline summary

| Scenario | Result | Help | Invalid | Gate violations | Primary finding |
| --- | ---: | ---: | ---: | ---: | --- |
| S-001 | 10/10 | 0 | 0 | 0 | runtime recognition Unknown |
| S-002 | 10/10 | 2 | 0 | 0 | one-time syntax confirmation |
| S-003 | 6/10 blocked | 0 | 0 | 0 | Claude runner could not execute Bash |
| S-004 | 6/10 blocked | 0 | 0 | 0 | stale Skill recognized; Bash unavailable |

## Classification

Facts:

- Codex completed both routine scenarios with zero invalid invocations and zero
  gate violations.
- S-002 used help once for entry body syntax and once for link direction because
  the current Skill names the commands but provides no executable lifecycle
  recipe.
- Claude explicitly recognized `belay-trace` in both sessions.
- Claude could not start any command because its runner failed to create a
  per-session environment directory.
- In S-004 the runtime recognized the stale one-line Skill rather than canonical
  generated guidance.

Conclusion:

- The content gap is narrow, not systemic. Add compact routine recipes for the
  lifecycle and assurance paths without copying the full CLI reference.
- The larger observability gap is distinguishing generated, installed,
  repository-active/current, stale/malformed, runtime-recognized, and
  runtime-executable states. Belay can deterministically report repository
  state only; runtime recognition and execution remain external observations.
- No new CLI state word or command is required. Improve existing Skill, README,
  doctor help/output guidance, and deterministic fixtures within the approved
  Plan boundary.

