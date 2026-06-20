# 20 — Security Architecture

Security is a **Core responsibility**, enforced centrally and identically for every module. No module implements its own authentication or authorization. This document is the security backbone; docs 21 (RBAC), 22 (Auth), 23 (Secrets), 41 (Agent), 24 (Audit) are its detailed arms.

## Security Principles

1. **Secure by default** — safe configuration out of the box; insecure options require explicit opt-in.
2. **Least privilege** — every subject (user, token, agent, module) gets the minimum it needs.
3. **Defense in depth** — no single control is trusted alone (e.g. app authz *and* RLS).
4. **Centralized enforcement** — one auth path, one authz engine, one secret store.
5. **Complete auditability** — every security-relevant action is recorded tamper-evidently.
6. **Tenant isolation** — a tenant boundary is a security boundary, enforced in the database.
7. **Assume breach** — limit blast radius; rotate; revoke; detect.

## Trust Boundaries

```text
[ Browser / Mobile PWA ]  ── TLS ──┐
[ API clients / tokens ]  ── TLS ──┤
                                   ▼
                          [ Autotim control plane ]
                          Auth → RBAC → API → Module
                                   │
                          [ PostgreSQL (RLS) ] [ Secrets KMS ]
                                   │
                          ── mTLS ──┘ (agent-initiated)
                                   ▼
                          [ Managed hosts / Agents ]
```

Every arrow is authenticated and encrypted. The agent boundary is the most sensitive: agents execute privileged operations on real infrastructure (doc 41).

## Defense-in-Depth Layers

```text
Network        TLS everywhere; agents use mTLS; rate limiting at the edge
Identity       Auth (local/MFA/passkeys/OIDC) — doc 22
Authorization  RBAC scoped grants + tenant scope — doc 21
Data           RLS tenant isolation; envelope-encrypted secrets — docs 12, 23
Application    input validation, idempotency, output encoding, CSRF/CORS
Audit          tamper-evident log of every sensitive action — doc 24
Operations     unseal, backups, rotation, incident response
```

A request must pass **all** relevant layers; defeating one (e.g. an app-logic bug) is contained by the next (RLS).

## Authentication (summary; detail in doc 22)

- Local accounts (Argon2id password hashing), MFA/TOTP, recovery codes.
- Passkeys / WebAuthn (FIDO2) — roadmap, designed-for now.
- OIDC (Authentik as a tested provider; the implementation is generic OIDC, so Entra ID / Okta / Keycloak / Ping work too).
- Sessions are server-side, revocable; tokens are scoped and revocable.

## Authorization (summary; detail in doc 21)

- Every endpoint passes through the Core `Authorizer` port. Modules **cannot** implement custom authz.
- Grants are `(subject, permission, scope)` where scope always carries `organization_id`.
- Permission-aware UI hides/disables what the user cannot do — but the server is the only real gate.

## Session Management

```text
session
├── id (opaque, high-entropy)
├── user_id, organization_id
├── device, ip, user_agent
├── created_at, last_seen_at, expires_at
├── mfa_satisfied (bool)
└── revoked_at (null)
```

