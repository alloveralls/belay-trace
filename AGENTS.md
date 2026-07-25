# Project Guidelines

## Core Principles

- Optimize for resumability, traceability, and safe autonomous execution.
- Preserve rationale, not only implementation details.
- Separate facts, assumptions, hypotheses, and conclusions.
- Use `belay-trace` as the source of truth for goals, plans, decisions, work,
  reviews, evidence, and durable notes.
- Keep managed Markdown under `.belay/entries/` tracked in version control.
- Treat `.belay/state/` as local operational state, never as a review artifact.

## Human-Gated Workflow

This project uses explicit human gates. Agents must not autonomously transition
between planning, implementation, issue creation, pull request creation, or
merge phases.

Explicit human approval is required before:

- creating an actual issue
- starting implementation
- creating a pull request
- merging a pull request

Ambiguous responses such as "looks good", "sounds fine", or "進めて" are not
sufficient unless the exact approved action is clear from the immediate
context.

## Start Every Task With Context

Before broad historical reads, prefer:

```sh
belay context compile "<task>" --format agent --budget 4000
```

If `compile` is unavailable, run:

```sh
belay context "<task>" --format agent --budget 2500
```

Then use:

```sh
belay search "<targeted query>"
belay show <entry-id>
```

Do not broadly scan `.belay/entries/`. Read a managed Markdown file directly
only when `belay show` or another focused command identifies it as relevant.

## Planning Flow

Use planning flow when the human asks to plan, clarify, design, scope, prepare
an issue, or compare approaches.

During planning:

1. Retrieve related context with `belay context compile`.
2. Classify the work as Tier 1, Tier 2, or Tier 3.
3. For Tier 2 and Tier 3, create or update a Goal and Plan before
   implementation.
4. Add an Intent Brief with Problem, Desired Outcome, Success Signals,
   Constraints, Non-goals, Assumptions, and Unknowns / Decisions Needed.
5. Add a Delivery Map with stable Task IDs, observable outcome tasks, and
   verification tasks.
6. Keep `implemented` and `verified` separate.
7. Create decision entries with `belay add decision` when meaningful tradeoffs
   or contracts are established.
8. Link related entries with `fulfills`, `supports`, or `references`.
9. Run `belay goal lint <goal-id>` for new or materially changed Goals.
10. Draft requested issue content without creating the actual issue.
11. Stop at the relevant human gate.

Planning does not authorize source changes, `jj new`, issue creation, pull
request creation, or merge execution.

When the human explicitly approves a plan, set it to `approved`:

```sh
belay status <plan-id> approved
```

Record approval as Evidence when the approval scope matters:

```sh
belay verify record \
  --kind human-approval \
  --verdict pass \
  --source "<message-or-url>" \
  --issuer "<actor>" \
  --summary "<approved scope>" \
  --verifies <plan-id>
```

## Model and Review Budgeting

Use model strength according to task risk.

- Planning, architecture, and tradeoff analysis: prefer high-reasoning models.
- Routine implementation: prefer medium-reasoning models.
- Mechanical edits: low-reasoning models are acceptable when the intended
  change is explicit and localized.
- Review: prefer high-reasoning models, scoped to `jj diff`, changed files,
  validation output, and linked belay entries.
- Cross-model review, such as Codex implementation followed by Claude Code
  review, is optional and reserved for high-risk, broad, security-sensitive,
  production-impacting, or architecturally significant changes.

Do not invoke `/subagents` automatically for every implementation. The default
review path is a focused high-reasoning diff review. Use Codex `/subagents`,
Claude Code `/agents`, or another external agent only when the change risk
justifies the additional token and context-transfer cost.

## Implementation Flow

Start implementation only after explicit human instruction.

Before implementation:

1. Retrieve the task context and inspect linked Goals, Plans, Delivery Maps,
   decisions, reviews, evidence, and issues.
2. Run `belay sync` and resolve any drift without overwriting conflicts.
3. Create a new change with `jj new`.
4. Create a work entry with `belay add work`.
5. Link the work entry to its Goal item or Goal using `fulfills`.

During implementation:

- keep the Work entry and Delivery Map current with progress, changed files,
  validation, blockers, observations, assumptions, hypotheses, and Task states
- use stable Delivery Map Task IDs as the active work units
- mark a task `implemented` only when the change exists
- mark a task `verified` only after passing Evidence checks the mapped outcome
- run `belay sync` after directly editing managed Markdown
- create new decision entries when implementation establishes a meaningful
  architectural, API, operational, or tradeoff decision
