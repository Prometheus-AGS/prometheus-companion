//! Health aggregation for the adopted pack services.
//!
//! Ported from HMA `assets/templates/tauri-tray/health-aggregator/`. One owner
//! folds per-service probe reports into a single [`TrayState`] that the tray
//! icon and the `/api/v1/services` route both read. Computing the icon at the
//! call site instead lets the two drift apart the moment either changes.
//!
//! # What the port had to change, and why
//!
//! The template assumes every supervised service can be probed. The pack's
//! generated `services.manifest.json` says otherwise — measured, not assumed:
//!
//! | Services | Binding | Probeable by endpoint |
//! |---|---|---|
//! | 2 of 12 | `tcp` port | yes |
//! | 10 of 12 | `null` | **no** |
//!
//! Six of those ten are long-running daemons with no listening port, three are
//! interval timers, and one is a one-shot. An aggregator that only knew how to
//! connect to an endpoint would report ten services `Down` forever, and the
//! tray would sit red on a perfectly healthy machine — a monitor that is always
//! alarming is one an operator learns to ignore.
//!
//! So liveness has two sources, and [`Probe`] records which one answered:
//!
//! - **Endpoint** — a service with a binding is asked whether it serves.
//! - **Supervisor** — `launchctl print gui/<uid>/<label>` (`systemctl --user
//!   show` on Linux) reports `state`, `pid`, and `last exit code` for any label,
//!   with or without a port. Verified against the installed services.
//!
//! # Timers are not daemons and must not be judged as such
//!
//! An interval timer is *supposed* to be not-running most of the time. Asking
//! "is there a pid" of a timer answers the wrong question: absent means idle,
//! and the real failure is a **non-zero last exit code**. [`ServiceKind`]
//! carries this distinction so the fold cannot silently treat a healthy idle
//! timer as a dead daemon.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use serde::Serialize;
use tokio::sync::watch;

/// What a probe observed about one service.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Liveness {
    Up,
    Down,
}

/// Whether a live service is able to serve.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Readiness {
    Ready,
    Degraded,
}

/// Which observation answered — kept because the two are not equally strong.
///
/// A supervisor saying "pid 4711 is running" proves the process exists, not
/// that it can serve. An endpoint answering proves it serves. Reporting them
/// identically would let a wedged daemon that holds its port look as healthy as
/// one that answers, so the distinction is carried to the API surface and shown
/// to the operator rather than flattened here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProbeSource {
    /// The service answered on its binding.
    Endpoint,
    /// The platform supervisor reported process state for the label.
    Supervisor,
    /// Nothing has reported yet.
    Unobserved,
}

/// How a service is expected to behave when healthy.
///
/// The manifest's schedule fields decide this, not a hand-maintained list:
/// a `startInterval` or `startCalendarInterval` means [`ServiceKind::Timer`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceKind {
    /// Expected to be running continuously. No pid means down.
    Daemon,
    /// Expected to run periodically and exit. No pid means idle, which is
    /// healthy; a non-zero last exit code is the failure.
    Timer,
}

/// A single probe result for one service.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Probe {
    pub liveness: Liveness,
    pub readiness: Readiness,
    pub source: ProbeSource,
}

impl Probe {
    /// Serving, confirmed on its own endpoint.
    pub const READY: Self = Self {
        liveness: Liveness::Up,
        readiness: Readiness::Ready,
        source: ProbeSource::Endpoint,
    };

    /// Running, but not serving correctly.
    pub const DEGRADED: Self = Self {
        liveness: Liveness::Up,
        readiness: Readiness::Degraded,
        source: ProbeSource::Endpoint,
    };

    /// Not running.
    pub const DOWN: Self = Self {
        liveness: Liveness::Down,
        readiness: Readiness::Degraded,
        source: ProbeSource::Unobserved,
    };

