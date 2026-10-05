//! Can this process sign?
//!
//! # The problem this module exists to make visible
//!
//! The device key that signs KBD commands lives in the OS credential store as
//! item `prometheus-kbd-device`, held there by the pack's `kbd-runtime` through
//! `keyring 3.6.3`. On macOS that is the legacy login keychain, where access is
//! governed by a **per-item application ACL**: the binary that created the item
//! reads it silently, and *any other binary* triggers an allow/deny prompt.
//!
//! The pack's daemon created that item. A Companion-hosted node is a different
//! signed binary. So:
//!
//! - **Windowed Companion**: the prompt appears, an operator clicks allow, and
//!   everything works from then on. Annoying once, not a failure.
//! - **Headless node**: there is nobody to click. The process would block or be
//!   denied at the first signature — *after* it had already reported itself
//!   healthy and accepted work.
//!
//! That second case is why this module exists. A headless node must decide
//! whether it can sign **before** it starts serving, and refuse to start if it
//! cannot. Failing at startup with a clear message beats failing later, halfway
//! through a mutation, with a keychain error no operator will recognise.
//!
//! # What this module does not do
//!
//! It does not read or cache key material or reimplement cryptographic checks.
//! It checks file type, permissions and readability before bind. Key validity
//! and enrollment still belong to kbd-runtime; preflight alone cannot certify
//! that a mutation signs as the journal's enrolled device.

use std::path::PathBuf;

/// Where a signing identity would come from, in `kbd-runtime`'s own precedence
/// order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SigningIdentity {
    /// `PROMETHEUS_DEVICE_KEY_FILE` points at a readable key file. Works
    /// unattended: no credential store, no prompt.
    ExplicitKeyFile(PathBuf),
    /// The managed key file the installer places for the launchd service.
    /// Also unattended.
    ManagedKeyFile(PathBuf),
    /// Only the OS credential store is available. Fine for a windowed process,
    /// unusable for a headless one that cannot answer a prompt.
    CredentialStoreOnly,
}

impl SigningIdentity {
    /// Whether this identity can be used with no human present.
    pub fn is_unattended(&self) -> bool {
        match self {
            SigningIdentity::ExplicitKeyFile(_) | SigningIdentity::ManagedKeyFile(_) => true,
            SigningIdentity::CredentialStoreOnly => false,
        }
    }
}

/// The managed key file path, mirroring `kbd_runtime::managed_device_key_path`.
///
/// This is XDG-style (`XDG_CONFIG_HOME` or `~/.config`), **not**
/// `dirs_next::config_dir()` — on macOS those differ, and using the wrong one
/// here would report "no key" for a host that has one.
fn managed_device_key_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs_next::home_dir().map(|home| home.join(".config")))?;
    Some(base.join("sovereign-sync").join("device-key.json"))
}

/// Resolve which signing identity this process would use.
pub fn resolve() -> SigningIdentity {
    if let Some(explicit) = std::env::var_os("PROMETHEUS_DEVICE_KEY_FILE") {
        let path = PathBuf::from(explicit);
        if path.is_file() {
            return SigningIdentity::ExplicitKeyFile(path);
        }
        // An invalid explicit override is not rescued by another identity:
        // kbd-runtime's own precedence is strict. Preflight names the error.
        return SigningIdentity::CredentialStoreOnly;
    }
    if let Some(managed) = managed_device_key_path().filter(|path| path.is_file()) {
        return SigningIdentity::ManagedKeyFile(managed);
    }
    SigningIdentity::CredentialStoreOnly
}

