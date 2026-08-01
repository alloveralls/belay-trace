# Project Guidelines

In Code Mode, within each bounded stage, run independent, functions.exec-available tool calls concurrently in one functions.exec call. Use `await Promise.allSettled([...])` when partial results are useful, and inspect every result; use `await Promise.all([...])` only when any failure should abort the batch. Keep dependencies, waits/resumes, approvals, conflicting or interdependent mutations, and adaptive investigations sequential.

## Core Principles

- Optimize for resumability, traceability, and safe autonomous execution.
- Preserve rationale, not only implementation details. Separate facts,
  assumptions, hypotheses, and conclusions.
- Use `belay-trace` as the source of truth for goals, plans, decisions, work,
  reviews, evidence, and durable notes.
- Keep managed Markdown under `.belay/entries/` tracked in version control.
  Treat `.belay/state/` as local operational state, never as a review artifact.
- Keep durable system documentation in `docs/`; link to it from trace entries
  instead of duplicating it.

## Workflow Source of Truth

Classify work with the repository risk rules below. For Tier 2 and Tier 3 work,
follow `.agents/skills/belay-trace/SKILL.md`. The Skill owns context retrieval
(`belay context compile` / `search` / `show`), the Intent Brief and Delivery
Map, reconciliation and the fixed status report, the belay command reference,
entry title and body rules, and sync-conflict safety.

Do not re-derive those rules here. `TRACE_GUIDE.md` is an optional reference
for entry-body templates; read it only when authoring an unfamiliar entry
type.

## Risk-Based Work Classification

Classify work by impact, uncertainty, and reversibility, not implementation
difficulty or diff size alone. Escalate when any required condition for a lower
tier is not established. A small diff can still be high-risk.

### Tier 1

Use Tier 1 only when all of these conditions hold:

- scope and expected behavior are clear
- the change is localized and easily reversible
- it does not affect security, authorization, permissions, persistent data,
  migrations, public API contracts, concurrency, billing, destructive
  operations, or production rollback
- focused automated or manual verification is available

Typical examples are documentation, messages, focused tests, localized bug
fixes, mechanical refactors, and development tooling. No Goal, Plan, Intent
Brief, Delivery Map, Work entry, independent review, or Review entry is
required. A Work entry is optional when the task is interrupted or durable
resumption context would be useful.

### Tier 2

Use Tier 2 when application or developer-facing behavior changes and its impact
is meaningful but bounded. The affected behavior, direct callers, validation
path, and rollback must be identifiable, with no Tier 3 area involved.

Create a concise Goal and Plan and follow the Skill's Frame, Map, Execute, and
reconciliation rules. Keep trace entries proportional: record durable intent,
decisions, work, and Evidence without duplicating implementation narration.

### Tier 3

Use Tier 3 for security, authorization, permissions, persistent-data changes,
migrations, public API contracts, concurrency, billing, destructive operations,
broad architecture, difficult rollback, broad production impact, or unresolved
high-impact uncertainty.

Apply the full belay-trace workflow. Explicit human approval of the Intent Brief
and Plan is required before implementation. Independent review is required.

## Human-Gated Workflow

This project uses explicit human gates. Agents must not autonomously
transition between planning, implementation, issue creation, pull request
creation, or merge phases.

Explicit human approval is required before:

- creating an actual issue
- starting implementation
- creating a pull request
- merging a pull request

Ambiguous responses such as "looks good", "sounds fine", or "進めて" are not
sufficient unless the exact approved action is clear from the immediate
context.

Planning does not authorize source changes, `jj new`, issue creation, pull
request creation, or merge execution. Draft requested issue content without
creating the actual issue, then stop at the relevant human gate.

When the human explicitly approves a plan:

```sh
belay status <plan-id> approved
belay verify record --kind human-approval --verdict pass \
  --source "<message-or-url>" --issuer "<actor>" \
  --summary "<approved scope>" --verifies <plan-id>
```

## Model and Review Budgeting

- The primary agent uses the repository default. Increase reasoning only when
  ambiguous planning, architecture, tradeoff analysis, or unresolved risk
  requires it.
- The project `implementer` uses low reasoning for approved, bounded Tier 2
  implementation.
- The project `reviewer` uses low reasoning for bounded Tier 2 review.
- The project `high_risk_reviewer` uses high reasoning for Tier 3, broad,
  security-sensitive, production-impacting, or difficult-to-roll-back review.
- Use one focused review agent when separate review is required. Do not invoke
  multiple review agents automatically.

## Agent Roles

Repository-scoped model and agent settings live in `.codex/config.toml` and
`.codex/agents/*.toml`. Do not duplicate model slugs or reasoning values here.

- The primary agent owns planning, architecture, tradeoff decisions, human
  gates, Belay reconciliation, integration of delegated work, and final
  delivery.
- Tier 1 does not use a subagent by default.
- For Tier 2 implementation, use the project `implementer` agent only after
  explicit implementation approval and only when the task is bounded, expected
  behavior is clear, focused verification is available, and no Tier 3 area or
  unresolved material decision is involved.
- Give the implementer the approved scope, fully qualified Delivery Map task,
  acceptance criteria, relevant decisions, allowed files or components, and
  validation requirements. The primary agent must inspect and integrate the
  returned implementation and Evidence.
