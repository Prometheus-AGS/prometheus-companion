//! Where shared state lives.
//!
//! # Why this module exists rather than Tauri's path resolver
//!
//! Tauri's `app_data_dir()` and `app_config_dir()` join the platform directory
//! with the **bundle identifier**, so a Companion using them would read and
//! write under `~/Library/Application Support/<bundle-id>/`. The skill pack's
//! daemon writes under `.../prometheus/`. Those are different directories, and
//! a node that resolved its data root through Tauri would silently start a
//! second, empty journal beside the real one — the operator's history would
//! appear to vanish.
//!
//! So the node resolves paths exactly as `kbd-runtime` does, and never through
//! the Tauri path API. The functions below are deliberately a mirror of
//! `kbd_runtime`'s own resolution: `PROMETHEUS_DATA_DIR`, then
//! `dirs_next::data_local_dir()`, then a temp fallback. If that chain ever
//! changes upstream, `continuity.rs` fails, which is the point.

use std::path::PathBuf;

/// Shared windowed/headless skills root, with explicit scope overrides first.
pub fn skills_dir() -> PathBuf {
    if let Some(explicit) = std::env::var_os("PROMETHEUS_SKILLS_DIR") {
        return PathBuf::from(explicit);
    }
    if let Some(config) = std::env::var_os("CLAUDE_CONFIG_DIR") {
        return PathBuf::from(config).join("skills");
    }
    dirs_next::home_dir()
        .map(|home| home.join(".claude").join("skills"))
        .unwrap_or_else(|| std::env::temp_dir().join("skills"))
}

/// The data root holding journals, receipts, and run history.
///
/// Order matches `kbd_runtime`: explicit override, platform data-local
/// directory, then a temp fallback for hosts that have neither.
pub fn data_root() -> PathBuf {
    std::env::var_os("PROMETHEUS_DATA_DIR")
        .map(PathBuf::from)
        .or_else(dirs_next::data_local_dir)
        .unwrap_or_else(|| std::env::temp_dir().join("prometheus-data"))
}

/// The control socket the pack's CLI discovers (seam 1 of the integration
/// contract).
///
/// `SOVEREIGN_SYNC_SOCKET` wins, exactly as the CLI's own transport chain
/// resolves it, so an operator who overrides one side does not have to override
/// the other. The name `sovereign-sync.sock` is contract surface kept in
/// contract v1 for CLI compatibility; it is not a claim about which binary is
/// listening.
pub fn control_socket() -> PathBuf {
    if let Some(explicit) = std::env::var_os("SOVEREIGN_SYNC_SOCKET") {
        return PathBuf::from(explicit);
    }
    dirs_next::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("prometheus")
        .join("run")
        .join("sovereign-sync.sock")
}

/// The directory the control socket lives in. Created before binding, because
/// `UnixListener::bind` fails on a missing parent.
pub fn control_socket_dir() -> PathBuf {
    control_socket()
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Asserts the override is honoured, not merely that the function returns
    /// something: a resolver that ignored the environment would still return a
    /// plausible-looking path.
    #[test]
    fn explicit_data_root_overrides_the_platform_directory() {
        // SAFETY: single-threaded test, restored immediately.
        let previous = std::env::var_os("PROMETHEUS_DATA_DIR");
        std::env::set_var("PROMETHEUS_DATA_DIR", "/tmp/cpc005-explicit-root");
        assert_eq!(data_root(), PathBuf::from("/tmp/cpc005-explicit-root"));
        match previous {
            Some(value) => std::env::set_var("PROMETHEUS_DATA_DIR", value),
            None => std::env::remove_var("PROMETHEUS_DATA_DIR"),
        }
    }

    #[test]
    fn explicit_socket_overrides_the_default_path() {
        let previous = std::env::var_os("SOVEREIGN_SYNC_SOCKET");
        std::env::set_var("SOVEREIGN_SYNC_SOCKET", "/tmp/cpc005/custom.sock");
        assert_eq!(control_socket(), PathBuf::from("/tmp/cpc005/custom.sock"));
        assert_eq!(control_socket_dir(), PathBuf::from("/tmp/cpc005"));
        match previous {
            Some(value) => std::env::set_var("SOVEREIGN_SYNC_SOCKET", value),
            None => std::env::remove_var("SOVEREIGN_SYNC_SOCKET"),
        }
    }

    /// The default must end at the contract's documented path. A refactor that
    /// changed this would break every installed `prometheus` CLI.
    #[test]
    fn default_socket_matches_the_documented_contract_path() {
        let previous = std::env::var_os("SOVEREIGN_SYNC_SOCKET");
        std::env::remove_var("SOVEREIGN_SYNC_SOCKET");
        let socket = control_socket();
        let tail: Vec<_> = socket
            .components()
            .rev()
            .take(3)
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        assert_eq!(tail, vec!["sovereign-sync.sock", "run", "prometheus"]);
        if let Some(value) = previous {
            std::env::set_var("SOVEREIGN_SYNC_SOCKET", value);
        }
    }
}
