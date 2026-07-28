#!/bin/sh
set -eu

scenario_id=${1:?scenario id is required}
fixture_dir=${2:?fixture directory is required}
belay_binary=${3:?belay binary path is required}

if [ ! -x "$belay_binary" ]; then
  echo "belay binary is not executable: $belay_binary" >&2
  exit 2
fi

if [ -e "$fixture_dir/.belay" ]; then
  echo "fixture directory is already initialized: $fixture_dir" >&2
  exit 2
fi

mkdir -p "$fixture_dir/.git"

case "$scenario_id" in
  S-001)
    (
      cd "$fixture_dir"
      "$belay_binary" init --update-agents --install-skill codex
      goal_output=$("$belay_binary" add goal \
        --title "Improve fixture reliability" \
        --body "## Summary

- Make fixture execution deterministic.

## Success Criteria

- [SC-001] Fixture execution produces the expected deterministic result.

## Constraints

- Do not change production state.

## Non-goals

- Implementing the fixture change.

## Verification

- Run the fixture validation.

## Risks

- Hidden environment coupling.")
      goal_id=${goal_output#Created }
      "$belay_binary" status "$goal_id" active
      decision_output=$("$belay_binary" add decision \
        --title "Keep fixture data local" \
        --body "Use repository-local deterministic fixture data.")
      decision_id=${decision_output#Created }
      "$belay_binary" status "$decision_id" accepted
      "$belay_binary" link "$decision_id" "$goal_id" --relation fulfills
      plan_output=$("$belay_binary" add plan \
        --title "Plan deterministic fixture execution" \
        --body "## Intent Brief

### Problem

Fixture execution is not deterministic.

### Desired Outcome

Deterministic fixture execution.

### Success Signals

- The fixed validation passes.

### Constraints

- Planning only.

### Non-goals

- Implementation before approval.

### Assumptions

- This is a hypothesis. Local data is sufficient.

### Unknowns / Decisions Needed

- Human approval is required before implementation.

## Delivery Map

| ID | Goal item | Outcome / Task | Actor | State | Verification / Evidence |
| --- | --- | --- | --- | --- | --- |
| T-001 | SC-001 | Prepare deterministic data | AI | not-started | Fixture validation |")
      plan_id=${plan_output#Created }
      "$belay_binary" link "$plan_id" "$goal_id" --relation fulfills
      "$belay_binary" link "$plan_id" "$decision_id" --relation references
    )
    ;;
  S-002)
    (
      cd "$fixture_dir"
      "$belay_binary" init --update-agents --install-skill codex
      goal_output=$("$belay_binary" add goal \
        --title "Ship fixture lifecycle" \
        --body "## Summary

- Preserve the fixture lifecycle.

## Success Criteria

- [SC-001] The required trace lifecycle is recorded.

## Constraints

- Trace updates only.

## Non-goals

- Source changes.

## Verification

- Inspect entries and relations.

## Risks

- Incorrect status or relation.")
      goal_id=${goal_output#Created }
      "$belay_binary" status "$goal_id" active
    )
    ;;
  S-003)
    (
      cd "$fixture_dir"
      "$belay_binary" init --update-agents --install-skill claude
      note_output=$("$belay_binary" add note \
        --title "Fixture source of truth" \
        --body "original body")
      note_id=${note_output#Created }
      note_path=$(find .belay/entries/notes -type f -name "$note_id.md")
      perl -0pi -e 's/original body/markdown intended body/' "$note_path"
      sqlite3 .belay/state/belay.sqlite \
        "UPDATE entries SET body = 'sqlite accidental body' WHERE display_id = '$note_id';"
    )
    ;;
  S-004)
    (
      cd "$fixture_dir"
      "$belay_binary" init --update-agents --install-skill claude
      goal_output=$("$belay_binary" add goal \
        --title "Diagnose fixture integration" \
        --body "## Summary

- Diagnose repository integration accurately.

## Success Criteria

- [SC-001] The stale Claude Skill is repaired without activating Codex.

## Constraints

- Use existing commands.

## Non-goals

- Proving runtime Skill recognition.

## Verification

- Record Evidence and run doctor.

## Risks

- Confusing repository state with runtime recognition.")
      goal_id=${goal_output#Created }
      "$belay_binary" status "$goal_id" active
      printf '%s\n' "stale installed Claude skill" > \
        .claude/skills/belay-trace/SKILL.md
    )
    ;;
  *)
    echo "unknown scenario: $scenario_id" >&2
    exit 2
    ;;
esac

printf '%s\n' "fixture ready: $scenario_id at $fixture_dir"
