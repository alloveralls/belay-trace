# Repository operating policy

## The workspace is the boundary

Reads and writes inside the workspace are allowed. Anything outside it is asked.

The workspace is the Git repository root plus `$TMPDIR`. Inside it, edit,
create, delete, rename, build, and run tests freely — no permission ritual, no
narrow command allowlist. Outside it, stop and let the human decide.

Codex enforces this with `sandbox_mode = "workspace-write"`, not with a rule
table. A command that tries to write outside the workspace is refused by the
sandbox and escalated through `approval_policy`, which is the human decision.
Network egress is allowed from inside the sandbox, so package managers, VCS
remotes, and API calls are ordinary work.

`$TMPDIR` is the scratch directory. It is sandbox-writable and already outside
version control; use it rather than `/tmp` or a repository-local `.tmp/`.

Run `/status` after installation and confirm `workspace-write` and an
interactive approval policy; inspect `/hooks` in the same pass.

## What is still off limits

- `.agent-safety/`, `.agents/`, `.claude/`, `.codex/`, `AGENTS.md`, and
  `CLAUDE.md` define this boundary, so the agent bound by them does not rewrite
  them. Patches are denied and a shell command naming one prompts. Propose the
  change to the human instead.
- `.git/` and `.jj/` internal state is not edited directly. Use the version
  control command.
- Actions whose effect leaves the workspace — pushing, publishing, deploying,
  changing production or any external system — need explicit approval for that
  exact action, whatever the sandbox permits.
- Treat a hook or sandbox denial as a hard boundary. Do not disable, rewrite, or
  work around it, and never use `--yolo` or danger-full-access.

Deletion and rename inside the workspace are deliberately allowed: version
control is the recovery mechanism, and gating every `rm` on a human bought less
safety than it cost. Work on a dedicated branch, jj change, or disposable
worktree, and never let two agents write to the same checkout — that, not a
denylist, is what keeps a mistake recoverable.

Stop after the same failure occurs three times, or when outcome, ownership,
scope, or verification becomes materially ambiguous.

<!-- belay-trace:start -->
## belay-trace

For Tier 2 and Tier 3 work, follow the repository-installed
`.agents/skills/belay-trace/SKILL.md`. The Skill owns context retrieval, Intent Briefs,
Delivery Maps, reconciliation, Evidence, and conflict-safe trace updates.

If the Skill is unavailable, run
`belay context compile "<task>" --format agent --budget 4000` before broad
history reads and preserve all repository-specific human approval gates.
<!-- belay-trace:end -->

## Safe autonomous execution

Use `.agents/skills/safe-autonomy/SKILL.md` when installed. In this checkout the
reviewable installation sources are `agent-config/codex/` for Codex and
`agent-config/claude/` for Claude Code; the shared canonical workflow is
`.agent-safety/safe-autonomy/SKILL.md`. Each product installs with its own
non-overwriting installer (`install.sh`) and both merge the same
`.agent-safety/` runtime.

Start unclear work with `/plan`; start an approved measurable outcome with
`/goal`. For Tier 2 and Tier 3 work, frame and map the task with `belay-trace`,
execute one Delivery Map task at a time, and attach fresh Evidence before
progressing.

Completion requires repository tests, the boundary fixture
(`python3 .agent-safety/safe-autonomy/scripts/test_boundary.py`), complete belay
coverage, and human acceptance. Permission to pursue `/goal` is not permission
for external or irreversible actions.

## Difficulty-based Codex routing

Keep design, planning, difficulty classification, orchestration, and final
synthesis on the root thread using the model and reasoning effort selected by
the human for that session. Do not replace or pin the root model.

Before implementation, record `difficulty: low|medium|high` in the belay
Delivery Map and follow `.agent-safety/routing.json` exactly:

- low: `implement_low` (`gpt-5.6-luna`, low); fresh review with `review_low`
  (`gpt-5.6-terra`, medium).
- medium: `implement_medium` (`gpt-5.6-luna`, high); fresh review with
  `review_medium` (`gpt-5.6-terra`, high).
- high: `implement_high` (`gpt-5.6-terra`, high); fresh review with
  `review_high` (`gpt-5.6-sol`, high).

Task intake is a belay fragment, not prose: the orchestrator passes
`PLN-...#t-nnn` and the worker resolves it with `belay show`. For the two-phase
split, the fixed spawn forms, the blocked-return protocol, and the recording
checkpoints, follow `.agent-safety/safe-autonomy/SKILL.md` (sections "Specify a
task" through "When to record"). Ask the human before proceeding when
classification is ambiguous or changes during execution.

### Claude Code routing

Claude Code uses the `claude` block of `.agent-safety/routing.json` and the
subagent definitions in `.claude/agents/`. It routes `implement_low`,
`implement_medium`, `implement_high` and the matching fresh reviewers under the
same orchestration constraints as the Codex matrix, including the
pointer-only handoff, and it must never claim to have run the OpenAI
profiles.
