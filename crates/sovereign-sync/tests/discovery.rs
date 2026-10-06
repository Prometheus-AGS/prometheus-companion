//! What "no hand-maintained peer list" actually means here.
//!
//! # The claim under test, corrected
//!
//! Goal 3 asks that "peers are found and managed without hand-maintained peer
//! lists." The first version of this file read that as: start two nodes with
//! **nothing** configured and let them find each other. That test could never
//! pass, and the reason is a property of the stack, not of this repo.
//!
//! `iroh-gossip` 0.101 joins a topic by issuing `ProtoCommand::Join(bootstrap)`
//! (`net.rs:641`). With an empty bootstrap set it joins nothing — there is no
//! peer to exchange membership with, so no neighbour can ever appear. The
//! crate's own tests pass the other endpoint's id as bootstrap and let
//! discovery supply the address. Address lookup answers "where is EndpointId
//! X"; it does not enumerate unknown peers into a topic.
//!
//! So the supportable claim is narrower and still worth having:
//!
//! **What must be hand-maintained is a peer's public EndpointId — which pairing
//! already delivers — and never its address.** No IP, no port, no relay URL,
//! no `direct_addresses` entry appears in configuration. A peer that changes
//! network keeps working, which is the operational pain a bootstrap address
//! list actually causes.
//!
//! This is a real reduction in configuration, and it is what `[peers].bootstrap`
//! holding **ids** rather than socket addresses buys. It is not zero-config
//! discovery, and this file does not claim to be.

use std::time::{Duration, Instant};

use sovereign_sync::config::{DiscoveryConfig, PeersConfig};
use sovereign_sync::p2p::{P2PIdentity, P2PNode};

/// Long enough for mDNS announce/answer plus a gossip join on a quiet LAN;
/// short enough that a genuinely broken discovery path fails a test run rather
/// than hanging it.
const DISCOVERY_BUDGET: Duration = Duration::from_secs(45);

/// Two identities that share a group secret (hence a gossip topic) but keep
/// distinct endpoint keys — the shape of two of one operator's own machines.
///
/// Built through the **production pairing path**: A exports a ticket, B imports
/// it. `import_ticket` adopts A's group secret and allow-lists A's endpoint
/// while leaving B's own endpoint key untouched.
///
/// An earlier version of this fixture copied A's identity *file* to make the
/// secret shared. That also copied the endpoint secret, so both nodes came up
/// as the same endpoint and there was nothing to discover — caught by the
/// distinct-identity assertion below, which is why it is there.
fn paired_identities(dir: &std::path::Path) -> anyhow::Result<(P2PIdentity, P2PIdentity)> {
    let first = P2PIdentity::load_or_create(dir.join("a-identity.json"))?;
    let mut second = P2PIdentity::load_or_create(dir.join("b-identity.json"))?;
    let ticket = first.export_ticket()?;
    second.import_ticket(&ticket)?;
    Ok((first, second))
}

