# Repository operating policy

## The workspace is the boundary

Reads and writes inside the workspace are allowed. Anything outside it is asked.

The workspace is the Git repository root plus `$TMPDIR`. Inside it, edit,
create, delete, rename, build, and run tests freely — no permission ritual, no
narrow command allowlist. Outside it, stop and let the human decide.

Claude Code's sandbox enforces this, not a rule table. A command that tries to
write outside the workspace fails inside the sandbox; retrying it unsandboxed
falls back to a normal permission prompt, which is the human decision. Network
egress is allowed from inside the sandbox, so package managers, VCS remotes, and
API calls are ordinary work.

`$TMPDIR` is the scratch directory. It is sandbox-writable and already outside
version control; use it rather than `/tmp` or a repository-local `.tmp/`.

## What is still off limits

- `.agent-safety/`, `.agents/`, `.claude/`, `.codex/`, `AGENTS.md`, and
  `CLAUDE.md` define this boundary, so the agent bound by them does not rewrite
  them. Edits are denied and a shell command naming one prompts. Propose the
  change to the human instead.
- `.git/` and `.jj/` internal state is not edited directly. Use the version
  control command.
- Actions whose effect leaves the workspace — pushing, publishing, deploying,
  changing production or any external system — need explicit approval for that
  exact action, whatever the sandbox permits.
- Treat a hook or sandbox denial as a hard boundary. Do not disable, rewrite, or
  work around it, and never run with `--dangerously-skip-permissions`.

Deletion and rename inside the workspace are deliberately allowed: version
control is the recovery mechanism, and gating every `rm` on a human bought less
safety than it cost. Work on a dedicated branch, jj change, or disposable
worktree, and never let two agents write to the same checkout — that, not a
denylist, is what keeps a mistake recoverable.

Stop after the same failure occurs three times, or when outcome, ownership, or
verification becomes materially ambiguous.

<!-- belay-trace:start -->

## belay-trace

For Tier 2 and Tier 3 work, follow the repository-installed
`.claude/skills/belay-trace/SKILL.md`. The Skill owns context retrieval, Intent Briefs,
Delivery Maps, reconciliation, Evidence, and conflict-safe trace updates.

If the Skill is unavailable, run
`belay context compile "<task>" --format agent --budget 4000` before broad
history reads and preserve all repository-specific human approval gates.
<!-- belay-trace:end -->

Use `.claude/skills/safe-autonomy/SKILL.md` whenever `/loop`, unattended execution,
or repeated implementation/repair cycles are requested.

## Bounded execution

Before editing, record the outcome, constraints, verification, and stop
conditions in the belay Intent Brief and Delivery Map. Execute one Delivery Map
task at a time, inspect the diff, validate it, record fresh Evidence, and
reconcile the map before continuing.

Goal completion requires passing repository tests, the boundary fixture
(`python3 .agent-safety/safe-autonomy/scripts/test_boundary.py`), complete belay
coverage, and human acceptance. Permission to pursue a goal is not permission
for external or irreversible actions.

## Difficulty routing

`.agent-safety/routing.json` holds two matrices. The top-level `implementation`
and `review` blocks are a Codex orchestration policy over OpenAI model profiles
that Claude Code cannot launch and must not claim to have followed. The
`claude` block is Claude Code's own matrix, backed by the subagent definitions
in `.claude/agents/`.

Task intake is a belay fragment, not prose: the orchestrator passes
`PLN-...#t-nnn` and the subagent resolves it with `belay show`. For the
difficulty mapping, routing constraints, the fixed spawn forms, the
blocked-return protocol, and the recording checkpoints, follow
`.claude/skills/safe-autonomy/SKILL.md` (section "Route implementation and
review with Claude subagents") and the canonical sections it cites. Subagents
are opt-in — do not spawn them unless the human asked for routed or
multi-agent execution.
