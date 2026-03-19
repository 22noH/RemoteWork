use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use prost::Message as ProstMessage;

use crate::session_registry::SessionRegistry;
use crate::{auth, json_protocol, relay};

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/remote_work.rs"));
}

use proto::envelope::Payload;
use proto::*;

// ---------------------------------------------------------------------------
// Server entry-point (plaintext WS)
// ---------------------------------------------------------------------------

pub async fn run_server(addr: String, registry: Arc<SessionRegistry>) -> anyhow::Result<()> {
    let listener = TcpListener::bind(&addr).await?;
    tracing::info!("Signaling server listening on ws://{}", addr);

    loop {
        let (stream, peer_addr) = listener.accept().await?;
        tracing::debug!("New connection from {}", peer_addr);
        let registry = registry.clone();
        tokio::spawn(async move {
            let ws_stream = match accept_async(stream).await {
                Ok(ws) => ws,
                Err(e) => {
                    tracing::error!("WebSocket handshake failed from {}: {}", peer_addr, e);
                    return;
                }
            };
            handle_ws_stream(ws_stream, registry, peer_addr.to_string()).await;
        });
    }
}

// ---------------------------------------------------------------------------
// Server entry-point (TLS / WSS)
// ---------------------------------------------------------------------------

pub async fn run_server_tls(
    addr: String,
    registry: Arc<SessionRegistry>,
    cert_path: String,
    key_path: String,
) -> anyhow::Result<()> {
    use native_tls::Identity;

    // Load certificate and key
    let cert_pem = std::fs::read(&cert_path)
        .map_err(|e| anyhow::anyhow!("Failed to read cert {}: {}", cert_path, e))?;
    let key_pem = std::fs::read(&key_path)
        .map_err(|e| anyhow::anyhow!("Failed to read key {}: {}", key_path, e))?;

    let identity = Identity::from_pkcs8(&cert_pem, &key_pem)
        .map_err(|e| anyhow::anyhow!("Failed to load TLS identity: {}", e))?;
    let tls_acceptor = native_tls::TlsAcceptor::builder(identity)
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build TLS acceptor: {}", e))?;
    let tls_acceptor = tokio_native_tls::TlsAcceptor::from(tls_acceptor);

    let listener = TcpListener::bind(&addr).await?;
    tracing::info!("Signaling server (WSS) listening on wss://{}", addr);

    loop {
        let (stream, peer_addr) = listener.accept().await?;
        tracing::debug!("New TLS connection from {}", peer_addr);
        let registry = registry.clone();
        let acceptor = tls_acceptor.clone();

        tokio::spawn(async move {
            let tls_stream = match acceptor.accept(stream).await {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("TLS handshake failed from {}: {}", peer_addr, e);
                    return;
                }
            };
            let ws_stream = match accept_async(tls_stream).await {
                Ok(ws) => ws,
                Err(e) => {
                    tracing::error!("WebSocket handshake failed from {}: {}", peer_addr, e);
                    return;
                }
            };
            handle_ws_stream(ws_stream, registry, peer_addr.to_string()).await;
        });
    }
}

// ---------------------------------------------------------------------------
// Per-connection handler (generic over stream type)
// ---------------------------------------------------------------------------

