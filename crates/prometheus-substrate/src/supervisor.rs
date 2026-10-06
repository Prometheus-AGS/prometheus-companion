//! Adopting and supervising the pack's installed services.
//!
//! # What "adopt" means here (D-02)
//!
//! The pack's services stay exactly where the pack installed them and keep
//! running under `launchctl`/`systemd --user`. This module never starts,
//! stops, or relocates a service on its own initiative — it reads the
//! generated manifest ([`crate::services`]), probes each entry, and restarts
//! one **through the platform supervisor** when it is found down. "Adopt" is
//! observation plus a restart trigger, never ownership.
//!
//! # Two probe sources, because most services have no endpoint
//!
//! Measured against the installed manifest (`change-cpc-007` task 1): 2 of 12
//! services have a `binding`; the other 10 — six daemons, three timers, one
//! one-shot — do not. An endpoint-only prober would report ten services `Down`
//! forever. So a service with a binding is probed on it; every other service
//! is probed through `launchctl print gui/<uid>/<label>`, which reports
//! `state`, `pid`, and `last exit code` for any label regardless of whether it
//! has a port. Confirmed directly against `ai.prometheus.surface-bridge` (no
//! binding, `state = running`) before writing this.
//!
//! # Named failure scenarios for every defensive branch (companion AGENTS.md §0.2)
//!
//! - **Restart of an unregistered label.** `launchctl kickstart -k
//!   gui/<uid>/ai.prometheus.does-not-exist` exits 113 with `Could not find
//!   service "..." in domain for user gui: 501` — confirmed on this machine
//!   before writing the retry path. A manifest entry whose plist was never
//!   loaded (skipped at install, or removed by an operator) must not retry
//!   forever against a label that can never succeed.
//! - **A flapping service.** `change-cpc-005`'s continuity work established
//!   that a process can be observably alive yet not the one that should be
//!   answering (wrong data root). A restart loop with no backoff and no cap
//!   would spin CPU and hide the real problem behind constant log noise.
//! - **An idle timer probed like a daemon.** `health.rs` (task 1) exists
//!   because this was nearly gotten wrong — see that module's header.

use std::collections::HashMap;
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;
use tokio::time::interval;

use crate::health::{Aggregator, Probe, ServiceKind, ServiceReport, TrayState};
use crate::services::{self, Binding, ManifestService};

/// Backoff floor. A restart attempt sooner than this cannot possibly reflect a
/// new observation — the previous attempt has not had time to take effect.
const BACKOFF_FLOOR: Duration = Duration::from_secs(1);

/// Backoff ceiling. Bounded so a permanently broken service still gets
/// attention within a minute rather than trailing off into silence.
const BACKOFF_CEILING: Duration = Duration::from_secs(60);

/// Consecutive restart failures before a service is marked disabled and no
/// longer retried automatically. Matches the spec's stated budget
/// (`change-cpc-007`): an operator must be able to trust that "adopted" does
/// not mean "the supervisor is silently retrying a lost cause every 60s
/// forever."
const MAX_CONSECUTIVE_FAILURES: u32 = 10;

/// How often the supervisor polls every adopted service.
const PROBE_INTERVAL: Duration = Duration::from_secs(5);

/// One adopted service's restart bookkeeping. Not the same struct as
/// [`crate::health::ServiceHealth`] — that owns *fold* state (down-since, for
/// the grace window); this owns *retry* state (backoff, failure count). They
/// answer different questions and conflating them would make either one hard
/// to reason about alone.
#[derive(Debug, Clone)]
struct RestartState {
    consecutive_failures: u32,
    next_attempt_not_before: Instant,
    disabled: bool,
}

impl RestartState {
    fn fresh(now: Instant) -> Self {
        Self {
            consecutive_failures: 0,
            next_attempt_not_before: now,
            disabled: false,
        }
    }

    fn backoff(&self) -> Duration {
        // 1s, 2s, 4s, 8s, ... capped at BACKOFF_CEILING. `min` before the
        // shift avoids the shift amount itself overflowing on a very large
        // failure count, though MAX_CONSECUTIVE_FAILURES never lets it get
        // there in practice.
        let exponent = self.consecutive_failures.min(6);
        let scaled = BACKOFF_FLOOR.saturating_mul(1u32 << exponent);
        scaled.min(BACKOFF_CEILING)
    }

