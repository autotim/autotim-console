//! Bootstrap configuration: structs, layered loading, and the startup
//! validation layer.
//!
//! This module owns exactly what doc 11 (Kernel) calls "bootstrap
//! config" — bind address, database URL, the Secrets key-provider
//! selection, logging. Nothing here is DB-backed or tenant-scoped;
//! that is Settings (doc 30). The full public/private deployment
//! model this module implements is documented in
//! `docs/security/public-private-boundary.md`.

use serde::Deserialize;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    Development,
    Staging,
    Production,
}

impl Environment {
    pub fn is_production(&self) -> bool {
        matches!(self, Environment::Production)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub bind: String,
    pub tls_cert: Option<String>,
    pub tls_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SecretsConfig {
    pub key_provider: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BootstrapConfig {
    pub environment: Environment,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub secrets: SecretsConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("config file not found: {0}")]
    NotFound(PathBuf),
    #[error("failed to read {0}: {1}")]
    Read(PathBuf, std::io::Error),
    #[error("failed to parse {0}: {1}")]
    Parse(PathBuf, toml::de::Error),
    #[error("invalid configuration: {0}")]
    Invalid(String),
}

/// Sentinel strings used across every `*.example.toml` template
/// (`config/`). If one of these survives into a loaded value, the
/// template was copied without being edited — refuse to boot rather
/// than run with a placeholder credential by accident.
const PLACEHOLDER_SENTINELS: &[&str] =
    &["CHANGE_ME", "REPLACE_WITH", "REPLACE_ME", "dev-only-fake"];

fn contains_placeholder(value: &str) -> bool {
    PLACEHOLDER_SENTINELS.iter().any(|s| value.contains(s))
}

impl BootstrapConfig {
    /// Loads bootstrap config in three layers, in order:
    ///   1. the base file at `base_path`
    ///   2. an optional environment overlay, `<stem>.<environment>.toml`
    ///      in the same directory (e.g. `config.toml` + `config.production.toml`,
    ///      or `autotim.toml` + `autotim.development.toml`)
    ///   3. environment variables, `AUTOTIM_SECTION__KEY`, which win
    ///      over both files (container/systemd-friendly overrides,
    ///      per doc 11)
    ///
    /// then runs `validate()` before returning.
    pub fn load(base_path: &Path) -> Result<Self, ConfigError> {
        let base = read_toml(base_path)?;

        let env_str = base
            .get("environment")
            .and_then(|v| v.as_str())
            .unwrap_or("development")
            .to_string();

        let stem = base_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("autotim");
        let overlay_path = base_path.with_file_name(format!("{stem}.{env_str}.toml"));

        let mut merged = base;
        if overlay_path.exists() {
            let overlay = read_toml(&overlay_path)?;
            merge_toml(&mut merged, overlay);
        }

        apply_env_overrides(&mut merged);

        let config: BootstrapConfig = serde::Deserialize::deserialize(merged)
            .map_err(|e: toml::de::Error| ConfigError::Parse(base_path.to_path_buf(), e))?;

        config.validate()?;
        Ok(config)
    }

    /// Startup validation (doc 11 §"Error Handling & Panic Strategy":
    /// boot-time errors are fatal, there is no best-effort boot).
    /// Returns an error rather than panicking, so the caller logs a
    /// clear message and exits non-zero.
    pub fn validate(&self) -> Result<(), ConfigError> {
        let mut problems = Vec::new();

        if self.server.bind.trim().is_empty() {
            problems.push("server.bind is empty".to_string());
        }
        if self.database.url.trim().is_empty() {
            problems.push("database.url is empty".to_string());
        }
        if contains_placeholder(&self.database.url) {
            problems.push("database.url still contains an unedited template placeholder".to_string());
        }

        const KNOWN_KEY_PROVIDERS: &[&str] = &["passphrase", "os-keystore", "kms"];
        if !KNOWN_KEY_PROVIDERS.contains(&self.secrets.key_provider.as_str()) {
            problems.push(format!(
                "secrets.key_provider '{}' is not one of {KNOWN_KEY_PROVIDERS:?}",
                self.secrets.key_provider
            ));
        }

        // Production gets stricter checks. Note: passphrase-unseal is a
        // valid production choice for self-hosted (doc 23) — it is not
        // flagged here, only missing/templated TLS material and any
        // surviving placeholder sentinel are.
        if self.environment.is_production() {
            if self.server.tls_cert.is_none() || self.server.tls_key.is_none() {
                problems.push(
                    "production requires server.tls_cert and server.tls_key to be set \
                     (terminate TLS here, or set both explicitly to a value acknowledging \
                     TLS is terminated upstream)"
                        .to_string(),
                );
            }
            for (label, value) in [
                ("server.tls_cert", self.server.tls_cert.as_deref().unwrap_or("")),
                ("server.tls_key", self.server.tls_key.as_deref().unwrap_or("")),
            ] {
                if contains_placeholder(value) {
                    problems.push(format!("{label} still contains an unedited template placeholder"));
                }
            }
        }

        if problems.is_empty() {
            Ok(())
        } else {
            Err(ConfigError::Invalid(problems.join("; ")))
        }
    }
}

fn read_toml(path: &Path) -> Result<toml::Value, ConfigError> {
    if !path.exists() {
        return Err(ConfigError::NotFound(path.to_path_buf()));
    }
    let text = std::fs::read_to_string(path).map_err(|e| ConfigError::Read(path.to_path_buf(), e))?;
    toml::from_str(&text).map_err(|e| ConfigError::Parse(path.to_path_buf(), e))
}

/// Table merge: keys in `overlay` win over keys in `base`, recursing
/// into nested tables (sufficient for the flat `[section] key = value`
/// shape of bootstrap config).
fn merge_toml(base: &mut toml::Value, overlay: toml::Value) {
    let (toml::Value::Table(_), toml::Value::Table(_)) = (&base, &overlay) else {
        return;
    };
    let toml::Value::Table(overlay_table) = overlay else {
        unreachable!()
    };
    let toml::Value::Table(base_table) = base else {
        unreachable!()
    };

    for (key, value) in overlay_table {
        let merged_value = match base_table.remove(&key) {
            Some(mut existing) if existing.is_table() && value.is_table() => {
                merge_toml(&mut existing, value);
                existing
            }
            _ => value,
        };
        base_table.insert(key, merged_value);
    }
}

/// `AUTOTIM_SECTION__KEY=value` overrides `[section] key = value`,
/// the convention already established in doc 11.
fn apply_env_overrides(value: &mut toml::Value) {
    for (env_key, env_val) in std::env::vars() {
        let Some(rest) = env_key.strip_prefix("AUTOTIM_") else {
            continue;
        };
        let path: Vec<String> = rest.to_lowercase().split("__").map(str::to_string).collect();
        set_path(value, &path, env_val);
    }
}

fn set_path(value: &mut toml::Value, path: &[String], new_val: String) {
    if path.is_empty() {
        return;
    }
    if !value.is_table() {
        *value = toml::Value::Table(toml::map::Map::new());
    }
    let toml::Value::Table(table) = value else {
        unreachable!()
    };
    if path.len() == 1 {
        table.insert(path[0].clone(), toml::Value::String(new_val));
        return;
    }
    let mut child = table
        .remove(&path[0])
        .unwrap_or_else(|| toml::Value::Table(toml::map::Map::new()));
    set_path(&mut child, &path[1..], new_val);
    table.insert(path[0].clone(), child);
}
