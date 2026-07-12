---
name: belay-trace
description: Use when a task needs project plans, decisions, work history, review context, or trace updates through the local belay CLI.
---

# belay-trace

## Retrieve context

1. Run `belay context compile "<task>" --format agent --budget 4000` at task start.
2. Use `belay search "<query>"` for targeted discovery.
3. Use `belay show <id>` only when the full entry is needed.
4. Avoid broad reads of `.belay/entries/` unless a command identifies a specific source path.

## Update trace

1. Use `belay add goal` for intent, then link Work/Decision entries to it with `fulfills`.
2. Run `belay goal lint <goal-id>` after drafting or materially editing a Goal.
3. Use `belay add`, `belay link`, and `belay status <id> <status>` for structured updates.
4. Record validation with `belay verify record` and inspect `belay coverage` before release decisions.
5. Run `belay sync` after direct managed Markdown edits.
6. Run `belay doctor` when generated or active integration may be stale.

## Conflict safety

Never overwrite an unresolved sync conflict. Inspect both sides and use
`belay sync --prefer markdown <id>` or `belay sync --prefer sqlite <id>` only
after the intended source of truth is known.

Repository-specific policy belongs in the repository `AGENTS.md` or
`CLAUDE.md`, as applicable, not in this generic skill.
