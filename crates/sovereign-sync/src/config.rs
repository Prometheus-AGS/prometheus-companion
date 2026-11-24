use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    #[serde(default = "default_skills_dir")]
    pub skills_dir: String,
    #[serde(default = "default_p2p_identity_file")]
    pub p2p_identity_file: String,
    /// Explicit local-only mode: disabled transport creates no P2P identity.
    #[serde(default = "default_true")]
    pub p2p_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PeersConfig {
    #[serde(default)]
    pub bootstrap: Vec<String>,
    /// Limit discovery and connections to this link; disable public relays and DHT.
    #[serde(default)]
    pub local_only: bool,
}

/// Peer discovery (`change-cpc-006`).
///
/// Two lookups, deliberately defaulted differently:
///
/// - **mDNS is on.** It is confined to the local link, so a node announcing
///   itself on a LAN tells nobody outside it anything. This is what removes the
///   need for a hand-maintained bootstrap list between an operator's own
///   machines. Note the limit: gossip still joins via `ProtoCommand::Join`, so a
///   peer's **EndpointId** must still be known. What mDNS removes is the need to
///   configure its *address* — no IP, port, or relay URL, so a peer that changes
///   network keeps working.
/// - **Mainline DHT is off.** Enabling it *publishes* this node's signed pkarr
///   record — its EndpointId, relay URL, and direct addresses — to the public
///   BitTorrent DHT, where anyone can read it. That is a real disclosure, and a
///   local-first product must not make it silently. An operator who needs WAN
///   discovery without the n0 relay turns it on knowingly.
///
/// Neither lookup grants admission. Discovery answers "where is this
/// EndpointId"; whether that peer may sync is decided by Ed25519 pairing and the
/// endpoint allowlist, unchanged by this change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    /// Local-link discovery. On by default.
    #[serde(default = "default_true")]
    pub mdns: bool,
    /// Mainline DHT lookup *and publication*. Off by default; see above.
    #[serde(default)]
    pub dht: bool,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            mdns: true,
            dht: false,
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_port")]
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SovereignConfig {
    #[serde(default)]
    pub node: NodeConfig,
    #[serde(default)]
    pub peers: PeersConfig,
    #[serde(default)]
    pub discovery: DiscoveryConfig,
    #[serde(default)]
    pub server: ServerConfig,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            skills_dir: default_skills_dir(),
            p2p_identity_file: default_p2p_identity_file(),
            p2p_enabled: true,
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
        }
    }
}

fn default_skills_dir() -> String {
    if let Some(explicit) = std::env::var_os("PROMETHEUS_SKILLS_DIR") {
        return PathBuf::from(explicit).to_string_lossy().into_owned();
    }
    if let Some(config) = std::env::var_os("CLAUDE_CONFIG_DIR") {
        return PathBuf::from(config).join("skills").to_string_lossy().into_owned();
    }
    dirs_next::home_dir()
        .map(|h| {
            h.join(".claude")
                .join("skills")
                .to_string_lossy()
                .into_owned()
        })
        .unwrap_or_else(|| "~/.claude/skills".to_string())
}

fn default_port() -> u16 {
    7892
}

fn default_p2p_identity_file() -> String {
    if let Some(config) = std::env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(config).join("sovereign-sync/p2p-identity.json").to_string_lossy().into_owned();
    }
    dirs_next::home_dir()
        .map(|home| {
            home.join(".config")
                .join("sovereign-sync")
                .join("p2p-identity.json")
                .to_string_lossy()
                .into_owned()
        })
        .unwrap_or_else(|| "p2p-identity.json".into())
}

impl SovereignConfig {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn default_path() -> PathBuf {
        if let Some(explicit) = std::env::var_os("SOVEREIGN_SYNC_CONFIG") {
            return PathBuf::from(explicit);
        }
        if let Some(config) = std::env::var_os("XDG_CONFIG_HOME") {
            return PathBuf::from(config).join("sovereign-sync/config.toml");
        }
        dirs_next::home_dir()
            .map(|h| h.join(".config").join("sovereign-sync").join("config.toml"))
            .unwrap_or_else(|| PathBuf::from("config.toml"))
    }

    pub fn write_default(path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let default = Self::default();
        let content = toml::to_string_pretty(&default)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_no_voter_or_quorum_surface() {
        let rendered = toml::to_string_pretty(&SovereignConfig::default()).unwrap();
        assert!(!rendered.contains("[kbd]"));
        assert!(!rendered.contains("voter"));
        assert!(!rendered.contains("quorum"));
    }
}
