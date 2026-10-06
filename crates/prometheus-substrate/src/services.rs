//! Reading the pack's generated service manifest (integration contract seam 3).
//!
//! # Where the manifest lives, and the gap this module works around
//!
//! `shared/services.manifest.json` is generated at the pack's repository root
//! (`docs/integration-contract.md` §Seam 3). Every existing reader of it is a
//! **CLI invocation** run from inside that repository — `prometheus contract
//! show` defaults its `--path` to `.` (`prometheus-cli/src/main.rs`) — so
//! nothing in the pack has ever needed to answer "where is the pack root" from
//! a *different*, long-running process.
//!
//! The Companion is exactly that: a daemon with no reason to run with the pack
//! as its working directory. Checked before writing this: the pack's installer
//! (`scripts/prometheus-services.sh`) substitutes `__PROMETHEUS_ROOT__` into
//! *rendered* launchd plists at install time, but persists that resolved path
//! nowhere a later reader could look it up. There is no existing discovery
//! mechanism this module can lean on — this is a real gap in contract v1.0.0,
//! not a detail missed here.
//!
//! So this fails closed on a required override rather than guess: a directory
//! walk for something literally named `prometheus-skill-pack` would work on
//! this machine and be wrong the moment a pack checkout uses a different
//! directory name, which is worse than an honest error naming the fix. Report
//! this gap for a future contract revision to close with a persisted answer
//! (a written marker file, or a value the installer hands the Companion at
//! setup) — an env var is the interim, not the intended final shape.

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Resolve the installed pack's root directory.
///
/// `PROMETHEUS_PACK_ROOT` is the only source. Failing closed here, before any
/// caller starts depending on a guessed path, is deliberate: a supervisor that
/// silently found the wrong pack (or no pack) would report every service Down
/// with no indication why.
pub fn pack_root() -> anyhow::Result<PathBuf> {
    std::env::var_os("PROMETHEUS_PACK_ROOT")
        .map(PathBuf::from)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "PROMETHEUS_PACK_ROOT is not set. The Companion supervisor needs the \
                 installed prometheus-skill-pack's root to find shared/services.manifest.json \
                 (integration contract seam 3); set it to that repository's absolute path."
            )
        })
}

/// One platform's restart policy for a service, as the manifest records it.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestartPolicy {
    pub keep_alive: Option<bool>,
    pub throttle_interval: Option<u64>,
}

/// One platform's schedule for a service — present when it is a timer.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Schedule {
    #[serde(default)]
    pub run_at_load: bool,
    pub start_interval: Option<u64>,
}

/// The darwin platform block this supervisor reads. Linux support is out of
/// scope for this change — no named failure calls for it yet, and this crate
/// only ever runs the Companion on macOS today (§0.2: no unobserved code).
#[derive(Debug, Clone, Deserialize)]
pub struct DarwinPlatform {
    #[serde(default)]
    pub schedule: Schedule,
    pub restart: Option<RestartPolicy>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Platforms {
    pub darwin: Option<DarwinPlatform>,
}

/// A TCP or socket binding a service listens on, when the manifest records one.
///
/// Internally tagged on `kind` to match the manifest's actual shape —
/// `{"kind":"tcp","value":8942}` — confirmed against
/// `shared/services.manifest.json`'s `forge-mcp` entry. The default
/// externally-tagged derive (`{"tcp":{"value":8942}}`) does not match this and
/// was caught by a test, not assumed correct.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Binding {
    Tcp { value: u16 },
    Socket { value: String },
}

/// One manifest entry.
#[derive(Debug, Clone, Deserialize)]
pub struct ManifestService {
    pub label: String,
    pub binding: Option<Binding>,
    pub platforms: Platforms,
}

#[derive(Debug, Clone, Deserialize)]
struct ManifestFile {
    #[serde(default)]
    services: Vec<ManifestService>,
}

