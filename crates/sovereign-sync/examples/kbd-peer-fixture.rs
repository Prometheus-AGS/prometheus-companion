//! Fixture operations for the real-process KBD peer scenarios.

use std::io::{self, Read};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use kbd_runtime::{ensure_device_key_file, verify_ed25519_signature, DeviceSigner, Runtime};
use serde_json::{json, Value};
use sovereign_sync::{
    config::SovereignConfig, domains::SyncEnvelope, kbd_sync::KbdAuthorityPayload,
    p2p::{P2PIdentity, MAX_PEER_FRAME_SIZE}, rest_api::SignedSyncPushRequest,
};

fn fixture_signer() -> Result<DeviceSigner> {
    let path = PathBuf::from(
        std::env::var_os("PROMETHEUS_DEVICE_KEY_FILE").context("missing fixture device key path")?,
    );
    anyhow::ensure!(path.is_file(), "fixture device key does not exist");
    Ok(ensure_device_key_file(&path)?)
}

fn stdin_bytes() -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    io::stdin().read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn fixture_transport() -> Result<SovereignConfig> {
    let config = SovereignConfig::load(&SovereignConfig::default_path())?;
    anyhow::ensure!(
        config.peers.local_only && config.discovery.mdns && !config.discovery.dht,
        "fixture transport requires local-only discovery"
    );
    Ok(config)
}

fn legacy_frame_size(project: String, target: String) -> Result<Value> {
    let runtime = Runtime::open_canonical_snapshot(std::env::current_dir()?)?;
    let signer = fixture_signer()?;
    let config = fixture_transport()?;
    let transport_identity = P2PIdentity::load_or_create(config.node.p2p_identity_file)?;
    let updates = runtime.export_project_updates()?;
    let mut envelope = SyncEnvelope {
        schema_version: "2".into(),
        domain: format!("kbd-control:{project}"),
        identity: Some(project.clone()),
        payload: KbdAuthorityPayload::encode(project, updates, Vec::new())?,
        target_endpoint_ids: vec![target],
        signer_key_id: None,
        signature: None,
    };
    envelope.sign(&signer);
    let payload = serde_json::to_vec(&envelope)?;
    // Recreate the retired full-history transport shape. A real Ed25519
    // URL-safe base64 signature has the same encoded length and requires
    // no JSON escaping. No full-history frame is sent.
    let mut frame = json!({
        "schemaVersion": "1",
        "frameId": uuid::Uuid::new_v4(),
        "issuedAtMs": SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
        "endpointId": transport_identity.endpoint_id().to_string(),
        "signingKeyFingerprint": transport_identity.signing_key_fingerprint(),
        "payload": payload,
    });
    frame["signature"] = json!(URL_SAFE_NO_PAD.encode([0_u8; 64]));
    Ok(json!({
        "frameBytes": serde_json::to_vec(&frame)?.len(),
        "maximumBytes": MAX_PEER_FRAME_SIZE,
    }))
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let mode = args.next().context("missing fixture mode")?;
    match mode.as_str() {
        "sign-bytes" => {
            let signer = fixture_signer()?;
            println!("{}", signer.sign_base64(&stdin_bytes()?));
        }
        "verify-bytes" => {
            let key = args.next().context("missing public verification key")?;
            let signature = args.next().context("missing fixture signature")?;
            anyhow::ensure!(
                verify_ed25519_signature(&key, &stdin_bytes()?, &signature),
                "fixture signature verification failed"
            );
        }
        "resign-push" => {
            let mut request: SignedSyncPushRequest = serde_json::from_slice(&stdin_bytes()?)?;
            request.sign(&fixture_signer()?);
            println!("{}", serde_json::to_string(&request)?);
        }
        "legacy-frame-size" => {
            let project = args.next().context("missing fixture project")?;
            let target = args.next().context("missing fixture target")?;
            println!("{}", legacy_frame_size(project, target)?);
        }
        _ => anyhow::bail!("unknown fixture mode"),
    }
    Ok(())
}
