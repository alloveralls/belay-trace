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

For Tier 2 and Tier 3 work, follow `.agents/skills/belay-trace/SKILL.md`. It
owns: tier classification, context retrieval (`belay context compile` /
`search` / `show`), the Intent Brief and Delivery Map, reconciliation and the
fixed status report, the belay command reference, entry title and body rules,
and sync-conflict safety.

Do not re-derive those rules here. `TRACE_GUIDE.md` is an optional reference
for entry-body templates; read it only when authoring an unfamiliar entry
type.

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

- Planning, architecture, tradeoff analysis, and review: high-reasoning
  models.
- Routine implementation: medium-reasoning models. Mechanical, localized
  edits: low-reasoning models are acceptable.
- The default review path is one focused high-reasoning diff review in a
  fresh sub-agent or session. Do not invoke multiple review agents
  automatically. Reserve cross-model review for high-risk, broad,
  security-sensitive, production-impacting, or architecturally significant
  changes.

## Implementation Flow

Start implementation only after explicit human instruction. Then:

1. Run `belay sync` and resolve any drift without overwriting conflicts.
2. Create a new change with `jj new` and a work entry with `belay add work`,
   linked `fulfills` to its Goal item.
3. Implement per the SKILL.md Execute rules, validating with the project's
   test, lint, typecheck, and build commands.
4. Run the required validation, then perform the independent review (below).

## Review Rules

Every implementation requires one independent implementation-time review
before pull request preparation.

- Reviews require context separation: a fresh sub-agent or session that did
  not implement the change. Higher reasoning effort in the implementation
  context is not independent review.
- Give the reviewer everything it needs in the spawn prompt: the `jj diff`,
  changed files, acceptance criteria, relevant decisions, and the latest
  validation output. The reviewer must not run `belay context compile`, read
  skill files, or load unrelated trace history.
- Record the review with `belay add review`, linked `reviews` to the work
  entry.
- Classify findings as `blocking` (likely correctness, security, data-loss,
  contract, Success Criterion, or required-validation failure) or
  `non-blocking` (everything else).
- Address all blocking findings in one consolidated remediation pass.
  Non-blocking findings may be fixed when trivial or deferred with rationale
  and an owner. Remediation is part of the original implementation and does
  not automatically require another review.

A second and final independent review is allowed only when the remediation
materially changes architecture, an API contract, security behavior, a
migration, rollback, or production operations; expands beyond the approved
scope; required validation still fails; a blocking finding cannot be resolved
through objective evidence; or the human explicitly requests it. Scope it to
the remediation only.

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

When preparing a release, update the package version in `Cargo.toml` and the
root `belay-trace` package entry in `Cargo.lock` together. Before opening the
release PR, verify that `belay --version`, `CARGO_PKG_VERSION`, and the root
lockfile package version agree. Keep historical trace entries and versions of
external dependencies unchanged. The CLI regression test in `tests/cli.rs`
must remain passing so future version bumps cannot omit one of these locations.

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

For Tier 2 and Tier 3 work, follow the repository-installed
`.agents/skills/belay-trace/SKILL.md`. The Skill owns context retrieval, Intent Briefs,
Delivery Maps, reconciliation, Evidence, and conflict-safe trace updates.

If the Skill is unavailable, run
`belay context compile "<task>" --format agent --budget 4000` before broad
history reads and preserve all repository-specific human approval gates.
<!-- belay-trace:end -->