    fn record_failure(&mut self, now: Instant, label: &str) {
        self.consecutive_failures += 1;
        if self.consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
            self.disabled = true;
            tracing::warn!(
                %label,
                consecutive_failures = self.consecutive_failures,
                "service disabled after repeated restart failures — named failure: \
                 launchctl kickstart exits 113 for a label whose plist was never loaded, \
                 and retrying that forever would spin CPU on an unfixable condition"
            );
        }
        self.next_attempt_not_before = now + self.backoff();
    }

    fn record_recovery(&mut self) {
        self.consecutive_failures = 0;
        self.disabled = false;
    }
}

/// What platform actually runs the restart. A trait so the retry/backoff logic
/// above is testable without invoking real `launchctl`/`systemctl` processes —
/// the fixture in this module's tests exercises the "unregistered label" and
/// "flapping service" scenarios named above without process spawns racing a
/// real supervisor.
trait PlatformSupervisor: Send + Sync {
    /// Ask the platform for `state`, `pid`, and `last exit code` for `label`.
    /// `None` means the platform reports the label is not loaded at all —
    /// distinct from "loaded but not running."
    fn probe(&self, label: &str) -> Option<SupervisorObservation>;

    /// Restart `label` through the platform supervisor. `Ok(())` means the
    /// supervisor *accepted* the restart request, not that the service is
    /// healthy afterward — the next probe answers that.
    fn restart(&self, label: &str) -> Result<(), String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SupervisorObservation {
    running: bool,
    last_exit_code: Option<i32>,
}

struct LaunchctlSupervisor {
    uid: u32,
}

impl LaunchctlSupervisor {
    fn current_user() -> Self {
        // SAFETY-equivalent note: `libc` is not a dependency here, so shell out
        // to `id -u` rather than add one for a single integer. `launchctl`
        // itself is already a process spawn; this is not a new class of cost.
        let uid = Command::new("id")
            .arg("-u")
            .output()
            .ok()
            .and_then(|out| String::from_utf8(out.stdout).ok())
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0);
        Self { uid }
    }
}

