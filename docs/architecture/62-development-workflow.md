# 62 — Development Workflow

## Source Control

Git is mandatory. All changes are tracked; direct production changes are forbidden. This repository (`autotim-console`, AGPL v3, doc 10) is the public, community-facing one; a private commercial Enterprise repository exists separately and is outside the scope of this documentation set. A CLA is required before merging external contributions, so commercial licensing of community-contributed code stays clean.

## Branch Strategy

```text
main        production-ready, protected, reviewed-only
develop     integration
feature/*   feature/auth-oidc, feature/dns-zones, feature/agent-stream
fix/*       fix/login-validation, fix/job-retry-loop
release/*   release/1.2.0
```

## Conventional Commits (English, required)

```text
feat:     new feature
fix:      bug fix
refactor: behavior-preserving change
docs:     documentation
test:     tests
build:    build/deps/CI
chore:    maintenance
```

Examples:

```text
feat(auth): add OIDC login flow with PKCE
feat(tenancy): add organization_id and RLS policies
fix(api): validate hostname before host creation
refactor(rbac): resolve permissions via scoped grants
docs(architecture): add foundation v2 document set
```

Each commit is one logical change. Avoid "misc changes" / "wip". Prefer multiple clean commits over one large commit.

## Pull Requests

Must include: description, purpose, testing notes, and migration/compatibility notes when schema or contracts change. Reviews focus on correctness, security, maintainability, and architecture compliance (dependency law, tenancy, central authz/secrets).

## CI Gates (every PR)

```text
build (CE and EE) · unit + integration + API + frontend tests
clippy + fmt + vue-tsc
cargo audit / cargo deny (supply chain)
permission-registry linter
architecture-compliance check (layer law, org_id presence)
```

A red pipeline blocks merge. A failing test blocks release.

## Migration & Refactor Rules

Schema/contract changes require migration files, a rollback plan, and a compatibility review (docs 14, 60). Large refactors require impact analysis, a migration strategy, and approval before starting (per the project's standing rule). Prefer incremental improvements; protect backward compatibility.

## Documentation

Each module ships a `README.md` (purpose, permissions, settings, events). Architecture docs (this set) evolve with the code; a change that alters a documented contract updates the doc in the same PR.

## Release Process

```text
develop → review → test → release/* → tag (SemVer) → validate → main
```

Pre-release: dependency review, secret/config validation, vulnerability scan, CE+EE build validation, upgrade+rollback rehearsal on a staging dataset.

## Definition of Done (per feature)

- Code + tests (incl. the relevant security/correctness tests, doc 61).
- Docs/README updated; OpenAPI updated; permissions registered.
- Migrations forward+rollback; audit events emitted where required.
- Works on mobile (doc 50) if it has UI.
- Proposed **commit message**, **next milestone**, and **remaining work before release**.
