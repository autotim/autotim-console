//! Autotim Console — Community Edition.
//!
//! Single binary: the Vue 3 production build is embedded via
//! `rust-embed` and served by Axum with SPA fallback (doc 10, doc 50).
//!
//! Status: scaffold. Loads bootstrap config (doc 11; see also
//! `docs/security/public-private-boundary.md` for where real config
//! lives in each environment), boots the kernel, registers compiled-in
//! Infrastructure modules, and serves embedded frontend assets plus a
//! liveness endpoint. Real Core port wiring (RBAC, Secrets, EventBus,
//! Jobs, Audit, Settings, Tenancy) is the next milestone per the agreed
//! build sequence.

use autotim_kernel::config::BootstrapConfig;
use axum::{routing::get, Router};
use rust_embed::RustEmbed;
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(RustEmbed)]
#[folder = "../../frontend/dist"]
#[allow(dead_code)] // not yet wired into routes — see doc 10 §"Single-Binary Packaging"
struct WebAssets;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Config must load before logging is configured (its own
    // logging.level/format come from this file) — so failures here go
    // to stderr via eprintln!, not tracing. This is what doc 11 means
    // by "boot-time errors are fatal": no half-configured fallback.
    let config_path = std::env::var("AUTOTIM_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/etc/autotim/config.toml"));

    let bootstrap = match BootstrapConfig::load(&config_path) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!(
                "fatal: failed to load bootstrap config from {}: {e}\n\
                 see docs/security/public-private-boundary.md for how to create it from\n\
                 config/autotim.example.toml",
                config_path.display()
            );
            std::process::exit(1);
        }
    };

    let format = bootstrap.logging.format.as_str();
    if format == "pretty" {
        tracing_subscriber::fmt()
            .with_target(false)
            .with_env_filter(&bootstrap.logging.level)
            .init();
    } else {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(&bootstrap.logging.level)
            .init();
    }

    tracing::info!(environment = ?bootstrap.environment, config = %config_path.display(), "bootstrap config loaded");

    let kernel = autotim_kernel::Kernel::new();
    // Module registration happens here once Core crates implement
    // `autotim_sdk::Module` (kernel + tenancy is the next milestone).
    kernel.validate().map_err(|e| anyhow::anyhow!(e))?;

    tracing::info!(modules = ?kernel.module_names(), "autotim starting");

    let app = Router::new().route("/api/v1/health", get(health));
    // Frontend embedding (doc 10, doc 50) and SPA fallback land with the
    // first frontend build; the WebAssets embed above documents the
    // intended packaging shape.

    let addr: SocketAddr = bootstrap
        .server
        .bind
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid server.bind '{}': {e}", bootstrap.server.bind))?;
    tracing::info!(%addr, "listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health() -> &'static str {
    "ok"
}
