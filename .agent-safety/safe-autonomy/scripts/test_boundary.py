#!/usr/bin/env python3
"""Fixtures for the workspace-boundary guard.

Each case builds a throwaway workspace and leaves the surrounding repository
untouched, so this runs the same way in this source bundle and in any project
that installed it under `.agent-safety/`.

Run: python3 agent-safety/safe-autonomy/scripts/test_boundary.py
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

GUARD = Path(__file__).resolve().parent / "guard_tool.py"
POLICY = Path(__file__).resolve().parents[2] / "policy.json"

DECISION_KEYS = {"hookEventName", "permissionDecision", "permissionDecisionReason"}

failures: list[str] = []


def run(payload: dict, workspace: Path, tmpdir: Path | None = None) -> dict | None:
    """Invoke the guard and return its decision, or None when it stayed silent."""
    env = dict(os.environ)
    env["ERWIN_WORKSPACE_ROOT"] = str(workspace)
    env["ERWIN_POLICY"] = str(POLICY)
    env["TMPDIR"] = str(tmpdir) if tmpdir else str(workspace / ".no-tmpdir")
    result = subprocess.run(
        [sys.executable, str(GUARD)],
        input=json.dumps(payload), capture_output=True, text=True, env=env, timeout=30,
    )
    if result.returncode != 0:
        raise AssertionError(f"guard exited {result.returncode}: {result.stderr}")
    if not result.stdout.strip():
        return None
    return json.loads(result.stdout)


def check(name: str, payload: dict, workspace: Path, expected: str | None,
          tmpdir: Path | None = None) -> None:
    """Assert the decision for one payload; `expected` of None means silence."""
    try:
        output = run(payload, workspace, tmpdir)
    except Exception as exc:  # noqa: BLE001 - a fixture crash is a failure, not an abort
        failures.append(f"{name}: guard raised {type(exc).__name__}: {exc}")
        return

    if expected is None:
        if output is not None:
            failures.append(f"{name}: expected silence, got {output}")
        return

    if output is None:
        failures.append(f"{name}: expected '{expected}', got silence")
        return

    inner = output.get("hookSpecificOutput", {})
    # Codex rejects a decision object carrying any unknown key, reports
    # `hook returned invalid pre-tool-use JSON output`, and then runs the command
    # anyway. An extra field therefore turns every denial into an allow, and
    # Claude Code hides the fault by ignoring extra keys.
    if set(inner) != DECISION_KEYS:
        failures.append(f"{name}: decision keys are {sorted(inner)}, "
                        f"expected {sorted(DECISION_KEYS)}")
    if inner.get("hookEventName") != "PreToolUse":
        failures.append(f"{name}: hookEventName is {inner.get('hookEventName')!r}")
    if inner.get("permissionDecision") != expected:
        failures.append(f"{name}: expected '{expected}', "
                        f"got '{inner.get('permissionDecision')}'")
    if not str(inner.get("permissionDecisionReason", "")).strip():
        failures.append(f"{name}: decision carries no reason")


def edit(path: str) -> dict:
    return {"tool_name": "Edit", "tool_input": {"file_path": path}}


def bash(command: str) -> dict:
    return {"tool_name": "Bash", "tool_input": {"command": command}}


def patch(body: str) -> dict:
    return {"tool_name": "apply_patch", "tool_input": {"command": body}}


def main() -> int:
    with tempfile.TemporaryDirectory() as raw:
        base = Path(raw).resolve()
        workspace = base / "workspace"
        (workspace / "src").mkdir(parents=True)
        (workspace / ".claude").mkdir()
        (workspace / ".git").mkdir()
        (workspace / "src" / "main.rs").write_text("fn main() {}\n", encoding="utf-8")

        tmpdir = base / "scratch"
        tmpdir.mkdir()

        outside = base / "elsewhere"
        outside.mkdir()
        (outside / "secret.txt").write_text("x\n", encoding="utf-8")

        # A symlink inside the workspace pointing out of it is still an escape.
        escape = workspace / "escape"
        escape.symlink_to(outside, target_is_directory=True)

        # --- inside the workspace: no decision, no prompt, no tokens spent ---
        check("edit existing file", edit("src/main.rs"), workspace, None)
        check("edit new file", edit("src/added.rs"), workspace, None)
        check("edit nested new dir", edit("docs/design/v2.md"), workspace, None)
        check("edit by absolute in-workspace path",
              edit(str(workspace / "src" / "main.rs")), workspace, None)
        check("patch inside workspace",
              patch("*** Begin Patch\n*** Update File: src/main.rs\n"), workspace, None)

        # Destructive work inside the workspace is allowed; git is the recovery path.
        check("delete inside workspace", bash("rm -rf build/"), workspace, None)
        check("hard reset", bash("git reset --hard HEAD~1"), workspace, None)
        check("git clean", bash("git clean -fd"), workspace, None)
        check("rename inside workspace", bash("mv src/main.rs src/app.rs"), workspace, None)
        check("network command", bash("curl -sS https://example.com"), workspace, None)
        check("package install", bash("npm install"), workspace, None)
        check("inline interpreter", bash("python3 -c 'print(1)'"), workspace, None)
        check("shell wrapper", bash("bash -c 'ls -la'"), workspace, None)
        check("unparseable shell", bash("echo 'unterminated"), workspace, None)

        # --- outside the workspace: ask ---
        check("edit absolute outside path",
              edit(str(outside / "secret.txt")), workspace, "ask")
        check("edit via parent traversal", edit("../elsewhere/secret.txt"), workspace, "ask")
        check("edit through escaping symlink", edit("escape/secret.txt"), workspace, "ask")
        check("edit in home directory", edit("~/notes.md"), workspace, "ask")
        check("patch outside workspace",
              patch(f"*** Begin Patch\n*** Add File: {outside / 'new.txt'}\n"),
              workspace, "ask")

        # $TMPDIR is part of the workspace boundary.
        check("edit in TMPDIR", edit(str(tmpdir / "scratch.txt")), workspace, None, tmpdir)
        check("edit outside with TMPDIR set",
              edit(str(outside / "secret.txt")), workspace, "ask", tmpdir)

        # --- the safety configuration itself: deny ---
        for target in (".claude/settings.json", ".codex/config.toml",
                       ".agent-safety/policy.json", ".agents/skills/x/SKILL.md",
                       "AGENTS.md", "CLAUDE.md"):
            check(f"edit protected {target}", edit(target), workspace, "deny")
        check("patch protected path",
              patch("*** Begin Patch\n*** Update File: .claude/settings.json\n"),
              workspace, "deny")
        check("write protected path",
              {"tool_name": "Write",
               "tool_input": {"file_path": ".claude/settings.json", "content": "{}"}},
              workspace, "deny")

        # VCS internals are denied for structured writes but never scanned in shell,
        # so ordinary version control commands stay silent.
        check("edit git internals", edit(".git/config"), workspace, "deny")
        check("edit jj internals", edit(".jj/repo/store"), workspace, "deny")
        check("git command is not scanned", bash("git status --short"), workspace, None)
        check("ripgrep excluding .git", bash("rg --glob '!.git' pattern"), workspace, None)

        # --- shell naming a protected path: ask once, without classifying ---
        check("shell writes protected path",
              bash("echo '{}' > .claude/settings.json"), workspace, "ask")
        check("shell writes protected path, no space",
              bash("echo '{}' >.claude/settings.json"), workspace, "ask")
        check("shell reads protected path", bash("cat .codex/config.toml"), workspace, "ask")
        check("shell copies over AGENTS.md", bash("cp /tmp/x AGENTS.md"), workspace, "ask")
        check("similar name is not protected", bash("cat .claudette"), workspace, None)

        # --- malformed input must not block work: the sandbox holds the boundary ---
        check("unknown tool", {"tool_name": "WebFetch", "tool_input": {"url": "x"}},
              workspace, None)
        check("missing tool_input", {"tool_name": "Edit"}, workspace, None)
        check("empty payload", {}, workspace, None)

        env = dict(os.environ)
        env["ERWIN_WORKSPACE_ROOT"] = str(workspace)
        env["ERWIN_POLICY"] = str(POLICY)
        broken = subprocess.run([sys.executable, str(GUARD)], input="not json",
                                capture_output=True, text=True, env=env, timeout=30)
        if broken.returncode != 0 or broken.stdout.strip():
            failures.append("malformed hook JSON: expected silent exit 0, got "
                            f"rc={broken.returncode} stdout={broken.stdout!r}")

    for failure in failures:
        print(f"FAIL {failure}")
    if failures:
        print(f"\n{len(failures)} boundary fixture(s) failed.")
        return 1
    print("All workspace-boundary fixtures passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
