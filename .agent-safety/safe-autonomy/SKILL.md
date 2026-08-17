---
name: safe-autonomy
description: Run long-lived Codex goals or Claude Code loops safely inside a sandboxed workspace boundary, with recoverable checkouts, explicit stop conditions, and belay intent/evidence tracking. Use for autonomous or unattended coding, multi-step implementation, repeated repair loops, or any task where an agent may edit files without continuous human review.
---

# Safe Autonomy

## Establish boundaries

The boundary is the workspace: reads and writes inside the Git repository root
and `$TMPDIR` are allowed, and anything outside is asked. The product sandbox
enforces it at the OS level, so unattended work does not depend on recognising
a command.

1. Work in a dedicated branch, jj change, or disposable worktree. Never share a
   writable checkout between concurrent agents. Inside the workspace the agent
   may delete and rename, so version control is what makes a mistake
   recoverable — not a denylist.
2. Confirm the sandbox is actually on before starting unattended work: Codex
   `/status` shows `workspace-write`, Claude Code `/hooks` and `/permissions`
   show the project settings and the boundary hook loaded. A session that
   cannot sandbox must stop, not continue unconfined.
3. Read the repository `AGENTS.md` or `CLAUDE.md`, then use `belay-trace` for
   Tier 2 or Tier 3 work.
4. State outcome, constraints, verification, and stop conditions before editing.
   Permission to pursue a goal is not permission to push, publish, deploy, or
   alter any external state; those leave the workspace whatever the sandbox
   permits.
5. Do not weaken the boundary during the run. `.agent-safety/`, `.agents/`,
   `.claude/`, `.codex/`, `AGENTS.md`, and `CLAUDE.md` are denied to the agent
   they bind; propose changes to the human instead.

## Execute bounded units

1. Use one Delivery Map task as the current unit.
2. Inspect status and diff before the unit.
3. Make the smallest reversible change that advances one success criterion.
4. Run focused validation and record fresh Evidence.
5. Reconcile the Delivery Map before starting another unit.

## Specify a task

Belay owns the shape of a task record. The Delivery Map row defines the task
and holds its state, and it is the sole definition site of the `#t-nnn`
fragment; the matching `## T-NNN` section in the same Plan holds the detail and
is what `belay show <plan-id>#t-nnn` returns. Baseline fields — Objective,
Scope, Steps, Acceptance, Verification — and the check that every row has a
section are belay's, enforced by `belay plan lint`. Read
`docs/id-reference-standard.md` §2.1 for that standard rather than restating it
here.

What routed execution adds is one bar and two fields.

The bar: a task section must be sufficient on its own. A reader with no prior
conversation — a cold subagent, or a human who has never seen this thread — can
carry out the task from that section plus the entries its Inputs name. Nothing
required to do the work may live only in the orchestrator's context, because
that context is exactly what a spawned worker does not receive. A task that
fails this bar is not ready to route.

The two fields:

- **Difficulty** — `low`, `medium`, or `high`, per `.agent-safety/routing.json`.
  It selects the worker profile, so it belongs in the record the worker reads,
  not only in the orchestrator's head.
- **Assumption latitude** — what the worker may assume and proceed on, and what
  it must return instead. Defaults to the blocked-return protocol below; state
  it per task when that task needs a tighter or looser line.

Add a `Difficulty` column to the Delivery Map as well. Belay does not mandate a
column set and `plan lint` ignores columns it does not require, so this is a
permitted extension, not a divergence.

## Route implementation and review

This model matrix applies to Codex orchestration. Read
`.agent-safety/routing.json`, classify each implementation Delivery Map task,
record `difficulty: low|medium|high`, and spawn the exact implementation agent:

- low: `implement_low` (`gpt-5.6-luna`, low)
- medium: `implement_medium` (`gpt-5.6-luna`, high)
- high: `implement_high` (`gpt-5.6-terra`, high)

After implementation, spawn the matching review agent as a separate agent:

