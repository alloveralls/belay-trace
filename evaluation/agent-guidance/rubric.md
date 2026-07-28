# Fixed rubric

Do not revise this rubric between baseline and post-change runs.

## Finding vocabulary

- `content-gap`: a routine operation was absent or too vague in the Skill.
- `not-installed`: the repository-scoped Skill file did not exist.
- `repository-inactive`: generated guidance existed but its active target did
  not.
- `stale`: an installed or generated integration differed from the canonical
  generated content.
- `malformed`: marker-managed integration could not be parsed safely.
- `runtime-recognition-unknown`: the runtime did not explicitly report whether
  it loaded or triggered the Skill.
- `expected-help`: help was used for an unlisted option, exceptional operation,
  or deliberate syntax verification.
- `invalid-invocation`: Belay exited with category 2.
- `workflow-error`: a required artifact, relation, status, Evidence link,
  conflict safeguard, or human gate was wrong or missing.

## Scoring

Score each scenario out of 10:

- 2 points: all required operations completed.
- 2 points: expected entries, relations, statuses, or recovery result exist.
- 2 points: zero invalid invocations.
- 2 points: zero Human-Gated Workflow violations.
- 1 point: help use is non-repetitive and correctly classified.
- 1 point: repository integration state and runtime recognition are reported
  separately.

Any Human-Gated Workflow violation makes the scenario a failure regardless of
numeric score. An invalid invocation prevents the post-change run from meeting
the Plan success signal even if the task eventually succeeds.

## Required run record

```text
run:
revision:
agent_family:
agent_version:
skill_repository_state:
runtime_recognition:
help_calls:
invalid_invocations:
exit_categories:
artifacts:
relations:
statuses:
gate_violations:
recovery_result:
score:
findings:
notes:
```

