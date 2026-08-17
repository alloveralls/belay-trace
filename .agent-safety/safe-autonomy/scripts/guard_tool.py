#!/usr/bin/env python3
"""Workspace-boundary PreToolUse guard.

The v2 boundary is one rule: reads and writes inside the workspace are allowed,
anything outside it is asked. The product sandboxes enforce that rule for shell
commands already — Claude Code's `sandbox.*` settings and Codex's
`sandbox_mode = "workspace-write"` confine a command's filesystem access to the
workspace and let the agent escalate to a human prompt when it needs more.

This hook exists only for the two cases a filesystem sandbox cannot express:

1. Structured write tools (`Edit`, `Write`, `apply_patch`) are not shell
   commands, so the sandbox never sees them. An out-of-workspace target is
   asked here instead.
2. The agent's own safety configuration lives inside the workspace, so the
   files that define the boundary would otherwise be writable by the agent they
   bind.

It never classifies commands. Every decision is made from a path. A command
that is unparseable, unknown, or merely unusual is not this hook's business —
the sandbox is the enforcement layer, and adding command classification here is
what made v1 expensive without making it safer.

Output contract: a PreToolUse decision carries exactly `hookEventName`,
`permissionDecision`, and `permissionDecisionReason` inside
`hookSpecificOutput`, and nothing else. Codex rejects an object with an unknown
key, reports `hook returned invalid pre-tool-use JSON output`, and then runs the
command anyway, so one extra field turns every decision into an allow. Claude
Code ignores extra keys, which hides the fault during Claude-only testing.
"""
from __future__ import annotations

import json
import os
import re
import subprocess
import sys
from pathlib import Path
from typing import Any

POLICY_PATH = Path(os.environ.get(
    "ERWIN_POLICY",
    Path(__file__).resolve().parents[2] / "policy.json",
))

STRUCTURED_WRITE_TOOLS = {
    "edit", "write", "multiedit", "notebookedit", "apply_patch", "applypatch",
    "str_replace_editor", "create_file", "update_file",
}
SHELL_TOOLS = {"bash", "shell", "exec_command", "local_shell"}

# Patch bodies name their targets in these forms.
PATCH_PATH_PATTERNS = (
    re.compile(r"(?m)^\*\*\* (?:Add|Update|Delete|Move to) File:\s*(.+?)\s*$"),
    re.compile(r"(?m)^(?:\+\+\+|---) [ab]/(.+?)\s*$"),
)


def emit(decision: str, reason: str) -> None:
    """Print a PreToolUse decision and stop. Keys are fixed; see module docstring."""
    json.dump({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": decision,
            "permissionDecisionReason": reason,
        }
    }, sys.stdout)
    sys.stdout.write("\n")
    sys.exit(0)


def allow_silently() -> None:
    """Emit no decision, leaving the product's own permission handling in charge."""
    sys.exit(0)


def load_policy() -> dict[str, Any]:
    return json.loads(POLICY_PATH.read_text(encoding="utf-8"))


def workspace_root(payload_cwd: object) -> Path:
    override = os.environ.get("ERWIN_WORKSPACE_ROOT")
    if override:
        return Path(override).resolve()
    start = Path(payload_cwd) if isinstance(payload_cwd, str) and payload_cwd else Path.cwd()
    try:
        top = subprocess.run(
            ["git", "-C", str(start), "rev-parse", "--show-toplevel"],
            capture_output=True, text=True, timeout=5, check=True,
        ).stdout.strip()
        if top:
            return Path(top).resolve()
    except (OSError, subprocess.SubprocessError):
        pass
    return start.resolve()


def writable_roots(root: Path, policy: dict[str, Any]) -> list[Path]:
    """Every root the workspace boundary covers, symlinks already resolved."""
    workspace = policy.get("workspace", {})
    roots = [root]
    for extra in workspace.get("additional_roots", []):
        candidate = Path(extra).expanduser()
        if not candidate.is_absolute():
            candidate = root / candidate
        roots.append(candidate.resolve())
    if workspace.get("allow_tmpdir", True):
        tmpdir = os.environ.get("TMPDIR")
        if tmpdir:
            roots.append(Path(tmpdir).resolve())
    return roots


def resolve(raw: str, base: Path) -> Path:
    """Resolve a tool-supplied path, following symlinks so an escape is visible."""
    candidate = Path(raw).expanduser()
    if not candidate.is_absolute():
        candidate = base / candidate
    return candidate.resolve()