- Do not delegate Tier 3 implementation to the `implementer`. The primary agent
  retains it unless the human explicitly approves another qualified approach.
- When independent review is required for bounded Tier 2 work, use the project
  `reviewer` agent in a fresh context.
- For Tier 3, security-sensitive, broad, production-impacting, or
  difficult-to-roll-back review, use the project `high_risk_reviewer` agent in
  a fresh context.
- An implementation agent must never review its own work. Review agents must
  remain read-only and must not remediate their findings.

## Implementation Flow

Start implementation only after explicit human instruction. Then:

1. Create a new change with `jj new`.
2. For Tier 2 and Tier 3, run `belay sync` and resolve any drift without
   overwriting conflicts.
3. For Tier 2 and Tier 3, create a work entry with `belay add work`, linked
   `fulfills` to its Goal item, and implement per the Skill's Execute rules.
4. For Tier 1, implement the focused change without mandatory trace entries.
5. Run proportionate tests, lint, typecheck, build, and review as required below.

## Review Rules

Apply review effort according to the classified risk:

- Tier 1: no separate review by default. Focused verification is required.
- Tier 2: perform one focused review of the changed behavior and its direct
  callers. Independent fresh-context review is optional only when focused tests
  pass, rollback is clear, and no Tier 3 area or unresolved high-impact
  uncertainty is involved. Otherwise it is required.
- Tier 3: perform one independent fresh-context review of the Intent Brief,
  Goal, Delivery Map, actual diff, direct dependencies, and Evidence. Include
  security-focused inspection within that review when security-sensitive.

- When independent review is required, use a fresh sub-agent or session that
  did not implement the change. Higher reasoning effort in the implementation
  context is not independent review.
- Give the reviewer everything it needs in the spawn prompt: the `jj diff`,
  changed files, acceptance criteria, relevant decisions, and the latest
  validation output. The reviewer must not run `belay context compile`, read
  skill files, or load unrelated trace history.
- Create a Review entry only when a separate review was performed. Link it with
  `reviews` to the Work entry when one exists.
- Classify findings as `blocking` (likely correctness, security, data-loss,
  contract, Success Criterion, or required-validation failure) or
  `non-blocking` (everything else).
- Address all blocking findings in one consolidated remediation pass.
  Non-blocking findings may be fixed when trivial or deferred with rationale
  and an owner. Remediation is part of the original implementation and does
  not automatically require another review.

A second and final independent review is allowed only when remediation addresses
a blocking finding and materially changes behavior, architecture, an API
contract, security behavior, a migration, rollback, or production operations;
expands beyond the approved scope; required validation still fails; a blocking
finding cannot be resolved through objective evidence; or the human explicitly
requests it. Scope it to the remediation only.

The review-round limit is absolute within one implementation cycle: at most
two independent review rounds, never a third even with human approval. After
the second review, make at most one final consolidated remediation pass. If
blocking uncertainty remains, mark the work blocked and request human
direction. New non-blocking findings never justify another round.

Set `requires_human_review: true` in the review body when security
implications exist, assumptions cannot be validated, rollback is unclear,
production impact is uncertain, or architectural impact is broad. Human
review is additional to, not a replacement for, independent agent review.

## Release Versioning

In a repository containing the root Rust `belay-trace` package, update the
package version in `Cargo.toml` and its root package entry in `Cargo.lock`
together when preparing a release. Before opening the release PR, verify that
`belay --version`, `CARGO_PKG_VERSION`, and the root lockfile package version
agree. Keep historical trace entries and versions of external dependencies
unchanged. The CLI regression test in `tests/cli.rs` must remain passing so
future version bumps cannot omit one of these locations. This section does not
apply to repositories without that package.

## Version Control

Use `jj` for all version-control operations. Do not use raw `git` commands
unless the human explicitly instructs it.

| Purpose      | Command                    |
| ------------ | -------------------------- |
| Status       | `jj st`                    |
| Diff         | `jj diff`                  |
| History      | `jj log`                   |
| New change   | `jj new`                   |
| Edit message | `jj describe -m "message"` |
| Squash       | `jj squash`                |
| Bookmark     | `jj bookmark set <name>`   |
| Push         | `jj git push`              |

Use Conventional Commits (`<type>[optional scope]: <description>`) with types
`feat`, `fix`, `refactor`, `docs`, `test`, `chore`, `perf`, `ci`.

## Merge Rules

- Never merge without explicit human instruction.
- Never merge with failing CI.
- Use squash merge.
- Never push directly to `main` unless explicitly instructed.

## Final Validation

Before handing off any completed implementation:

```sh
jj st
jj diff
```

For Tier 2 and Tier 3, also run:

```sh
belay sync
belay doctor
belay coverage
```

For Tier 2 and Tier 3, record the validation outcome in the Work entry and in a
Review entry when a separate review was performed.

@RTK.md

<!-- belay-trace:start -->
## belay-trace

For Tier 2 and Tier 3 work, follow the repository-installed
`.agents/skills/belay-trace/SKILL.md`. The Skill owns context retrieval, Intent Briefs,
Delivery Maps, reconciliation, Evidence, and conflict-safe trace updates.

If the Skill is unavailable, run
`belay context compile "<task>" --format agent --budget 4000` before broad
history reads and preserve all repository-specific human approval gates.
<!-- belay-trace:end -->
