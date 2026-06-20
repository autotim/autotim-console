# Contributing to Autotim Console

Thank you for your interest in contributing.

## Contributor License Agreement (CLA)

Autotim Console follows an **Open Core** model: this repository is licensed
under AGPL-3.0, while a separate private repository holds commercial Enterprise
modules. To keep dual licensing legally clean, **all external contributors must
sign the CLA before their first contribution can be merged.** The CLA grants
Autotim the rights needed to distribute your contribution under both the
AGPL-3.0 (here) and the commercial license (Enterprise), while you retain
copyright of your work.

The CLA check runs automatically on pull requests. A maintainer will provide
the signing link on your first PR.

## Ground Rules (from the architecture)

- **English everywhere** in code, comments, docs, commit messages, API
  contracts, and database identifiers.
- **Conventional Commits**, one logical change per commit:
  `feat: · fix: · refactor: · docs: · test: · build: · chore:`
  e.g. `feat(rbac): resolve permissions via scoped grants`.
- **Architecture compliance is enforced in CI:** the layer dependency law
  (Core → Infrastructure → Business; Core never depends upward), the
  permission-naming linter, and presence of `organization_id` on
  tenant-scoped tables. See `docs/architecture/`.
- **Security & mobile-first** are first-class: changes to auth/authz/secrets
  go through Core services (never reimplemented in a module), and any UI change
  must work on a phone (`docs/architecture/50-frontend-mobile-first.md`).
- Schema or contract changes require migration files (forward + rollback) and
  migration/compatibility notes in the PR.
- **No real infrastructure data ever enters this repository:** hostnames, IPs,
  usernames, emails, tokens, or deployment paths. Use `config/*.example.toml`
  and `deploy/**/*.example` for any new configuration surface, with
  placeholder values only. See
  `docs/security/public-private-boundary.md` before touching `config/`,
  `deploy/`, or `.gitignore`.

## Workflow

1. Branch from `develop`: `feature/...` or `fix/...`.
2. Make focused commits (Conventional Commits).
3. Ensure CI passes: build (CE), tests, clippy, fmt, `cargo audit`/`deny`,
   permission linter, architecture check, frontend type-check.
4. Open a PR with description, purpose, testing notes, and migration notes.

See `docs/architecture/62-development-workflow.md` for the full workflow.