async fn handle_ws_stream<S>(
    ws_stream: WebSocketStream<S>,
    registry: Arc<SessionRegistry>,
    peer_addr: String,
) where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Outgoing channel: Vec<u8> that is *either* Protobuf binary or UTF-8 JSON
    // depending on whether the remote client is the Rust host or TS viewer.
    let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();

    // This flag is set to `true` the first time we receive a Text frame.
    // The send_task checks it for every outgoing message to decide the frame type.
    let use_json = Arc::new(AtomicBool::new(false));
    let use_json_send = use_json.clone();

    let mut this_host_id: Option<String> = None;
    let mut this_viewer_id: Option<String> = None;

    // -----------------------------------------------------------------------
    // Outgoing task: drain the channel -> WebSocket
    // -----------------------------------------------------------------------
    let send_task = tokio::spawn(async move {
        while let Some(data) = rx.recv().await {
            let msg = if use_json_send.load(Ordering::Relaxed) {
                // data is guaranteed to be valid UTF-8 JSON produced by json_protocol helpers
                Message::Text(String::from_utf8_lossy(&data).into_owned())
            } else {
                Message::Binary(data)
            };
            if ws_sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    // -----------------------------------------------------------------------
    // Receive loop with idle timeout (5 minutes)
    // -----------------------------------------------------------------------
    let idle_timeout = Duration::from_secs(300);
    loop {
        let msg = tokio::select! {
            msg = ws_receiver.next() => {
                match msg {
                    Some(msg) => msg,
                    None => break,
                }
            }
            _ = tokio::time::sleep(idle_timeout) => {
                tracing::info!("Idle timeout ({}s) for {}, disconnecting", idle_timeout.as_secs(), peer_addr);
                break;
            }
        };

        let msg = match msg {
            Ok(m) => m,
            Err(e) => {
                tracing::debug!("WebSocket error from {}: {}", peer_addr, e);
                break;
            }
        };

        match msg {
            // ------------------------------------------------------------------
            // JSON text frame — TypeScript viewer client
            // ------------------------------------------------------------------
            Message::Text(text) => {
                // Latch this connection as a JSON client.
                use_json.store(true, Ordering::Relaxed);

                let json_msg = match json_protocol::parse_json_message(&text) {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::warn!("Invalid JSON from {}: {}", peer_addr, e);
                        let err = json_protocol::json_error(
                            "PARSE_ERROR".to_string(),
                            format!("Invalid JSON: {}", e),
                        );
                        let _ = tx.send(err.into_bytes());
                        continue;
                    }
                };

                handle_json_message(
                    json_msg,
                    &tx,
                    &registry,
                    &mut this_host_id,
                    &mut this_viewer_id,
                    &peer_addr,
                )
                .await;
            }

            // ------------------------------------------------------------------
            // Binary frame — Rust host agent (Protobuf)
            // ------------------------------------------------------------------
            Message::Binary(data) => {
                // Protobuf clients are the default; use_json stays false.
                let envelope = match Envelope::decode(data.as_slice()) {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::warn!("Failed to decode Protobuf envelope from {}: {}", peer_addr, e);
                        continue;
                    }
                };

                let payload = match envelope.payload {
                    Some(p) => p,
                    None => continue,
                };

                handle_proto_payload(
                    payload,
                    &tx,
                    &registry,
                    &mut this_host_id,
                    &mut this_viewer_id,
                    &peer_addr,
                )
                .await;
            }

            Message::Close(_) => break,
            Message::Ping(payload) => {
                tracing::trace!("WebSocket Ping from {} ({} bytes)", peer_addr, payload.len());
            }
            _ => {}
        }
    }

    // -----------------------------------------------------------------------
    // Cleanup on disconnect
    // -----------------------------------------------------------------------
    if let Some(host_id) = &this_host_id {
        tracing::info!("Host {} disconnected", host_id);
        registry.remove_host(host_id);
    }
    if let Some(viewer_id) = &this_viewer_id {
        tracing::info!("Viewer {} disconnected", viewer_id);
        registry.remove_viewer(viewer_id);
    }
    send_task.abort();
}

// ---------------------------------------------------------------------------
// JSON (viewer) message handler
// ---------------------------------------------------------------------------

