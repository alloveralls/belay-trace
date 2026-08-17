# Shared safety runtime

This directory is the source bundle that `agent-config/install.sh` copies into a
project as `.agent-safety/`. It holds one policy file, one hook, and one fixture
suite.

## The boundary

**Reads and writes inside the workspace are allowed. Anything outside it is
asked.** That is the whole rule.

The workspace is the Git repository root plus `$TMPDIR`. Inside it the agent
edits, creates, deletes, renames, builds, and runs tests without a prompt.
Outside it, nothing happens without a human decision.

Deletion and rename are inside the boundary on purpose. Version control is the
recovery mechanism for a workspace, and pricing every `rm` at one human
interruption bought less safety than it cost.

Network egress is allowed from inside the sandbox. Package managers, VCS
remotes, and API calls are ordinary development work; confining them to the
sandbox is the control, blocking them was not.

## What enforces it

The product sandboxes, not this hook. Claude Code's `sandbox.*` settings and
Codex's `sandbox_mode = "workspace-write"` already confine a shell command's
filesystem access to the workspace at the OS level, and both escalate to the
human when a command needs more. That is a stronger boundary than any command
allowlist, because it does not depend on recognising the command.

`safe-autonomy/scripts/guard_tool.py` covers only the two gaps a filesystem
sandbox cannot express:

1. **Structured write tools.** `Edit`, `Write`, and `apply_patch` are not shell
   commands, so the sandbox never sees them. A target outside the workspace is
   asked here instead.
2. **The safety configuration itself.** `.agent-safety/`, `.agents/`,
   `.claude/`, `.codex/`, `AGENTS.md`, and `CLAUDE.md` live inside the
   workspace, so without this the files that define the boundary would be
   writable by the agent they bind. Structured writes to them are denied; a
   shell command that names one prompts.

The guard never classifies commands. Every decision it makes comes from a path.

## Facts and limits

- A PreToolUse decision must carry exactly `hookEventName`,
  `permissionDecision`, and `permissionDecisionReason` inside
  `hookSpecificOutput`, and nothing else. Codex rejects an object with an
  unknown key, reports `hook returned invalid pre-tool-use JSON output`, and
  then runs the command anyway, so one extra field turns every decision into an
  allow. Claude Code ignores extra keys, which hides the fault during
  Claude-only testing. `test_boundary.py` fails any decision that reintroduces
  an unknown key.
- The guard fails open. A malformed payload, an unknown tool, or an internal
  error produces no decision, because the sandbox still holds the boundary and a
  hook that blocks work when it breaks is how v1 lost its budget.
- The shell scan is a literal path match, not a parse. A command that merely
  mentions a protected path prompts once, whether it reads or writes. That costs
  an occasional needless prompt and buys back an entire command grammar.
- On Claude Code, `sandbox.filesystem.deny` blocks shell writes to the protected
  paths at the OS level. Codex's `workspace-write` sandbox has no equivalent
  per-path deny, so on Codex a shell write to a protected path is stopped by the
  hook's prompt rather than by the filesystem. Both products deny structured
  writes to those paths.
- Symlink escapes are resolved before the boundary check, so a link inside the
  workspace pointing outside it is treated as outside.
- `sandbox.failIfUnavailable` is `true` and `allow_login_shell` is `false`. The
  boundary depends on the sandbox starting, so a session that cannot sandbox
  must fail rather than run unconfined.

## Local validation

The fixture builds its own throwaway workspace and leaves the surrounding
repository untouched, so it holds both in this source bundle and in any project
that installed it:

```sh
python3 agent-safety/safe-autonomy/scripts/test_boundary.py
```

In this configuration repository only, two more fixtures validate the
`agent-config/` bundle against `routing.json`; the installer deliberately leaves
them uninstalled:

```sh
python3 agent-safety/safe-autonomy/scripts/test_agent_routing.py
python3 agent-safety/safe-autonomy/scripts/test_claude_agent_routing.py
```
