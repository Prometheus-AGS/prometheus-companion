//! State written by the pack's daemon is read and extended by the Companion node.
//!
//! # The question this answers
//!
//! `change-cpc-004` moved the control plane into this repository. The operator's
//! existing journal, receipts, and run history did not move — they sit in a data
//! root on disk, written by the pack's `sovereign-sync` daemon. The phase goal
//! requires that state to "migrate intact", and that phrase has to mean
//! something falsifiable.
//!
//! It means this: a data root populated by **one binary** must be read at the
//! same revision by a **different binary**, and appended to without losing
//! anything that was already there.
//!
//! Two processes, sequentially, on one data root:
//!
//! 1. the pack's daemon writes state (a run and a phase),
//! 2. the Companion's headless node opens the same root and appends,
//! 3. every prior receipt and run is still present afterwards.
//!
//! An in-process test cannot answer this. Sharing a process means sharing
//! resolved paths, an open journal, and a signing identity — precisely the three
//! things that could differ between binaries and silently split the operator's
//! history in two.
//!
//! # What proves *the node* served a request
//!
//! One fact shapes this test and was discovered by a negative control rather
//! than assumed: `prometheus kbd` **does not require a node**. It reads the
//! local signed runtime straight out of `PROMETHEUS_DATA_DIR`, and only uses
//! the control socket when a control plane is present. So a CLI call
//! succeeding proves nothing about the node — an early draft of this test
//! passed even when the Companion was pointed at a completely different data
//! root.
//!
//! Therefore the node is asserted **directly over its socket**, where nothing
//! answers unless it is running, and the shared-journal claim is asserted
//! through the CLI against the one data root both binaries used.
//!
//! # Skips
//!
//! Skips loudly when the pack's installed CLI is absent, because without it
//! there is no way to write or read canonical state. A skip says why.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn unique_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time before unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("cpc005-{label}-{nanos}"));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

fn installed_cli() -> Option<PathBuf> {
    if let Some(explicit) = std::env::var_os("PROMETHEUS_CLI_TEST_BINARY") {
        let path = PathBuf::from(explicit);
        return path.exists().then_some(path);
    }
    dirs_next::home_dir()
        .map(|home| home.join(".local/bin/prometheus"))
        .filter(|path| path.exists())
}

/// The pack's daemon, installed alongside the CLI. Absent on a machine that
/// never enabled sharing, which is a legitimate skip.
fn pack_daemon() -> Option<PathBuf> {
    dirs_next::home_dir()
        .map(|home| home.join(".local/bin/sovereign-sync"))
        .filter(|path| path.exists())
}

fn companion_headless() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_prometheus-companion-headless"))
}

struct Process(Child);

impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn wait_for_socket(socket: &Path, child: &mut Child, budget: Duration) -> Result<(), String> {
    let deadline = Instant::now() + budget;
    while Instant::now() < deadline {
        if let Ok(Some(status)) = child.try_wait() {
            return Err(format!("process exited before binding: {status}"));
        }
        if socket.exists() && std::os::unix::net::UnixStream::connect(socket).is_ok() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err(format!(
        "socket {} unreachable after {budget:?}",
        socket.display()
    ))
}

fn write_project(project_root: &Path) -> String {
    let project_id = uuid::Uuid::new_v4().to_string();
    fs::create_dir_all(project_root.join(".prometheus")).expect("create .prometheus");
    fs::create_dir_all(project_root.join(".kbd-orchestrator")).expect("create .kbd-orchestrator");
    fs::write(
        project_root.join(".prometheus").join("project.json"),
        serde_json::json!({
            "schemaVersion": "1",
            "projectId": project_id,
            "repositoryFingerprint": "sha256:cpc005-continuity"
        })
        .to_string(),
    )
    .expect("write project manifest");
    project_id
}

/// Run the pack CLI against whichever node is currently listening.
fn cli(
    binary: &Path,
    project: &Path,
    data: &Path,
    socket: &Path,
    args: &[&str],
) -> std::process::Output {
    Command::new(binary)
        .args(["kbd", "--path"])
        .arg(project)
        .args(args)
        .env("SOVEREIGN_SYNC_SOCKET", socket)
        .env("PROMETHEUS_DATA_DIR", data)
        .env_remove("PROMETHEUS_CONTROL_ENDPOINT")
        .output()
        .expect("run the pack CLI")
}

/// A minimal HTTP/1.1 GET over a Unix socket.
///
/// Deliberately dependency-free and deliberately *not* routed through the CLI:
/// the whole point is to talk to the node directly, so that a response is
/// evidence the node exists.
fn socket_get(socket: &Path, path: &str) -> Result<String, String> {
    use std::io::{Read, Write};
    let mut stream = std::os::unix::net::UnixStream::connect(socket)
        .map_err(|error| format!("cannot connect to {}: {error}", socket.display()))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|error| error.to_string())?;
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
    )
    .map_err(|error| error.to_string())?;
    let mut body = String::new();
    stream
        .read_to_string(&mut body)
        .map_err(|error| error.to_string())?;
    Ok(body)
}

