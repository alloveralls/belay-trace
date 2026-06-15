# `.github` Directory (Folder-Specific)

This file is shown when browsing the `.github/` folder.
The repository's main landing README is at [../README.md](../README.md).
This directory contains repository-level GitHub configuration for public
development workflows.

## Files

- `pull_request_template.md` — standard pull request structure
- `ISSUE_TEMPLATE/` — issue forms for bugs and planned work
- `CODEOWNERS` — default review ownership
- `CONTRIBUTING.md` — concise contributor workflow guide
- `labels.md` — shared label policy
- `workflows/` — GitHub Actions used for validation and repository hygiene

## Current Workflows

- `pr-title.yml` — validates pull request titles against Conventional Commits
- `docs-ci.yml` — runs Markdown lint, link checks, and typo checks for documentation paths

## Notes

- Human-facing contribution guidance lives in [CONTRIBUTING.md](./CONTRIBUTING.md).
