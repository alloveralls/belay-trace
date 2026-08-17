# Bounded goal template

The workspace boundary is enforced by the sandbox, so it does not belong in the
prompt. State here only what the sandbox cannot judge: what counts as done, what
would reach past the workspace, and when to stop.

```text
Outcome:
- <observable result>

Constraints:
- Stay inside the workspace; escalate anything that reaches past it.
- Do not edit agent safety controls during this run.
- Do not push, merge, deploy, access production, or modify external state.
- Work on one Delivery Map task at a time.
- <scope limits specific to this goal: files, subsystems, interfaces to leave alone>

Verification:
- <focused tests>
- <full tests or review criteria>
- Record fresh belay Evidence for each success criterion.

Budgets:
- <files and changed lines this goal should not exceed>
- <iteration, time, or cost ceiling for an unattended run>

Stop conditions:
- An irreversible or externally visible operation appears necessary.
- The requested outcome has materially different valid interpretations.
- The same failure occurs three times.
- Validation cannot distinguish a pre-existing failure from a regression.

Completion report:
- Success-criterion coverage
- Changed files and validation results
- Unknowns, assumptions, and remaining risks
```