- Cookies: `HttpOnly`, `Secure`, `SameSite=Lax` (or `Strict` where possible).
- Idle and absolute lifetimes; sliding refresh.
- **Revocation is server-side and immediate** (single session, all sessions, all of a user's tokens).
- Re-authentication / step-up MFA required for sensitive actions (secret reveal, role changes, agent command execution).

## Trusted Devices

Users may register trusted devices to reduce MFA friction; trusted devices are listed in the Security Dashboard and individually revocable. A trusted device never bypasses authorization, only re-prompts.

## API Security

- TLS only; HSTS.
- Token validation + RBAC on every endpoint.
- **Idempotency keys** on mutating endpoints (prevents duplicate side effects under retry — pairs with at-least-once async).
- Strict input validation (typed deserialization; reject unknown fields where appropriate).
- Output encoding; no reflection of unsanitized input.
- CORS locked to known origins; CSRF protection for cookie-based requests (double-submit or `SameSite` + token).
- Per-route **rate limiting** (see below).

## Rate Limiting & Lockout

| Surface | Policy (defaults, configurable) |
|---------|--------------------------------|
| Login | e.g. 5 attempts / minute / account + IP; progressive backoff; temporary lockout |
| MFA verify | tight per-session limit to stop brute force of 6-digit codes |
| API tokens | per-token quota |
| Agent endpoints | per-agent quota; reject unregistered identities fast |
| Password reset | strict per-account + per-IP |

Lockouts and suspicious patterns emit security events and notifications.

## Secrets Protection (summary; detail in doc 23)

- All secrets stored only through the Secrets module.
- **Envelope encryption** (per-secret DEK wrapped by a KEK); pluggable key provider (passphrase-unseal, OS keystore/TPM, external KMS/Vault).
- Settings never store plaintext secrets — only secret references.
- Every secret access is authorized (RBAC) and audited.

## Agent Security (summary; detail in doc 41)

- Mutual TLS; agent-initiated connections (works behind NAT).
- Registration approval before an agent is trusted; per-agent identity and token.
- Capability validation — an agent executes only commands within its granted capabilities.
- Token rotation; signed commands; every command audited.
- Agents make **no authorization decisions** — all authz stays in Core RBAC.

## Input, Output & Injection Defenses

- SQL: parameterized queries via SQLx (compile-checked); no string-built SQL.
- Command execution on agents: structured, allow-listed operations — never arbitrary shell from user input.
- SSRF: outbound integrations (notification webhooks, OIDC discovery) validate/allow-list destinations.
- Deserialization: typed, size-limited request bodies.

## Frontend Security (pairs with doc 50)

- Strict **Content-Security-Policy**; no inline scripts (Vue build is CSP-friendly).
- Tokens kept in memory / `HttpOnly` cookies — **never** `localStorage` for session credentials.
- Permission-aware rendering, but never the security boundary.
- Any future third-party UI extension is sandboxed (iframe/WASM) and signed — never trusted in the main context.

## Audit & Detection (summary; detail in doc 24)

Security-sensitive actions always logged: login success/failure, logout, password change, MFA enable/disable, role/permission change, secret access, token issue/revoke, agent registration/command, module enable/disable, tenant switch. The audit log is tamper-evident (hash-chained) and partitioned for retention.

## Incident Response

Built-in controls:

- account lockout / disable,
- session revocation (single / all),
- token revocation (single / all of a user),
- agent quarantine (revoke token, drop connection),
- **emergency disable** of a module or integration,
- secret rotation on suspected compromise.

## Threat Model — Representative Risks & Mitigations

| Threat | Mitigation |
|--------|-----------|
| Credential stuffing / brute force | Rate limit + lockout + MFA + breach-aware password policy |
| Session hijacking | HttpOnly/Secure/SameSite cookies, TLS, revocation, device binding |
| Privilege escalation | Central RBAC, scoped grants, step-up auth, audit alerts on grant changes |
| Cross-tenant data leak | RLS in the DB + tenant-scoped authz (two independent controls) |
| Secret exfiltration | Envelope encryption, KMS/unseal, access authz + audit, rotation |
| Compromised agent | mTLS, capability limits, signed commands, quarantine, audit |
| SSRF via webhooks/OIDC | Destination allow-listing/validation |
| Supply chain (deps/plugins) | `cargo audit`/`cargo deny` in CI; signed releases; sandboxed future plugins |
| Insider misuse | Least privilege, immutable audit, alerting on sensitive actions |
| Lost master key | Documented key backup/recovery; KMS option; DR runbook (doc 24/15) |

## Secure Defaults Checklist (shipped on)

- TLS required; HSTS on.
- Strong password hashing (Argon2id) and policy.
- MFA available; enforceable per-organization.
- RLS enabled on all tenant tables.
- Audit on for all security events.
- Rate limiting on auth surfaces.
- Secrets sealed; no plaintext at rest.
- CSP + secure cookies on.

## Invariant

Security policy belongs to Core and applies equally to every module. No module may bypass authentication, authorization, the secret store, or the audit log.