fn status_json(output: &std::process::Output) -> serde_json::Value {
    let text = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("status is not JSON ({error}): {text}"))
}

#[test]
fn companion_node_reads_and_extends_state_written_by_the_pack_daemon() {
    let Some(cli_bin) = installed_cli() else {
        eprintln!(
            "SKIP: no installed `prometheus` CLI. Continuity cannot be checked without \
             the pack's own client."
        );
        return;
    };
    let Some(daemon_bin) = pack_daemon() else {
        eprintln!(
            "SKIP: no installed `sovereign-sync` daemon at ~/.local/bin. This test needs the \
             *pack's* binary to write the state that the Companion node then reads; using the \
             Companion's own binary for both halves would prove nothing about continuity."
        );
        return;
    };

    let project = unique_dir("project");
    let data = unique_dir("data");
    let run = unique_dir("run");
    let socket = run.join("sovereign-sync.sock");
    let key_file = run.join("device-key.json");
    let project_id = write_project(&project);

    // ---- Phase 1: the PACK's daemon writes canonical state ------------------
    let mut pack = Process(
        Command::new(&daemon_bin)
            .args(["--mode", "daemon", "--socket"])
            .arg(&socket)
            .env("PROMETHEUS_DATA_DIR", &data)
            .env("RUST_LOG", "sovereign_sync=warn")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn the pack daemon"),
    );
    wait_for_socket(&socket, &mut pack.0, Duration::from_secs(30))
        .unwrap_or_else(|error| panic!("pack daemon never became reachable: {error}"));

    let created = cli(
        &cli_bin,
        &project,
        &data,
        &socket,
        &[
            "phase",
            "create",
            "--command-id",
            "cpc005-continuity-pack",
            "--id",
            "written-by-pack",
            "--title",
            "written by the pack daemon",
        ],
    );
    assert!(
        created.status.success(),
        "the pack daemon could not commit a phase: {}",
        String::from_utf8_lossy(&created.stderr)
    );

    let before = status_json(&cli(
        &cli_bin,
        &project,
        &data,
        &socket,
        &["status", "--json"],
    ));
    let before_revision = before["revision"].as_u64().expect("revision before");
    let before_frontier = before["frontier"].clone();
    let before_receipts = before["latestBoundaryReceipts"].clone();
    let before_run = before["runId"].clone();
    let audit_before = cli(&cli_bin, &project, &data, &socket, &["audit", "--json"]);
    let audit_before_len = audit_before.stdout.len();
    assert!(
        audit_before.status.success() && audit_before_len > 0,
        "signed audit export produced nothing before the handover"
    );

    // The daemon must be fully stopped before the Companion opens the same
    // root: two writers on one journal is a different (and unsupported) test.
    drop(pack);
    std::thread::sleep(Duration::from_millis(500));

    // ---- Phase 2: the COMPANION's node opens the same data root -------------
    // A key file keeps the headless node unattended; without it the node
    // correctly refuses to start (see identity.rs), which is a different test.
    fs::write(&key_file, "{}").expect("write placeholder key file");

    let mut companion = Process(
        Command::new(companion_headless())
            .env("PROMETHEUS_DATA_DIR", &data)
            .env("SOVEREIGN_SYNC_SOCKET", &socket)
            .env("PROMETHEUS_DEVICE_KEY_FILE", &key_file)
            .env("PROMETHEUS_SKILLS_DIR", project.join("skills"))
            .env("RUST_LOG", "prometheus_substrate=warn,sovereign_sync=warn")
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn the Companion headless node"),
    );
    if let Err(error) = wait_for_socket(&socket, &mut companion.0, Duration::from_secs(30)) {
        panic!("Companion node never became reachable on the pack's data root: {error}");
    }

    // ---- Assertion 1: the COMPANION node is what is answering ---------------
    // Nothing responds on this socket unless the Companion's own node is
    // serving it: the pack daemon was stopped above, and its socket file was
    // replaced when the Companion bound.
    let health = socket_get(&socket, "/health").expect("the Companion node must answer /health");
    assert!(
        health.contains("\"status\":\"ok\""),
        "the Companion node did not report healthy over its socket: {health}"
    );

    // The node's registry is read from *its own* data root, so this is the
    // assertion that catches a Companion pointed at the wrong root. A negative
    // control proved it: without this, the test still passed when the node
    // opened a completely different data root, because `prometheus kbd` reads
    // the local runtime directly and never needed the node at all.
    let node_projects =
        socket_get(&socket, "/api/v1/kbd/projects").expect("node must serve its project registry");
    assert!(
        node_projects.contains(&project_id),
        "the Companion node's registry does not contain the project the pack daemon wrote \
         ({project_id}) — the node opened a different data root.\nnode said: {node_projects}"
    );

    // ---- Assertion 2: nothing was lost in the handover ----------------------
    let after = status_json(&cli(
        &cli_bin,
        &project,
        &data,
        &socket,
        &["status", "--json"],
    ));

    assert_eq!(
        after["revision"].as_u64(),
        Some(before_revision),
        "the Companion node reports a different revision than the daemon left behind — \
         it opened a different journal, not the same one"
    );
    assert_eq!(
        after["frontier"], before_frontier,
        "frontier diverged across the binary handover"
    );
    assert_eq!(
        after["runId"], before_run,
        "the Companion node invented a new run instead of continuing the existing one"
    );
    assert_eq!(
        after["latestBoundaryReceipts"], before_receipts,
        "receipts written by the pack daemon are missing after the handover"
    );
    assert!(
        after["phases"].get("written-by-pack").is_some(),
        "the phase the pack daemon committed is absent from the Companion's view: {}",
        after["phases"]
    );
    assert_eq!(
        after["projectId"].as_str(),
        Some(project_id.as_str()),
        "the Companion node resolved a different project"
    );

    // ---- And the Companion can extend it ------------------------------------
    let appended = cli(
        &cli_bin,
        &project,
        &data,
        &socket,
        &[
            "phase",
            "create",
            "--command-id",
            "cpc005-continuity-companion",
            "--id",
            "written-by-companion",
            "--title",
            "written by the companion node",
        ],
    );
    assert!(
        appended.status.success(),
        "the Companion node could not append to the pack's journal: {}",
        String::from_utf8_lossy(&appended.stderr)
    );

    let final_state = status_json(&cli(
        &cli_bin,
        &project,
        &data,
        &socket,
        &["status", "--json"],
    ));
    assert!(
        final_state["revision"].as_u64().expect("revision after") > before_revision,
        "the append did not advance the revision"
    );
    // Both phases present: the append added history rather than replacing it.
    assert!(
        final_state["phases"].get("written-by-pack").is_some(),
        "appending destroyed the phase the pack daemon had written"
    );
    assert!(
        final_state["phases"].get("written-by-companion").is_some(),
        "the Companion's own phase is missing after its append"
    );

    let audit_after = cli(&cli_bin, &project, &data, &socket, &["audit", "--json"]);
    assert!(
        audit_after.status.success() && audit_after.stdout.len() >= audit_before_len,
        "the signed audit export shrank across the handover: {} bytes before, {} after",
        audit_before_len,
        audit_after.stdout.len()
    );
}
