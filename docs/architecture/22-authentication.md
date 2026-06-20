# 22 — Authentication

Local accounts, MFA, OIDC, and the planned passkey support all funnel through one Core Auth path — no module implements its own login, session, or token logic.

## Supported Methods

| Method | Status | Notes |
|--------|--------|-------|
| Local accounts (password) | v1 | Argon2id hashing |
| MFA / TOTP + recovery codes | v1 | enforceable per organization |
| OIDC | v1 | generic; Authentik is a tested provider |
| Passkeys / WebAuthn (FIDO2) | designed-for, near-term | passwordless / second factor |
| API tokens | v1 | scoped, revocable |
| QR mobile login | roadmap | approve a web session from the mobile PWA |
| LDAP / AD / SAML | Enterprise | implemented in the private Enterprise repository |

## Identity Model

```text
user
├── id, organization memberships
├── status (active | disabled | locked)
└── profile, preferences

user_identity (one user → many identities)
├── kind (local | oidc)
├── provider (e.g. authentik)
├── subject (provider 'sub')
└── linked_at
```

A user may have a local password and linked OIDC identities. Account linking requires proof of both.

## Password Authentication

- Hashing: **Argon2id** with sane cost params; per-hash salt.
- Policy (configurable): length, breach-list check, no reuse window.
- Reset via signed, single-use, short-lived token delivered through Notifications.
- All password events audited.

## MFA / TOTP

- TOTP (RFC 6238), authenticator-app compatible.
- One-time **recovery codes** (hashed at rest).
- Enrollment requires verifying a live code.
- **Step-up MFA** for sensitive operations even within an authenticated session (secret reveal, grant changes, agent command execution).
- MFA enforceable at the organization level (admins can require it).

## OIDC (generic)

- Standard Authorization Code + PKCE flow.
- Provider config (issuer, client id, **client secret stored in Secrets module**, scopes) is per-organization in EE, global in CE.
- Discovery via the issuer's well-known document (destination validated — SSRF guard).
- Claims mapped to user/profile; group/role mapping optional.
- Works with Authentik, Entra ID, Okta, Keycloak, Ping — nothing Authentik-specific in Core.

## Passkeys / WebAuthn (designed-for)

- `webauthn_credentials` table modeled now; flows implemented near-term.
- Supports passwordless login and phishing-resistant second factor.
- Mobile-first: platform authenticators (Face ID / fingerprint) make the PWA login excellent on phones.

## Sessions

Server-side, revocable (see doc 20 for the session model and cookie flags). Sessions track `mfa_satisfied` for step-up. Idle + absolute lifetimes; immediate server-side revocation (single / all).

## Tokens

```text
api_token
├── id, user_id, organization_id
├── name, scopes (permission subset)
├── hashed_secret (only hash stored)
├── created_at, last_used_at, expires_at
└── revoked_at
```

- Tokens carry a **subset** of the user's permissions (never more).
- Shown once on creation; only the hash is stored.
- Revocable individually or all-at-once; usage audited.

## QR Mobile Login (roadmap)

Web shows a QR encoding a short-lived challenge; the authenticated mobile PWA scans and approves; the web session is elevated. Useful for fast, secure desktop login from a trusted phone.

## Account Lifecycle

```text
invited / created → active → (locked on policy) → active
                          → disabled (admin) → deleted (soft)
```

Disabling a user immediately revokes sessions and tokens.

## Audit

Login success/failure, logout, MFA changes, OIDC link/unlink, token issue/revoke, password change, lockouts — all audited and capable of raising security notifications.

## Invariant

One authentication path for the whole platform. Identity is established once; everything downstream consumes it.
