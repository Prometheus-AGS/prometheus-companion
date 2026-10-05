//! The Prometheus Companion substrate.
//!
//! # What this crate is
//!
//! The Companion is the control and extension framework for
//! `prometheus-skill-pack`. This crate is the part of it that is not UI: the
//! node the Companion hosts, and the supervisor that adopts the pack's
//! services where they already run.
//!
//! # What this crate is not
//!
//! It is not a replacement for the pack. Operator decision D-02 (option D)
//! draws the line: the pack is open source and complete on its own, and every
//! skill in it works with no Companion installed. What lives here is what the
//! pack has no structure to own — cross-machine sync, peer discovery, and the
//! supervision of the services actually listed in the selected pack manifest.
//!
//! The dependency direction is one-way and load-bearing. This crate consumes
//! the pack's open crates by git rev; nothing in the pack may depend on
//! anything here. See the integration contract published by the pack
//! (`docs/integration-contract.md`, contract v1.0.0) for the four seams that
//! make that possible.
//!
//! # Current state
//!
//! Source includes windowed/headless node ownership, shared paths, unattended
//! identity preflight, configured P2P discovery/pairing, and manifest-based
//! supervision. The connected MCP tools use the selected host's Unix socket.
//! Source presence is not compilation, installation or functional acceptance;
//! those receipts belong to the final local integration gate. The window's
//! application screens remain the starter UI; the tray observes service health.

pub mod health;
pub mod identity;
pub mod pairing;
pub mod paths;
pub mod services;

#[cfg(feature = "sovereign")]
pub mod node;

#[cfg(feature = "sovereign")]
pub mod supervisor;

/// The integration-contract version this substrate is built against.
///
/// The pack reports its own with `prometheus contract show --json`. A mismatch
/// in the major component means this build was compiled against a contract the
/// installed pack no longer implements.
pub const REQUIRED_CONTRACT_VERSION: &str = "1.0.0";

/// Which surface preset this build was compiled for.
///
/// `headless` exists so the node can run with no window and no Stronghold
/// vault: it is what the two-process integration harness launches, and what a
/// server-side deployment runs. The distinction is compile-time because a
/// headless build must not link a UI it can never present.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    Desktop,
    Headless,
}

impl Surface {
    /// Resolve the surface from the features this binary was built with.
    pub const fn current() -> Self {
        // `desktop` is checked first: enabling both features is a build
        // configuration error, and desktop is the superset.
        if cfg!(feature = "kpi-open") {
            Surface::Desktop
        } else {
            Surface::Headless
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Surface::Desktop => "desktop",
            Surface::Headless => "headless",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_resolves_from_build_features() {
        // Asserts the value, not merely that the call compiles: a preset that
        // silently resolved to the wrong surface would let a headless build
        // believe it has a window.
        let surface = Surface::current();
        if cfg!(feature = "kpi-open") {
            assert_eq!(surface, Surface::Desktop);
            assert_eq!(surface.as_str(), "desktop");
        } else {
            assert_eq!(surface, Surface::Headless);
            assert_eq!(surface.as_str(), "headless");
        }
    }

    #[test]
    fn contract_version_is_the_one_the_pack_publishes() {
        assert_eq!(REQUIRED_CONTRACT_VERSION, "1.0.0");
    }
}
