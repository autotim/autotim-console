# 61 — Testing Strategy

## Pyramid

```text
        E2E            few, critical user journeys
     Integration       module + Core interactions
        Unit           majority; fast, isolated, deterministic
```

## Categories

| Type | Validates | Examples |
|------|-----------|----------|
| Unit | individual functions/components | permission evaluation, settings validation, hash-chain link, scope matching |
| Integration | component interactions | RBAC + API, Jobs + Outbox, Settings + Secrets, Agent + Jobs |
| API | contracts, authz, validation | `GET /api/v1/hosts`, `POST /api/v1/dns/zones` |
| Frontend | components, permission-aware rendering, **mobile layouts** | hidden buttons, stacked-table mode, route guards |
| E2E | real journeys | login → MFA → create host → install agent → verify health |
| Security | the security backbone | see below |
| Performance | hot paths | API latency, job throughput, event delivery |

## Security & Correctness Tests (first-class, from the review)

These exist because the architecture depends on them:

- **Tenant isolation:** queries and APIs cannot read/write across `organization_id`; RLS holds even with a deliberately wrong query (defense-in-depth proof).
- **Authorization:** scoped grants allow/deny correctly; a missing/typo'd permission fails closed; the permission-registry linter passes in CI.
- **Outbox / idempotency:** crash between commit and publish loses no event; duplicate events/jobs execute effects once.
- **Auth/session:** lockout, revocation, step-up MFA, token scope limits.
- **Secrets:** sealed-state blocks access; rotation emits `SecretRotated`; no plaintext at rest.
- **Audit:** hash chain detects tampering; audit role is INSERT-only.
- **Agent:** out-of-capability command rejected; unsigned command rejected; quarantine drops the stream.

## Rust Backend

- `cargo test` for unit/integration; SQLx tests against an ephemeral Postgres (per-test schema/txn rollback).
- `cargo clippy` (lint), `cargo fmt` (format), `cargo audit` / `cargo deny` (supply-chain) in CI.

## Vue Frontend

- Component/unit tests (Vitest); E2E (Playwright) including mobile viewport runs.
- Type checks (`vue-tsc`).

## Test Data

Repeatable, isolated, disposable. Never production data. Each test owns its tenant/org fixtures.

## Continuous Integration

Every PR runs: unit + integration + API + frontend tests, linting/formatting, `cargo audit`/`deny`, the **permission linter**, and the **architecture-compliance check** (dependency-direction law, tenant-column presence on tenant-scoped tables). A failing test blocks release.

## Coverage

Track but don't chase a number. Guarantee coverage on critical paths: auth, RBAC, tenancy/RLS, outbox/jobs, secrets, audit, agent command path.

## Future

Load testing, chaos testing (kill workers/instances; verify no lost jobs/events), fuzzing (input validation, agent protocol), contract testing against the OpenAPI spec.

## Constraint

If it is important — tenancy, authorization, secrets, audit, the agent boundary — it must be tested, and the test must fail when the guarantee is broken.