/// Dispatch a parsed `JsonMessage` from a TypeScript viewer client.
///
/// Responses are always produced by the `json_protocol` helper functions and
/// sent as UTF-8 bytes through `tx`; the send_task will wrap them in a
/// `Message::Text` frame because `use_json` is already `true`.
async fn handle_json_message(
    msg: json_protocol::JsonMessage,
    tx: &mpsc::UnboundedSender<Vec<u8>>,
    registry: &Arc<SessionRegistry>,
    this_host_id: &mut Option<String>,
    this_viewer_id: &mut Option<String>,
    peer_addr: &str,
) {
    let payload = &msg.payload;

    match msg.msg_type.as_str() {
        // ------------------------------------------------------------------
        // connect_request — viewer wants to connect to a host
        // ------------------------------------------------------------------
        "connect_request" => {
            let target_host_id = match payload["target_host_id"].as_str() {
                Some(v) => v.to_string(),
                None => {
                    let _ = tx.send(json_protocol::json_error(
                        "MISSING_FIELD".to_string(),
                        "target_host_id is required".to_string(),
                    ).into_bytes());
                    return;
                }
            };
            let password_hash = payload["password"].as_str().unwrap_or("").to_string();
            let viewer_session_id = match payload["viewer_session_id"].as_str() {
                Some(v) => v.to_string(),
                None => {
                    let _ = tx.send(json_protocol::json_error(
                        "MISSING_FIELD".to_string(),
                        "viewer_session_id is required".to_string(),
                    ).into_bytes());
                    return;
                }
            };

            tracing::info!(
                "Viewer {} requesting connection to host {}",
                viewer_session_id,
                target_host_id
            );

            registry.register_viewer(viewer_session_id.clone(), tx.clone());
            *this_viewer_id = Some(viewer_session_id.clone());

            // Rate-limit check
            if registry.check_rate_limit(&target_host_id) {
                let _ = tx.send(json_protocol::json_connect_response(
                    false,
                    String::new(),
                    "Too many failed attempts. Please wait 10 minutes.".to_string(),
                ).into_bytes());
                return;
            }

            let host_info = registry
                .hosts
                .get(&target_host_id)
                .map(|h| (h.password_hash.clone(), h.client_tx.clone()));

            if let Some((stored_hash, host_tx)) = host_info {
                if auth::verify_password(&password_hash, &stored_hash) {
                    registry.reset_failed_attempts(&target_host_id);
                    let session_token =
                        registry.create_session(target_host_id.clone(), viewer_session_id.clone());

                    // Tell the viewer it was accepted
                    let _ = tx.send(json_protocol::json_connect_response(
                        true,
                        session_token.clone(),
                        String::new(),
                    ).into_bytes());

                    // Tell the host someone is connecting.
                    // The host is a Protobuf client so we encode as Protobuf binary.
                    let incoming = Envelope {
                        payload: Some(Payload::IncomingConnection(IncomingConnection {
                            viewer_session_id,
                        })),
                    };
                    let _ = host_tx.send(incoming.encode_to_vec());
                } else {
                    registry.record_failed_attempt(&target_host_id);
                    let _ = tx.send(json_protocol::json_connect_response(
                        false,
                        String::new(),
                        "Invalid password".to_string(),
                    ).into_bytes());
                }
            } else {
                let _ = tx.send(json_protocol::json_connect_response(
                    false,
                    String::new(),
                    "Host not found or offline".to_string(),
                ).into_bytes());
            }
        }

        // ------------------------------------------------------------------
        // sdp_offer — viewer sends SDP offer to host
        // ------------------------------------------------------------------
        "sdp_offer" => {
            let sdp = payload["sdp"].as_str().unwrap_or("").to_string();
            let session_token = payload["session_token"].as_str().unwrap_or("").to_string();

            // The host expects Protobuf, so relay as Protobuf binary.
            let envelope = Envelope {
                payload: Some(Payload::SdpOffer(SdpOffer {
                    sdp,
                    session_token: session_token.clone(),
                })),
            };
            let sender_is_host = this_host_id.is_some();
            if let Err(e) = relay::relay_message(
                registry.clone(),
                &session_token,
                sender_is_host,
                envelope.encode_to_vec(),
            )
            .await
            {
                tracing::warn!("Failed to relay JSON SDP offer from {}: {}", peer_addr, e);
            }
        }

        // ------------------------------------------------------------------
        // sdp_answer — viewer sends SDP answer (only if viewer initiates answer)
        // ------------------------------------------------------------------
        "sdp_answer" => {
            let sdp = payload["sdp"].as_str().unwrap_or("").to_string();
            let session_token = payload["session_token"].as_str().unwrap_or("").to_string();

            let envelope = Envelope {
                payload: Some(Payload::SdpAnswer(SdpAnswer {
                    sdp,
                    session_token: session_token.clone(),
                })),
            };
            let sender_is_host = this_host_id.is_some();
            if let Err(e) = relay::relay_message(
                registry.clone(),
                &session_token,
                sender_is_host,
                envelope.encode_to_vec(),
            )
            .await
            {
                tracing::warn!("Failed to relay JSON SDP answer from {}: {}", peer_addr, e);
            }
        }

        // ------------------------------------------------------------------
        // ice_candidate — viewer relays ICE candidate to host
        // ------------------------------------------------------------------
        "ice_candidate" => {
            let candidate = payload["candidate"].as_str().unwrap_or("").to_string();
            let sdp_mid = payload["sdp_mid"].as_str().unwrap_or("").to_string();
            let sdp_mline_index = payload["sdp_mline_index"].as_i64().unwrap_or(0) as i32;
            let session_token = payload["session_token"].as_str().unwrap_or("").to_string();

            let envelope = Envelope {
                payload: Some(Payload::IceCandidate(IceCandidate {
                    candidate,
                    sdp_mid,
                    sdp_mline_index,
                    session_token: session_token.clone(),
                })),
            };
            let sender_is_host = this_host_id.is_some();
            if let Err(e) = relay::relay_message(
                registry.clone(),
                &session_token,
                sender_is_host,
                envelope.encode_to_vec(),
            )
            .await
            {
                tracing::warn!("Failed to relay JSON ICE candidate from {}: {}", peer_addr, e);
            }
        }

        // ------------------------------------------------------------------
        // ping — application-level keep-alive from viewer
        // ------------------------------------------------------------------
        "ping" => {
            let timestamp_ms = payload["timestamp_ms"].as_u64().unwrap_or(0);
            let _ = tx.send(json_protocol::json_pong(timestamp_ms).into_bytes());
        }

        // ------------------------------------------------------------------
        // register_host — a JSON-speaking host (rare, but supported for dev)
        // ------------------------------------------------------------------
        "register_host" => {
            let host_id = match payload["host_id"].as_str() {
                Some(v) => v.to_string(),
                None => {
                    let _ = tx.send(json_protocol::json_error(
                        "MISSING_FIELD".to_string(),
                        "host_id is required".to_string(),
                    ).into_bytes());
                    return;
                }
            };
            let password_hash = payload["password_hash"].as_str().unwrap_or("").to_string();

            tracing::info!("JSON host registering with ID: {}", host_id);
            registry.register_host(host_id.clone(), password_hash, tx.clone());
            *this_host_id = Some(host_id.clone());

            let _ = tx.send(json_protocol::json_register_ack(host_id, true).into_bytes());
        }

        unknown => {
            tracing::debug!("Unknown JSON message type '{}' from {}", unknown, peer_addr);
            let _ = tx.send(json_protocol::json_error(
                "UNKNOWN_TYPE".to_string(),
                format!("Unknown message type: {}", unknown),
            ).into_bytes());
        }
    }
}