- validate with the project's test, lint, typecheck, and build commands
- record durable validation with `belay verify record` when it supports Goal
  coverage or release decisions
- reconcile Intent Brief, Goal, Delivery Map, actual diff, and Evidence at
  meaningful checkpoints and before completion

After implementation:

1. Perform an independent implementation-time review.
   - Default: run a focused high-reasoning diff review against `jj diff`,
     changed files, validation results, and linked trace entries.
   - Use Codex `/subagents`, Claude Code `/agents`, or another external agent
     only when the change is high-risk, broad, security-sensitive,
     production-impacting, architecturally significant, or when the implementer
     cannot confidently review the affected area.
2. Create a review entry with `belay add review`.
3. Link the review to the work entry with relation `reviews`.
4. Address findings or record why they are deferred.
5. Set completed Review and Work entries to `completed`.
6. Run `belay sync`, `belay doctor`, and `belay coverage` when Goals are
   active.
7. Prepare or create a pull request only when explicitly instructed.

## Trace Entry Guidance

Follow [TRACE_GUIDE.md](./TRACE_GUIDE.md) for recommended entry bodies,
statuses, relations, and lifecycle examples.

Use these entry types:

| Type | Purpose |
|---|---|
| `goal` | Durable desired outcome, success criteria, constraints, and non-goals. |
| `plan` | Scope, approach, risks, and acceptance criteria. |
| `decision` | A concrete decision and its rationale or tradeoffs. |
| `work` | Implementation progress, evidence, blockers, and validation. |
| `review` | Findings, risks, recommendations, and review outcome. |
| `evidence` | Append-only verification records stored under `.belay/evidence/`. |
| `note` | Durable context that does not fit another entry type. |

Use display IDs in commands and cross-references. Prefer explicit links such as
`fulfills`, `supports`, `verifies`, `reviews`, `implements`, and `references`
over duplicated narrative.

## Decision Rules

Create or update a decision entry when:

- architecture changes
- API contracts change
- important tradeoffs are made
- operational rules change
- significant refactors occur
- temporary decisions are introduced
- an earlier decision is rejected or superseded

When replacing a decision:

1. Create the new decision.
2. Link it to the older decision with relation `supersedes`.
3. Set the older decision status to `superseded`.
4. Record what changed and why.

## Review Rules

Every implementation requires an independent implementation-time review before
pull request preparation.

Review entries should include:

- review method, such as `focused-high-review`, `subagent-review`,
  `cross-model-review`, or `human-review`
- model budget used for planning, implementation, and review when relevant
- findings ordered by severity
- file and line references where applicable
- risks and recommendations
- validation performed
- positive findings
- follow-up actions and owners
- whether human review is additionally required

Set `requires_human_review: true` in the review body when:

- security implications exist
- assumptions cannot be validated
- rollback strategy is unclear
- production impact is uncertain
- architectural impact is broad

Human review is additional to, not a replacement for, independent agent review.

## Conflict Safety

Never overwrite an unresolved sync conflict.

Inspect both sides and use one of these only after the intended source of truth
is known:

```sh
belay sync --prefer markdown <entry-id>
belay sync --prefer sqlite <entry-id>
```

Deletion does not propagate through normal sync. Use a terminal status such as
`abandoned`, `rejected`, `superseded`, or `archived` instead of deleting trace
history.

## Fixed Status Report

When reporting checkpoint status for Tier 2 or Tier 3 work, use this shape and
keep it consistent with the Delivery Map:

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

## Version Control

Use `jj` for all version-control operations. Do not use raw `git` commands
unless the human explicitly instructs it.

| Purpose | Command |
|---|---|
| Status | `jj st` |
| Diff | `jj diff` |
| History | `jj log` |
| New change | `jj new` |
| Edit message | `jj describe -m "message"` |
| Squash | `jj squash` |
| Bookmark | `jj bookmark set <name>` |
| Push | `jj git push` |

Use Conventional Commits:

```text
<type>[optional scope]: <description>
```

Allowed types are `feat`, `fix`, `refactor`, `docs`, `test`, `chore`, `perf`,
and `ci`.

## Merge Rules

- Never merge without explicit human instruction.
- Never merge with failing CI.
- Use squash merge.
- Never push directly to `main` unless explicitly instructed.

## Final Validation

Before handing off completed implementation:

