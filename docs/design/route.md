# Belay Route MVP

Route prepares one Belay decision thread for external AI reasoning and explicit
human selection. Belay remains the source of truth. Route artifacts are
non-authoritative local operational state until an accepted preview is
materialized.

## Responsibility boundary

- Belay deterministically snapshots trace state, validates artifacts and
  references, checks staleness, previews writes, applies explicitly selected
  operations, and reconciles results.
- An external AI produces the semantic Assessment and Proposal and converts a
  human's chat response into a structured Human Response.
- The human authorizes an exact Proposal revision and an exact set of operation
  IDs. Applying the preview is a separate content-bound gate.
- `Human Response.issuer` is unverified attribution, not authenticated
  identity. Authorization comes from the repository's human-gate process and
  the exact preview hash supplied to `apply`. Consult a specialist before using
  this MVP where authenticated identity or non-repudiation is required.
- Route does not call an LLM, manage API keys, provide an MCP surface, or manage
  repository-wide task scheduling.

## Local run bundle

Runs are ignored local state:

```text
.belay/state/route/<run-id>/
├── manifest.json
├── input-001.json
├── assessment-001.json
├── proposal-001.json
├── response-001.json
├── preview-001.json
└── reconciliation-001.json
```

Artifacts are immutable and revisioned. `manifest.json` is atomically replaced
and points to the latest revision and its SHA-256 hash. Losing a run bundle does
not lose accepted Belay state: start a new run from the same seed. Assessment
and Proposal regeneration is intentionally not deterministic.

## CLI flow

```sh
belay route start --seed GOAL-... [--include DEC-...]
belay route template <run-id> assessment
belay route submit <run-id> assessment --file assessment.json
belay route template <run-id> proposal
belay route submit <run-id> proposal --file proposal.json
belay route template <run-id> response
belay route submit <run-id> response --file response.json
belay route preview <run-id>
belay route apply <run-id> --approve <preview-hash>
belay route status <run-id>
```

`start` includes the primary seed, its directly linked entries, explicit
`--include` entries, and Goal Coverage for Goals in that set. The input
fingerprint covers normalized content, links, and coverage. Any change to those
inputs makes the run stale before the first apply.

`template` supplies current run IDs, revisions, and hashes. It is the preferred
starting point for AI-generated JSON. A response template defaults to `defer`
and cannot accidentally authorize a write.

## Artifact authority

| Artifact | Producer | Authority |
| --- | --- | --- |
| Route Input | Belay | Deterministic snapshot |
| Assessment | External AI | Advisory |
| Proposal | External AI | Advisory |
| Human Response | AI serialization of human chat | Authorizes only exact selected operation IDs when `accept` |
| Materialization Preview | Belay | Deterministic description of proposed writes |
| Reconciliation Result | Belay | Actual per-operation result |

Assessment classifications are `fact`, `human-observation`, `assumption`,
`hypothesis`, `unknown`, and `conflict`. Facts, human observations, and
conflicts require at least one reference to an entry in Route Input.

Proposal outcomes are `continue`, `stop`, `insufficient-context`, and
`no-safe-route`. Only `continue` may contain operations.

Human actions are:

- `accept`: selects at least one exact operation ID and can be previewed.
- `revise`: contains requested changes, selects no operations, and requires a
  new Proposal revision.
- `reject` or `defer`: selects no operations and cannot be previewed.

## Materialization operations

The MVP supports:

```json
{
  "kind": "create-entry",
  "operation_id": "op-001",
  "alias": "new-decision",
  "type": "decision",
  "title": "Use operation journaling",
  "body": "Decision rationale."
}
```

```json
{
  "kind": "link",
  "operation_id": "op-002",
  "from": "$new-decision",
  "to": "GOAL-...",
  "relation": "supports"
}
```

```json
{
  "kind": "set-status",
  "operation_id": "op-003",
  "target": "PLN-...",
  "status": "approved"
}
```

Aliases must be created before an operation uses them. Entry types, statuses,
relations, IDs, fragments, and references use the existing Belay contracts.
Existing mutation targets must be present in Route Input; use `--include` when
starting the run rather than introducing an un-snapshotted target in Proposal.

## Safety and recovery

- Artifact files and manifest hashes must match.
- Revisions are monotonic per artifact type.
- Assessment, Proposal, Human Response, and Preview bind to exact prior hashes.
- `preview` rechecks the original input fingerprint and has no trace side
  effects.
- `apply` requires the exact embedded preview hash.
- Preview records the revision of every existing entry that an operation may
  mutate. The SQLite mutation transaction rejects a different current revision,
  closing the gap between run-level stale checking and each write.
- Before the first mutation, the manifest moves to `applying`, durably binding
  the run to the exact preview.
- Every create, link, and status mutation writes a unique operation receipt in
  the same SQLite transaction as the mutation. A retry after a process crash
  replays that receipt, so it cannot duplicate an already committed operation.
- Managed Markdown and SQLite are not one cross-resource transaction. If a
  process stops after a mirror write but before SQLite commit, apply stops on
  the orphan mirror; run `belay sync` to recover it. The Route metadata fallback
  then adopts that entry and recreates its receipt instead of duplicating it.
- `belay rebuild` preserves Route receipts. Create replay can also reconstruct a
  missing receipt from the unique run/operation metadata retained in Markdown.
- Multi-operation apply is operation-atomic, not batch-atomic. Reconciliation
  records `applied`, `unchanged`, or `failed` for every operation.
- Preview and Reconciliation preserve operation IDs that the human did not
  select, and Reconciliation records post-apply Goal Coverage.
- Reconciliation stores a post-apply fingerprint covering the original Input
  set and all created aliases. A retry is allowed only when current Belay state
  still matches that exact post-apply state.

The MVP assumes a trusted local filesystem without a concurrent hostile actor.
Static symlink checks prevent accidental redirection but do not claim to close
an adversarial path-check/path-use race. An expanded threat model requires
fd-relative traversal with `NOFOLLOW`; consult a specialist before relying on
Route across an untrusted filesystem boundary.
