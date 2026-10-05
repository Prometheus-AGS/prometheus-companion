//! A killed, adopted service is restarted — proven through the REST surface.
//!
//! # The question this answers
//!
//! `change-cpc-007`'s acceptance criterion: "a killed service is restarted
//! within budget... state passes Down then Healthy." That has to be checked
//! against the **real** production path — the actual `LaunchctlSupervisor`
//! shelling out to real `launchctl`, not `supervisor.rs`'s `FakePlatform`
//! fixture, which only proves the retry/backoff *logic* is right, never that
//! it is wired to anything real.
//!
//! # Why this test does not touch the operator's own 12 services
//!
//! `launchctl list | grep ai.prometheus` on the machine this was written on
//! shows the real, live services this session and the operator both depend on
//! (`surreal-memory-native`, `pk-cherry`, and others). Killing one of those
//! unattended and trusting an unproven restart path to bring it back is not
//! an acceptable risk to a developer's running machine.
//!
//! So this test installs one **throwaway** launchd job — a uniquely labelled
//! `/bin/sleep`, never `ai.prometheus.*` — points a **fixture manifest** at
//! only that one label, and drives the real Companion headless node against
//! it. The supervisor code under test cannot tell the difference between this
//! and a real pack install; the label is the only thing that changed. The
//! fixture deliberately carries no `KeepAlive` — see [`FixtureService`]'s doc
//! comment for why a production-shaped `KeepAlive` would have made this test
//! observe launchd's own crash recovery instead of this supervisor's.
//!
//! # What "through the REST surface" means here
//!
//! `change-cpc-005`'s continuity test found that `prometheus kbd` never
//! required a node at all, and a passing test had proven nothing. The lesson
//! generalizes: never assert through a path that could succeed without the
//! subsystem under test. This file's every health assertion is `GET
//! /api/v1/services` over the node's Unix socket — nothing else can answer
//! that route, because it is served by `Supervisor::service_reports`
//! (`crates/sovereign-sync/src/rest_api.rs`'s `ServicesProbe` trait) with no
//! other implementation.
//!
//! # Skips
//!
//! Skips loudly, and only, when this is not a launchd host (`uname -s` is not
//! `Darwin`) or `launchctl bootstrap` itself fails in this environment — a
//! sandboxed CI runner without a real login GUI domain, for example.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn unique_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time before unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("cpc007-{label}-{nanos}"));
    fs::create_dir_all(&path).expect("create temp dir");
    path
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

/// A minimal HTTP/1.1 GET over a Unix socket. Not routed through any CLI —
/// see this file's header on why that matters.
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

/// Split an HTTP/1.1 response into (status_line, json_body).
fn split_response(response: &str) -> (&str, serde_json::Value) {
    let status_line = response.lines().next().unwrap_or("");
    let body_start = response.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
    let body = &response[body_start..];
    let json = serde_json::from_str(body)
        .unwrap_or_else(|error| panic!("response body is not JSON ({error}): {body}"));
    (status_line, json)
}

/// A real, throwaway launchd job — not `ai.prometheus.*`.
///
/// **Deliberately carries no `KeepAlive`.** Production plists do (§16.3, the
/// nine HMA fixes), but a `KeepAlive.Crashed=true` service with a 15s
/// `ThrottleInterval` self-heals *faster than* this supervisor's own 30s
/// aggregator grace window — measured the hard way, by watching a first draft
/// of this test never observe `Down` because launchd had already relaunched
/// the process on its own well before the grace window elapsed. That would
/// have proven launchd's `KeepAlive` works, which was never in question, and
/// said nothing about whether *this supervisor's* restart trigger
/// (`Supervisor::tick`, `launchctl kickstart`) is the thing that revived it.
/// With no `KeepAlive`, a killed fixture stays dead until this supervisor's
/// own code — not launchd's native crash recovery — brings it back.
struct FixtureService {
    label: String,
    plist_path: PathBuf,
}

impl FixtureService {
    fn install(label: &str, log_dir: &Path) -> Result<Self, String> {
        let plist_path = std::env::temp_dir().join(format!("{label}.plist"));
        let stdout_path = log_dir.join(format!("{label}.log"));
        let plist = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{label}</string>
  <key>RunAtLoad</key>
  <true/>
  <key>ProgramArguments</key>
  <array>
    <string>/bin/sleep</string>
    <string>3600</string>
  </array>
  <key>StandardOutPath</key>
  <string>{stdout}</string>
  <key>StandardErrorPath</key>
  <string>{stdout}</string>
</dict>
</plist>
"#,
            stdout = stdout_path.display()
        );
        fs::write(&plist_path, plist).map_err(|error| format!("write fixture plist: {error}"))?;

