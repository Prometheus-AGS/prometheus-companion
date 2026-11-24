//! Disposable PEER-004 receiver. Normal signed sync uses the running node;
//! the two size-boundary inputs are read exclusively from local sparse files.

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{ensure, Context, Result};
use prometheus_substrate::{identity, node};
use serde_json::json;
use sovereign_sync::p2p::{read_peer_frame, PeerFrameReadError, MAX_PEER_FRAME_SIZE};

fn sparse_file(directory: &Path, bytes: u64) -> Result<tokio::fs::File> {
    let file = tempfile::tempfile_in(directory)?;
    file.set_len(bytes)?;
    Ok(tokio::fs::File::from_std(file))
}

async fn check_size(directory: &Path) -> Result<()> {
    let maximum = MAX_PEER_FRAME_SIZE as u64;
    let mut exact = sparse_file(directory, maximum)?;
    let accepted = read_peer_frame(&mut exact).await?.len();
    ensure!(
        accepted == MAX_PEER_FRAME_SIZE,
        "exact-limit file was truncated"
    );

    let mut over = sparse_file(directory, maximum + 1)?;
    let rejected = over.metadata().await?.len();
    match read_peer_frame(&mut over).await {
        Err(PeerFrameReadError::TooLarge) => {}
        Err(error) => return Err(error.into()),
        Ok(_) => anyhow::bail!("oversized file was admitted"),
    }
    let receipt = json!({
        "pid": std::process::id(),
        "maximumBytes": maximum,
        "acceptedBytes": accepted,
        "rejectedBytes": rejected,
        "rejection": "too_large",
    });
    let mut output = io::stdout().lock();
    writeln!(output, "KBD_FRAME_SIZE_RECEIPT {receipt}")?;
    output.flush()?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    identity::require_unattended()?;
    let skills = PathBuf::from(
        std::env::var_os("PROMETHEUS_SKILLS_DIR").context("fixture skills directory")?,
    );
    let directory =
        PathBuf::from(std::env::var_os("TMPDIR").context("fixture temporary directory")?);
    let (handle, readiness) = node::spawn(&skills).await?;
    if !matches!(readiness, node::NodeReadiness::Ready) {
        handle.shutdown().await?;
        anyhow::bail!("receiver did not open its project authority");
    }

    // A regular thread keeps a blocked stdin read off the async runtime, so
    // SIGINT can still shut the node down before the fixture is triggered.
    let (send, command) = tokio::sync::oneshot::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        let result = io::stdin().read_line(&mut line).map(|_| line);
        let _ = send.send(result);
    });
    let check = async {
        ensure!(
            command.await??.trim_end() == "check-size",
            "expected check-size"
        );
        check_size(&directory).await?;
        std::future::pending::<Result<()>>().await
    };
    let result = tokio::select! {
        result = check => result,
        signal = tokio::signal::ctrl_c() => signal.map_err(anyhow::Error::from),
    };
    let shutdown = handle.shutdown().await;
    result.and(shutdown)
}
