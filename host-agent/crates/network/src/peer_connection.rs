use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use webrtc::{
    api::{media_engine::MediaEngine, APIBuilder},
    data_channel::{data_channel_message::DataChannelMessage, RTCDataChannel},
    ice_transport::{
        ice_candidate::{RTCIceCandidate, RTCIceCandidateInit},
        ice_server::RTCIceServer,
    },
    peer_connection::{
        configuration::RTCConfiguration,
        peer_connection_state::RTCPeerConnectionState,
        sdp::session_description::RTCSessionDescription,
        RTCPeerConnection,
    },
    rtp_transceiver::rtp_codec::RTCRtpCodecCapability,
    track::track_local::track_local_static_sample::TrackLocalStaticSample,
    MIME_TYPE_OPUS, MIME_TYPE_VP8,
};

/// All channel senders/receivers needed by the data channels.
pub struct DataChannelHandlers {
    pub input_tx: mpsc::UnboundedSender<Vec<u8>>,
    pub chat_tx: mpsc::UnboundedSender<Vec<u8>>,
    pub chat_reply_rx: mpsc::UnboundedReceiver<Vec<u8>>,
    pub file_tx: mpsc::UnboundedSender<Vec<u8>>,
    pub file_reply_rx: mpsc::UnboundedReceiver<Vec<u8>>,
    pub audio_rx_tx: mpsc::UnboundedSender<Vec<u8>>,
}

pub struct HostPeerConnection {
    pc: Arc<RTCPeerConnection>,
    video_track: Arc<TrackLocalStaticSample>,
    audio_track: Arc<TrackLocalStaticSample>,
}

impl HostPeerConnection {
    /// Create a new peer connection.
    ///
    /// # Arguments
    /// * `stun_servers` - list of STUN server URLs
    /// * `ice_tx`       - channel for trickle ICE candidates `(candidate, sdp_mid, sdp_mline_index)`
    /// * `handlers`     - data channel handlers for input, chat, file, and audio
    pub async fn new(
        stun_servers: Vec<String>,
        ice_tx: mpsc::UnboundedSender<(String, String, i32)>,
        handlers: DataChannelHandlers,
    ) -> Result<Self> {
        let mut m = MediaEngine::default();
        m.register_default_codecs()?;

        let api = APIBuilder::new().with_media_engine(m).build();

        let config = RTCConfiguration {
            ice_servers: vec![RTCIceServer {
                urls: stun_servers,
                ..Default::default()
            }],
            ..Default::default()
        };

        let pc = Arc::new(api.new_peer_connection(config).await?);

        // VP8 video track
        let video_track = Arc::new(TrackLocalStaticSample::new(
            RTCRtpCodecCapability {
                mime_type: MIME_TYPE_VP8.to_owned(),
                ..Default::default()
            },
            "video".to_owned(),
            "remote-work".to_owned(),
        ));

        pc.add_track(
            Arc::clone(&video_track)
                as Arc<dyn webrtc::track::track_local::TrackLocal + Send + Sync>,
        )
        .await?;

        // Opus audio track
        let audio_track = Arc::new(TrackLocalStaticSample::new(
            RTCRtpCodecCapability {
                mime_type: MIME_TYPE_OPUS.to_owned(),
                clock_rate: 48000,
                channels: 1,
                ..Default::default()
            },
            "audio".to_owned(),
            "remote-work".to_owned(),
        ));

        pc.add_track(
            Arc::clone(&audio_track)
                as Arc<dyn webrtc::track::track_local::TrackLocal + Send + Sync>,
        )
        .await?;

        // on_track: receive incoming audio RTP from the viewer
        {
            let audio_rx_tx = handlers.audio_rx_tx.clone();
            pc.on_track(Box::new(move |track, _receiver, _transceiver| {
                let audio_rx_tx = audio_rx_tx.clone();
                Box::pin(async move {
                    if let Some(track) = track {
                        let mime = track.codec().capability.mime_type.clone();
                        if mime.contains("opus") {
                            tracing::info!("Receiving incoming audio track");
                            tokio::spawn(async move {
                                let mut buf = vec![0u8; 1500];
                                loop {
                                    match track.read(&mut buf).await {
                                        Ok((n, _)) => {
                                            let _ = audio_rx_tx.send(buf[..n].to_vec());
                                        }
                                        Err(e) => {
                                            tracing::debug!("Audio track read ended: {}", e);
                                            break;
                                        }
                                    }
                                }
                            });
                        }
                    }
                })
            }));
        }

        // ICE candidate callback
        {
            let ice_tx_clone = ice_tx.clone();
            pc.on_ice_candidate(Box::new(move |c: Option<RTCIceCandidate>| {
                let ice_tx = ice_tx_clone.clone();
                Box::pin(async move {
                    if let Some(candidate) = c {
                        match candidate.to_json() {
                            Ok(init) => {
                                let _ = ice_tx.send((
                                    init.candidate,
                                    init.sdp_mid.unwrap_or_default(),
                                    init.sdp_mline_index.unwrap_or(0) as i32,
                                ));
                            }
                            Err(e) => {
                                tracing::warn!("Failed to serialise ICE candidate: {}", e);
                            }
                        }
                    }
                })
            }));
        }

        // Connection-state callback
        pc.on_peer_connection_state_change(Box::new(|state: RTCPeerConnectionState| {
            Box::pin(async move {
                tracing::info!("PeerConnection state changed: {}", state);
            })
        }));

        // Data channel callback -- handles "input", "chat", and "file" channels
        {
            let input_tx = handlers.input_tx.clone();
            let chat_tx = handlers.chat_tx.clone();
            let file_tx = handlers.file_tx.clone();

            // Wrap receivers in Arc<Mutex<Option<...>>> so we can .take() them once
            let chat_reply_rx = Arc::new(Mutex::new(Some(handlers.chat_reply_rx)));
            let file_reply_rx = Arc::new(Mutex::new(Some(handlers.file_reply_rx)));

            pc.on_data_channel(Box::new(move |dc: Arc<RTCDataChannel>| {
                let input_tx = input_tx.clone();
                let chat_tx = chat_tx.clone();
                let file_tx = file_tx.clone();
                let chat_reply_rx = Arc::clone(&chat_reply_rx);
                let file_reply_rx = Arc::clone(&file_reply_rx);

                Box::pin(async move {
                    let label = dc.label().to_string();
                    match label.as_str() {
                        "input" => {
                            tracing::info!("Input data channel opened");
                            dc.on_message(Box::new(move |msg: DataChannelMessage| {
                                let input_tx = input_tx.clone();
                                Box::pin(async move {
                                    let _ = input_tx.send(msg.data.to_vec());
                                })
                            }));
                        }
                        "chat" => {
                            tracing::info!("Chat data channel opened");
                            let dc_send = Arc::clone(&dc);
                            dc.on_message(Box::new(move |msg: DataChannelMessage| {
                                let chat_tx = chat_tx.clone();
                                Box::pin(async move {
                                    let _ = chat_tx.send(msg.data.to_vec());
                                })
                            }));
                            // Drain reply_rx and send back on the data channel
                            let chat_reply_rx = Arc::clone(&chat_reply_rx);
                            dc.on_open(Box::new(move || {
                                let dc_send = dc_send.clone();
                                let chat_reply_rx = chat_reply_rx.clone();
                                Box::pin(async move {
                                    let mut rx_guard = chat_reply_rx.lock().await;
                                    if let Some(mut rx) = rx_guard.take() {
                                        tokio::spawn(async move {
                                            while let Some(data) = rx.recv().await {
                                                if let Err(e) = dc_send.send(&bytes::Bytes::from(data)).await {
                                                    tracing::warn!("Chat reply send error: {}", e);
                                                    break;
                                                }
                                            }
                                        });
                                    }
                                })
                            }));
                        }
                        "file" => {
                            tracing::info!("File data channel opened");
                            let dc_send = Arc::clone(&dc);
                            dc.on_message(Box::new(move |msg: DataChannelMessage| {
                                let file_tx = file_tx.clone();
                                Box::pin(async move {
                                    let _ = file_tx.send(msg.data.to_vec());
                                })
                            }));
                            // Drain reply_rx and send back on the data channel
                            let file_reply_rx = Arc::clone(&file_reply_rx);
                            dc.on_open(Box::new(move || {
                                let dc_send = dc_send.clone();
                                let file_reply_rx = file_reply_rx.clone();
                                Box::pin(async move {
                                    let mut rx_guard = file_reply_rx.lock().await;
                                    if let Some(mut rx) = rx_guard.take() {
                                        tokio::spawn(async move {
                                            while let Some(data) = rx.recv().await {
                                                if let Err(e) = dc_send.send(&bytes::Bytes::from(data)).await {
                                                    tracing::warn!("File reply send error: {}", e);
                                                    break;
                                                }
                                            }
                                        });
                                    }
                                })
                            }));
                        }
                        other => {
                            tracing::debug!("Data channel '{}' opened (not handled)", other);
                        }
                    }
                })
            }));
        }

        Ok(Self {
            pc,
            video_track,
            audio_track,
        })
    }

