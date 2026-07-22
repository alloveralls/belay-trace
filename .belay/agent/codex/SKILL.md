---
name: belay-trace
description: Use for Tier 2 or Tier 3 coding work, or whenever a task needs project intent, plans, decisions, delivery status, review context, or trace updates through the local belay CLI. Frame an Intent Brief and Delivery Map before implementation, reconcile them during execution, and assure completion against fresh Evidence.
---

# belay-trace

## Classify the work

- Tier 1 is a small, reversible change with clear scope. A separate Plan is optional.
- Tier 2 includes features and non-trivial changes. Create or update a Goal and Plan before implementation, and give the human an opportunity to correct the Intent Brief.
- Tier 3 includes architecture, API contracts, security, migrations, and irreversible operations. Require explicit human approval of the Intent Brief and Plan before implementation.
- Escalate when scope, reversibility, or risk is uncertain.

## Frame

1. Run `belay context compile "<task>" --format agent --budget 4000` at task start.
2. Use `belay search "<query>"` for targeted discovery.
3. Use `belay show <id>` only when the full entry is needed.
4. Avoid broad reads of `.belay/entries/` unless a command identifies a specific source path.
5. Draft an Intent Brief in the Plan with non-empty Problem, Desired Outcome, Success Signals, Constraints, Non-goals, Assumptions, and Unknowns / Decisions Needed sections. Use `None identified` when there are no items.
6. Separate facts, assumptions, unknowns, and human decisions. Ask before choices that materially change the outcome, affect security or data loss, create external commitments, or are irreversible. Explicitly record and proceed with small, reversible assumptions.

## Map

1. Give each Goal Success Criterion a stable ID.
2. Add a Delivery Map to the Plan with columns: ID, Goal item, Outcome / Task, Actor, State, and Verification / Evidence.
3. Map every Success Criterion to an observable outcome task and a verification task. Explain any task that has no Goal item.
4. Keep Task IDs stable. Use only `not-started`, `in-progress`, `blocked`, `implemented`, `verified`, and `dropped`.
5. `implemented` means the change exists; `verified` requires fresh passing Evidence that actually checks the mapped outcome. A test definition is not passing Evidence.
6. Keep dropped tasks visible and record the reason and approval source.

## Execute

1. Use the Delivery Map Task ID as the active work unit and keep its state current.
2. Add newly discovered tasks, assumptions, unknowns, constraints, and scope changes instead of silently absorbing them.
3. Link Work and Evidence to the relevant Goal item using existing `fulfills` and `verifies` relations.
4. Reconcile after a meaningful task, a discovered requirement or risk, a scope or design change, before interruption or handoff, when the human asks for status, and before declaring completion.

Use this fixed reconciliation report and make it agree with the Delivery Map:

```text
Current state
- verified: <n>/<total>
- implemented, unverified: <n>/<total>
- in progress: <n>/<total>
- blocked: <n>/<total>

Goal coverage
- <criterion>: <verified|partial|not started>

Changed assumptions
- <change or None identified>

Human decisions needed
- <decision or None identified>

Next action
- <single next action>
```

## Assure completion

Use a fresh context that did not implement the change to review the Intent Brief, Goal, Delivery Map, actual diff, and Evidence. Do not declare the Goal complete until:

- every Success Criterion has delivery tasks and relevant passing Evidence;
- no `implemented`, `blocked`, or important unknown item is counted as complete;
- the diff respects Constraints and Non-goals;
- specification changes and dropped tasks have reasons and approval sources; and
- the human has accepted the final outcome and that acceptance is recorded as Evidence.

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