```sh
belay sync
belay doctor
belay coverage
jj st
jj diff
```

Record the validation outcome in the work and review entries.

<!-- belay-trace:start -->
## belay-trace

### Workflow tiers

- Tier 1 (small, reversible changes): a direct user instruction is sufficient approval. Keep the change focused; a separate Plan, Decision, and Review entry is optional.
- Tier 2 (features and non-trivial changes): create or update a Goal/Plan before implementation. The Plan must contain an Intent Brief and Delivery Map; give the human an opportunity to correct the Brief before implementation. Record Work and review the completed diff in a fresh context.
- Tier 3 (architecture, API contracts, security, migrations, or irreversible operations): use the Tier 2 trace plus explicit human approval of the Intent Brief and Plan before implementation. Record material Decisions and use an independent reviewer in a fresh context. Prefer cross-model review when its added cost is justified by the risk.
- Escalate a change to the higher tier whenever scope, reversibility, or risk is uncertain. Do not create trace entries that add no durable decision, evidence, or retrieval value.

### Delivery assurance for Tier 2 and Tier 3

- Frame intent in the Plan before implementation with these non-empty Intent Brief sections: Problem, Desired Outcome, Success Signals, Constraints, Non-goals, Assumptions, and Unknowns / Decisions Needed. Write `None identified` when a section has no items.
- Label uncertain statements as assumptions or unknowns. Ask about decisions that materially change the outcome, security, data loss, external commitments, or irreversible work; proceed with explicitly recorded, small, reversible assumptions.
- Give Goal criteria document-local IDs (`SC-001`, `SC-002`, ...) and Delivery Map tasks document-local IDs (`T-001`, `T-002`, ...). Never renumber or reuse an ID. Use a fully qualified reference (`GOAL-...#sc-001` or `PLN-...#t-001`) outside the defining document.
- Map every Goal Success Criterion to observable outcome and verification tasks in a Delivery Map. Keep Task IDs stable and use only `not-started`, `in-progress`, `blocked`, `implemented`, `verified`, or `dropped`.
- Treat `implemented` and `verified` as different states. Passing Evidence is required for `verified`; a code change or test definition alone is not verification. Preserve the reason and approval source for every `dropped` task.
- Reconcile the Intent Brief, Goal, Delivery Map, actual diff, and Evidence after a meaningful task, a discovered requirement or risk, a scope or design change, before interruption or handoff, when asked for status, and before declaring completion.
- At each reconciliation, report: Current state counts; Goal coverage; Changed assumptions; Human decisions needed; Next action. Update the Delivery Map instead of reporting a state that it does not contain.
- Before completion, use a fresh context to check every Success Criterion has a task and valid Evidence, no blocked or implemented-only item is treated as complete, the diff respects Constraints and Non-goals, and changed scope or dropped tasks have approval. Record human acceptance for the final outcome.

### Trace and approval

- Run `belay context "<task>" --format agent --budget 2500` before broad historical log reads.
- Prefer `belay context compile "<task>" --format agent --budget 4000` at task start when available.
- Search for related Goals before creating Work or Decision entries; link Work/Decision to Goals with `fulfills`.
- Run `belay goal lint <goal-id>` after drafting or materially editing a Goal.
- Record validation with `belay verify record` and inspect coverage with `belay coverage` before release decisions.
- Use `belay search "<query>"` for targeted discovery and `belay show <id>` only when a full entry is needed.
- Use `belay add`, `belay link`, and `belay status <id> <status>` for trace updates.
- Before marking a Plan approved, preserve who approved it, when, and the source message or URL. Prefer append-only Evidence with `belay verify record --kind human-approval --verdict pass --issuer "<actor>" --source "<message-or-url>" --summary "<scope>" --verifies <plan-id>`.
- Run `belay sync` after direct managed Markdown edits.
- Never overwrite an unresolved sync conflict. Inspect it and use an explicit `belay sync --prefer markdown <id>` or `belay sync --prefer sqlite <id>` only after the intended source of truth is known.
- Do not scan `.belay/entries/` broadly unless a specific source path is required.

### Review and documentation boundaries

- Independent review requires context separation: use a fresh sub-agent or session that did not implement the change. Higher reasoning effort in the implementation context is not independent review.
- Keep durable system documentation in `docs/`. Use Goal, Plan, Decision, Work, and Review entries for intent, history, trade-offs, execution, and evidence; link to durable docs instead of duplicating them.
<!-- belay-trace:end -->