/// Refuse to continue unless a signing identity is usable unattended.
///
/// Called by the headless entry point before the node binds anything. The error
/// names the two ways out, because "keychain access denied" three minutes into
/// a run tells an operator nothing actionable.
pub fn require_unattended() -> anyhow::Result<SigningIdentity> {
    if let Some(explicit) = std::env::var_os("PROMETHEUS_DEVICE_KEY_FILE") {
        let path = PathBuf::from(explicit);
        check_key_file(&path)?;
        return Ok(SigningIdentity::ExplicitKeyFile(path));
    }
    let identity = resolve();
    if let SigningIdentity::ManagedKeyFile(path) = &identity {
        check_key_file(path)?;
        return Ok(identity);
    }

    let stale_override = std::env::var_os("PROMETHEUS_DEVICE_KEY_FILE")
        .map(PathBuf::from)
        .filter(|path| !path.is_file());

    let mut message = String::from(
        "no unattended signing identity is available, so this headless node refuses to start.\n\
         The device key is held in the OS credential store (item `prometheus-kbd-device`), \
         whose per-item ACL prompts when a binary other than its creator reads it. \
         A headless process cannot answer that prompt.\n\
         Fix it one of two ways:\n  \
         1. Point PROMETHEUS_DEVICE_KEY_FILE at an existing enrolled mode-0600 key file, or\n  \
         2. Restore/provision that enrolled key at XDG_CONFIG_HOME/sovereign-sync/device-key.json (default ~/.config).\n\
         Authorizing the windowed binary's keychain prompt does not authorize a different headless binary.",
    );
    if let Some(path) = stale_override {
        message.push_str(&format!(
            "\nNote: PROMETHEUS_DEVICE_KEY_FILE is set to {} but no file exists there.",
            path.display()
        ));
    }
    anyhow::bail!(message)
}

fn check_key_file(path: &std::path::Path) -> anyhow::Result<()> {
    let metadata = std::fs::symlink_metadata(path).map_err(|error| {
        anyhow::anyhow!("PROMETHEUS_DEVICE_KEY_FILE/managed key {} is unavailable: {error}", path.display())
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        anyhow::bail!("signing key {} must be a regular non-symlink file", path.display());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            anyhow::bail!("signing key {} must exclude group/other access (mode 0600)", path.display());
        }
    }
    std::fs::File::open(path).map_err(|error| {
        anyhow::anyhow!("signing key {} is not readable: {error}", path.display())
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Asserts the returned variant and its path, not just that the call
    /// succeeded: a resolver that fell through to the credential store while
    /// a key file existed would still "work" under a weaker assertion.
    #[test]
    fn an_existing_explicit_key_file_is_unattended() {
        let dir = tempfile::tempdir().expect("temp dir");
        let key = dir.path().join("device-key.json");
        let mut file = std::fs::File::create(&key).expect("create key file");
        writeln!(file, "{{}}").expect("write key file");

        let previous = std::env::var_os("PROMETHEUS_DEVICE_KEY_FILE");
        std::env::set_var("PROMETHEUS_DEVICE_KEY_FILE", &key);

        let identity = resolve();
        assert_eq!(identity, SigningIdentity::ExplicitKeyFile(key.clone()));
        assert!(identity.is_unattended());
        assert!(require_unattended().is_ok());

        match previous {
            Some(value) => std::env::set_var("PROMETHEUS_DEVICE_KEY_FILE", value),
            None => std::env::remove_var("PROMETHEUS_DEVICE_KEY_FILE"),
        }
    }

    /// The failure must name the stale path. An error that said only
    /// "no identity" would send an operator looking in the wrong place.
    #[test]
    fn a_dangling_override_is_reported_with_its_path() {
        let previous_key = std::env::var_os("PROMETHEUS_DEVICE_KEY_FILE");
        let previous_xdg = std::env::var_os("XDG_CONFIG_HOME");
        let empty = tempfile::tempdir().expect("temp dir");
        std::env::set_var("PROMETHEUS_DEVICE_KEY_FILE", "/nonexistent/cpc005/key.json");
        // Point XDG at an empty dir so the managed path cannot rescue the test
        // on a machine that happens to have a real managed key.
        std::env::set_var("XDG_CONFIG_HOME", empty.path());

        let error = require_unattended().expect_err("a dangling override must refuse");
        let text = error.to_string();
        assert!(
            text.contains("/nonexistent/cpc005/key.json"),
            "the refusal must name the stale path: {text}"
        );
        assert!(
            text.contains("PROMETHEUS_DEVICE_KEY_FILE"),
            "the refusal must name the remedy: {text}"
        );

        match previous_key {
            Some(value) => std::env::set_var("PROMETHEUS_DEVICE_KEY_FILE", value),
            None => std::env::remove_var("PROMETHEUS_DEVICE_KEY_FILE"),
        }
        match previous_xdg {
            Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
    }
}
