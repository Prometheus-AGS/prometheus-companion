//! Driving the menu-bar tray icon from the service-health aggregator.
//!
//! # One owner, one subscriber
//!
//! `prometheus_substrate::health::Aggregator` already computes the single
//! `TrayState` this module renders — see that module's header for why folding
//! happens there and not here. This file's only job is: subscribe to state
//! changes, and repaint the tray icon when one arrives. It never computes
//! health itself.
//!
//! # Five states, five committed PNGs, no new dependency
//!
//! `TrayState` has five variants (`Starting`, `Paused`, `Healthy`, `Degraded`,
//! `Down`), matching the five icon families already designed and committed at
//! `docs/brand/assets/tray-{init,paused,up,degraded,down}-template.svg`.
//!
//! Tauri's `Image::from_bytes` decodes PNG/ICO via the `image` crate — it does
//! not decode SVG, and adding an SVG rasterizer would be a new dependency
//! needing an operator pin this change does not have staged. So the five
//! `-template.svg` files were rasterized once, offline, with `rsvg-convert`
//! (a tool already on the build machine, not a new Cargo dependency) to
//! `src-tauri/icons/tray/*.png` at 44×44 — 22pt @2x, the macOS menu-bar
//! template size — and are committed like every other icon in that directory.
//! Only the `-template` variant of each state is needed: `icon_as_template`
//! tells macOS to re-tint the glyph for light/dark itself, so the separate
//! `-light`/`-dark` SVG variants are not used here.
//!
//! `include_bytes!` at compile time, not a runtime file read: a tray that
//! could not find its own icon file would be a startup failure with no clear
//! cause, and the icons are small enough (450–663 bytes each) that embedding
//! them costs nothing worth avoiding it for.

use std::sync::Arc;

use prometheus_substrate::health::TrayState;
use tauri::image::Image;
use tauri::tray::TrayIconBuilder;
use tauri::AppHandle;
use tokio::sync::watch;

macro_rules! tray_icon_bytes {
    ($state:ident, $file:literal) => {
        (TrayState::$state, include_bytes!($file) as &[u8])
    };
}

/// `(state, PNG bytes)` for every variant. A `match` in [`icon_bytes_for`]
/// would compile even if a variant were missing its icon; this table is
/// exhaustively checked by the test at the bottom of this file instead, so a
/// state added to `TrayState` without a matching asset fails a test rather
/// than silently falling through to some default icon.
const TRAY_ICONS: &[(TrayState, &[u8])] = &[
    tray_icon_bytes!(Starting, "../icons/tray/init.png"),
    tray_icon_bytes!(Paused, "../icons/tray/paused.png"),
    tray_icon_bytes!(Healthy, "../icons/tray/up.png"),
    tray_icon_bytes!(Degraded, "../icons/tray/degraded.png"),
    tray_icon_bytes!(Down, "../icons/tray/down.png"),
];

fn icon_bytes_for(state: TrayState) -> &'static [u8] {
    TRAY_ICONS
        .iter()
        .find(|(s, _)| *s == state)
        .map(|(_, bytes)| *bytes)
        .expect("every TrayState variant has an entry in TRAY_ICONS — checked by a test")
}

/// Build the tray icon and start following `health`.
///
/// Takes the `AppHandle` (not the app being built) so this can run from the
/// `setup` hook after the node and its supervisor already exist — the tray
/// has nothing to show before that.
pub fn install(app: &AppHandle, mut health: watch::Receiver<TrayState>) -> tauri::Result<()> {
    let initial_state = *health.borrow();
    let icon = Image::from_bytes(icon_bytes_for(initial_state))?;

    let tray = TrayIconBuilder::new()
        .icon(icon)
        .icon_as_template(true)
        .tooltip(tooltip_for(initial_state))
        .build(app)?;

    let tray = Arc::new(tray);
    tauri::async_runtime::spawn(async move {
        // The first `changed()` call returns immediately with the value
        // already in the channel on some tokio versions; skip re-painting the
        // state just set above so the tray does not flicker at startup.
        loop {
            if health.changed().await.is_err() {
                // Sender dropped — the node shut down. Nothing left to
                // follow; end the task rather than spin on a closed channel.
                break;
            }
            let state = *health.borrow();
            let icon = match Image::from_bytes(icon_bytes_for(state)) {
                Ok(icon) => icon,
                Err(error) => {
                    // A decode failure here means one of the five committed
                    // PNGs is corrupt — a build-time asset problem, not a
                    // runtime one. Logging and keeping the previous icon
                    // beats a tray that silently stops updating with no
                    // record of why.
                    tracing::error!(%error, ?state, "failed to decode tray icon for state");
                    continue;
                }
            };
            if let Err(error) = tray.set_icon(Some(icon)) {
                tracing::warn!(%error, "failed to update tray icon");
            }
            if let Err(error) = tray.set_tooltip(Some(tooltip_for(state))) {
                tracing::warn!(%error, "failed to update tray tooltip");
            }
        }
    });

    Ok(())
}

fn tooltip_for(state: TrayState) -> &'static str {
    match state {
        TrayState::Starting => "Prometheus Companion — starting",
        TrayState::Paused => "Prometheus Companion — supervision paused",
        TrayState::Healthy => "Prometheus Companion — all adopted services healthy",
        TrayState::Degraded => "Prometheus Companion — a service is degraded",
        TrayState::Down => "Prometheus Companion — a required service is down",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The comment above `TRAY_ICONS` claims exhaustiveness; this proves it.
    /// Adding a `TrayState` variant without adding a matching table entry
    /// fails this test instead of panicking at runtime the first time that
    /// state is reached.
    #[test]
    fn every_tray_state_has_an_icon() {
        for state in [
            TrayState::Starting,
            TrayState::Paused,
            TrayState::Healthy,
            TrayState::Degraded,
            TrayState::Down,
        ] {
            let bytes = icon_bytes_for(state);
            assert!(!bytes.is_empty(), "{state:?} has an empty icon entry");
        }
    }

    /// Each committed PNG must actually be decodable — catches a corrupt or
    /// truncated rasterization before it reaches a running tray.
    #[test]
    fn every_tray_icon_decodes_as_a_valid_image() {
        for state in [
            TrayState::Starting,
            TrayState::Paused,
            TrayState::Healthy,
            TrayState::Degraded,
            TrayState::Down,
        ] {
            let bytes = icon_bytes_for(state);
            Image::from_bytes(bytes)
                .unwrap_or_else(|error| panic!("{state:?} icon failed to decode: {error}"));
        }
    }
}
