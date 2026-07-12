# belay-trace

`belay-trace` is a local-first CLI for preserving the goals, plans, decisions,
work logs, reviews, evidence, and notes behind AI-assisted software work.

SQLite is the operational store. Deterministic Markdown files under
`.belay/entries/` are the editable Git review and recovery surface.

## Install And Initialize

The v1 implementation requires Rust 1.87 or newer.

```sh
cargo build --release --locked
./target/release/belay init
```

`belay init` creates `.belay/config.toml`, local SQLite state, managed entry
directories, and generated agent integration templates. It does not modify
`AGENTS.md` or install a skill unless explicitly requested:

```sh
belay init --update-agents
belay init --install-skill codex
belay init --install-skill claude
belay init --install-skill codex --install-skill claude
```

`--install-skill` is repeatable so repositories using both agents can activate
both integrations explicitly in one command. If a copied template may contain
local SQLite state, rebuild that ignored state from tracked Markdown rather
than trusting or deleting it in place:

```sh
belay init --reset-state
```

The reset is an atomic rebuild from `.belay/entries/`; it does not delete the
tracked Markdown source. Template archives and copy scripts should exclude
`.belay/state/` entirely.

For repositories that use belay as their trace system, keep the recommended
project instructions in the repository root `AGENTS.md`. The root file should
make belay entries the source of truth for plans, decisions, work, and reviews.
`belay init --update-agents` manages only the marker-scoped
`<!-- belay-trace:start -->` / `<!-- belay-trace:end -->` section, so existing
repository-specific instructions outside that section remain under normal human
review.

## Create And Link Traces

Create entries non-interactively with inline content, a file, or standard input:

```sh
belay add goal --title "Reliable repository sync"

belay add decision \
  --title "Use SQLite for operational state" \
  --body "Keep local retrieval fast and rebuild from tracked Markdown."

belay add work \
  --title "Implement repository sync" \
  --body-file ./work-notes.md

printf '%s\n' "Review findings" | \
  belay add review --title "Sync implementation review" --stdin
```

Goal entries can omit a body source; belay writes the required Goal sections as
an editable template. Other entry types require `--body`, `--body-file`, or
`--stdin`. Entry types are `goal`, `plan`, `decision`, `work`, `review`, and
`note`. Commands print the generated display ID, for example:

```text
GOAL-20260607T085900-001-reliable-repository-sync
DEC-20260607T090000-001-use-sqlite
```

Use display IDs for links and status transitions:

```sh
belay link \
  WRK-20260607T091000-001-implement-sync \
  GOAL-20260607T085900-001-reliable-repository-sync \
  --relation fulfills

belay status DEC-20260607T090000-001-use-sqlite accepted
belay show DEC-20260607T090000-001-use-sqlite
```

Review Goal quality without calling an LLM:

```sh
belay goal lint GOAL-20260607T085900-001-reliable-repository-sync
belay goal improve GOAL-20260607T085900-001-reliable-repository-sync --budget 3000
```

## Synchronize Markdown And SQLite

Managed Markdown is editable. Import direct edits or regenerate stale mirrors:

```sh
belay sync
belay sync DEC-20260607T090000-001-use-sqlite
```

When both sides changed, sync preserves both and exits with conflict category
`5`. Resolve one entry explicitly:

```sh
belay sync --prefer markdown DEC-20260607T090000-001-use-sqlite
belay sync --prefer sqlite DEC-20260607T090000-001-use-sqlite
```

Deletion does not propagate in v1. A missing SQLite row or mirror is restored
from the remaining side.

## Search And Context

Search supports exact display IDs, structured filters, and deduplicated
FTS5/BM25 keyword results:

```sh
belay search "sqlite migration"
belay search --type decision --status accepted
belay search --tag release
belay search --id DEC-20260607T090000-001-use-sqlite
```

Generate bounded, source-attributed context for humans or agents:

```sh
belay context "implement repository sync" --format human --budget 2500
belay context "implement repository sync" --format agent --budget 2500
belay context compile "implement repository sync" --profile task-start --budget 4000
```

Context selection follows BM25 relevance, then linked entries in deterministic
order. Agent output admits a budget-scaled set of candidates, keeps at least one
Markdown evidence unit for included entries, and weights remaining space toward
higher-ranked entries while preserving the 90 percent selection limit. The
`compile` form adds Goal, constraints, non-goals, Evidence, and past-failure
sections before the ranked context.

Context uses direct links and a deterministic token estimate. Embeddings are not
required for the v1 workflow.

## Verify And Cover Goals

Record append-only Evidence for existing verification systems:

```sh
belay verify record \
  --kind test \
  --verdict pass \
  --source "cargo test" \
  --summary "unit and integration tests passed" \
  --verifies GOAL-20260607T085900-001-reliable-repository-sync

belay verify import --junit target/junit.xml --verifies WRK-20260607T091000-001-implement-sync
belay verify status GOAL-20260607T085900-001-reliable-repository-sync
```

Evidence is mirrored to `.belay/evidence/YYYY-MM.ndjson` and indexed in SQLite.
Compute Goal Coverage with traceability and verified counts kept separate:

```sh
belay coverage
belay coverage --format json --fail-under verified=60
```

## Export Snapshots

Exports are external point-in-time artifacts. They are not managed mirrors and
are never sync or rebuild inputs.

```sh
belay export markdown --output ./artifacts/belay-export.md
belay export json \
  --type decision \
  --status accepted \
  --output ./artifacts/accepted-decisions.json
belay export ndjson --tag release --output ./artifacts/release.ndjson
belay export json \
  --id DEC-20260607T090000-001-use-sqlite \
  --output ./artifacts/decision.json
```

Filters are optional and combine with AND semantics. Export ordering is
deterministic. Normal exports contain display IDs and never expose internal
SQLite integer IDs. Destinations inside `.belay/entries/` are rejected.

## Diagnose And Rebuild

Run read-only repository health checks:

```sh
belay doctor
```

Doctor checks configuration, SQLite schema and foreign keys, FTS5/BM25,
managed Markdown validity, Goal sections, Evidence freshness, sync drift,
temporary files, and agent integration.

Rebuild SQLite and search indexes from all validated managed Markdown:

```sh
belay rebuild
```

Rebuild uses a temporary database and replaces active state only after the new
database is complete.

## Exit Status

| Code | Meaning |
|---:|---|
| `0` | success, help, or version |
| `2` | invalid invocation or arguments |
| `3` | repository not initialized or configuration unavailable |
| `4` | input, entry, schema, path, or not-found validation failure |
| `5` | sync conflict or repository drift requiring resolution |
| `6` | filesystem, SQLite, storage, or runtime capability failure |

Run `belay --help` or `belay <command> --help` for command-specific behavior,
side effects, examples, and related commands.

## Repository Layout

```text
.belay/
  config.toml
  entries/
    goals/
    plans/
    decisions/
    work/
    reviews/
    notes/
  evidence/
    YYYY-MM.ndjson
  state/
    belay.sqlite
  agent/
    AGENTS.md.snippet
    claude/SKILL.md
    codex/SKILL.md
```

The SQLite database is local operational state and ignored by Git by default.
The managed Markdown entries are the tracked review and recovery surface.
The root `AGENTS.md` is the recommended place for repository-specific agent
workflow policy; `.belay/agent/AGENTS.md.snippet` is the generated integration
snippet used by `belay init --update-agents`.

Use `docs/` for durable system documentation. Keep intent, approval, trade-off
history, execution notes, reviews, and evidence in belay entries, linking to
the durable document instead of maintaining two copies of the same truth.
