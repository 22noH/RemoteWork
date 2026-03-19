use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

const SAMPLE_RATE: u32 = 48000;
const CHANNELS: u16 = 1;

pub fn playback_thread(
    mut frame_rx: mpsc::UnboundedReceiver<Vec<u8>>,
    mut stop_rx: tokio::sync::oneshot::Receiver<()>,
) -> Result<()> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| anyhow::anyhow!("No default audio output device"))?;

    tracing::info!("Audio playback device: {}", device.name().unwrap_or_default());

    let config = cpal::StreamConfig {
        channels: CHANNELS,
        sample_rate: cpal::SampleRate(SAMPLE_RATE),
        buffer_size: cpal::BufferSize::Default,
    };

    let playback_buf: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let playback_buf_clone = Arc::clone(&playback_buf);

    let stream = device.build_output_stream(
        &config,
        move |data: &mut [f32], _| {
            let mut buf = playback_buf_clone.lock().unwrap();
            for sample in data.iter_mut() {
                *sample = if buf.is_empty() { 0.0 } else { buf.remove(0) };
            }
        },
        |err| tracing::error!("Audio output stream error: {}", err),
        None,
    )?;
    stream.play()?;

    let mut decoder = crate::decoder::OpusDecoder::new()?;

    loop {
        if stop_rx.try_recv().is_ok() {
            break;
        }

        match frame_rx.try_recv() {
            Ok(encoded) => match decoder.decode(&encoded) {
                Ok(pcm) => {
                    let mut buf = playback_buf.lock().unwrap();
                    buf.extend_from_slice(&pcm);
                    // Prevent buffer from growing too large (prevent latency buildup)
                    if buf.len() > SAMPLE_RATE as usize * 2 {
                        let excess = buf.len() - SAMPLE_RATE as usize;
                        buf.drain(..excess);
                    }
                }
                Err(e) => tracing::warn!("Opus decode error: {}", e),
            },
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => break,
        }
    }

    Ok(())
}
