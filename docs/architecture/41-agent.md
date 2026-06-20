# 41 — Agent Architecture

## Purpose

The Agent extends the control plane to managed hosts and executes approved operations. It is the most security-sensitive boundary: agents act with privilege on real infrastructure.

## Connection Model (the scaling decision)

**Agent-initiated, persistent bidirectional gRPC stream over mTLS.**

```text
Agent (behind NAT/firewall)  ──dials out──▶  Agent Gateway  ──▶ Core
        ▲ persistent stream, mTLS                  │
        └────────── commands / results ────────────┘
```

Why agent-initiated: self-hosted customers run hosts behind NAT/firewalls; the control plane usually cannot reach in. Agents dial out and hold a stream, so commands flow without inbound firewall changes.

## Agent Gateway Tier (horizontal scale)

The component terminating agent connections is a **separate, horizontally scalable tier**, not the API node:

```text
10–100 agents     gateway co-located in the single binary
1,000             dedicated gateway process(es)
10,000            gateway fleet; connections sharded across nodes;
                  fan-out to Core via the broker (doc 31)
```

Designing the gateway as its own tier now (even if co-located) makes the 1k→10k transition a deployment change, not a rewrite.

## Reconnect Behavior (anti-thundering-herd)

On control-plane restart, 10k agents must not reconnect simultaneously. Agents use **exponential backoff with jitter**. Heartbeats are batched/aggregated and never write to Postgres synchronously per beat at scale (write-behind / aggregate).

## Lifecycle

```text
Unregistered → Registering → (Approved) → Connected → Healthy
                                              │
                                  Disconnected ⇄ Reconnected
                                              │
                                        Quarantined (security)
```

## Registration & Identity

```text
Agent generates identity (keypair) → requests registration →
operator/policy approval → Core issues agent token + capability set →
mTLS established → Connected
```

```text
agent
├── id (uuid), host_id (uuid), organization_id
├── public_key / cert fingerprint
├── capabilities (granted set)
├── version, last_seen_at, status
└── token (hashed; rotatable)
```

Approval is required before an agent is trusted — no auto-trust of unknown identities.

## Capabilities (least privilege)

An agent executes **only** operations within its granted capabilities:

```text
inventory · monitoring · backup · dns · ssl · files · mail
```

A command outside the agent's capability set is rejected and audited. Capabilities are granted explicitly, per agent.

## Command Execution (security)

```text
Source (Job / Infra module / admin)
  → Core authorizes (RBAC, scoped)      ← authz happens HERE, in Core
  → Core signs the command + capability check
  → Gateway streams to agent
  → Agent verifies signature + capability, executes structured op
  → Result streamed back → Job/Audit updated
```

- Commands are structured, allow-listed operations — never arbitrary shell from user input.
- Every command is audited (who, what, which agent, result).
- **Agents make no authorization decisions.** All authz remains in Core RBAC (doc 21).

## Security Requirements

- mTLS (mutual authentication) on every connection.
- Per-agent token, **rotatable**; rotation does not drop the host.
- Signed commands; capability validation on both ends.
- **Quarantine**: revoke token + drop stream instantly on suspected compromise (doc 20 incident response).
- TLS-only; certificate pinning where feasible.

## Host Relationship

One host normally owns one agent; agent state reflects into Hosts inventory.

```text
Host └── Agent (status, capabilities, version, last_seen)
```

Cross-module link is by UUID; `HostDeleted`/`AgentDisconnected` events keep inventory consistent (doc 13 integrity rule).

## Upgrades

Manual, scheduled, and rolling upgrades; upgrade status reported and audited. Version compatibility between agent and control plane is declared and checked.

## Health

`Healthy | Warning | Degraded | Offline`, derived from heartbeats and self-reported checks; surfaced in dashboards and alerts (doc 51).

## Future Roadmap

Agent groups, remote log streaming, remote terminal (heavily audited, step-up MFA), file transfer, distributed execution.

## Invariant

Agents connect outward, prove identity with mTLS, act only within granted capabilities, and never decide what they are allowed to do — Core does.
