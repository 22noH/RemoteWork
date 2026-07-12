use crate::config::Config;
use bytes::Bytes;
use prost::Message as ProstMessage;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering},
        Arc, OnceLock,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    sync::{broadcast, mpsc},
    time::interval,
};
use webrtc::{
    media::Sample,
    track::track_local::track_local_static_sample::TrackLocalStaticSample,
};

use input::InputHandler;
use network::{DataChannelHandlers, HostPeerConnection, NetworkConfig, SignalingClient, SignalingEvent};
use proto::remote_work::InputEvent;

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// State shared between the network layer and the egui UI thread.
#[derive(Clone)]
pub struct Shared {
    /// Live view-only toggle: when false, viewer input is ignored.
    pub allow_control: Arc<AtomicBool>,
    /// Number of currently connected viewers (for the UI status line).
    pub viewer_count: Arc<AtomicUsize>,
    /// Set by the UI "Disconnect all" button; the reconnect loop polls it.
    pub disconnect_all: Arc<AtomicBool>,
    /// True while a connection request is waiting for the host to allow/deny.
    pub pending_approval: Arc<AtomicBool>,
    /// UI's answer to a pending request: 0 = undecided, 1 = allow, 2 = deny.
    pub approval_decision: Arc<AtomicU8>,
    /// egui context (set once the UI starts) so the network thread can pop the
    /// window to the front when a request arrives, even if minimized to tray.
    pub ctx: Arc<OnceLock<eframe::egui::Context>>,
    /// Chat transcript shown in the host window (incoming + host-sent).
    pub chat_log: Arc<std::sync::Mutex<Vec<ChatLine>>>,
    /// Sender for host-typed chat (set per session); None when nobody connected.
    pub chat_send: Arc<std::sync::Mutex<Option<mpsc::UnboundedSender<String>>>>,
    /// The host's in-progress chat input (shared so the separate chat window,
    /// an egui viewport, can own it).
    pub chat_input: Arc<std::sync::Mutex<String>>,
    /// Whether the chat window is currently shown. Closing it sets this false;
    /// a new incoming message reopens it (notification-style).
    pub chat_open: Arc<AtomicBool>,
}

/// One line in the host chat transcript.
#[derive(Clone)]
pub struct ChatLine {
    pub from_me: bool,
    pub text: String,
}

/// Append a chat line, keeping the transcript bounded.
fn push_chat(log: &Arc<std::sync::Mutex<Vec<ChatLine>>>, line: ChatLine) {
    let mut l = log.lock().unwrap();
    l.push(line);
    let len = l.len();
    if len > 200 {
        l.drain(0..len - 200);
    }
}

/// Block until the host allows or denies the pending connection. Auto-denies
/// after 30s so an ignored prompt can't wedge the event loop forever.
async fn request_approval(shared: &Shared) -> bool {
    shared.approval_decision.store(0, Ordering::Relaxed);
    shared.pending_approval.store(true, Ordering::Relaxed);
    if let Some(ctx) = shared.ctx.get() {
        ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Focus);
        ctx.request_repaint();
    }
    let mut waited = Duration::ZERO;
    let timeout = Duration::from_secs(30);
    let decision = loop {
        match shared.approval_decision.load(Ordering::Relaxed) {
            1 => break true,
            2 => break false,
            _ => {
                if waited >= timeout {
                    tracing::warn!("Connection request timed out — auto-denied");
                    break false;
                }
                tokio::time::sleep(Duration::from_millis(150)).await;
                waited += Duration::from_millis(150);
            }
        }
    };
    shared.pending_approval.store(false, Ordering::Relaxed);
    decision
}

pub struct App {
    config: Arc<Config>,
    shared: Shared,
}