    /// Process an SDP offer and return the SDP answer string.
    pub async fn handle_offer(&self, sdp: String) -> Result<String> {
        let offer = RTCSessionDescription::offer(sdp)?;
        self.pc.set_remote_description(offer).await?;

        let answer = self.pc.create_answer(None).await?;

        let mut gather_complete = self.pc.gathering_complete_promise().await;
        self.pc.set_local_description(answer).await?;
        let _ = gather_complete.recv().await;

        let local_desc = self
            .pc
            .local_description()
            .await
            .ok_or_else(|| anyhow::anyhow!("No local description after gathering"))?;

        Ok(local_desc.sdp)
    }

    /// Add a trickle ICE candidate received from the remote viewer.
    pub async fn add_ice_candidate(
        &self,
        candidate: String,
        sdp_mid: String,
        sdp_mline_index: i32,
    ) -> Result<()> {
        let init = RTCIceCandidateInit {
            candidate,
            sdp_mid: Some(sdp_mid),
            sdp_mline_index: Some(sdp_mline_index as u16),
            username_fragment: None,
        };
        self.pc.add_ice_candidate(init).await?;
        Ok(())
    }

    pub fn video_track(&self) -> Arc<TrackLocalStaticSample> {
        Arc::clone(&self.video_track)
    }

    pub fn audio_track(&self) -> Arc<TrackLocalStaticSample> {
        Arc::clone(&self.audio_track)
    }

    pub fn connection_state(&self) -> RTCPeerConnectionState {
        self.pc.connection_state()
    }

    pub async fn close(&self) -> Result<()> {
        self.pc.close().await?;
        Ok(())
    }
}