impl PlatformSupervisor for LaunchctlSupervisor {
    fn probe(&self, label: &str) -> Option<SupervisorObservation> {
        let output = Command::new("launchctl")
            .args(["print", &format!("gui/{}/{label}", self.uid)])
            .output()
            .ok()?;
        if !output.status.success() {
            // Confirmed failure text on this machine: "Could not find service
            // ... in domain for user gui: 501" with exit 113 — the label is
            // not loaded, not merely stopped.
            return None;
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let running = text
            .lines()
            .any(|line| line.trim_start().starts_with("state = running"));
        let last_exit_code = text.lines().find_map(|line| {
            line.trim_start()
                .strip_prefix("last exit code = ")
                .and_then(|value| value.trim().parse().ok())
        });
        Some(SupervisorObservation {
            running,
            last_exit_code,
        })
    }

    fn restart(&self, label: &str) -> Result<(), String> {
        let output = Command::new("launchctl")
            .args(["kickstart", "-k", &format!("gui/{}/{label}", self.uid)])
            .output()
            .map_err(|error| format!("could not spawn launchctl: {error}"))?;
        if output.status.success() {
            return Ok(());
        }
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

/// Whether a service can be probed on its own endpoint. Only TCP is checked —
/// the two bound services in the current manifest are both TCP
/// (`forge-mcp`, `surrealdb-native`); a Unix-socket binding has no named
/// failure yet, so it is not handled speculatively (§0.2).
fn probe_endpoint(binding: &Binding) -> Option<bool> {
    match binding {
        Binding::Tcp { value } => {
            let addr = format!("127.0.0.1:{value}");
            Some(
                std::net::TcpStream::connect_timeout(
                    &addr.parse().ok()?,
                    Duration::from_millis(500),
                )
                .is_ok(),
            )
        }
        Binding::Socket { .. } => None,
    }
}

/// Owns adoption, probing, and restart for every manifest service.
///
/// Implements [`sovereign_sync::rest_api::ServicesProbe`] so the node can
/// attach it to `AppState` without `sovereign-sync` ever naming this type —
/// see that trait's doc comment for why the dependency runs only one way.
pub struct Supervisor {
    aggregator: Arc<RwLock<Aggregator>>,
    restart_state: Arc<RwLock<HashMap<String, RestartState>>>,
    services: Vec<ManifestService>,
    platform: Arc<dyn PlatformSupervisor>,
}

impl Supervisor {
    /// Build a supervisor from the manifest at `pack_root`. Fails closed if
    /// the manifest cannot be read — an empty supervisor that silently adopts
    /// nothing would look identical to "everything is fine" on the API surface.
    pub fn from_pack_root(pack_root: &std::path::Path) -> anyhow::Result<Self> {
        let services = services::read_manifest(pack_root)?;
        Ok(Self::new(
            services,
            Arc::new(LaunchctlSupervisor::current_user()),
        ))
    }

    fn new(services: Vec<ManifestService>, platform: Arc<dyn PlatformSupervisor>) -> Self {
        Self::new_with_grace(services, platform, Duration::from_secs(30))
    }

    /// Same as [`Self::new`] with an explicit grace window. Test-only entry
    /// point: the real 30s grace window (matching `health.rs`'s aggregator
    /// default) cannot be fast-forwarded, since `down_since` is stamped from
    /// real `Instant::now()`. Without this, a test proving the restart path
    /// would have to sleep 30 real seconds per case — slow, and exactly the
    /// shape of test that quietly degrades into "trust the sleep" instead of
    /// "prove the transition."
    fn new_with_grace(
        services: Vec<ManifestService>,
        platform: Arc<dyn PlatformSupervisor>,
        grace: Duration,
    ) -> Self {
        let mut aggregator = Aggregator::new(grace);
        let now = Instant::now();
        let mut restart_state = HashMap::new();
        for service in &services {
            let kind = if services::is_timer(service) {
                ServiceKind::Timer
            } else {
                ServiceKind::Daemon
            };
            // Every manifest entry is treated as required: the manifest is
            // generated only from services the pack actually installs, so
            // there is no "optional" tier to distinguish yet (§0.2 — no
            // unobserved distinction).
            aggregator.register(service.label.clone(), true, kind);
            restart_state.insert(service.label.clone(), RestartState::fresh(now));
        }
        Self {
            aggregator: Arc::new(RwLock::new(aggregator)),
            restart_state: Arc::new(RwLock::new(restart_state)),
            services,
            platform,
        }
    }

    /// Subscribe to the folded [`TrayState`]. The tray (`change-cpc-007`
    /// task 4) holds one of these; the node grabs it before handing this
    /// supervisor's ownership to `AppState::with_services`, since after that
    /// the tray has no other way to reach the aggregator inside.
    pub async fn subscribe(&self) -> tokio::sync::watch::Receiver<TrayState> {
        self.aggregator.read().await.subscribe()
    }

    /// Probe every adopted service once and report the results into the
    /// aggregator. Public so a caller (or a test) can drive one pass without
    /// waiting on [`Self::run`]'s interval.
    pub async fn probe_once(&self) {
        let now = Instant::now();
        for service in &self.services {
            let probe = self.probe_service(service);
            self.aggregator
                .write()
                .await
                .report(&service.label, probe, now);
        }
    }

    fn probe_service(&self, service: &ManifestService) -> Probe {
        if let Some(binding) = &service.binding {
            if let Some(reachable) = probe_endpoint(binding) {
                return if reachable { Probe::READY } else { Probe::DOWN };
            }
        }
        match self.platform.probe(&service.label) {
            None => Probe::DOWN,
            Some(observation) if services::is_timer(service) => {
                match (observation.running, observation.last_exit_code) {
                    (true, _) => Probe::RUNNING,
                    (false, Some(0)) | (false, None) => Probe::IDLE_OK,
                    (false, Some(_)) => Probe::DOWN,
                }
            }
            Some(observation) if observation.running => Probe::RUNNING,
            Some(_) => Probe::DOWN,
        }
    }

    /// One supervision pass: probe everything, then restart whatever the
    /// aggregator now folds to [`TrayState::Down`] and whose backoff window
    /// has elapsed.
    pub async fn tick(&self) {
        self.probe_once().await;
        let now = Instant::now();
        let reports: Vec<ServiceReport> = self.aggregator.read().await.reports(now);
        for report in reports {
            if report.state != TrayState::Down {
                continue;
            }
            let mut states = self.restart_state.write().await;
            let Some(state) = states.get_mut(&report.label) else {
                continue;
            };
            if state.disabled || now < state.next_attempt_not_before {
                continue;
            }
            match self.platform.restart(&report.label) {
                Ok(()) => {
                    tracing::info!(label = %report.label, "restart requested");
                    // Recovery is confirmed by the *next* probe finding it
                    // Up, not by this call succeeding — kickstart accepting
                    // the request is not evidence the process is healthy.
                }
                Err(reason) => {
                    tracing::warn!(label = %report.label, %reason, "restart failed");
                    state.record_failure(now, &report.label);
                }
            }
        }
    }

    /// Run supervision forever at [`PROBE_INTERVAL`]. Spawned once from the
    /// node's startup sequence; dropping the returned `JoinHandle` (or the
    /// runtime) stops it.
    pub fn spawn(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut ticker = interval(PROBE_INTERVAL);
            loop {
                ticker.tick().await;
                self.tick().await;
                // A service that recovered on its own (not via our restart)
                // must still clear its failure count, or a later transient
                // blip would inherit a stale backoff from an unrelated past
                // failure.
                let now = Instant::now();
                let reports = self.aggregator.read().await.reports(now);
                let mut states = self.restart_state.write().await;
                for report in &reports {
                    if report.state == TrayState::Healthy || report.state == TrayState::Degraded {
                        if let Some(state) = states.get_mut(&report.label) {
                            if state.consecutive_failures > 0 && !state.disabled {
                                state.record_recovery();
                            }
                        }
                    }
                }
            }
        })
    }
}

impl sovereign_sync::rest_api::ServicesProbe for Supervisor {
    fn service_reports(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Vec<sovereign_sync::rest_api::ServiceStatus>>
                + Send
                + '_,
        >,
    > {
        Box::pin(async move {
            let now = Instant::now();
            self.aggregator
                .read()
                .await
                .reports(now)
                .into_iter()
                .map(|report| sovereign_sync::rest_api::ServiceStatus {
                    label: report.label,
                    required: report.required,
                    kind: format!("{:?}", report.kind).to_lowercase(),
                    state: report.state.as_str().to_string(),
                    liveness: format!("{:?}", report.liveness).to_lowercase(),
                    readiness: format!("{:?}", report.readiness).to_lowercase(),
                    source: format!("{:?}", report.source).to_lowercase(),
                })
                .collect()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex as StdMutex;

    /// A platform double that returns scripted observations and records
    /// restart calls, so the retry/backoff logic is testable without shelling
    /// out to a real `launchctl`.
    struct FakePlatform {
        observations: StdMutex<HashMap<String, Option<SupervisorObservation>>>,
        restart_calls: StdMutex<Vec<String>>,
        restart_result: StdMutex<Result<(), String>>,
    }

    impl FakePlatform {
        fn new() -> Self {
            Self {
                observations: StdMutex::new(HashMap::new()),
                restart_calls: StdMutex::new(Vec::new()),
                restart_result: StdMutex::new(Ok(())),
            }
        }

        fn set(&self, label: &str, observation: Option<SupervisorObservation>) {
            self.observations
                .lock()
                .unwrap()
                .insert(label.to_string(), observation);
        }

        fn fail_restarts_with(&self, reason: &str) {
            *self.restart_result.lock().unwrap() = Err(reason.to_string());
        }
    }

    impl PlatformSupervisor for FakePlatform {
        fn probe(&self, label: &str) -> Option<SupervisorObservation> {
            self.observations
                .lock()
                .unwrap()
                .get(label)
                .copied()
                .flatten()
        }

        fn restart(&self, label: &str) -> Result<(), String> {
            self.restart_calls.lock().unwrap().push(label.to_string());
            self.restart_result.lock().unwrap().clone()
        }
    }

    fn manifest_service(label: &str, is_timer: bool) -> ManifestService {
        let json = if is_timer {
            format!(
                r#"{{"label":"{label}","binding":null,"platforms":{{"darwin":{{"schedule":{{"runAtLoad":true,"startInterval":3600}},"restart":null}}}}}}"#
            )
        } else {
            format!(
                r#"{{"label":"{label}","binding":null,"platforms":{{"darwin":{{"schedule":{{"runAtLoad":true,"startInterval":null}},"restart":null}}}}}}"#
            )
        };
        serde_json::from_str(&json).expect("manifest fixture parses")
    }

    /// Within the grace window, a down service is reported Degraded, not Down,
    /// and must not be restarted yet — a restart must not fire on the very
    /// first probe that finds a service unavailable.
    #[tokio::test]
    async fn a_down_daemon_within_grace_is_not_yet_restarted() {
        let platform = Arc::new(FakePlatform::new());
        platform.set(
            "api",
            Some(SupervisorObservation {
                running: false,
                last_exit_code: Some(1),
            }),
        );
        let supervisor = Supervisor::new_with_grace(
            vec![manifest_service("api", false)],
            platform.clone(),
            Duration::from_secs(30),
        );

        supervisor.tick().await;
        let reports = supervisor.aggregator.read().await.reports(Instant::now());
        assert_eq!(
            reports[0].state,
            TrayState::Degraded,
            "must be Degraded, not Down, on the very first failed probe — proves the \
             assertion below (no restart) is because of grace, not because Down was never reached"
        );
        assert!(
            platform.restart_calls.lock().unwrap().is_empty(),
            "must not restart before the grace window elapses"
        );
    }

    /// The scenario named in this module's header: a service down long enough
    /// to cross the grace window gets a restart request.
    #[tokio::test]
    async fn a_down_daemon_past_grace_gets_restarted() {
        let platform = Arc::new(FakePlatform::new());
        platform.set(
            "api",
            Some(SupervisorObservation {
                running: false,
                last_exit_code: Some(1),
            }),
        );
        // Zero grace: the first probe's Down state is immediately eligible,
        // so this test proves the restart trigger without a real sleep.
        let supervisor = Supervisor::new_with_grace(
            vec![manifest_service("api", false)],
            platform.clone(),
            Duration::ZERO,
        );

        supervisor.tick().await;
        let reports = supervisor.aggregator.read().await.reports(Instant::now());
        assert_eq!(
            reports[0].state,
            TrayState::Down,
            "zero grace must reach Down immediately"
        );
        assert_eq!(
            platform.restart_calls.lock().unwrap().as_slice(),
            &["api".to_string()],
            "a service Down past its grace window must be restarted"
        );
    }

    /// The unregistered-label scenario from this module's header: a restart
    /// that always fails must stop retrying at MAX_CONSECUTIVE_FAILURES,
    /// never loop forever.
    #[tokio::test]
    async fn repeated_restart_failure_disables_the_service_rather_than_retrying_forever() {
        let platform = Arc::new(FakePlatform::new());
        platform.set(
            "ghost",
            Some(SupervisorObservation {
                running: false,
                last_exit_code: None,
            }),
        );
        // The named failure this scenario models: launchctl kickstart on an
        // unregistered label — confirmed verbatim on this machine before
        // writing the restart path.
        platform.fail_restarts_with(
            "Could not find service \"ai.prometheus.ghost\" in domain for user gui: 501",
        );
        // Zero grace: every tick's probe is immediately Down-eligible, so each
        // of the MAX_CONSECUTIVE_FAILURES ticks below drives exactly one
        // restart attempt, deterministically, with no real sleeping.
        let supervisor = Supervisor::new_with_grace(
            vec![manifest_service("ghost", false)],
            platform.clone(),
            Duration::ZERO,
        );

        for _ in 0..MAX_CONSECUTIVE_FAILURES {
            // Backoff between attempts still applies at zero grace, so clear
            // it before each tick — this test is about the failure-count cap,
            // not the backoff timer, which has its own test below.
            {
                let mut states = supervisor.restart_state.write().await;
                if let Some(state) = states.get_mut("ghost") {
                    state.next_attempt_not_before = Instant::now();
                }
            }
            supervisor.tick().await;
        }

        {
            // Scoped: this read guard must be released before the write
            // below acquires the same lock, or the write blocks forever on a
            // reader that never drops — a single-threaded runtime cannot make
            // progress on either after that, since nothing else can run to
            // release it. That was this test's actual bug, found by sampling
            // the hung process's stack (parked in the tokio time driver with
            // no test code running) rather than assumed.
            let states = supervisor.restart_state.read().await;
            let ghost = states.get("ghost").expect("ghost tracked");
            assert!(
                ghost.disabled,
                "a service whose restart can never succeed must stop being retried"
            );
        }
        {
            let calls = platform.restart_calls.lock().unwrap();
            assert_eq!(
                calls.len(),
                MAX_CONSECUTIVE_FAILURES as usize,
                "must attempt exactly the failure cap, not fewer (proves each tick actually \
                 attempted) and not more (proves the cap is enforced): {calls:?}"
            );
        }

        // And the cap must actually stop it: one more eligible tick must not
        // add a further restart call.
        {
            let mut states = supervisor.restart_state.write().await;
            if let Some(state) = states.get_mut("ghost") {
                state.next_attempt_not_before = Instant::now();
            }
        }
        supervisor.tick().await;
        assert_eq!(
            platform.restart_calls.lock().unwrap().len(),
            MAX_CONSECUTIVE_FAILURES as usize,
            "a disabled service must not be retried even when its backoff window is clear"
        );
    }

    /// A flapping service (recovers, then fails again) must not accumulate
    /// backoff across unrelated failures — recovery resets the count.
    #[tokio::test]
    async fn recovery_resets_the_backoff_counter() {
        let mut state = RestartState::fresh(Instant::now());
        state.record_failure(Instant::now(), "flapper");
        state.record_failure(Instant::now(), "flapper");
        assert_eq!(state.consecutive_failures, 2);
        state.record_recovery();
        assert_eq!(state.consecutive_failures, 0);
        assert!(!state.disabled);
    }

    /// Backoff must actually grow, and must be bounded — otherwise the
    /// ceiling constant is decorative.
    #[test]
    fn backoff_grows_and_is_capped() {
        let mut state = RestartState::fresh(Instant::now());
        let mut previous = Duration::from_secs(0);
        for _ in 0..3 {
            state.record_failure(Instant::now(), "svc");
            assert!(state.backoff() > previous, "backoff must increase");
            previous = state.backoff();
        }
        for _ in 0..10 {
            state.record_failure(Instant::now(), "svc");
        }
        assert!(state.backoff() <= BACKOFF_CEILING);
    }

    /// An idle timer must be probed as healthy, not as a down daemon — the
    /// same distinction health.rs tests, exercised here through the
    /// supervisor's actual probe dispatch rather than the aggregator alone.
    #[test]
    fn probe_dispatch_treats_an_idle_timer_as_healthy() {
        let platform = Arc::new(FakePlatform::new());
        platform.set(
            "nudge",
            Some(SupervisorObservation {
                running: false,
                last_exit_code: Some(0),
            }),
        );
        let supervisor = Supervisor::new(vec![manifest_service("nudge", true)], platform);
        let probe = supervisor.probe_service(&manifest_service("nudge", true));
        assert_eq!(probe, Probe::IDLE_OK);
    }

    /// The companion of the test above: a daemon with the identical
    /// observation (not running, no exit code) IS down. Proves the dispatch
    /// branches on `is_timer`, not on the observation shape alone.
    #[test]
    fn probe_dispatch_treats_a_stopped_daemon_as_down() {
        let platform = Arc::new(FakePlatform::new());
        platform.set(
            "api",
            Some(SupervisorObservation {
                running: false,
                last_exit_code: None,
            }),
        );
        let supervisor = Supervisor::new(vec![manifest_service("api", false)], platform);
        let probe = supervisor.probe_service(&manifest_service("api", false));
        assert_eq!(probe, Probe::DOWN);
    }

    /// A label the platform does not recognize at all (no plist loaded) must
    /// probe as Down, not panic or silently skip — this is the state that
    /// leads to the unregistered-label restart failure this module names.
    #[test]
    fn an_unrecognized_label_probes_as_down() {
        let platform = Arc::new(FakePlatform::new());
        // Deliberately not calling platform.set — None is the default,
        // matching launchctl's "Could not find service" case.
        let supervisor = Supervisor::new(vec![manifest_service("ghost", false)], platform);
        let probe = supervisor.probe_service(&manifest_service("ghost", false));
        assert_eq!(probe, Probe::DOWN);
    }
}
