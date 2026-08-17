---
name: safe-autonomy
description: Run Codex /goal work safely inside a sandboxed workspace boundary, with recoverable worktrees, stop conditions, and belay evidence. Use for long-running, autonomous, unattended, repeated repair, or multi-step repository work.
---

# Safe Autonomy for Codex

Read and follow `.agent-safety/safe-autonomy/SKILL.md`; it is the canonical
cross-product workflow. Then apply these Codex-specific checks:

1. Run `/status` and confirm `workspace-write` and an interactive approval
   policy. Inspect `/hooks` after installation. The workspace boundary is the
   sandbox, so a session without `workspace-write` has no boundary and must not
   run unattended.
2. Start unclear work with `/plan`; start the approved measurable outcome with
   `/goal`. Include the shared goal template's constraints and stop conditions.
3. Treat hook/sandbox denial as final. Never use `--yolo`, danger-full-access,
   or hook-trust bypasses for autonomous work.
4. Use `belay-trace` before implementation for Tier 2 or Tier 3 work and record
   fresh Evidence after each Delivery Map unit.
5. Before completion, run the boundary fixture, repository validation,
   `belay coverage`, and `/diff`.

Local tooling needs no routing table: `git`, `jj`, `belay`, `rtk`, builds,
tests, and package managers all run under the sandbox like any other command,
and the sandbox is what keeps them inside the workspace. Network access is
allowed from inside the sandbox. Escalate instead for anything whose effect
leaves the workspace — pushing, publishing, deploying, or changing an external
system.

Use the six project custom agents and exact model/effort matrix in
`.agent-safety/routing.json`. Keep design and orchestration on the human-selected
root model, and isolate every review from the implementation context.

Task intake is a belay fragment, not prose. The orchestrator passes
`PLN-...#t-nnn`; the worker resolves it with `belay show` and reads the Plan's
Intent Brief for Constraints and Non-goals. The canonical sections "Specify a
task" through "When to record" hold the two-phase split, the fixed spawn forms,
the blocked-return protocol, and the recording checkpoints.