- low: `review_low` (`gpt-5.6-terra`, medium)
- medium: `review_medium` (`gpt-5.6-terra`, high)
- high: `review_high` (`gpt-5.6-sol`, high)

Claude Code cannot launch those profiles. It uses the `claude` block of
`.agent-safety/routing.json` with the same orchestration constraints: the root
model stays on the human's session selection, the reviewer is a separate agent,
and the review prompt excludes the implementation transcript, implementer
conclusions, expected findings, and any suggested verdict.

Keep design, planning, difficulty classification, orchestration, and final
synthesis in the root thread. Never set or replace the root model or reasoning
effort; use the human's active session selection. Do not let an implementation
agent review its own work; spawn the reviewer as a separate agent, not a
follow-up message to the implementer. Escalate ambiguous classifications to the
human before spawning. The prompts for both are fixed forms — see "Hand off by
pointer" below.

### Two phases

Separate the run into a plan phase and a loop phase. They have opposite cost
shapes, and conflating them is what inflates a long run.

The **plan phase** is write-heavy and happens once: framing, mapping, authoring
each task section, classifying difficulty. Detail written here costs O(1) per
task. It ends when the Delivery Map and its task sections pass `belay plan lint`
and the human has approved them, and it is the natural point to compact — every
durable output is in belay by then.

The **loop phase** is spawn, return, reconcile, repeated per task. Cost here is
O(rounds), so nothing that belongs in a record may travel in a prompt. During
the loop the orchestrator is a pointer broker, not a context carrier.

### Hand off by pointer

Every subagent spawn starts cold, so the temptation is to make its prompt
self-contained — and the orchestrator then pays for that content twice, once
composing it and once holding it. Pass identifiers instead, and let each worker
resolve them.

The implementer spawn prompt carries exactly:

```text
Task:       PLN-...#t-nnn
Difficulty: low|medium|high
Workspace:  <branch, jj change, or worktree>
Resolve the task with `belay show PLN-...#t-nnn` before doing anything else.
Read the Plan's Intent Brief for Constraints and Non-goals.
```

Nothing else. No restated objective, no repeated constraints, no diff body, no
evidence output. If the task cannot be understood from that fragment, the task
section is inadequate — fix the section, do not pad the prompt.

`belay show PLN-...#t-nnn` returns one task's row and section rather than the
whole Plan: measured at 6x less output on a six-task Plan and 8x on a ten-task
one, and the gap widens as a Plan grows. That is the whole reason the pointer
is a fragment and not an entry ID.

The implementer persists its outcome as belay entries while it works, not as
prose in its final report: `belay add work` holding the diff locator (branch, jj
change ID, or commit range — never the diff body) plus assumptions and unknowns,
and `belay verify record` for each validation run, linked to the task with
`implements` / `verifies`. These entries hold facts only — locators, commands,
exit codes, raw output — never a verdict or expected findings for the reviewer
to rubber-stamp. Its report back is a pointer: task fragment, Work ID, Evidence
IDs, one-line pass/fail.

The reviewer spawn prompt is the same fixed shape:

```text
Task:     PLN-...#t-nnn
Work:     WRK-...
Evidence: EVD-..., EVD-...
Resolve each pointer yourself before forming an opinion. A pointer that does
not resolve is a gap to report, not something to guess past.
```

Because the reviewer receives only identifiers, the implementation transcript
and the implementer's conclusions have no channel into the review. That makes
the exclusion rule structural rather than a promise — though it remains an
orchestration obligation, not proof that a product runtime erased inherited
context. Inspect the spawned thread when the distinction is material.

The reviewer resolves pointers with its own read-only tools — `belay show`,
`jj diff <locator>` / `git diff <locator>`, direct file reads — and reports back
pointer-sized too: severity-ranked findings with file and line evidence, not a
restatement of the diff.

If belay is unavailable, fall back to a fixed per-task path (for example
`.agent-safety/handoff/<task-id>/diff.patch` and `.../evidence.md`) written by
the implementer and read by the reviewer. The orchestrator still names the path;
it never carries the content.

