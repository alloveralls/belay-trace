---
name: safe-autonomy
description: Run Claude Code /loop or repeated agent work safely inside a sandboxed workspace boundary, with recoverable worktrees, stop conditions, and belay evidence. Use for long-running, autonomous, unattended, repeated repair, or multi-step repository work.
---

# Safe Autonomy for Claude Code

Read and follow `.agent-safety/safe-autonomy/SKILL.md`; it is the canonical
cross-product workflow. Then apply these Claude-specific checks:

1. Inspect `/permissions` and `/hooks`; confirm project settings are loaded and
   the boundary PreToolUse hook is active. Check `/sandbox` too — the workspace
   boundary is the sandbox, so an unsandboxed session has no boundary and must
   not run unattended.
2. Put the shared goal template's constraints and stop conditions in `/loop` or
   the repeated task prompt. Set a finite iteration, time, and cost budget.
3. Treat hook/permission denial as final. Never use
   `--dangerously-skip-permissions` or disable hooks or the sandbox for
   autonomous work.
4. Use `belay-trace` before implementation for Tier 2 or Tier 3 work and record
   fresh Evidence after each Delivery Map unit.
5. Before completion, run the boundary fixture, repository validation,
   `belay coverage`, and a final diff review.

Local tooling needs no routing table: `git`, `jj`, `belay`, `rtk`, builds,
tests, and package managers all run under the sandbox like any other command,
and the sandbox is what keeps them inside the workspace. Escalate instead for
anything whose effect leaves the workspace — pushing, publishing, deploying, or
changing an external system.

## Route implementation and review with Claude subagents

The OpenAI model/effort matrix under the top-level `implementation` and `review`
keys of `.agent-safety/routing.json` is a Codex execution contract. Claude Code
cannot launch or attest those profiles; never claim to have followed them.

Claude Code has its own matrix in the `claude` block of the same file, backed by
the subagent definitions in `.claude/agents/`. When the human asks for routed
execution, record `difficulty: low|medium|high` in the Delivery Map and spawn the
matching subagent through the Agent tool:

- low: `implement_low` (`sonnet`), then a fresh `review_low` (`sonnet`)
- medium: `implement_medium` (`sonnet`), then a fresh `review_medium` (`opus`)
- high: `implement_high` (`opus`), then a fresh `review_high` (`opus`)

Constraints that make this equivalent to the Codex contract:

- Keep design, planning, difficulty classification, orchestration, and final
  synthesis on the root thread, under the human's session model and effort.
  Never pin or replace the root model.
- Never let the implementing subagent review its own work. Spawn the reviewer as
  a separate agent, not a follow-up message to the implementer.
- Build the review prompt from only the approved Intent Brief, success criteria,
  actual diff, raw Evidence, assumptions, and unknowns. Exclude the
  implementation transcript, implementer conclusions, expected findings, and any
  suggested verdict. This is an orchestration obligation, not proof that the
  runtime erased inherited context.
- Escalate an ambiguous or shifting difficulty classification to the human
  before spawning.

Two differences from the Codex contract are real and must not be papered over:

- Claude Code subagent definitions pin a model but cannot pin per-agent
  reasoning effort, so `low` and `medium` implementation share one model instead
  of differing by effort. Say so rather than claiming effort parity.
- Reviewer read-only status comes from an empty write-tool set plus the shared
  PreToolUse guard, not from a runtime sandbox mode.

Task intake is a belay fragment, not prose. The orchestrator passes
`PLN-...#t-nnn`; the worker resolves it with `belay show` and reads the Plan's
Intent Brief for Constraints and Non-goals. The canonical sections "Specify a
task" through "When to record" hold the two-phase split, the fixed spawn forms,
the blocked-return protocol, and the recording checkpoints.

Subagents are opt-in. Do not spawn them unless the human asked for routed or
multi-agent execution.