/// Read and parse the manifest from a pack root.
///
/// Kept separate from [`pack_root`] so the supervisor's tests can point this at
/// a fixture file without setting the environment.
pub fn read_manifest(pack_root: &Path) -> anyhow::Result<Vec<ManifestService>> {
    let path = pack_root.join("shared/services.manifest.json");
    let raw = std::fs::read_to_string(&path)
        .map_err(|error| anyhow::anyhow!("cannot read {}: {error}", path.display()))?;
    let parsed: ManifestFile = serde_json::from_str(&raw)
        .map_err(|error| anyhow::anyhow!("cannot parse {}: {error}", path.display()))?;
    Ok(parsed.services)
}

/// Whether a manifest entry's schedule marks it as a periodic timer rather
/// than a continuously running daemon.
///
/// Measured against the installed manifest before writing the health
/// aggregator (`change-cpc-007` task 1): 3 of 12 services set
/// `startInterval`, and judging them as daemons would report a healthy idle
/// timer as permanently down.
pub fn is_timer(service: &ManifestService) -> bool {
    service
        .platforms
        .darwin
        .as_ref()
        .is_some_and(|darwin| darwin.schedule.start_interval.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_fixture(dir: &std::path::Path, body: &str) {
        let shared = dir.join("shared");
        std::fs::create_dir_all(&shared).expect("create shared dir");
        std::fs::write(shared.join("services.manifest.json"), body).expect("write fixture");
    }

    #[test]
    fn reads_a_daemon_and_a_timer_entry() {
        let dir = tempfile::tempdir().expect("temp dir");
        write_fixture(
            dir.path(),
            r#"{
                "services": [
                    {
                        "label": "ai.prometheus.surface-bridge",
                        "binding": null,
                        "platforms": {
                            "darwin": {
                                "schedule": { "runAtLoad": true, "startInterval": null },
                                "restart": { "keepAlive": true, "throttleInterval": 10 }
                            }
                        }
                    },
                    {
                        "label": "ai.prometheus.prometheus-nudge",
                        "binding": null,
                        "platforms": {
                            "darwin": {
                                "schedule": { "runAtLoad": true, "startInterval": 3600 },
                                "restart": { "keepAlive": null, "throttleInterval": null }
                            }
                        }
                    }
                ]
            }"#,
        );

        let services = read_manifest(dir.path()).expect("parse fixture");
        assert_eq!(services.len(), 2);
        assert!(!is_timer(&services[0]), "surface-bridge is a daemon");
        assert!(is_timer(&services[1]), "prometheus-nudge is a timer");
    }

    #[test]
    fn a_tcp_binding_deserializes() {
        let dir = tempfile::tempdir().expect("temp dir");
        write_fixture(
            dir.path(),
            r#"{
                "services": [
                    {
                        "label": "ai.prometheus.surrealdb-native",
                        "binding": { "kind": "tcp", "value": 28000 },
                        "platforms": { "darwin": null }
                    }
                ]
            }"#,
        );
        let services = read_manifest(dir.path()).expect("parse fixture");
        match &services[0].binding {
            Some(Binding::Tcp { value }) => assert_eq!(*value, 28000),
            other => panic!("expected a TCP binding, got {other:?}"),
        }
    }

    #[test]
    fn a_missing_manifest_is_a_named_error_not_an_empty_list() {
        let dir = tempfile::tempdir().expect("temp dir");
        let error = read_manifest(dir.path()).unwrap_err();
        assert!(
            error.to_string().contains("services.manifest.json"),
            "error should name the missing file: {error}"
        );
    }

    #[test]
    fn pack_root_fails_closed_when_unset() {
        // SAFETY: single-threaded test, restored immediately.
        let previous = std::env::var_os("PROMETHEUS_PACK_ROOT");
        std::env::remove_var("PROMETHEUS_PACK_ROOT");
        let result = pack_root();
        assert!(result.is_err(), "must fail closed rather than guess");
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("PROMETHEUS_PACK_ROOT"));
        if let Some(value) = previous {
            std::env::set_var("PROMETHEUS_PACK_ROOT", value);
        }
    }
}
