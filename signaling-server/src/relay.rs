use std::sync::Arc;
use crate::session_registry::SessionRegistry;

/// WebSocket relay for when P2P connection fails.
/// Routes binary messages between host and viewer via session token.
pub async fn relay_message(
    registry: Arc<SessionRegistry>,
    session_token: &str,
    sender_is_host: bool,
    data: Vec<u8>,
) -> anyhow::Result<()> {
    if let Some(peer_tx) = registry.get_session_peer_tx(session_token, sender_is_host) {
        let _ = peer_tx.send(data);
        Ok(())
    } else {
        anyhow::bail!(
            "Session not found or peer disconnected: {}",
            session_token
        )
    }
}