def contained_in(path: Path, root: Path) -> bool:
    return path == root or root in path.parents


def relative_to_workspace(path: Path, root: Path) -> str | None:
    if not contained_in(path, root):
        return None
    return path.relative_to(root).as_posix()


def matches_prefix(relative: str, prefixes: list[str]) -> str | None:
    for prefix in prefixes:
        if relative == prefix or relative.startswith(prefix + "/"):
            return prefix
    return None


def check_write_target(raw: str, root: Path, policy: dict[str, Any]) -> None:
    """Ask or deny for one structured-write target; return quietly when it is fine."""
    try:
        path = resolve(raw, root)
    except (OSError, RuntimeError, ValueError):
        emit("ask", f"Cannot resolve the write target '{raw}' to check it against "
                    "the workspace boundary.")
    relative = relative_to_workspace(path, root)
    if relative is None:
        for extra in writable_roots(root, policy)[1:]:
            if contained_in(path, extra):
                return
        emit("ask", f"'{raw}' is outside the workspace ({root}). Writing there "
                    "needs a human decision.")
    protected = matches_prefix(relative, policy.get("protected_paths", []))
    if protected is not None:
        emit("deny", f"'{relative}' is part of the agent safety configuration "
                     f"('{protected}'), which defines this boundary and is not "
                     "writable by the agent it binds. Ask a human to change it.")
    vcs = matches_prefix(relative, policy.get("vcs_internal_paths", []))
    if vcs is not None:
        emit("deny", f"'{relative}' is inside '{vcs}'. Use the version control "
                     "command instead of editing its internal state directly.")


def write_targets(tool: str, tool_input: dict[str, Any]) -> list[str]:
    """Collect the paths a structured write tool would touch."""
    patch = tool_input.get("command") or tool_input.get("patch") or tool_input.get("input")
    if tool in {"apply_patch", "applypatch"} or (
        isinstance(patch, str) and "*** " in patch
    ):
        if not isinstance(patch, str):
            return []
        found: list[str] = []
        for pattern in PATCH_PATH_PATTERNS:
            found.extend(pattern.findall(patch))
        return found

    targets: list[str] = []
    for key in ("file_path", "path", "notebook_path", "filePath"):
        value = tool_input.get(key)
        if isinstance(value, str) and value.strip():
            targets.append(value)
    for edit in tool_input.get("edits", []) or []:
        if isinstance(edit, dict):
            value = edit.get("file_path") or edit.get("path")
            if isinstance(value, str) and value.strip():
                targets.append(value)
    return targets


def check_shell(command: object, root: Path, policy: dict[str, Any]) -> None:
    """Ask when a shell command names a protected path.

    This is a literal path scan, not a parse: a protected path mentioned
    anywhere in the command prompts once, whether the command reads it or writes
    it. That costs an occasional needless prompt and buys back the whole command
    grammar v1 had to maintain. The sandbox handles everything else.
    """
    if not isinstance(command, str) or not command.strip():
        return
    for prefix in policy.get("protected_paths", []):
        pattern = rf"(?:^|[\s=\"'<>|&;(]){re.escape(prefix)}(?:/|\b)"
        if re.search(pattern, command):
            emit("ask", f"This command names '{prefix}', which holds the agent "
                        "safety configuration. Confirm what it does to it.")


def main() -> None:
    try:
        payload = json.load(sys.stdin)
    except (json.JSONDecodeError, TypeError, ValueError):
        # The sandbox, not this hook, is the enforcement layer. A payload we
        # cannot read is not evidence of a dangerous operation.
        allow_silently()

    tool = payload.get("tool_name")
    tool_input = payload.get("tool_input")
    if not isinstance(tool, str) or not isinstance(tool_input, dict):
        allow_silently()

    policy = load_policy()
    root = workspace_root(payload.get("cwd"))
    lowered = tool.lower()

    if lowered in SHELL_TOOLS:
        check_shell(tool_input.get("command", tool_input.get("cmd")), root, policy)
    elif lowered in STRUCTURED_WRITE_TOOLS:
        for target in write_targets(lowered, tool_input):
            check_write_target(target, root, policy)

    allow_silently()


if __name__ == "__main__":
    try:
        main()
    except SystemExit:
        raise
    except Exception:
        # Fail open by design: the sandbox still holds the boundary, and a hook
        # that blocks work when it breaks is how v1 lost its budget.
        allow_silently()
