//! The pack's installed CLI drives the Companion-built daemon over the socket.
//!
//! # Why this test exists
//!
//! Every other test in this crate calls the router in-process. That proves the
//! code is correct; it does not prove the **product boundary** is intact. The
//! whole point of `change-cpc-004` is that the control plane moved to the
//! Companion while the `prometheus` CLI stayed in the open-source pack. If the
//! relocation changed the socket path, the request framing, or the signed
//! command contract, an in-process test would still pass and the two halves
//! would no longer speak to each other.
//!
//! So this test crosses the real boundary: it launches the Companion-built
//! `sovereign-sync` binary as a **separate process**, points the pack's
//! **installed** `prometheus` CLI at its socket through the documented
//! `SOVEREIGN_SYNC_SOCKET` discovery variable (seam 1 of the pack's integration
//! contract), and asserts the CLI can read canonical state back.
//!
//! # Skip conditions
//!
//! The test skips — loudly, never silently — when the installed CLI is absent,
//! because a machine without it cannot answer the question. A skip prints why.
//! `PROMETHEUS_CLI_TEST_BINARY` overrides the CLI path, matching the pack's own
//! `tests/kbd.rs`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn unique_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time before unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("cpc004-{label}-{nanos}"));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

/// The pack's installed CLI. Absent on a machine that has not installed the
/// pack, which is a legitimate skip rather than a failure.
fn installed_cli() -> Option<PathBuf> {
    if let Some(explicit) = std::env::var_os("PROMETHEUS_CLI_TEST_BINARY") {
        let path = PathBuf::from(explicit);
        return path.exists().then_some(path);
    }
    let default = dirs_next::home_dir()?.join(".local/bin/prometheus");
    default.exists().then_some(default)
}

/// The Companion-built daemon. `CARGO_BIN_EXE_` is set by Cargo for this
/// crate's own binary, so this is unambiguously the relocated build.
fn companion_daemon() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sovereign-sync"))
}

fn write_project_manifest(project_root: &Path) -> String {
    let project_id = uuid::Uuid::new_v4().to_string();
    fs::create_dir_all(project_root.join(".prometheus")).expect("create .prometheus");
    // `prometheus kbd` resolves a project by walking upward for this directory;
    // the manifest alone is not enough. Creating it here keeps the fixture
    // hermetic — without it the CLI would climb out of the temp dir and find
    // some real project on the machine.
    fs::create_dir_all(project_root.join(".kbd-orchestrator")).expect("create .kbd-orchestrator");
    fs::write(
        project_root.join(".prometheus").join("project.json"),
        serde_json::json!({
            "schemaVersion": "1",
            "projectId": project_id,
            "repositoryFingerprint": "sha256:cpc004-cli-socket-contract"
        })
        .to_string(),
    )
    .expect("write project manifest");
    project_id
}

