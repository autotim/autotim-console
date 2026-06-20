# Public/Private Boundary

## Purpose

`autotim-console` is a public, AGPL-3.0 repository. The actual Autotim
deployment — its production hostnames and nameservers, PowerDNS
configuration, private IP ranges, operator usernames and emails, API
tokens, OIDC client secrets, SSH hosts, and deployment-specific
filesystem paths — is infrastructure data, not product source code.
This document defines the boundary between the two and how the same
public binary is configured for a real, private deployment without
any of that data ever entering Git.

## Architecture Decision

```text
Public repository (autotim-console)   = product source code
Private deployment repo, or /etc/autotim/  = real infrastructure configuration
```

The public repo ships the binary and a set of `.example` configuration
templates. No file under version control contains a real hostname, IP
address, username, email address, credential, or deployment path. A
deployment of this software against real Autotim infrastructure is
configured entirely outside the repository: at `/etc/autotim/` on the
target host, through systemd `EnvironmentFile=`, and/or in a private
deployment repository that is never merged into this one.

## What Belongs in the Public Repo

- Source code, the `Module` trait and SDK, provider trait
  implementations (doc 42) — the logic for talking to PowerDNS,
  NetBird, etc., not which PowerDNS or NetBird instance.
- `config/*.example.toml` — schema and structure of bootstrap config,
  every value a placeholder.
- `deploy/systemd/*.example` — the shape of the systemd unit and
  environment file, no real paths beyond the standard
  `/etc/autotim/`, `/var/lib/autotim/` filesystem layout already
  documented in doc 10.
- Architecture documentation (`docs/architecture/`) and this document.
- CI configuration, tests, fixtures using fake/generated data.

## What Must Stay Private

- Real hostnames: production auth/API endpoints, nameservers, and any
  other production DNS name.
- Real IP addresses, internal or external.
- Operator usernames, email addresses, SSH user/host pairs.
- API tokens, OIDC client secrets, database passwords, the Secrets
  module's KEK/unseal material.
- Deployment-specific filesystem paths beyond the standard layout
  (anything that would reveal how a specific host is organized).
- Any exported settings/secrets backup (doc 23, doc 30 — these export
  ciphertext and references, not plaintext, but they are still
  deployment artifacts and do not belong in a public repo regardless).

If a value would let someone reach, identify, or impersonate a real
Autotim system, it stays out of this repository — no exception for
"it's just a hostname" or "it's just a username."

## Configuration Loading Model

Bootstrap config (doc 11, "Two Kinds of Configuration") loads in three
layers, lowest to highest precedence:

```text
1. Base file           e.g. /etc/autotim/config.toml, or config/autotim.toml locally
2. Environment overlay  <same-directory>/<base-stem>.<environment>.toml, if present
3. Environment variables AUTOTIM_SECTION__KEY=value — always wins
```

The `environment` key (read from the base file, or from
`AUTOTIM_ENVIRONMENT`) selects which overlay file the loader looks
for. A missing overlay file is not an error — the base file alone is
a complete, valid config; the overlay is an optional refinement.

This is implemented in `autotim-kernel::config` (`BootstrapConfig::load`)
and is the only place bootstrap config is parsed. Nothing here is
DB-backed; everything DB-backed (provider credentials, OIDC issuer
URLs, notification channels, UI theme) is Settings (doc 30), configured
after boot, through the UI or API, never through a file in this
repository.

## Environments

| Environment | Typical use | Validator strictness |
|---|---|---|
| `development` | Local machine, `cargo run` against a local Postgres | Baseline checks only |
| `staging` | Pre-production, real-shaped data, not customer-facing | Baseline checks only |
| `production` | Real Autotim infrastructure | Baseline checks + TLS material required + zero tolerance for template placeholders |

The environment is a single field (`environment = "..."`) read at the
top of the bootstrap config file; it is not a different config *file
format*, just a value that changes which overlay file is looked for
and how strictly the validator behaves.

## Creating Local Config From Examples

```bash
cp config/autotim.example.toml config/autotim.toml
cp config/secrets.example.toml config/secrets.toml
# optional, only if you want providers pre-selected:
cp config/providers.example.toml config/providers.toml

# edit config/autotim.toml:
#   - database.url -> your local Postgres
#   - leave environment = "development"
```

Run with `AUTOTIM_CONFIG=config/autotim.toml cargo run -p autotim-bin`.
The copied files are gitignored; `git status` should never show them
as untracked-but-about-to-be-added — if it does, the file was created
outside the `config/` directory pattern this document assumes, and
that is worth investigating before committing anything.

## How Production Config Is Loaded

On a real Autotim host, none of the files above exist in a checkout of
this repository at all. The deployment looks like this:

```text
/usr/local/bin/autotim                 the binary, built from this repo
/etc/autotim/config.toml               real bootstrap config (not in Git, ever)
/etc/autotim/autotim.env               systemd EnvironmentFile (not in Git, ever)
/etc/systemd/system/autotim.service    built from deploy/systemd/autotim.service.example
```

The two `.example` files under `deploy/systemd/` show the shape of the
unit and environment file; the real files are created once, by hand or
by the private deployment tooling, directly on the host (or delivered
by whatever configuration-management system the private deployment
repository uses — Ansible, a small internal script, manual
provisioning — this repository does not need to know which).

