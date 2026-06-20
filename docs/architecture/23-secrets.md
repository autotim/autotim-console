# 23 — Secrets Architecture

All sensitive credentials are stored only through the Secrets module. No module implements its own secret storage. Plaintext at rest is forbidden.

## What Is a Secret

OIDC client secrets, SMTP passwords, DNS API tokens, database passwords, SSH private keys, agent registration tokens, webhook secrets, ACME account keys, and any other credential.

## Envelope Encryption (the core mechanism)

```text
Root Key (KEK)  ── provided/unsealed by a Key Provider, never stored in plaintext on disk
     │ wraps
Data Encryption Key (DEK)  ── one per secret (or per small group), random
     │ encrypts
Secret value (AEAD: AES-256-GCM or ChaCha20-Poly1305)
```

Each secret stores its wrapped DEK and the AEAD ciphertext. Compromising the database alone yields nothing without the KEK. Rotating the KEK re-wraps DEKs without re-encrypting every secret.

## Key Provider (pluggable)

| Provider | Use case |
|----------|----------|
| Passphrase unseal | Simple self-hosted: KEK derived from an operator passphrase; **unseal on start** (like Vault). The passphrase/material is never persisted in plaintext. |
| OS keystore / TPM | Hardware-bound KEK on a single host |
| External KMS / HashiCorp Vault | Enterprise; KEK never leaves the KMS |

The provider is selected in config; the Secrets API is identical regardless.

## Unseal Flow

```text
Service starts → KEK not yet available → "sealed" state (no secret access)
Operator provides unseal material (passphrase / KMS auth / TPM) →
KEK loaded into memory only → "unsealed" → secret operations enabled
```

Sealed-state behavior is explicit: dependent integrations wait; the platform does not boot into an insecure fallback.

## Secret Structure

```text
secret
├── id (uuid)
├── organization_id        -- tenant-scoped
├── name, type
├── owner (module/user)
├── wrapped_dek
├── ciphertext (AEAD)
├── created_at / updated_at
├── rotation_policy
└── last_rotated_at
```

## Access Control & Audit

- Access authorized through RBAC: `secrets.secret.read|create|update|delete`, scoped to org/resource.
- Modules receive **only** the secrets they require, via the `SecretStore` port.
- **Every access is audited** (created, viewed, updated, deleted, rotated) — viewing a secret is a sensitive, step-up-MFA action.

## Rotation

- Manual, scheduled, and forced rotation.
- On rotation, a `SecretRotated` event is published so consumers (settings references, integrations, agents) refresh — rotation never silently breaks a live integration.
- Rotation jobs run on the Core async substrate (doc 31).

## Settings Integration

Settings store references, never values:

```text
smtp.password  →  secret_ref: <secret-uuid>
```

The Settings UI shows "configured / not configured", never the value. Reading the value is a separate, audited, authorized action.

## Backup & Recovery

- Backups preserve ciphertext and wrapped DEKs, never plaintext.
- The KEK / unseal material is backed up **separately** with its own controls (documented in the DR runbook, doc 24/15). Losing the KEK without backup = unrecoverable secrets, by design.

## Invariant

Secrets belong to the Secrets module, encrypted with envelope encryption under a key the database never sees in plaintext. Every read is authorized and audited.
