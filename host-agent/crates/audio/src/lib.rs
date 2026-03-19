pub mod capture;
pub mod encoder;
pub mod playback;
pub mod decoder;

use anyhow::Result;
use tokio::sync::mpsc;

/// Captures audio from the default input device, encodes as Opus, and provides
/// encoded frames via the returned receiver.
/// Returns a stop sender -- drop it or send () to stop capture.
pub fn start_audio_capture() -> Result<(mpsc::UnboundedReceiver<Vec<u8>>, tokio::sync::oneshot::Sender<()>)> {
    let (frame_tx, frame_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();

    std::thread::spawn(move || {
        if let Err(e) = capture::capture_thread(frame_tx, stop_rx) {
            tracing::error!("Audio capture thread error: {}", e);
        }
    });

    Ok((frame_rx, stop_tx))
}

/// Starts an audio playback thread. Feed it Opus-encoded frames via the returned sender.
pub fn start_audio_playback() -> Result<(mpsc::UnboundedSender<Vec<u8>>, tokio::sync::oneshot::Sender<()>)> {
    let (frame_tx, frame_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();

    std::thread::spawn(move || {
        if let Err(e) = playback::playback_thread(frame_rx, stop_rx) {
            tracing::error!("Audio playback thread error: {}", e);
        }
    });

    Ok((frame_tx, stop_tx))
}