/// A daemon that is killed when the test ends, however it ends.
struct Daemon(Child);

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Wait for the socket to exist and accept a connection. Polls rather than
/// sleeping a fixed interval: a fixed sleep either flakes on a slow machine or
/// wastes time on a fast one.
fn wait_for_socket(socket: &Path, daemon: &mut Child, budget: Duration) -> Result<(), String> {
    let deadline = Instant::now() + budget;
    while Instant::now() < deadline {
        if let Ok(Some(status)) = daemon.try_wait() {
            return Err(format!("daemon exited before binding its socket: {status}"));
        }
        if socket.exists() && std::os::unix::net::UnixStream::connect(socket).is_ok() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err(format!(
        "socket {} did not accept a connection within {:?}",
        socket.display(),
        budget
    ))
}

#[test]
fn installed_pack_cli_reads_canonical_state_from_the_companion_daemon() {
    let Some(cli) = installed_cli() else {
        eprintln!(
            "SKIP: no installed `prometheus` CLI (looked at $PROMETHEUS_CLI_TEST_BINARY and \
             ~/.local/bin/prometheus). This test proves the pack CLI can drive the \
             Companion-built daemon; without the CLI the question cannot be answered."
        );
        return;
    };

    let project = unique_dir("project");
    let data = unique_dir("data");
    let run = unique_dir("run");
    let socket = run.join("sovereign-sync.sock");
    let project_id = write_project_manifest(&project);

    // The Companion-built daemon, as a real process on a real socket.
    let child = Command::new(companion_daemon())
        .args(["--mode", "daemon", "--socket"])
        .arg(&socket)
        .env("PROMETHEUS_DATA_DIR", &data)
        .env("RUST_LOG", "sovereign_sync=warn")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the Companion-built sovereign-sync daemon");
    let mut daemon = Daemon(child);

    if let Err(error) = wait_for_socket(&socket, &mut daemon.0, Duration::from_secs(30)) {
        panic!("Companion daemon never became reachable: {error}");
    }

    // The pack's installed CLI, discovering the endpoint exactly as the
    // integration contract documents (seam 1): SOVEREIGN_SYNC_SOCKET wins over
    // the default path, and is honoured even mid-restart.
    let output = Command::new(&cli)
        .args(["kbd", "--path"])
        .arg(&project)
        .args(["status", "--json"])
        .env("SOVEREIGN_SYNC_SOCKET", &socket)
        .env("PROMETHEUS_DATA_DIR", &data)
        .env_remove("PROMETHEUS_CONTROL_ENDPOINT")
        .output()
        .expect("run the installed prometheus CLI");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "the pack CLI could not read state from the Companion daemon.\n\
         cli: {}\nsocket: {}\nstdout: {stdout}\nstderr: {stderr}",
        cli.display(),
        socket.display()
    );

    // Assert on the returned value, not merely on exit status: a CLI that
    // printed nothing and exited 0 would pass a status-only check while proving
    // the boundary is broken.
    let report: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|error| panic!("CLI stdout is not JSON ({error}): {stdout}"));

    // A fresh data root has no signed events, so the honest answer is
    // "initialization required" — not an error, and not a fabricated revision.
    // What proves the boundary is that the runtime path the daemon reports
    // carries *this* test's project id: the CLI asked about a project, the
    // Companion-built daemon resolved it, and the answer came back through the
    // socket.
    assert_eq!(
        report.get("runtimeInitialized").and_then(|v| v.as_bool()),
        Some(false),
        "a fresh data root should report an uninitialized runtime: {report}"
    );
    let runtime_path = report
        .get("runtimePath")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| {
            panic!("status carries no runtimePath; this is not the KBD runtime: {report}")
        });
    assert!(
        runtime_path.contains(&project_id),
        "the daemon resolved a different project than the CLI asked about.\n\
         expected id {project_id} in runtimePath, got: {runtime_path}"
    );

    // Now the part the change actually promises: a *signed typed mutation*
    // submitted by the pack's CLI and committed by the Companion-built daemon.
    // Reading state could in principle be served by a stale projection; a
    // commit cannot.
    let mutation = Command::new(&cli)
        .args(["kbd", "--path"])
        .arg(&project)
        .args([
            "phase",
            "create",
            "--command-id",
            "cpc004-socket-proof",
            "--id",
            "cpc004-proof",
            "--title",
            "cpc-004 socket proof",
        ])
        .env("SOVEREIGN_SYNC_SOCKET", &socket)
        .env("PROMETHEUS_DATA_DIR", &data)
        .env_remove("PROMETHEUS_CONTROL_ENDPOINT")
        .output()
        .expect("run the signed typed mutation");

    assert!(
        mutation.status.success(),
        "the signed typed mutation was rejected.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&mutation.stdout),
        String::from_utf8_lossy(&mutation.stderr)
    );

    // Read back through the same socket and prove the commit is durable.
    let after = Command::new(&cli)
        .args(["kbd", "--path"])
        .arg(&project)
        .args(["status", "--json"])
        .env("SOVEREIGN_SYNC_SOCKET", &socket)
        .env("PROMETHEUS_DATA_DIR", &data)
        .env_remove("PROMETHEUS_CONTROL_ENDPOINT")
        .output()
        .expect("re-read status after the mutation");
    let after_report: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&after.stdout)).unwrap_or_else(|error| {
            panic!(
                "post-mutation stdout is not JSON ({error}): {}",
                String::from_utf8_lossy(&after.stdout)
            )
        });

    let revision = after_report
        .get("revision")
        .and_then(|v| v.as_u64())
        .unwrap_or_else(|| panic!("post-mutation status carries no revision: {after_report}"));
    assert!(
        revision > 0,
        "a committed signed mutation must advance the revision past zero: {after_report}"
    );
    assert!(
        after_report
            .get("phases")
            .and_then(|p| p.get("cpc004-proof"))
            .is_some(),
        "the phase committed through the socket is absent from canonical state: {after_report}"
    );
}