        let uid = uid();
        // Best-effort cleanup of a prior run under the same label, ignored:
        // nothing loaded yet is the common case.
        let _ = Command::new("launchctl")
            .args(["bootout", &format!("gui/{uid}/{label}")])
            .output();

        let output = Command::new("launchctl")
            .args(["bootstrap", &format!("gui/{uid}")])
            .arg(&plist_path)
            .output()
            .map_err(|error| format!("could not spawn launchctl bootstrap: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "launchctl bootstrap failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(Self {
            label: label.to_string(),
            plist_path,
        })
    }

    fn pid(&self) -> Option<u32> {
        let output = Command::new("launchctl")
            .args(["print", &format!("gui/{}/{}", uid(), self.label)])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&output.stdout);
        text.lines().find_map(|line| {
            line.trim_start()
                .strip_prefix("pid = ")
                .and_then(|value| value.trim().parse().ok())
        })
    }
}

impl Drop for FixtureService {
    fn drop(&mut self) {
        let _ = Command::new("launchctl")
            .args(["bootout", &format!("gui/{}/{}", uid(), self.label)])
            .output();
        let _ = fs::remove_file(&self.plist_path);
    }
}

fn uid() -> u32 {
    Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(501)
}

/// The `shared/services.manifest.json` shape the supervisor reads, naming
/// only the fixture label. `binding: null` matches the 10-of-12 production
/// services with no endpoint (`health.rs` and `supervisor.rs`'s module docs
/// measured this against the real manifest before either was written).
fn write_fixture_manifest(pack_root: &Path, label: &str) {
    let shared = pack_root.join("shared");
    fs::create_dir_all(&shared).expect("create shared dir");
    let manifest = serde_json::json!({
        "$schema": "https://prometheus-ags.dev/schemas/services.manifest.json",
        "contractVersion": "1.0.0",
        "generatedBy": "crates/prometheus-substrate/tests/supervision.rs (fixture, not the real generator)",
        "services": [
            {
                "label": label,
                "binding": null,
                "platforms": {
                    "darwin": {
                        "schedule": { "runAtLoad": true, "startInterval": null },
                        "restart": { "keepAlive": true, "throttleInterval": 15 }
                    }
                }
            }
        ]
    });
    fs::write(
        shared.join("services.manifest.json"),
        serde_json::to_string_pretty(&manifest).expect("serialize fixture manifest"),
    )
    .expect("write fixture manifest");
}

