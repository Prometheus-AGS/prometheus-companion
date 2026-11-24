/// Public API surface for integration tests.
pub mod ag_ui;
pub mod config;
pub mod crdt;
pub mod domains;
pub mod error;
pub mod health_check;
pub mod kbd_control;
pub mod kbd_sync;
pub mod mcp_client_pool;
pub mod mcp_server;
pub mod p2p;
pub mod rest_api;
pub mod store;

/// Apply messages accepted by the production transport. Only an authenticated
/// direct connection conveys an endpoint identity to the addressed sync handler.
pub async fn consume_peer_messages(
    state: rest_api::AppState,
    mut incoming: tokio::sync::mpsc::Receiver<p2p::PeerMessage>,
) {
    while let Some(message) = incoming.recv().await {
        if message.direct {
            let result = rest_api::apply_incoming_peer_message(
                &state,
                &message.from.to_string(),
                &message.payload,
            )
            .await;
            message.respond(result.map_err(|error| error.to_string()));
        } else {
            rest_api::handle_incoming_message(&state, &message.payload).await;
        }
    }
}
