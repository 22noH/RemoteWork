use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

const SAMPLE_RATE: u32 = 48000;
const CHANNELS: u16 = 1;
const FRAME_SIZE: usize = 960; // 20ms at 48kHz

pub fn capture_thread(
    frame_tx: mpsc::UnboundedSender<Vec<u8>>,
    mut stop_rx: tokio::sync::oneshot::Receiver<()>,
) -> Result<()> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| anyhow::anyhow!("No default audio input device"))?;

    tracing::info!("Audio capture device: {}", device.name().unwrap_or_default());

    let config = cpal::StreamConfig {
        channels: CHANNELS,
        sample_rate: cpal::SampleRate(SAMPLE_RATE),
        buffer_size: cpal::BufferSize::Default,
    };

    let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::with_capacity(SAMPLE_RATE as usize)));
    let buffer_clone = Arc::clone(&buffer);

    let stream = device.build_input_stream(
        &config,
        move |data: &[f32], _| {
            if let Ok(mut buf) = buffer_clone.lock() {
                buf.extend_from_slice(data);
                // Keep buffer from growing unboundedly
                if buf.len() > SAMPLE_RATE as usize * 2 {
                    let excess = buf.len() - SAMPLE_RATE as usize * 2;
                    buf.drain(..excess);
                }
            }
        },
        |err| tracing::error!("Audio input stream error: {}", err),
        None,
    )?;
    stream.play()?;

    let mut encoder = crate::encoder::OpusEncoder::new()?;

    loop {
        if stop_rx.try_recv().is_ok() {
            break;
        }

        // Drain a frame if available
        let frame = {
            let mut buf = buffer.lock().unwrap();
            if buf.len() >= FRAME_SIZE {
                let frame: Vec<f32> = buf.drain(..FRAME_SIZE).collect();
                Some(frame)
            } else {
                None
            }
        };

        if let Some(pcm) = frame {
            match encoder.encode_f32(&pcm) {
                Ok(encoded) => {
                    let _ = frame_tx.send(encoded);
                }
                Err(e) => tracing::warn!("Opus encode error: {}", e),
            }
        } else {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    Ok(())
}
