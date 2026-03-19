use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use prost::Message as ProstMessage;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use proto::remote_work::{envelope::Payload, *};
use crate::NetworkConfig;

/// Events emitted by the signaling client to the application layer.
#[derive(Debug, Clone)]
pub enum SignalingEvent {
    /// Host successfully registered with the signaling server.
    Registered { host_id: String },
    /// A viewer is requesting a connection (pre-WebRTC notification).
    IncomingConnection { viewer_session_id: String },
    /// Viewer has sent an SDP offer; host must create and send an SDP answer.
    SdpOffer { sdp: String, session_token: String },
    /// Viewer sent a trickle ICE candidate.
    IceCandidate {
        candidate: String,
        sdp_mid: String,
        sdp_mline_index: i32,
        session_token: String,
    },
    /// WebSocket connection to the signaling server was closed.
    Disconnected,
    /// A non-fatal error was reported by the signaling server.
    Error(String),
}

/// Bidirectional signaling client.
///
/// # Usage
/// ```no_run
/// let (client, mut events) = SignalingClient::connect(config, shutdown).await?;
/// while let Some(event) = events.recv().await {
///     match event { ... }
/// }
/// ```
pub struct SignalingClient {
    /// Send a raw protobuf-encoded `Envelope` to the signaling server.
    send_tx: mpsc::UnboundedSender<Vec<u8>>,
}

impl SignalingClient {
    /// Connect to the signaling server, register the host, and start the
    /// background I/O task.  Returns the client handle and an event receiver.
    pub async fn connect(
        config: Arc<NetworkConfig>,
        mut shutdown: broadcast::Receiver<()>,
    ) -> Result<(Self, mpsc::UnboundedReceiver<SignalingEvent>)> {
        let url = url::Url::parse(&config.signaling_server_url)?;
        tracing::info!("Connecting to signaling server: {}", url);

        let (ws_stream, _) = connect_async(url).await?;
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Argon2id hash of the password — server stores this and verifies
        // the viewer's plaintext password against it.
        let password_hash = auth::hash_password(&config.password)
            .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?;
        let register = Envelope {
            payload: Some(Payload::RegisterHost(RegisterHost {
                host_id: config.host_id.clone(),
                password_hash,
            })),
        };
        ws_sender
            .send(Message::Binary(register.encode_to_vec()))
            .await?;
        tracing::info!("RegisterHost sent for host_id={}", config.host_id);

        let (event_tx, event_rx) = mpsc::unbounded_channel::<SignalingEvent>();
        let (send_tx, mut send_rx) = mpsc::unbounded_channel::<Vec<u8>>();

        // Spawn the background task that bridges the WebSocket with channels.
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Application wants to send a message to the server.
                    Some(bytes) = send_rx.recv() => {
                        if let Err(e) = ws_sender.send(Message::Binary(bytes)).await {
                            tracing::error!("WS send error: {}", e);
                            let _ = event_tx.send(SignalingEvent::Disconnected);
                            break;
                        }
                    }

                    // Incoming message from the server.
                    msg = ws_receiver.next() => {
                        match msg {
                            Some(Ok(Message::Binary(data))) => {
                                match Envelope::decode(data.as_slice()) {
                                    Ok(envelope) => {
                                        if let Some(evt) = envelope_to_event(envelope) {
                                            let _ = event_tx.send(evt);
                                        }
                                    }
                                    Err(e) => {
                                        tracing::warn!("Failed to decode envelope: {}", e);
                                    }
                                }
                            }
                            Some(Ok(Message::Ping(payload))) => {
                                // tungstenite auto-responds with Pong, but we
                                // may still receive the callback.
                                tracing::trace!("WS ping received ({} bytes)", payload.len());
                            }
                            Some(Ok(Message::Close(_))) | None => {
                                tracing::warn!("Signaling server disconnected");
                                let _ = event_tx.send(SignalingEvent::Disconnected);
                                break;
                            }
                            Some(Err(e)) => {
                                tracing::error!("WebSocket error: {}", e);
                                let _ = event_tx.send(SignalingEvent::Disconnected);
                                break;
                            }
                            _ => {}
                        }
                    }

                    // Graceful shutdown requested.
                    _ = shutdown.recv() => {
                        tracing::info!("Signaling client shutting down");
                        break;
                    }
                }
            }
        });

        Ok((Self { send_tx }, event_rx))
    }

    // -------------------------------------------------------------------------
    // Outgoing message helpers
    // -------------------------------------------------------------------------

    /// Send an SDP answer back to the viewer identified by `session_token`.
    pub fn send_sdp_answer(&self, sdp: String, session_token: String) {
        let msg = Envelope {
            payload: Some(Payload::SdpAnswer(SdpAnswer { sdp, session_token })),
        };
        let _ = self.send_tx.send(msg.encode_to_vec());
    }

    /// Send a ping message with a timestamp for keep-alive.
    pub fn send_ping(&self, timestamp_ms: u64) {
        let msg = Envelope {
            payload: Some(Payload::Ping(Ping { timestamp_ms })),
        };
        let _ = self.send_tx.send(msg.encode_to_vec());
    }

    /// Forward a trickle ICE candidate to the viewer.
    pub fn send_ice_candidate(
        &self,
        candidate: String,
        sdp_mid: String,
        sdp_mline_index: i32,
        session_token: String,
    ) {
        let msg = Envelope {
            payload: Some(Payload::IceCandidate(IceCandidate {
                candidate,
                sdp_mid,
                sdp_mline_index,
                session_token,
            })),
        };
        let _ = self.send_tx.send(msg.encode_to_vec());
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Map a received `Envelope` to a `SignalingEvent`, returning `None` for
/// messages that do not require application-level handling (e.g. Pong).
fn envelope_to_event(envelope: Envelope) -> Option<SignalingEvent> {
    match envelope.payload? {
        Payload::RegisterAck(ack) => {
            if ack.success {
                tracing::info!("Registered with signaling server as {}", ack.host_id);
                Some(SignalingEvent::Registered { host_id: ack.host_id })
            } else {
                tracing::error!("Registration rejected for host_id={}", ack.host_id);
                Some(SignalingEvent::Error(format!(
                    "Registration rejected for host_id={}",
                    ack.host_id
                )))
            }
        }
        Payload::IncomingConnection(ic) => {
            tracing::info!(
                "Incoming connection request from viewer session={}",
                ic.viewer_session_id
            );
            Some(SignalingEvent::IncomingConnection {
                viewer_session_id: ic.viewer_session_id,
            })
        }
        Payload::SdpOffer(offer) => {
            tracing::info!("SDP offer received for session={}", offer.session_token);
            Some(SignalingEvent::SdpOffer {
                sdp: offer.sdp,
                session_token: offer.session_token,
            })
        }
        Payload::IceCandidate(ice) => {
            tracing::debug!(
                "ICE candidate received for session={}",
                ice.session_token
            );
            Some(SignalingEvent::IceCandidate {
                candidate: ice.candidate,
                sdp_mid: ice.sdp_mid,
                sdp_mline_index: ice.sdp_mline_index,
                session_token: ice.session_token,
            })
        }
        Payload::Error(err) => {
            tracing::error!("Signaling error {}: {}", err.code, err.message);
            Some(SignalingEvent::Error(format!(
                "code={} msg={}",
                err.code, err.message
            )))
        }
        Payload::Ping(ping) => {
            tracing::trace!("Ping ts={}", ping.timestamp_ms);
            None
        }
        Payload::Pong(pong) => {
            tracing::trace!("Pong ts={}", pong.timestamp_ms);
            None
        }
        _ => None,
    }
}