`/etc/autotim/config.toml` carries the real `database.url`,
`server.bind`, TLS paths, and `secrets.key_provider`. The Secrets
unseal passphrase (or KMS auth) does **not** go in that file — it goes
in `/etc/autotim/autotim.env`, which systemd injects as an environment
variable (`AUTOTIM_SECRETS__PASSPHRASE`), per the environment-variable
override layer above. This keeps the one genuinely catastrophic value
— the thing that unseals every other secret — out of any file that
might be casually opened, `cat`'d, or backed up alongside the rest of
`/etc/autotim/`.

## Secrets Handling — Two Different Things Named "Secrets"

It is worth being explicit that this document's `config/secrets.example.toml`
and doc 23's Secrets module are not the same mechanism, deliberately:

```text
config/secrets.toml (local dev only)
  → a few fake credentials, loaded so a developer can exercise
    integration code paths without configuring everything through
    the Settings UI by hand. Never used outside development.

doc 23 Secrets module (every environment, including development)
  → the actual, envelope-encrypted, audited store for every real
    credential the platform holds: DNS API tokens, OIDC client
    secrets, SMTP passwords, agent tokens. Populated through the
    Settings UI/API after boot. This is where real Autotim
    credentials live, full stop — never in a TOML file, never in Git.
```

The only credential-shaped value that legitimately lives in bootstrap
config at all is the Secrets module's own unseal material — and even
that lives in an environment variable, not a tracked file.

## Startup Validation

`autotim-kernel::config::BootstrapConfig::validate()` runs as part of
loading, before the kernel proceeds to construct any Core port. It
checks, for every environment: `server.bind` and `database.url` are
non-empty; `secrets.key_provider` is one of the three known values
(doc 23). In `production` specifically, it additionally requires
`server.tls_cert`/`server.tls_key` to be set, and rejects the value if
it still contains one of the placeholder sentinels used throughout
every `.example.toml` file (`CHANGE_ME`, `REPLACE_WITH`, etc.) — so
copying a template into `/etc/autotim/config.toml` without editing it
fails loudly at boot rather than starting with a fake database
password or no TLS.

This is additive to, not a replacement for, the kernel's existing
boot-time validation (module dependency law, `sdk_version`
compatibility, doc 11 §"Boot Sequence") — config validation is simply
the first check in that sequence, since nothing else can proceed
without a valid bootstrap config.

## Local Development Against Real Infrastructure

Bootstrap config loading (`autotim-kernel::config::BootstrapConfig::load`)
accepts any path via `AUTOTIM_CONFIG` — there is no requirement that it
point inside this repository's working tree. Developing or testing
against real infrastructure does not require, and should not use, a
config or notes directory nested inside this repo, even a gitignored
one: a directory that is merely *promised* to be ignored still sits in
the same working tree that gets `git add`ed, packaged, or
re-initialized, and a single `-f`, a packaging script that copies the
whole tree, or a fresh `git init` run carelessly can defeat that
promise. A directory that is not part of this working tree at all
cannot leak through a git operation on this repository, by
construction.

The recommended pattern is a second, separate, private repository —
e.g. `environments/<env>/config.toml`, an inventory file, working
notes, anything that needs real values — never merged into this one,
referenced only by path at run time:

```bash
cd autotim-console
AUTOTIM_CONFIG=../autotim-deployment/environments/dev/config.toml cargo run
```

This also gives the real configs and notes proper version history,
which a gitignored-and-never-committed directory cannot.

## Security Checklist

Before pushing any commit to the public repository:

- [ ] No real production hostname or subdomain, under any domain.
- [ ] No real IP address (use `203.0.113.0/24` or `198.51.100.0/24` — RFC 5737 ranges reserved for documentation — if an example needs one).
- [ ] No real username, email address, or SSH host/user pair.
- [ ] No API token, OIDC client secret, or database password that resolves to anything real.
- [ ] No file under `config/` without the `.example` suffix.
- [ ] No file under `deploy/` without the `.example` suffix.
- [ ] `git status` shows no untracked `*.toml` file outside `*.example.toml`.
- [ ] `git diff --staged` reviewed line by line for anything that looks like a real value, not just a glance at filenames.

Before merging a PR that touches `config/`, `deploy/`, `.gitignore`, or
this document specifically:

- [ ] The change was reviewed by someone other than the author.
- [ ] CI's secret-scanning step (if configured — see doc 62) passed.

## If a Secret Is Committed by Mistake

Removing a file in a later commit does not remove it from Git
history. If a real credential, hostname, or other private value is
ever committed:

1. Rotate the credential immediately — assume it is compromised the
   moment it is pushed, regardless of how quickly it is reverted (doc
   20 §"Assume breach").
2. Do not rely on `git revert` or deleting the file alone; history
   still contains it. Rewriting history (`git filter-repo` or
   equivalent) and force-pushing is a separate, disruptive operation —
   coordinate it, do not do it unilaterally on a shared branch.
3. Treat it as a security incident per doc 20's incident response
   controls, even though the exposure path here is "public Git" rather
   than "compromised agent" — the response shape (rotate, audit,
   notify) is the same.

## Golden Path Summary

```text
Want to run it locally?        cp config/*.example.toml → edit → AUTOTIM_CONFIG=...
Want to deploy it for real?    /etc/autotim/config.toml + /etc/autotim/autotim.env,
                                created once on the host, never in this repo
Want to add a new config key?  add it to config/*.example.toml with a placeholder
                                value, document it here if it's security-relevant
Found a real value in Git?     rotate it now, then fix the history
```
