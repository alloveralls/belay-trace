# Agent guidance evaluation

This fixture compares Belay agent guidance before and after a change without
adding an LLM dependency to the Belay binary or its test suite.

The protocol is intentionally qualitative. It uses four fixed scenarios, one
fresh isolated repository per run, and the rubric in `rubric.md`. Codex runs
S-001 and S-002; Claude runs S-003 and S-004. The same agent family, prompt,
fixture setup, and rubric must be used for the baseline and post-change run.
Fixture setup requires a POSIX shell plus `perl` and the `sqlite3` CLI; these are
evaluation-runner prerequisites, not Belay runtime dependencies.

## Run protocol

1. Build the exact Belay revision under evaluation.
2. Create an empty temporary directory outside this repository.
3. Run `setup-fixture.sh <scenario> <directory> <belay-binary>`.
4. Start a fresh, non-resumed agent session in that directory with the matching
   file under `scenarios/` as the complete task prompt.
5. Capture the agent transcript and final repository observations.
6. Score the run with `rubric.md`; do not change the rubric after baseline.

Runtime Skill recognition is a Fact only if the agent surface explicitly
reports it. Repository installation or `belay doctor` output does not prove
that the runtime read or triggered a Skill. Otherwise record
`runtime-recognition-unknown`.

## Fixed assignment

| Scenario | Agent family | Routine under evaluation |
| --- | --- | --- |
| S-001 | Codex | Context and planning orientation |
| S-002 | Codex | Trace lifecycle |
| S-003 | Claude | Sync and conflict recovery |
| S-004 | Claude | Assurance and integration diagnosis |

## Baseline

The initial shared Skill is 4,937 bytes and 89 source lines when measured from
the `SHARED_SKILL` raw string declaration, including its declaration and
terminator. This is a source-level measurement, not the installed file size.

Baseline and post-change run records live under `runs/`. A run record must
include the exact revision or `jj` change ID, agent family/version, Skill
repository state, runtime recognition, help calls, invalid invocations, exit
categories, artifacts and relations created, status transitions, gate result,
recovery result, rubric score, and classified findings.

## Recorded protocol deviation

The 2026-07-26 baseline/post-change execution surface did not expose raw Codex
sub-agent transcripts for versioning. The stored run records preserve the final
agent reports and available command inventories; Claude runner summaries record
the repeated EPERM boundary. Exact Codex runtime versions and transcript-level
independent rescoring remain Unknown. Do not treat these summaries as stronger
evidence than the limitation allows.

A later human-arranged Claude Code 2.1.220 rerun completed S-003 and S-004 in
fresh isolated sessions. Its independent scorer inspected raw JSONL transcripts
and corrected an unsupported S-004 runtime-recognition claim, but those
transcripts remain in an external scratch directory and are not versioned here.
