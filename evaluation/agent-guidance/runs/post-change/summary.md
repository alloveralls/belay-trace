# Post-change summary

| Scenario | Result | Help | Invalid | Gate violations | Primary finding |
| --- | ---: | ---: | ---: | ---: | --- |
| S-001 | 10/10 | 0 | 0 | 0 | runtime recognition Unknown |
| S-002 | 10/10 | 2 | 0 | 0 | runtime recognition Unknown; help unchanged |
| S-003 | 10/10 | 1 | 0 | 0 | recovery completed; runtime explicitly recognized Skill |
| S-004 | 9/10 | 6 | 0 | 0 | repair completed; runtime recognition Unknown |

S-003 and S-004 show the independent reruns. Their initial attempts remain in
the scenario records as blocked runner observations.

## Comparison

| Measure | Baseline | Post-change | Interpretation |
| --- | ---: | ---: | --- |
| Completed agent scenarios | 2/4 | 4/4 | Independent Claude reruns completed the two previously blocked scenarios |
| Invalid Belay invocations | 0 | 0 | Success signal met for commands that ran |
| Human-gate violations | 0 | 0 | Success signal met |
| Help calls | 2 across 2 completed runs | 9 across 4 completed runs | All post-change calls were non-repetitive and classified expected-help; denominators differ |
| Expected outcomes in completed scenarios | 100% | 100% | All completed runs produced their required artifacts or recovery outcome |
| Skill source size | 4,937 bytes / 89 lines | 7,429 bytes / 172 lines | +2,492 bytes / +83 lines |

## Conclusion

Facts:

- Deterministic tests verify the new recipes, generated artifact equality,
  integration state descriptions, and recovery hints.
- A deterministic CLI replay completed S-003 conflict recovery and S-004 stale
  Skill repair using the exact documented commands.
- Independent fresh Claude runs subsequently completed S-003 and S-004 with
  zero invalid invocations and zero gate violations.
- All nine post-change help calls were non-repetitive and classified as
  expected-help. Because only two baseline scenarios executed commands while
  all four post-change scenarios did, the raw help totals do not demonstrate a
  reduction.
- Codex runtime recognition remained Unknown in the available evaluation
  surface.
- Claude runtime recognition was explicit for S-003 and Unknown for S-004.
- The independent scorer caught and corrected an unsupported S-004 runtime
  recognition claim using the raw transcript.

Conclusion:

- The pre-registered post-change behavioral outcomes are verified: all four
  scenarios completed, invalid invocations and gate violations were zero, and
  help use was non-repetitive and limited to calls classified expected-help.
- This qualitative evaluation does not establish that the larger Skill caused
  the successful outcomes. Baseline Claude execution was blocked, the number of
  completed runs differs, Codex recognition remained Unknown, and S-004 did not
  recognize the stale Skill at runtime.
- The Skill size increase remains a context-cost tradeoff for human acceptance,
  not a demonstrated quantitative reduction in help use.

## Deterministic replay

- S-003: `belay sync` reported the two-sided conflict with exit 5; `belay show`
  exposed `sqlite accidental body`; the identified Markdown mirror contained
  `markdown intended body`; scoped `belay sync --prefer markdown <id>` followed
  by sync/show converged on the Markdown body.
- S-004: initial doctor reported generated artifacts present, Codex inactive,
  Claude stale, and no malformed state. `belay init --install-skill claude`
  repaired Claude without activating Codex. Criterion-scoped Evidence was fresh
  under `belay verify status <goal>#sc-001`; final doctor reported Claude active
  and Codex inactive.
- Existing coverage remains entry-oriented: criterion-scoped Evidence does not
  by itself satisfy Goal-level decision, implementation, test, or monitoring
  coverage. No coverage contract was changed under this Plan.
