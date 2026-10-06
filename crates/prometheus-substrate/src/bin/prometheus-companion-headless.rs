//! The Companion node with no window.
//!
//! Same node the windowed Companion hosts, same socket, same journal — just no
//! Tauri shell and no Stronghold vault. Two callers need it:
//!
//! - the two-process integration harness (`change-cpc-011`), which must launch
//!   a real node without a GUI;
//! - a server-side or headless deployment.
//!
//! It refuses to start unless it can sign unattended. See `identity.rs` for why
//! that check has to happen here, before anything binds, rather than at the
//! first mutation.

use prometheus_substrate::{identity, node, paths};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                tracing_subscriber::EnvFilter::new("prometheus_substrate=info,sovereign_sync=warn")
            }),
        )
        .init();

    // Fail closed, and fail first. A headless node that discovered its signing
    // problem at the first mutation would already have reported itself healthy
    // and accepted work it cannot finish.
    let signing = identity::require_unattended()?;
    tracing::info!(identity = ?signing, "unattended signing identity resolved");

    let skills_dir = paths::skills_dir();
    tracing::info!(
        data_root = %paths::data_root().display(),
        skills = %skills_dir.display(),
        "starting the headless companion node"
    );

    let (handle, readiness) = node::spawn(&skills_dir).await?;
    match readiness {
        node::NodeReadiness::Ready => {
            tracing::info!(socket = %handle.socket().display(), "node ready");
        }
        node::NodeReadiness::DiagnosticsOnly => {
            // Reported, not hidden: the socket answers health checks but the
            // journal did not open, and an operator needs to know which.
            tracing::warn!(
                socket = %handle.socket().display(),
                "node is serving DIAGNOSTICS ONLY — canonical authority failed to open"
            );
        }
    }

    let listener_exited = tokio::select! {
        _ = sovereign_sync::rest_api::shutdown_signal() => {
            tracing::info!("shutdown signal received");
            false
        },
        _ = handle.wait_for_exit() => {
            tracing::error!("control listener exited unexpectedly");
            true
        },
    };
    handle.shutdown().await?;
    if listener_exited {
        anyhow::bail!("Companion control listener exited unexpectedly");
    }
    Ok(())
}
