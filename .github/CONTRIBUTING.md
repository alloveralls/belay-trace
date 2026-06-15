# Contributing

Contributions are welcome. Keep changes focused, reviewable, and covered by
appropriate tests.

## Workflow

1. Fork the repository and create a focused branch.
2. Implement the smallest coherent change.
3. Run formatting and tests locally.
4. Use a Conventional Commit message.
5. Push the branch and open a pull request.

## Pull Requests

- Use the PR template and fill every section.
- Keep the PR scoped so it can be reviewed in one pass.
- Re-request review after material updates.
- Ensure `cargo fmt -- --check` and `cargo test --all-targets --locked` pass.

## Review Ownership

- `CODEOWNERS` defines the default review owner as `@alloveralls`.
- Changes to `.github` are also owned by `@alloveralls`.
