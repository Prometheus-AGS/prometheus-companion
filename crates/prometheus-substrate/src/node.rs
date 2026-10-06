//! Windowed/headless ownership of the existing control and P2P production path.
//! Bind diagnostic HTTP before authority initialization, install local routes,
//! then start the isolated P2P runtime and its signed-message consumer.
//! `Ready` means local authority is installed; transport and service health are
//! separate observations. Optional transport failure must not hide local control.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::{paths, services};
use sovereign_sync::{config::SovereignConfig, p2p, rest_api};

/// Drop requests cancellation; [`Self::shutdown`] waits for bounded drain.
pub struct NodeHandle {
    socket: PathBuf,
    service: Option<rest_api::HttpService>,
    supervisor_task: Option<tokio::task::JoinHandle<()>>,
    p2p_supervisor: Option<p2p::P2PSupervisor>,
    incoming_consumer: Option<tokio::task::JoinHandle<()>>,
    health: Option<tokio::sync::watch::Receiver<crate::health::TrayState>>,
}

impl NodeHandle {
    pub fn socket(&self) -> &std::path::Path {
        &self.socket
    }

    pub fn subscribe_health(&self) -> Option<tokio::sync::watch::Receiver<crate::health::TrayState>> {
        self.health.clone()
    }

    pub async fn wait_for_exit(&self) {
        if let Some(service) = self.service.as_ref() {
            service.wait_for_exit().await;
        }
    }

    /// Stop accepting work, stop transport, drain signed incoming messages,
    /// then join the listener. Preserve the first error while stopping all owners.
    pub async fn shutdown(mut self) -> anyhow::Result<()> {
        if let Some(service) = self.service.as_mut() {
            service.begin_shutdown().await;
        }
        let mut failure = None;
        if let Some(task) = self.supervisor_task.take() {
            task.abort();
            if let Err(error) = task.await {
                if !error.is_cancelled() {
                    failure = Some(anyhow::anyhow!("service supervisor failed: {error}"));
                }
            }
        }
        if let Some(supervisor) = self.p2p_supervisor.take() {
            if let Err(error) = supervisor.shutdown().await {
                failure.get_or_insert(error);
            }
        }
        if let Some(mut consumer) = self.incoming_consumer.take() {
            match tokio::time::timeout(Duration::from_secs(5), &mut consumer).await {
                Ok(Ok(())) => {},
                Ok(Err(error)) => {
                    failure.get_or_insert(anyhow::anyhow!("P2P authority consumer failed: {error}"));
                },
                Err(_) => {
                    consumer.abort();
                    let _ = consumer.await;
                    failure.get_or_insert(anyhow::anyhow!("P2P authority consumer exceeded five-second drain"));
                },
            }
        }
        if let Some(service) = self.service.as_mut() {
            if let Err(error) = service.join().await {
                failure.get_or_insert(error);
            }
        }
        match failure {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

impl Drop for NodeHandle {
    fn drop(&mut self) {
        // JoinHandle Drop detaches; explicitly cancel node-owned tasks.
        if let Some(task) = self.supervisor_task.take() {
            task.abort();
        }
        if let Some(task) = self.incoming_consumer.take() {
            task.abort();
        }
        // P2PSupervisor Drop closes its command sender; HttpService's dropped
        // shutdown sender requests graceful listener stop. Neither joins here.
        self.p2p_supervisor.take();
        self.service.take();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeReadiness {
    /// Local authority installed; inspect P2P/service status separately.
    Ready,
    /// Listener alive, but local authority unavailable.
    DiagnosticsOnly,
}

/// Read selected configuration without rewriting it or guessing another identity.
pub async fn spawn(skills_dir: &std::path::Path) -> anyhow::Result<(NodeHandle, NodeReadiness)> {
    let config_path = SovereignConfig::default_path();
    if std::env::var_os("SOVEREIGN_SYNC_CONFIG").is_some() && !config_path.is_file() {
        anyhow::bail!("explicit SOVEREIGN_SYNC_CONFIG {} is unavailable", config_path.display());
    }
    let config = SovereignConfig::load(&config_path)?;
    spawn_with_config(skills_dir, config).await
}

pub async fn spawn_with_config(
    skills_dir: &std::path::Path,
    config: SovereignConfig,
) -> anyhow::Result<(NodeHandle, NodeReadiness)> {
    let socket = paths::control_socket();
    std::fs::create_dir_all(paths::control_socket_dir())?;
    let service = rest_api::HttpService::start_unix(&socket).await?;
    let gate = service.gate().clone();
    let transport = if config.node.p2p_enabled {
        p2p::P2PHandle::pending()
    } else {
        p2p::P2PHandle::disabled()
    };
    let state = match rest_api::AppState::try_new_with_startup_handle(
        skills_dir, transport.clone(), &gate,
    ).await {
        Ok(state) => state,
        Err(error) => {
            tracing::warn!(%error, "local authority initialization failed; diagnostic routes remain active");
            gate.fail("local authority initialization failed; inspect the Companion log").await;
            return Ok((NodeHandle {
                socket, service: Some(service), supervisor_task: None,
                p2p_supervisor: None, incoming_consumer: None, health: None,
            }, NodeReadiness::DiagnosticsOnly));
        }
    };

    let (state, supervisor_task, health) = match services::pack_root()
        .and_then(|root| crate::supervisor::Supervisor::from_pack_root(&root))
    {
        Ok(supervisor) => {
            let supervisor = Arc::new(supervisor);
            supervisor.probe_once().await;
            let health = supervisor.subscribe().await;
            let task = Arc::clone(&supervisor).spawn();
            (state.with_services(supervisor), Some(task), Some(health))
        },
        Err(error) => {
            tracing::warn!(%error, "service supervision not attached; set PROMETHEUS_PACK_ROOT to an installed pack");
            (state, None, None)
        },
    };
    gate.install(state.clone()).await;
    let (p2p_supervisor, incoming_consumer) = if config.node.p2p_enabled {
        let started = p2p::P2PIdentity::load_or_create(&config.node.p2p_identity_file)
            .and_then(|identity| p2p::P2PSupervisor::spawn_with_discovery(
                identity, config.peers, config.discovery, transport.clone(),
            ));
        match started {
            Ok((supervisor, mut incoming)) => {
                let consumer = tokio::spawn(async move {
                    while let Some(message) = incoming.recv().await {
                        rest_api::handle_incoming_message(&state, &message.payload).await;
                    }
                });
                (Some(supervisor), Some(consumer))
            },
            Err(error) => {
                tracing::warn!(%error, "selected P2P identity/runtime unavailable; local authority remains available");
                transport.mark_failed("selected P2P identity/runtime is unavailable");
                (None, None)
            },
        }
    } else {
        tracing::info!("P2P explicitly disabled; no endpoint or identity is created");
        (None, None)
    };
    Ok((NodeHandle {
        socket, service: Some(service), supervisor_task, p2p_supervisor,
        incoming_consumer, health,
    }, NodeReadiness::Ready))
}