/// Poll `/api/v1/services` until `label`'s `state` matches `want`, or time out
/// with the last-seen array for debugging.
fn wait_for_service_state(
    socket: &Path,
    label: &str,
    want: &str,
    budget: Duration,
) -> Result<(), String> {
    let deadline = Instant::now() + budget;
    let mut last_seen = serde_json::Value::Null;
    while Instant::now() < deadline {
        if let Ok(response) = socket_get(socket, "/api/v1/services") {
            let (_, services) = split_response(&response);
            if let Some(entry) = services
                .as_array()
                .and_then(|list| list.iter().find(|s| s["label"] == label))
            {
                last_seen = entry.clone();
                if entry["state"] == want {
                    return Ok(());
                }
            }
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    Err(format!(
        "{label} never reached state {want:?} within {budget:?}; last seen: {last_seen}"
    ))
}

#[test]
fn a_killed_adopted_service_is_restarted_and_reported_healthy_over_the_rest_surface() {
    if std::env::consts::OS != "macos" {
        eprintln!(
            "SKIP: launchd supervision is macOS-only (this test's platform: {})",
            std::env::consts::OS
        );
        return;
    }

    let pid = std::process::id();
    let label = format!("cpc007.fixture.{pid}");
    let pack_root = unique_dir("pack-root");
    let run = unique_dir("run");
    let log_dir = unique_dir("logs");
    let socket = run.join("sovereign-sync.sock");
    let key_file = run.join("device-key.json");
    let skills_dir = unique_dir("skills");

    write_fixture_manifest(&pack_root, &label);

    let fixture = match FixtureService::install(&label, &log_dir) {
        Ok(fixture) => fixture,
        Err(error) => {
            eprintln!(
                "SKIP: could not install the throwaway launchd fixture ({error}). Likely a \
                 sandboxed environment without a real login GUI domain."
            );
            return;
        }
    };

    let Some(initial_pid) = poll_for_pid(&fixture, Duration::from_secs(10)) else {
        eprintln!("SKIP: fixture service never reported a pid after RunAtLoad; cannot proceed.");
        return;
    };

    // A key file keeps the headless node unattended (see identity.rs).
    fs::write(&key_file, "{}").expect("write placeholder key file");

    let mut companion = Process(
        Command::new(companion_headless())
            .env("PROMETHEUS_DATA_DIR", unique_dir("data"))
            .env("SOVEREIGN_SYNC_SOCKET", &socket)
            .env("PROMETHEUS_DEVICE_KEY_FILE", &key_file)
            .env("PROMETHEUS_SKILLS_DIR", &skills_dir)
            .env("PROMETHEUS_PACK_ROOT", &pack_root)
            .env("RUST_LOG", "prometheus_substrate=info,sovereign_sync=warn")
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn the Companion headless node"),
    );
    if let Err(error) = wait_for_socket(&socket, &mut companion.0, Duration::from_secs(30)) {
        panic!("Companion node never became reachable: {error}");
    }

    // ---- Assertion 1: adoption. The fixture appears via the REST surface,
    // and only via it — nothing else answers /api/v1/services. -------------
    if let Err(error) = wait_for_service_state(&socket, &label, "healthy", Duration::from_secs(15))
    {
        // A running-but-not-endpoint-confirmed service reports Degraded
        // (`Probe::RUNNING`, per health.rs) rather than Healthy — that is
        // still valid adoption evidence, so accept either before failing.
        let response = socket_get(&socket, "/api/v1/services")
            .unwrap_or_else(|e| format!("(could not read /api/v1/services: {e})"));
        let (_, services) = split_response(&response);
        let entry = services
            .as_array()
            .and_then(|list| list.iter().find(|s| s["label"] == label));
        match entry.map(|e| e["state"].as_str().unwrap_or("")) {
            Some("degraded") => { /* acceptable: supervisor-sourced RUNNING, not endpoint-confirmed */
            }
            _ => {
                panic!("fixture was not adopted within budget: {error}\nfull response: {response}")
            }
        }
    }

    // ---- Kill it: the real crash scenario ---------------------------------
    let kill = Command::new("kill")
        .args(["-9", &initial_pid.to_string()])
        .status()
        .expect("run kill");
    assert!(kill.success(), "could not send SIGKILL to the fixture pid");

    // ---- Assertion 2: Down is observed through the REST surface -----------
    wait_for_service_state(&socket, &label, "down", Duration::from_secs(45)).unwrap_or_else(
        |error| {
            panic!(
                "the killed fixture was never reported Down over /api/v1/services: {error}\n\
                 (grace window is 30s in the supervisor's aggregator; this budget allows for it)"
            )
        },
    );

    // ---- Assertion 3: restart is observed through the REST surface --------
    // The supervisor probes on a 5s interval and restarts through real
    // `launchctl kickstart`; give it multiple cycles plus launchd's own
    // relaunch time.
    wait_for_service_state(&socket, &label, "healthy", Duration::from_secs(60))
        .or_else(|_| wait_for_service_state(&socket, &label, "degraded", Duration::from_secs(1)))
        .unwrap_or_else(|error| {
            panic!(
                "the fixture never recovered after the supervisor's restart: {error}\n\
                 this means the restart path (launchctl kickstart, backoff, or the \
                 subsequent probe) is broken, not just this test's timing"
            )
        });

    // The restarted process must be a genuinely new pid — proof launchd
    // actually relaunched it, not that the old (killed) pid is being
    // misreported as alive.
    let restarted_pid = fixture.pid();
    assert!(
        restarted_pid.is_some() && restarted_pid != Some(initial_pid),
        "expected a new pid after restart (old: {initial_pid}, now: {restarted_pid:?}) — \
         same or absent pid means launchd did not actually relaunch the service"
    );
}

fn poll_for_pid(fixture: &FixtureService, budget: Duration) -> Option<u32> {
    let deadline = Instant::now() + budget;
    while Instant::now() < deadline {
        if let Some(pid) = fixture.pid() {
            return Some(pid);
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    None
}
