# Configuration Templates

Every file in this directory ending in `.example.toml` is a public
template. None of them contain real credentials, hostnames, or
infrastructure details. The full model is documented in
[`docs/security/public-private-boundary.md`](../docs/security/public-private-boundary.md).

## Quick start (local development)

```bash
cp config/autotim.example.toml config/autotim.toml
cp config/secrets.example.toml config/secrets.toml
# edit config/autotim.toml: point database.url at your local Postgres
```

The copied files (`config/*.toml`, without `.example`) are gitignored
and never committed — only the templates are tracked.

## Files

| File | Purpose |
|------|---------|
| `autotim.example.toml` | Base bootstrap config (doc 11) |
| `autotim.development.example.toml` | Optional development overlay |
| `autotim.staging.example.toml` | Optional staging overlay |
| `autotim.production.example.toml` | Optional production overlay |
| `providers.example.toml` | Optional first-boot Integration Provider selection (doc 42) — no credentials |
| `secrets.example.toml` | Fake secrets for local development only |

## Production

Production config does not come from this directory at all. It lives
at `/etc/autotim/config.toml` on the target host, delivered via
systemd `EnvironmentFile=` and/or the private deployment repository —
see the boundary document for the full loading order.