impl App {
    pub fn new(config: Config, shared: Shared) -> Self {
        Self {
            config: Arc::new(config),
            shared,
        }
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let (shutdown_tx, _) = broadcast::channel::<()>(1);
        let config = self.config.clone();

        let net_config = Arc::new(NetworkConfig {
            host_id: config.host_id.clone(),
            password: config.password.clone(),
            signaling_server_url: config.signaling_server_url.clone(),
            stun_servers: config.stun_servers.clone(),
        });

        let event_loop_handle = {
            let net_config = Arc::clone(&net_config);
            let config = Arc::clone(&config);
            let shutdown_tx_clone = shutdown_tx.clone();
            let shared = self.shared.clone();
            tokio::spawn(run_reconnect_loop(net_config, config, shutdown_tx_clone, shared))
        };

        tokio::signal::ctrl_c().await?;
        tracing::info!("Ctrl+C received — shutting down");
        let _ = shutdown_tx.send(());
        event_loop_handle.abort();

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Reconnect loop with exponential backoff
// ---------------------------------------------------------------------------

async fn run_reconnect_loop(
    net_config: Arc<NetworkConfig>,
    config: Arc<Config>,
    shutdown_tx: broadcast::Sender<()>,
    shared: Shared,
) {
    let mut backoff = Duration::from_secs(1);
    let max_backoff = Duration::from_secs(30);

    'reconnect: loop {
        let signaling_shutdown = shutdown_tx.subscribe();
        match SignalingClient::connect(Arc::clone(&net_config), signaling_shutdown).await {
            Ok((client, mut event_rx)) => {
                backoff = Duration::from_secs(1); // reset on successful connect
                let client = Arc::new(client);
                let mut sessions: HashMap<String, Session> = HashMap::new();
                let mut shutdown = shutdown_tx.subscribe();
                let mut ping_interval = interval(Duration::from_secs(30));
                ping_interval.tick().await; // consume the immediate first tick
                let mut health_interval = interval(Duration::from_secs(30));
                health_interval.tick().await;
                // ponytail: 250ms poll of the UI "disconnect all" flag — simplest
                // bridge from the sync egui thread; a Notify would be tidier if the
                // UI ever needs more commands.
                let mut ui_poll = interval(Duration::from_millis(250));

                loop {
                    tokio::select! {
                        _ = shutdown.recv() => {
                            tracing::info!("Shutdown received, closing all sessions");
                            cleanup_sessions(&mut sessions).await;
                            shared.viewer_count.store(0, Ordering::Relaxed);
                            break 'reconnect;
                        }
                        _ = ui_poll.tick() => {
                            if shared.disconnect_all.swap(false, Ordering::Relaxed) {
                                tracing::info!("Disconnect all requested from UI");
                                cleanup_sessions(&mut sessions).await;
                                shared.viewer_count.store(0, Ordering::Relaxed);
                            }
                        }
                        _ = ping_interval.tick() => {
                            client.send_ping(now_ms());
                        }
                        _ = health_interval.tick() => {
                            let mut dead_tokens: Vec<String> = Vec::new();
                            let now = std::time::Instant::now();
                            for (token, session) in sessions.iter() {
                                let state = session.pc.connection_state();
                                match state {
                                    webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState::Failed
                                    | webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState::Closed => {
                                        tracing::info!("Cleaning up dead session {} (state: {})", token, state);
                                        dead_tokens.push(token.clone());
                                    }
                                    webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState::Disconnected => {
                                        if now.duration_since(session.created_at) > std::time::Duration::from_secs(60) {
                                            tracing::info!("Cleaning up disconnected session {} (>60s)", token);
                                            dead_tokens.push(token.clone());
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            for token in dead_tokens {
                                if let Some(session) = sessions.remove(&token) {
                                    let _ = session.capture_stop_tx.send(());
                                    if let Some(stop) = session.audio_capture_stop {
                                        let _ = stop.send(());
                                    }
                                    if let Some(stop) = session.audio_playback_stop {
                                        let _ = stop.send(());
                                    }
                                    let _ = session.pc.close().await;
                                }
                            }
                            shared.viewer_count.store(sessions.len(), Ordering::Relaxed);
                        }
                        event = event_rx.recv() => {
                            match event {
                                Some(SignalingEvent::Disconnected) => {
                                    tracing::warn!("Disconnected from signaling server, reconnecting in {:?}", backoff);
                                    cleanup_sessions(&mut sessions).await;
                                    tokio::time::sleep(backoff).await;
                                    backoff = (backoff * 2).min(max_backoff);
                                    continue 'reconnect;
                                }
                                Some(event) => {
                                    handle_event(event, &client, &config, &mut sessions, &shared).await;
                                    shared.viewer_count.store(sessions.len(), Ordering::Relaxed);
                                }
                                None => {
                                    tracing::warn!("Event channel closed, reconnecting in {:?}", backoff);
                                    cleanup_sessions(&mut sessions).await;
                                    tokio::time::sleep(backoff).await;
                                    backoff = (backoff * 2).min(max_backoff);
                                    continue 'reconnect;
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Signaling connect failed: {e}, retrying in {backoff:?}");
                let mut shutdown = shutdown_tx.subscribe();
                tokio::select! {
                    _ = tokio::time::sleep(backoff) => {}
                    _ = shutdown.recv() => {
                        tracing::info!("Shutdown during reconnect backoff");
                        break 'reconnect;
                    }
                }
                backoff = (backoff * 2).min(max_backoff);
            }
        }
    }
}

async fn cleanup_sessions(sessions: &mut HashMap<String, Session>) {
    for (token, session) in sessions.drain() {
        tracing::info!("Closing session {}", token);
        let _ = session.capture_stop_tx.send(());
        if let Some(stop) = session.audio_capture_stop {
            let _ = stop.send(());
        }
        if let Some(stop) = session.audio_playback_stop {
            let _ = stop.send(());
        }
        let _ = session.pc.close().await;
    }
}

// ---------------------------------------------------------------------------
// Session
// ---------------------------------------------------------------------------

struct Session {
    pc: Arc<HostPeerConnection>,
    capture_stop_tx: broadcast::Sender<()>,
    chat_reply_tx: mpsc::UnboundedSender<Vec<u8>>,
    file_reply_tx: mpsc::UnboundedSender<Vec<u8>>,
    audio_capture_stop: Option<tokio::sync::oneshot::Sender<()>>,
    audio_playback_stop: Option<tokio::sync::oneshot::Sender<()>>,
    created_at: std::time::Instant,
}

async fn handle_event(
    event: SignalingEvent,
    client: &Arc<SignalingClient>,
    config: &Arc<Config>,
    sessions: &mut HashMap<String, Session>,
    shared: &Shared,
) {
    match event {
        SignalingEvent::Registered { host_id } => {
            tracing::info!("Host registered: {}", host_id);
        }

        SignalingEvent::IncomingConnection { viewer_session_id } => {
            tracing::info!(
                "Viewer {} is requesting a connection — waiting for SDP offer",
                viewer_session_id
            );
        }

        SignalingEvent::SdpOffer { sdp, session_token } => {
            tracing::info!("Connection request for session {}", session_token);

            // Require the host to approve before accepting the connection.
            if !request_approval(shared).await {
                tracing::info!("Connection denied by host for session {}", session_token);
                return;
            }
            tracing::info!("Connection approved — processing SDP offer for {}", session_token);

            // Enumerate monitors; capture the primary by default. The viewer can
            // switch monitors over the "control" channel.
            let monitors = capture::Capturer::list().unwrap_or_default();
            let primary_idx = capture::Capturer::primary_index().unwrap_or(0);
            let selected_monitor = Arc::new(AtomicUsize::new(primary_idx));
            let geom = {
                let m = monitors.get(primary_idx);
                Arc::new(std::sync::Mutex::new(input::MonitorGeom {
                    x: m.map(|m| m.x).unwrap_or(0),
                    y: m.map(|m| m.y).unwrap_or(0),
                    width: m.map(|m| m.width).unwrap_or(1920),
                    height: m.map(|m| m.height).unwrap_or(1080),
                }))
            };
            let control_hello = monitors_json(&monitors, primary_idx);

            let (ice_tx, mut ice_rx) = mpsc::unbounded_channel::<(String, String, i32)>();

            // Input channel
            let (input_tx, input_rx) = mpsc::unbounded_channel::<Vec<u8>>();
            tokio::spawn(run_input_loop(
                input_rx,
                geom.clone(),
                shared.allow_control.clone(),
            ));

            // Control channel: viewer → monitor selection.
            let (control_tx, control_rx) = mpsc::unbounded_channel::<Vec<u8>>();
            tokio::spawn(run_control_loop(
                control_rx,
                selected_monitor.clone(),
                geom.clone(),
            ));

            // Chat channels
            let (chat_tx, chat_rx) = mpsc::unbounded_channel::<Vec<u8>>();
            let (chat_reply_tx, chat_reply_rx) = mpsc::unbounded_channel::<Vec<u8>>();
            tokio::spawn(run_chat_loop(chat_rx, shared.clone()));

            // Host → viewer chat: the UI pushes text; encode it, forward to the
            // viewer over the reply channel, and echo it into the transcript.
            {
                let (chat_out_tx, mut chat_out_rx) = mpsc::unbounded_channel::<String>();
                *shared.chat_send.lock().unwrap() = Some(chat_out_tx);
                shared.chat_open.store(true, Ordering::Relaxed);
                let chat_reply_tx = chat_reply_tx.clone();
                let chat_log = shared.chat_log.clone();
                tokio::spawn(async move {
                    use prost::Message;
                    use proto::remote_work::{chat_envelope::Payload, ChatEnvelope, ChatMessage};
                    while let Some(text) = chat_out_rx.recv().await {
                        let env = ChatEnvelope {
                            payload: Some(Payload::Message(ChatMessage {
                                id: String::new(),
                                sender: "host".to_string(),
                                content: text.clone(),
                                timestamp_ms: now_ms(),
                            })),
                        };
                        let _ = chat_reply_tx.send(env.encode_to_vec());
                        push_chat(&chat_log, ChatLine { from_me: true, text });
                    }
                });
            }

            // File transfer channels
            let (file_tx, file_rx) = mpsc::unbounded_channel::<Vec<u8>>();
            let (file_reply_tx, file_reply_rx) = mpsc::unbounded_channel::<Vec<u8>>();
            tokio::spawn(run_file_loop(
                file_rx,
                file_reply_tx.clone(),
                config.allowed_dirs.clone(),
            ));

            // Audio channels
            let (audio_rx_tx, mut audio_rx_rx) = mpsc::unbounded_channel::<Vec<u8>>();

            let handlers = DataChannelHandlers {
                input_tx,
                chat_tx,
                chat_reply_rx,
                file_tx,
                file_reply_rx,
                audio_rx_tx,
                control_tx,
                control_hello,
            };

            let pc = match HostPeerConnection::new(config.stun_servers.clone(), ice_tx, handlers).await {
                Ok(pc) => Arc::new(pc),
                Err(e) => {
                    tracing::error!("Failed to create peer connection: {}", e);
                    return;
                }
            };

            {
                let client = Arc::clone(client);
                let token = session_token.clone();
                tokio::spawn(async move {
                    while let Some((candidate, sdp_mid, sdp_mline_index)) = ice_rx.recv().await {
                        client.send_ice_candidate(
                            candidate,
                            sdp_mid,
                            sdp_mline_index,
                            token.clone(),
                        );
                    }
                });
            }

            let answer_sdp = match pc.handle_offer(sdp).await {
                Ok(sdp) => sdp,
                Err(e) => {
                    tracing::error!("handle_offer failed: {}", e);
                    return;
                }
            };

            client.send_sdp_answer(answer_sdp, session_token.clone());
            tracing::info!("SDP answer sent for session {}", session_token);

            // Video capture
            let video_track = pc.video_track();
            let (capture_stop_tx, capture_stop_rx) = broadcast::channel::<()>(1);
            tokio::spawn(capture_loop(video_track, capture_stop_rx, selected_monitor.clone()));

            // Audio capture: encode local mic and send via WebRTC audio track
            let audio_track = pc.audio_track();
            let mut audio_capture_stop = None;
            let mut audio_playback_stop = None;

            match audio::start_audio_capture() {
                Ok((mut frame_rx, stop_tx)) => {
                    audio_capture_stop = Some(stop_tx);
                    let track = Arc::clone(&audio_track);
                    tokio::spawn(async move {
                        while let Some(encoded) = frame_rx.recv().await {
                            let sample = Sample {
                                data: Bytes::from(encoded),
                                duration: Duration::from_millis(20),
                                ..Default::default()
                            };
                            if let Err(e) = track.write_sample(&sample).await {
                                tracing::debug!("Audio write_sample ended: {}", e);
                                break;
                            }
                        }
                    });
                    tracing::info!("Audio capture started for session {}", session_token);
                }
                Err(e) => {
                    tracing::warn!("Audio capture unavailable: {}", e);
                }
            }

            // Audio playback: decode incoming audio RTP from viewer
            match audio::start_audio_playback() {
                Ok((playback_tx, stop_tx)) => {
                    audio_playback_stop = Some(stop_tx);
                    tokio::spawn(async move {
                        while let Some(rtp_data) = audio_rx_rx.recv().await {
                            let _ = playback_tx.send(rtp_data);
                        }
                    });
                    tracing::info!("Audio playback started for session {}", session_token);
                }
                Err(e) => {
                    tracing::warn!("Audio playback unavailable: {}", e);
                }
            }

            sessions.insert(
                session_token,
                Session {
                    pc,
                    capture_stop_tx,
                    chat_reply_tx,
                    file_reply_tx,
                    audio_capture_stop,
                    audio_playback_stop,
                    created_at: std::time::Instant::now(),
                },
            );
        }

        SignalingEvent::IceCandidate {
            candidate,
            sdp_mid,
            sdp_mline_index,
            session_token,
        } => {
            if let Some(session) = sessions.get(&session_token) {
                if let Err(e) = session
                    .pc
                    .add_ice_candidate(candidate, sdp_mid, sdp_mline_index)
                    .await
                {
                    tracing::warn!(
                        "add_ice_candidate failed for session {}: {}",
                        session_token,
                        e
                    );
                }
            } else {
                tracing::warn!("ICE candidate for unknown session {}", session_token);
            }
        }

        SignalingEvent::Disconnected => {
            // Handled by the reconnect loop — sessions are cleaned up there.
            tracing::warn!("Disconnected event received (reconnect loop handles cleanup)");
        }

        SignalingEvent::Error(msg) => {
            tracing::error!("Signaling error: {}", msg);
        }
    }
}

// ---------------------------------------------------------------------------
// Input processing loop
// ---------------------------------------------------------------------------

/// Receives raw Protobuf-encoded `InputEvent` bytes from the WebRTC data
/// channel and dispatches them to the local `InputHandler` (enigo).
async fn run_input_loop(
    mut rx: mpsc::UnboundedReceiver<Vec<u8>>,
    geom: Arc<std::sync::Mutex<input::MonitorGeom>>,
    allow_control: Arc<AtomicBool>,
) {
    let mut handler = match InputHandler::new(geom) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("Failed to create InputHandler: {}", e);
            return;
        }
    };

    tracing::info!("Input loop started");

    while let Some(bytes) = rx.recv().await {
        // Live view-only gate: when control is disabled, drop the event.
        if !allow_control.load(Ordering::Relaxed) {
            continue;
        }
        match InputEvent::decode(bytes.as_slice()) {
            Ok(event) => {
                if let Err(e) = handler.handle(event) {
                    tracing::warn!("Input dispatch error: {}", e);
                }
            }
            Err(e) => tracing::warn!("Failed to decode InputEvent: {}", e),
        }
    }

    tracing::info!("Input loop stopped");
}

// ---------------------------------------------------------------------------
// Monitor control loop (viewer picks which monitor to view)
// ---------------------------------------------------------------------------

/// Serialize the monitor list for the viewer's picker.
fn monitors_json(monitors: &[capture::MonitorInfo], selected: usize) -> Vec<u8> {
    let list: Vec<serde_json::Value> = monitors
        .iter()
        .map(|m| {
            serde_json::json!({
                "index": m.index,
                "name": m.name,
                "width": m.width,
                "height": m.height,
                "primary": m.is_primary,
            })
        })
        .collect();
    serde_json::to_vec(&serde_json::json!({
        "type": "monitors",
        "list": list,
        "selected": selected,
    }))
    .unwrap_or_default()
}

/// Apply monitor-selection messages from the viewer: update the shared selected
/// index (the capture loop rebuilds) and the input-mapping geometry.
async fn run_control_loop(
    mut rx: mpsc::UnboundedReceiver<Vec<u8>>,
    selected: Arc<AtomicUsize>,
    geom: Arc<std::sync::Mutex<input::MonitorGeom>>,
) {
    while let Some(bytes) = rx.recv().await {
        let msg: serde_json::Value = match serde_json::from_slice(&bytes) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if msg["type"] == "select_monitor" {
            if let Some(index) = msg["index"].as_u64().map(|i| i as usize) {
                match capture::Capturer::list() {
                    Ok(monitors) => {
                        if let Some(m) = monitors.get(index) {
                            *geom.lock().unwrap() = input::MonitorGeom {
                                x: m.x,
                                y: m.y,
                                width: m.width,
                                height: m.height,
                            };
                            selected.store(index, Ordering::Relaxed);
                            tracing::info!("Viewer selected monitor {}", index);
                        } else {
                            tracing::warn!("Viewer selected out-of-range monitor {}", index);
                        }
                    }
                    Err(e) => tracing::warn!("Failed to list monitors: {}", e),
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Chat processing loop
// ---------------------------------------------------------------------------

async fn run_chat_loop(mut rx: mpsc::UnboundedReceiver<Vec<u8>>, shared: Shared) {
    use prost::Message;
    use proto::remote_work::ChatEnvelope;
    let chat_log = &shared.chat_log;

    while let Some(bytes) = rx.recv().await {
        match ChatEnvelope::decode(bytes.as_slice()) {
            Ok(envelope) => {
                use proto::remote_work::chat_envelope::Payload;
                match envelope.payload {
                    Some(Payload::Message(msg)) => {
                        tracing::info!("[Chat] {}: {}", msg.sender, msg.content);
                        push_chat(chat_log, ChatLine { from_me: false, text: msg.content });
                        // Reopen the chat window (notification-style) and wake the UI.
                        shared.chat_open.store(true, Ordering::Relaxed);
                        if let Some(ctx) = shared.ctx.get() {
                            ctx.request_repaint();
                        }
                    }
                    Some(Payload::Typing(t)) => {
                        tracing::debug!(
                            "[Chat] {} is {}typing",
                            t.sender,
                            if t.is_typing { "" } else { "not " }
                        );
                    }
                    None => {}
                }
            }
            Err(e) => tracing::warn!("Failed to decode ChatEnvelope: {}", e),
        }
    }
}

// ---------------------------------------------------------------------------
// File transfer processing loop
// ---------------------------------------------------------------------------

async fn run_file_loop(
    mut rx: mpsc::UnboundedReceiver<Vec<u8>>,
    reply_tx: mpsc::UnboundedSender<Vec<u8>>,
    allowed_dirs: Vec<std::path::PathBuf>,
) {
    use prost::Message;
    use proto::remote_work::{
        file_transfer_message::Payload, FileTransferAccept, FileTransferComplete,
        FileTransferError, FileTransferMessage, FileTransferReject,
    };

    let fs_access = file_transfer::fs_access::FsAccess::new(allowed_dirs);
    let mut receiver = file_transfer::FileReceiver::new();

    while let Some(bytes) = rx.recv().await {
        match FileTransferMessage::decode(bytes.as_slice()) {
            Ok(msg) => match msg.payload {
                Some(Payload::Request(req)) => {
                    let dest =
                        std::path::Path::new(&req.destination_path).join(&req.file_name);
                    match fs_access.validate_path(&dest) {
                        Ok(safe_path) => {
                            tracing::info!(
                                "[FileTransfer] Accept: {} ({} bytes)",
                                req.file_name,
                                req.file_size
                            );
                            match receiver
                                .start_receive(
                                    req.transfer_id.clone(),
                                    &safe_path,
                                    req.sha256_hash,
                                )
                                .await
                            {
                                Ok(()) => {
                                    let accept = FileTransferMessage {
                                        payload: Some(Payload::Accept(FileTransferAccept {
                                            transfer_id: req.transfer_id,
                                        })),
                                    };
                                    let _ = reply_tx.send(accept.encode_to_vec());
                                }
                                Err(e) => {
                                    tracing::error!("start_receive failed: {}", e);
                                    let reject = FileTransferMessage {
                                        payload: Some(Payload::Reject(FileTransferReject {
                                            transfer_id: req.transfer_id,
                                            reason: e.to_string(),
                                        })),
                                    };
                                    let _ = reply_tx.send(reject.encode_to_vec());
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!("[FileTransfer] Path rejected: {}", e);
                            let reject = FileTransferMessage {
                                payload: Some(Payload::Reject(FileTransferReject {
                                    transfer_id: req.transfer_id,
                                    reason: format!("Access denied: {}", e),
                                })),
                            };
                            let _ = reply_tx.send(reject.encode_to_vec());
                        }
                    }
                }
                Some(Payload::Chunk(chunk)) => {
                    match receiver
                        .receive_chunk(&chunk.transfer_id, &chunk.data, chunk.last_chunk)
                        .await
                    {
                        Ok(Some(hash)) => {
                            tracing::info!(
                                "[FileTransfer] Complete: {} hash={}",
                                chunk.transfer_id,
                                hash
                            );
                            let complete = FileTransferMessage {
                                payload: Some(Payload::Complete(FileTransferComplete {
                                    transfer_id: chunk.transfer_id,
                                    sha256_hash: hash,
                                })),
                            };
                            let _ = reply_tx.send(complete.encode_to_vec());
                        }
                        Ok(None) => {} // more chunks coming
                        Err(e) => {
                            tracing::error!("[FileTransfer] Chunk error: {}", e);
                            let err_msg = FileTransferMessage {
                                payload: Some(Payload::Error(FileTransferError {
                                    transfer_id: chunk.transfer_id,
                                    error: e.to_string(),
                                })),
                            };
                            let _ = reply_tx.send(err_msg.encode_to_vec());
                        }
                    }
                }
                Some(Payload::Cancel(cancel)) => {
                    tracing::info!("[FileTransfer] Cancelled: {}", cancel.transfer_id);
                }
                _ => {}
            },
            Err(e) => tracing::warn!("Failed to decode FileTransferMessage: {}", e),
        }
    }
}

// ---------------------------------------------------------------------------
// Capture loop (~30 fps)
// ---------------------------------------------------------------------------

async fn capture_loop(
    video_track: Arc<TrackLocalStaticSample>,
    mut stop_rx: broadcast::Receiver<()>,
    selected_monitor: Arc<AtomicUsize>,
) {
    // The screen capturer (xcap) and VP8 encoder (vpx) hold raw pointers and
    // are therefore not `Send`. Keep them on a dedicated OS thread so this
    // async future stays `Send` (required by `tokio::spawn`). Encoded frames
    // are forwarded to the async side over a channel for `write_sample`.
    let (frame_tx, mut frame_rx) = mpsc::unbounded_channel::<(Vec<u8>, Duration)>();
    let stop_flag = Arc::new(AtomicBool::new(false));
    let thread_stop = Arc::clone(&stop_flag);

    std::thread::spawn(move || {
        let frame_duration = Duration::from_millis(33);
        // Rebuilt whenever the viewer switches monitors (usize::MAX forces the
        // first build). Monitor resolutions differ, so the VP8 encoder — which
        // is fixed-size — is rebuilt alongside the capturer.
        let mut current = usize::MAX;
        let mut capturer: Option<capture::Capturer> = None;
        let mut encoder: Option<capture::Encoder> = None;

        while !thread_stop.load(Ordering::Relaxed) {
            let want = selected_monitor.load(Ordering::Relaxed);
            if want != current || capturer.is_none() {
                match capture::Capturer::for_index(want) {
                    Ok(c) => {
                        let (w, h) = (c.width(), c.height());
                        match capture::Encoder::new(w, h, 30, 2000) {
                            Ok(e) => {
                                capturer = Some(c);
                                encoder = Some(e);
                                current = want;
                                tracing::info!("Capturing monitor {} ({}x{} @ 30fps VP8)", want, w, h);
                            }
                            Err(e) => {
                                tracing::error!("Failed to init VP8 encoder: {}", e);
                                std::thread::sleep(frame_duration);
                                continue;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to init capturer for monitor {}: {}", want, e);
                        std::thread::sleep(frame_duration);
                        continue;
                    }
                }
            }

            let cap = capturer.as_ref().unwrap();
            let enc = encoder.as_mut().unwrap();

            let frame = match cap.capture_frame() {
                Ok(f) => f,
                Err(e) => {
                    tracing::warn!("Capture failed: {}", e);
                    std::thread::sleep(frame_duration);
                    continue;
                }
            };

            let encoded_frames = match enc.encode(&frame) {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!("Encode failed: {}", e);
                    continue;
                }
            };

            for ef in encoded_frames {
                if frame_tx.send((ef.data, frame_duration)).is_err() {
                    return; // async side dropped; stop capturing
                }
            }

            std::thread::sleep(frame_duration);
        }
        tracing::info!("Capture thread stopped");
    });

    loop {
        tokio::select! {
            _ = stop_rx.recv() => {
                tracing::info!("Capture loop stopped");
                stop_flag.store(true, Ordering::Relaxed);
                break;
            }
            maybe = frame_rx.recv() => {
                match maybe {
                    Some((data, duration)) => {
                        let sample = Sample {
                            data: Bytes::from(data),
                            duration,
                            ..Default::default()
                        };
                        if let Err(e) = video_track.write_sample(&sample).await {
                            tracing::warn!("write_sample failed (session may have ended): {}", e);
                            stop_flag.store(true, Ordering::Relaxed);
                            return;
                        }
                    }
                    None => break, // capture thread ended
                }
            }
        }
    }
}