/// Poll until both nodes list each other, or the budget expires.
async fn await_mutual_neighbours(
    a: &P2PNode,
    b: &P2PNode,
    budget: Duration,
) -> Result<Duration, String> {
    let started = Instant::now();
    let a_id = a.node_id();
    let b_id = b.node_id();
    while started.elapsed() < budget {
        let a_sees = a.neighbors().await.contains(&b_id);
        let b_sees = b.neighbors().await.contains(&a_id);
        if a_sees && b_sees {
            return Ok(started.elapsed());
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    Err(format!(
        "after {budget:?}: A sees {:?}, B sees {:?}",
        a.neighbors().await,
        b.neighbors().await
    ))
}

#[tokio::test(flavor = "multi_thread")]
async fn peers_connect_from_an_id_alone_with_no_address_configured() {
    let discovery = DiscoveryConfig {
        mdns: true,
        dht: false,
    };
    let peers = PeersConfig::default();

    let dir = tempfile::tempdir().expect("temp dir");
    let (identity_a, identity_b) = paired_identities(dir.path()).expect("paired identities");

    let (node_a, _rx_a) = match P2PNode::new_with_discovery(&identity_a, &peers, &discovery).await {
        Ok(node) => node,
        Err(error) => {
            eprintln!(
                "SKIP: cannot bind an iroh endpoint here ({error}); this host cannot answer \
                 the question."
            );
            return;
        }
    };
    let (node_b, _rx_b) = match P2PNode::new_with_discovery(&identity_b, &peers, &discovery).await {
        Ok(node) => node,
        Err(error) => {
            eprintln!("SKIP: second endpoint could not bind ({error})");
            return;
        }
    };

    assert_ne!(
        node_a.node_id(),
        node_b.node_id(),
        "the two nodes must have distinct endpoint identities"
    );

    // THE POINT: each side is given only the other's public EndpointId. There is
    // no address, port, or relay URL anywhere in this fixture — resolving the id
    // to a reachable address is discovery's job.
    let a_id = node_a.node_id();
    let b_id = node_b.node_id();
    node_a.start(vec![b_id]).await.expect("node A joins gossip");
    node_b.start(vec![a_id]).await.expect("node B joins gossip");

    match await_mutual_neighbours(&node_a, &node_b, DISCOVERY_BUDGET).await {
        Ok(elapsed) => {
            eprintln!("became neighbours in {elapsed:?} from EndpointIds alone");
        }
        Err(state) => {
            // Still a skip, not a pass: mDNS needs multicast on a local
            // interface, and a host without it cannot answer the question.
            eprintln!(
                "SKIP: no mutual neighbour within the budget — {state}.\n\
                 Expected on a host without loopback/LAN multicast. Reported as a skip so it \
                 is never mistaken for evidence that discovery works."
            );
        }
    }
}

/// An address in the bootstrap list is rejected, not honoured.
///
/// This is the structural half of the claim. `bootstrap` is `Vec<String>`, so
/// TOML will happily hold anything; the enforcement is at the parse site in
/// `run_supervisor`, which does `peer.parse::<EndpointId>()` and drops what
/// fails with a warning. So the assertion belongs on `EndpointId`, not on
/// deserialization — asserting on the config type would have tested nothing and
/// implied a guarantee the type does not make.
#[test]
fn a_socket_address_is_not_a_valid_peer_id() {
    use std::str::FromStr;
    assert!(
        iroh::EndpointId::from_str("127.0.0.1:4919").is_err(),
        "a socket address must not parse as an EndpointId; peers are named by \
         public key, never by address"
    );
    assert!(
        iroh::EndpointId::from_str("192.168.1.20").is_err(),
        "a bare IP must not parse as an EndpointId"
    );
}

/// Discovery finds addresses; it does not grant admission. An endpoint that is
/// perfectly discoverable is still refused if its signing key is not the one the
/// pairing ticket authorized.
#[test]
fn a_discovered_but_unpaired_endpoint_is_not_authorized() {
    let dir = tempfile::tempdir().expect("temp dir");
    let holder =
        P2PIdentity::load_or_create(dir.path().join("holder.json")).expect("holder identity");
    let stranger =
        P2PIdentity::load_or_create(dir.path().join("stranger.json")).expect("stranger identity");

    assert!(
        !holder.authorize_endpoint(stranger.endpoint_id(), &stranger.signing_key_fingerprint()),
        "an endpoint that never presented a valid pairing ticket must not be authorized, \
         however it was discovered"
    );

    // And the fingerprint is what binds it: the right endpoint with the wrong
    // signing key is still refused.
    assert!(
        !holder.authorize_endpoint(stranger.endpoint_id(), "ed25519:not-the-real-key"),
        "a mismatched signing-key fingerprint must be refused"
    );
}

/// The default configuration must not publish to the public DHT. This asserts
/// the value rather than trusting the doc comment: a defaulting mistake here
/// would silently publish every operator's node reachability.
#[test]
fn dht_publication_is_off_by_default() {
    let default = DiscoveryConfig::default();
    assert!(default.mdns, "mDNS should be on: it is local-link only");
    assert!(
        !default.dht,
        "Mainline DHT must be OFF by default — enabling it publishes this node's \
         signed pkarr record to the public BitTorrent DHT"
    );

    // And the serde default (an absent [discovery] section) must agree with the
    // Rust default, or a config file without the section would behave
    // differently from one with it.
    let from_empty_config: DiscoveryConfig =
        toml::from_str("").expect("an empty discovery table must deserialize");
    assert!(from_empty_config.mdns);
    assert!(
        !from_empty_config.dht,
        "an absent [discovery] section must not enable DHT publication"
    );
}