    /// The supervisor reports the label is running.
    ///
    /// Deliberately **not** [`Readiness::Ready`]: a live pid is not evidence the
    /// process can serve. For the ten services with no binding this is the best
    /// available observation, and calling it `Ready` would overstate it.
    pub const RUNNING: Self = Self {
        liveness: Liveness::Up,
        readiness: Readiness::Degraded,
        source: ProbeSource::Supervisor,
    };

    /// A timer that is idle between runs and last exited cleanly.
    ///
    /// This is the healthy state for a timer, which is why it is `Ready` while
    /// [`Probe::RUNNING`] is not: a completed run is positive evidence the
    /// service did its job, whereas a live pid is only evidence it exists.
    pub const IDLE_OK: Self = Self {
        liveness: Liveness::Up,
        readiness: Readiness::Ready,
        source: ProbeSource::Supervisor,
    };
}

/// The aggregated state the tray renders. Ordered by severity so that folding
/// many services is a `max()` over the set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TrayState {
    /// Supervision has not finished its first pass.
    Starting,
    /// Supervision deliberately paused by the operator.
    Paused,
    /// Every required service is up and ready.
    Healthy,
    /// At least one service is up but degraded.
    Degraded,
    /// At least one required service has been down past the grace window.
    Down,
}

impl TrayState {
    pub const fn as_str(self) -> &'static str {
        match self {
            TrayState::Starting => "starting",
            TrayState::Paused => "paused",
            TrayState::Healthy => "healthy",
            TrayState::Degraded => "degraded",
            TrayState::Down => "down",
        }
    }
}

/// One service's health as reported on the API surface.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceReport {
    pub label: String,
    pub required: bool,
    pub kind: ServiceKind,
    pub state: TrayState,
    pub liveness: Liveness,
    pub readiness: Readiness,
    pub source: ProbeSource,
}

#[derive(Clone, Debug)]
struct ServiceHealth {
    required: bool,
    kind: ServiceKind,
    probe: Probe,
    /// When the service first went down; cleared on recovery. A restart must
    /// not flash the tray red, so "down" only counts past the grace window.
    down_since: Option<Instant>,
}

/// Owns health state and publishes the folded [`TrayState`].
///
/// The tray subscribes to [`Aggregator::subscribe`]; it never polls services
/// directly. State lives here, not in the UI.
pub struct Aggregator {
    services: BTreeMap<String, ServiceHealth>,
    grace: Duration,
    paused: bool,
    started: bool,
    tx: watch::Sender<TrayState>,
}

impl Aggregator {
    /// `grace` is how long a required service must stay down before the tray
    /// reports it. Zero makes every restart visible as a failure.
    pub fn new(grace: Duration) -> Self {
        let (tx, _rx) = watch::channel(TrayState::Starting);
        Self {
            services: BTreeMap::new(),
            grace,
            paused: false,
            started: false,
            tx,
        }
    }

    /// Subscribe to state changes. The tray holds one of these.
    pub fn subscribe(&self) -> watch::Receiver<TrayState> {
        self.tx.subscribe()
    }

    /// Register a service before probing it, so a service that has never
    /// reported is distinguishable from one that reported healthy.
    pub fn register(&mut self, label: impl Into<String>, required: bool, kind: ServiceKind) {
        self.services.insert(
            label.into(),
            ServiceHealth {
                required,
                kind,
                probe: Probe::DOWN,
                down_since: None,
            },
        );
    }

    /// Record a probe result. `now` is injected so the grace window is testable
    /// without sleeping.
    pub fn report(&mut self, label: &str, probe: Probe, now: Instant) {
        let Some(service) = self.services.get_mut(label) else {
            return;
        };
        service.probe = probe;
        match probe.liveness {
            Liveness::Down => {
                service.down_since.get_or_insert(now);
            }
            Liveness::Up => {
                service.down_since = None;
            }
        }
        self.started = true;
        self.publish(now);
    }

