# 24 — Audit Architecture

The audit log is **tamper-evident**, not merely "immutable by convention" (the v1 weakness). It is the platform's record of who did what, when, to what, and with what result.

## What Is Audited

Authentication (login/logout, failures), MFA changes, password changes, authorization changes (role/grant), secret access (create/view/update/delete/rotate), token issue/revoke, configuration/settings changes, module lifecycle (enable/disable/upgrade), agent registration/commands, tenant switches, and all mutating API actions.

## Event Structure

```text
audit_event
├── id (uuid v7)
├── organization_id          -- tenant-scoped, RLS-protected
├── actor (user/token/agent/system)
├── action                   -- e.g. dns.zone.delete
├── target_type, target_id
├── result (success | denied | error)
├── correlation_id
├── metadata (jsonb)
├── created_at
├── prev_hash                -- hash chain
└── hash                     -- = H(prev_hash || canonical(event))
```

## Tamper-Evidence (hash chain)

Each event stores a hash over its canonical content plus the previous event's hash, forming a chain per organization (and a global chain). Any retroactive edit or deletion breaks the chain and is detectable by re-verification.

```text
e1.hash = H(GENESIS || e1)
e2.hash = H(e1.hash || e2)
e3.hash = H(e2.hash || e3)
```

- Verification job periodically re-walks the chain and alerts on breaks.
- Optional: periodic anchoring (publish the latest chain head to an external/WORM store) for stronger guarantees in regulated deployments.
- Application DB role has **INSERT-only** on `audit_events` (no UPDATE/DELETE), enforced by grants/policies; the maintenance role is used only for partition management.

## Append-Only & Partitioning

- Append-only by grant + policy.
- Range-partitioned by month (doc 14). Old partitions are archived/pruned per the retention policy, not edited.
- High write volume (agents, jobs at scale) is handled by partitioning + write batching; audit writes never block the request path more than necessary (async-friendly while preserving ordering for the chain).

## No Feedback Loops

Audit writes do **not** themselves emit domain events onto the bus (prevents loops and write amplification). The set of audited actions is explicit and bounded.

## Retention & Compliance

- Configurable retention per organization / per category.
- Export for compliance (signed, with chain proof).
- Enterprise: compliance reporting module builds on this (e.g. access reviews, change history).

## Access

Reading audit is itself an authorized, audited action (`audit.event.read`), tenant-scoped. Sensitive metadata is redacted per permission.

## Control-Plane DR (lives here + doc 60)

The audit chain, the secrets KEK/unseal material, and the control-plane Postgres are all part of the disaster-recovery plan:

```text
Backups:  Postgres (incl. audit + ciphertext secrets) + wrapped DEKs
Separate: KEK / unseal material (its own secured backup)
Runbook:  RPO/RTO targets; restore + unseal + chain-verify procedure
```

Losing the KEK without its separate backup makes secrets unrecoverable **by design** (doc 23); the runbook makes this explicit so operators back it up correctly.

## Invariant

The audit log can be read and verified but never silently altered. If the chain breaks, the platform says so.