### Blocked returns

A worker that cannot proceed stops and returns an identifier. What counts as
"cannot proceed" is graded, because stopping on every small uncertainty turns
each cold spawn into a wasted round.

Ambiguity is **material** when it would change the outcome, touch security or
data loss, be irreversible, create an external commitment, or widen scope. Only
material ambiguity stops the worker. It records a Work entry naming the
ambiguity and the decision needed, and returns that ID alone.

Everything else is not a reason to stop. Record the assumption in the Work entry
and continue, as the belay-trace workflow already directs for small reversible
assumptions. A task's Assumption latitude field overrides this default when that
task needs a different line.

A task admits at most **two** blocked returns. On the first, the orchestrator
amends the task section — the specification was inadequate — and respawns. On
the second, it stops and asks the human, carrying the task fragment, both Work
IDs, and the decision needed. It does not spawn a third time, and it does not
implement the task itself: doing so would pull the task's full context into the
orchestrator and bypass the difficulty routing. An unattended run ends here,
consistent with the other stop conditions.

### Who writes what

The orchestrator owns Delivery Map state. On every subagent return it writes the
new task state before doing anything else, so an interruption at any point
leaves the map truthful.

Concurrent belay writers are safe. `add`, `link`, and `status` serialize,
because the managed Markdown mirror is written inside the SQLite IMMEDIATE
transaction; see `docs/id-reference-standard.md` §6 for the guarantee and its
two bounds — a 5-second busy timeout under heavy contention, and the window
between mirror write and commit if a process is killed. Parallel workers may
therefore each record their own Work and Evidence. The existing conflict-safety
rule for `belay sync` still applies to direct Markdown edits.

### When to record

Checkpoint by cost, not by clock. Pointer operations — `belay status`, `belay
link`, a short `belay verify record` — are cheap; run them at every state
change. Body operations are bounded per task: one Work entry, one Evidence
entry per verification task, and one `belay context compile` at task start.

These are checkpoints: a Delivery Map state transition, a blocked return, a
changed assumption or scope, the end of the plan phase, and immediately before
any compaction.

These are not: intermediate implementer progress, individual file edits, and
validation commands that are not the acceptance evidence.

The invariant this serves is that the orchestrator's working state must be
reconstructible from belay at any moment. That, not a token threshold, is what
makes compaction safe — and `/compact` is a human command that the model cannot
invoke on itself, so the discipline has to hold for automatic compaction too.

## Work inside the boundary

Local tooling needs no special routing. `git`, `jj`, `belay`, `rtk`, build and
test commands, package managers, and network calls all run under the sandbox
like any other command, and the sandbox is what keeps them inside the
workspace. There is no command allowlist to consult and no proxy that grants
extra privilege.

What the sandbox cannot judge is intent that reaches past the workspace. Stop
and request a human decision before pushing, publishing, deploying, changing
production or any external system, editing the safety configuration, expanding
scope, or on the third repeat of the same failure.

Keep the disposable worktree or VCS boundary even so. The sandbox confines where
a process can write; it does not decide whether a write was correct, and an
approved test process can still mutate the workspace in ways no hook inspects.

## Complete

Run `scripts/test_boundary.py`, the repository tests, `belay plan lint
<plan-id>`, `belay coverage`, and a final diff review. `plan lint` is what
checks that every Delivery Map row still has a task section carrying its
required fields; erwin adds no validator of its own for this.
In the configuration source repository, also run
`scripts/test_agent_routing.py` and `scripts/test_claude_agent_routing.py`.
Those two validate the `agent-config/` bundle against `routing.json`, so the
installer deliberately leaves them uninstalled and they are absent in a project
checkout.

Do not claim completion while any criterion lacks fresh Evidence.

Read [references/goal-template.md](references/goal-template.md) when drafting a
`/goal`, `/loop`, or equivalent unattended prompt.