    /// Pause or resume supervision. Paused outranks service health: the
    /// operator asked for silence and the tray must not keep reporting alarms.
    pub fn set_paused(&mut self, paused: bool, now: Instant) {
        self.paused = paused;
        self.publish(now);
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// The current folded state.
    pub fn state(&self, now: Instant) -> TrayState {
        if self.paused {
            return TrayState::Paused;
        }
        if !self.started || self.services.is_empty() {
            return TrayState::Starting;
        }

        let mut worst = TrayState::Healthy;
        for service in self.services.values() {
            worst = worst.max(Self::observed(service, self.grace, now));
        }
        worst
    }

    /// Per-service state, for the API surface and the popover.
    ///
    /// The tray shows one colour; an operator fixing a problem needs to know
    /// *which* service caused it, so the same fold is exposed per entry rather
    /// than recomputed by the caller.
    pub fn reports(&self, now: Instant) -> Vec<ServiceReport> {
        self.services
            .iter()
            .map(|(label, service)| ServiceReport {
                label: label.clone(),
                required: service.required,
                kind: service.kind,
                state: if self.paused {
                    TrayState::Paused
                } else {
                    Self::observed(service, self.grace, now)
                },
                liveness: service.probe.liveness,
                readiness: service.probe.readiness,
                source: service.probe.source,
            })
            .collect()
    }

    /// Fold one service to a severity.
    ///
    /// A timer with no live process is **not** down: it is idle between runs,
    /// which is its healthy resting state. Judging it like a daemon is the
    /// mistake this branch exists to prevent — three of the twelve manifest
    /// services are timers, so getting it wrong would keep the tray permanently
    /// alarmed.
    fn observed(service: &ServiceHealth, grace: Duration, now: Instant) -> TrayState {
        match service.probe.liveness {
            Liveness::Down if service.kind == ServiceKind::Timer => {
                // The probe reports a timer Down only when its last run failed;
                // an idle timer that exited cleanly is reported IDLE_OK.
                TrayState::Degraded
            }
            Liveness::Down if service.required => {
                let elapsed = service
                    .down_since
                    .map(|since| now.saturating_duration_since(since))
                    .unwrap_or_default();
                if elapsed >= grace {
                    TrayState::Down
                } else {
                    TrayState::Degraded
                }
            }
            Liveness::Down => TrayState::Degraded,
            Liveness::Up => match service.probe.readiness {
                Readiness::Ready => TrayState::Healthy,
                Readiness::Degraded => TrayState::Degraded,
            },
        }
    }

    fn publish(&self, now: Instant) {
        let _ = self.tx.send(self.state(now));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aggregator() -> (Aggregator, Instant) {
        let mut agg = Aggregator::new(Duration::from_secs(30));
        agg.register("api", true, ServiceKind::Daemon);
        agg.register("indexer", false, ServiceKind::Daemon);
        (agg, Instant::now())
    }

    #[test]
    fn all_ready_is_healthy() {
        let (mut agg, t0) = aggregator();
        agg.report("api", Probe::READY, t0);
        agg.report("indexer", Probe::READY, t0);
        assert_eq!(agg.state(t0), TrayState::Healthy);
    }

    #[test]
    fn required_service_down_is_degraded_until_the_grace_window_elapses() {
        let (mut agg, t0) = aggregator();
        agg.report("indexer", Probe::READY, t0);
        agg.report("api", Probe::DOWN, t0);

        // A restart inside the window must not flash the tray red.
        assert_eq!(agg.state(t0 + Duration::from_secs(5)), TrayState::Degraded);
        assert_eq!(agg.state(t0 + Duration::from_secs(30)), TrayState::Down);
    }

    #[test]
    fn recovery_clears_the_down_window() {
        let (mut agg, t0) = aggregator();
        agg.report("indexer", Probe::READY, t0);
        agg.report("api", Probe::DOWN, t0);
        assert_eq!(agg.state(t0 + Duration::from_secs(60)), TrayState::Down);

        agg.report("api", Probe::READY, t0 + Duration::from_secs(61));
        assert_eq!(agg.state(t0 + Duration::from_secs(61)), TrayState::Healthy);
    }

    #[test]
    fn optional_service_down_never_reaches_down() {
        let (mut agg, t0) = aggregator();
        agg.report("api", Probe::READY, t0);
        agg.report("indexer", Probe::DOWN, t0);
        assert_eq!(
            agg.state(t0 + Duration::from_secs(600)),
            TrayState::Degraded
        );
    }

    #[test]
    fn paused_outranks_service_health() {
        let (mut agg, t0) = aggregator();
        agg.report("api", Probe::DOWN, t0);
        agg.set_paused(true, t0 + Duration::from_secs(60));
        assert_eq!(agg.state(t0 + Duration::from_secs(60)), TrayState::Paused);
    }

    #[test]
    fn subscribers_observe_transitions() {
        let (mut agg, t0) = aggregator();
        let rx = agg.subscribe();
        assert_eq!(*rx.borrow(), TrayState::Starting);
        agg.report("api", Probe::READY, t0);
        agg.report("indexer", Probe::READY, t0);
        assert_eq!(*rx.borrow(), TrayState::Healthy);
    }

    /// The reason this port diverges from the template. An idle timer is
    /// healthy; three of the twelve manifest services are timers, so treating
    /// "no pid" as Down would pin the tray to Down on a healthy machine.
    #[test]
    fn an_idle_timer_is_healthy_not_down() {
        let mut agg = Aggregator::new(Duration::from_secs(30));
        agg.register("nudge", true, ServiceKind::Timer);
        let t0 = Instant::now();
        agg.report("nudge", Probe::IDLE_OK, t0);

        // Long past any grace window: an idle timer must never age into Down.
        assert_eq!(
            agg.state(t0 + Duration::from_secs(3_600)),
            TrayState::Healthy
        );
    }

    /// A daemon with the same absence of a process IS down. Without this the
    /// test above would pass for the wrong reason — because nothing can ever
    /// reach Down — rather than because timers are treated differently.
    #[test]
    fn a_daemon_with_no_process_still_reaches_down() {
        let mut agg = Aggregator::new(Duration::from_secs(30));
        agg.register("exec", true, ServiceKind::Daemon);
        let t0 = Instant::now();
        agg.report("exec", Probe::DOWN, t0);
        assert_eq!(agg.state(t0 + Duration::from_secs(3_600)), TrayState::Down);
    }

    /// A failing timer is not silently healthy.
    #[test]
    fn a_timer_whose_last_run_failed_is_reported() {
        let mut agg = Aggregator::new(Duration::from_secs(30));
        agg.register("nudge", true, ServiceKind::Timer);
        let t0 = Instant::now();
        agg.report("nudge", Probe::DOWN, t0);
        assert_eq!(agg.state(t0), TrayState::Degraded);
    }

    /// A live pid is weaker evidence than an endpoint answering, and the fold
    /// must not launder one into the other — ten of twelve services can only
    /// ever be observed this way.
    #[test]
    fn a_supervisor_pid_alone_is_not_reported_as_fully_healthy() {
        let mut agg = Aggregator::new(Duration::from_secs(30));
        agg.register("surface-bridge", true, ServiceKind::Daemon);
        let t0 = Instant::now();
        agg.report("surface-bridge", Probe::RUNNING, t0);

        assert_eq!(agg.state(t0), TrayState::Degraded);
        let report = &agg.reports(t0)[0];
        assert_eq!(report.source, ProbeSource::Supervisor);
    }

    /// Per-service reports must name the offender, not just the worst state.
    #[test]
    fn reports_identify_which_service_is_unhealthy() {
        let (mut agg, t0) = aggregator();
        agg.report("api", Probe::READY, t0);
        agg.report("indexer", Probe::DOWN, t0);

        let reports = agg.reports(t0);
        assert_eq!(reports.len(), 2);
        let api = reports.iter().find(|r| r.label == "api").expect("api");
        let indexer = reports
            .iter()
            .find(|r| r.label == "indexer")
            .expect("indexer");
        assert_eq!(api.state, TrayState::Healthy);
        assert_eq!(indexer.state, TrayState::Degraded);
    }
}