// ---------------------------------------------------------------------------
// Protobuf (host) payload handler  — unchanged logic from the original
// ---------------------------------------------------------------------------

/// Dispatch a decoded Protobuf `Payload` from the Rust host agent.
///
/// Responses are encoded as Protobuf binary and sent through `tx`; the
/// send_task will wrap them in a `Message::Binary` frame because `use_json`
/// stays `false` for host connections.
///
/// When a Protobuf host needs to forward something to the viewer (which is a
/// JSON client), we must encode the outgoing data as JSON bytes and send them
/// via the *viewer's* channel.  The viewer's send_task has `use_json = true`
/// so it will frame them as `Message::Text` automatically.
async fn handle_proto_payload(
    payload: Payload,
    tx: &mpsc::UnboundedSender<Vec<u8>>,
    registry: &Arc<SessionRegistry>,
    this_host_id: &mut Option<String>,
    this_viewer_id: &mut Option<String>,
    peer_addr: &str,
) {
    match payload {
        Payload::RegisterHost(reg) => {
            tracing::info!("Host registering with ID: {}", reg.host_id);
            registry.register_host(reg.host_id.clone(), reg.password_hash, tx.clone());
            *this_host_id = Some(reg.host_id.clone());

            let ack = Envelope {
                payload: Some(Payload::RegisterAck(RegisterAck {
                    host_id: reg.host_id,
                    success: true,
                })),
            };
            let _ = tx.send(ack.encode_to_vec());
        }

        Payload::ConnectRequest(req) => {
            // A Protobuf client issuing a ConnectRequest acts as a viewer.
            tracing::info!(
                "Viewer {} requesting connection to host {}",
                req.viewer_session_id,
                req.target_host_id
            );

            registry.register_viewer(req.viewer_session_id.clone(), tx.clone());
            *this_viewer_id = Some(req.viewer_session_id.clone());

            if registry.check_rate_limit(&req.target_host_id) {
                let resp = Envelope {
                    payload: Some(Payload::ConnectResponse(ConnectResponse {
                        accepted: false,
                        session_token: String::new(),
                        error_message: "Too many failed attempts. Please wait 10 minutes."
                            .to_string(),
                    })),
                };
                let _ = tx.send(resp.encode_to_vec());
                return;
            }

            let host_info = registry
                .hosts
                .get(&req.target_host_id)
                .map(|h| (h.password_hash.clone(), h.client_tx.clone()));

            if let Some((stored_hash, host_tx)) = host_info {
                if auth::verify_password(&req.password_hash, &stored_hash) {
                    registry.reset_failed_attempts(&req.target_host_id);
                    let session_token = registry.create_session(
                        req.target_host_id.clone(),
                        req.viewer_session_id.clone(),
                    );

                    let resp = Envelope {
                        payload: Some(Payload::ConnectResponse(ConnectResponse {
                            accepted: true,
                            session_token: session_token.clone(),
                            error_message: String::new(),
                        })),
                    };
                    let _ = tx.send(resp.encode_to_vec());

                    let incoming = Envelope {
                        payload: Some(Payload::IncomingConnection(IncomingConnection {
                            viewer_session_id: req.viewer_session_id,
                        })),
                    };
                    let _ = host_tx.send(incoming.encode_to_vec());
                } else {
                    registry.record_failed_attempt(&req.target_host_id);
                    let resp = Envelope {
                        payload: Some(Payload::ConnectResponse(ConnectResponse {
                            accepted: false,
                            session_token: String::new(),
                            error_message: "Invalid password".to_string(),
                        })),
                    };
                    let _ = tx.send(resp.encode_to_vec());
                }
            } else {
                let resp = Envelope {
                    payload: Some(Payload::ConnectResponse(ConnectResponse {
                        accepted: false,
                        session_token: String::new(),
                        error_message: "Host not found or offline".to_string(),
                    })),
                };
                let _ = tx.send(resp.encode_to_vec());
            }
        }

        // ------------------------------------------------------------------
        // SDP / ICE relay — when the Rust host sends these they may need to
        // reach a JSON viewer.  We encode as JSON when relaying host→viewer.
        // ------------------------------------------------------------------
        Payload::SdpOffer(offer) => {
            let sender_is_host = this_host_id.is_some();
            let session_token = offer.session_token.clone();

            if sender_is_host {
                // Host -> viewer relay: send JSON bytes
                let json_bytes = json_protocol::json_sdp_offer(
                    offer.sdp,
                    offer.session_token,
                ).into_bytes();
                if let Err(e) = relay::relay_message(
                    registry.clone(),
                    &session_token,
                    sender_is_host,
                    json_bytes,
                )
                .await
                {
                    tracing::warn!("Failed to relay SDP offer to viewer: {}", e);
                }
            } else {
                // Viewer (Protobuf) -> host relay: keep as Protobuf binary.
                let envelope = Envelope {
                    payload: Some(Payload::SdpOffer(offer)),
                };
                if let Err(e) = relay::relay_message(
                    registry.clone(),
                    &session_token,
                    sender_is_host,
                    envelope.encode_to_vec(),
                )
                .await
                {
                    tracing::warn!("Failed to relay SDP offer to host: {}", e);
                }
            }
        }

        Payload::SdpAnswer(answer) => {
            let sender_is_host = this_host_id.is_some();
            let session_token = answer.session_token.clone();

            if sender_is_host {
                // Host -> viewer: JSON
                let json_bytes = json_protocol::json_sdp_answer(
                    answer.sdp,
                    answer.session_token,
                ).into_bytes();
                if let Err(e) = relay::relay_message(
                    registry.clone(),
                    &session_token,
                    sender_is_host,
                    json_bytes,
                )
                .await
                {
                    tracing::warn!("Failed to relay SDP answer to viewer: {}", e);
                }
            } else {
                // Protobuf viewer -> host: Protobuf binary
                let envelope = Envelope {
                    payload: Some(Payload::SdpAnswer(answer)),
                };
                if let Err(e) = relay::relay_message(
                    registry.clone(),
                    &session_token,
                    sender_is_host,
                    envelope.encode_to_vec(),
                )
                .await
                {
                    tracing::warn!("Failed to relay SDP answer to host: {}", e);
                }
            }
        }

        Payload::IceCandidate(ice) => {
            let sender_is_host = this_host_id.is_some();
            let session_token = ice.session_token.clone();

            if sender_is_host {
                // Host -> viewer: JSON
                let json_bytes = json_protocol::json_ice_candidate(
                    ice.candidate,
                    ice.sdp_mid,
                    ice.sdp_mline_index,
                    ice.session_token,
                ).into_bytes();
                if let Err(e) = relay::relay_message(
                    registry.clone(),
                    &session_token,
                    sender_is_host,
                    json_bytes,
                )
                .await
                {
                    tracing::warn!("Failed to relay ICE candidate to viewer: {}", e);
                }
            } else {
                // Protobuf viewer -> host: Protobuf binary
                let envelope = Envelope {
                    payload: Some(Payload::IceCandidate(ice)),
                };
                if let Err(e) = relay::relay_message(
                    registry.clone(),
                    &session_token,
                    sender_is_host,
                    envelope.encode_to_vec(),
                )
                .await
                {
                    tracing::warn!("Failed to relay ICE candidate to host: {}", e);
                }
            }
        }

        Payload::Ping(ping) => {
            let pong = Envelope {
                payload: Some(Payload::Pong(Pong {
                    timestamp_ms: ping.timestamp_ms,
                })),
            };
            let _ = tx.send(pong.encode_to_vec());
        }

        _ => {
            tracing::debug!("Unhandled Protobuf message type from {}", peer_addr);
        }
    }
}
