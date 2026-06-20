# 40 — API Architecture

## Style

**REST first**, documented with OpenAPI. GraphQL is optional and later; gRPC is internal-only (agent transport, doc 41). Frontend and external clients depend on the contract, never on implementation details.

## Base Structure & Versioning

```text
/api/v1/...
```

All public APIs are versioned; breaking changes require a new version (`/api/v2`). Examples:

```text
/api/v1/users
/api/v1/hosts
/api/v1/dns/zones
/api/v1/mail/domains
```

Module routes are mounted under the module namespace by the kernel (doc 13); Core never hardcodes module routes.

## Tenant-Aware Routing

Every request resolves an `organization_id` (doc 12) during authentication and sets the tenant context before any handler runs. RLS + authorizer both enforce isolation. Multi-org operators (EE) select the active organization; the choice is part of the session.

## Authentication & Authorization

- Auth: session cookie, bearer/API token, or OIDC access token (doc 22).
- Authz: **every endpoint** passes through the Core authorizer with a declared permission + scope (doc 21). Authorization is declared alongside the route so it cannot be forgotten:

```rust
route(get, "/dns/zones/:id", dns_zone_get)
    .require(perm::DNS_ZONE_READ, scope_from_path("id", "zone"));
```

## Idempotency

Mutating endpoints accept an `Idempotency-Key` header. The platform records the key + result; a retry with the same key returns the original result instead of repeating the side effect. Essential given at-least-once async and client retries.

## Response Envelope

```json
// success
{ "success": true, "data": { } }

// error
{ "success": false,
  "error": { "code": "permission_denied", "message": "Access denied",
             "details": {} } }
```

Error `code` values are stable, documented, machine-readable.

## Pagination, Filtering, Sorting

```text
?page=1&page_size=50         → { items, page, page_size, total }
?status=active&type=dns       (filtering)
?sort=name   ?sort=-created_at (sorting; '-' = desc)
```

Cursor pagination is offered on large, frequently-scrolled collections (logs, audit, events) for mobile performance.

## HTTP Status Codes

`200 / 201 / 204` success; `400` validation; `401` unauthenticated; `403` unauthorized; `404` not found; `409` conflict; `422` semantic validation; `429` rate-limited; `500` server error.

## Input Validation & Limits

Typed deserialization, body size limits, strict field validation, rejection of unexpected fields on sensitive endpoints. Validation errors return structured field-level details.

## Audit

All mutating requests (`POST/PUT/PATCH/DELETE`) generate audit entries (actor, action, target, org, result) — doc 24.

## OpenAPI Documentation

Every endpoint exposes summary, description, required permission(s), request/response schema, and error codes. The spec is generated from code (e.g. `utoipa`) so docs cannot drift from implementation. The same spec can generate a typed TypeScript client for the Vue frontend (doc 50).

## Rate Limiting & Observability

Per-route limits (doc 20). Every request carries a `request_id`/`correlation_id` propagated into events, jobs, and traces (doc 51). Latency, error rate, and throughput are exported per route.

## Internal Module Communication

Modules communicate through Core ports and events, not by calling each other's HTTP APIs or joining each other's tables. Use the event bus for decoupled reactions; use ports for synchronous Core services.

## Decision

Clients and modules rely on the versioned contract. The contract is the API; the implementation is free to change beneath it.
