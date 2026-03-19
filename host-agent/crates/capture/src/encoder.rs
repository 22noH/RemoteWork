use anyhow::Result;
use vpx_encode::{Codec, Config, Encoder as VpxEncoder};
use super::{EncodedFrame, Frame};
use std::time::SystemTime;

pub struct Encoder {
    inner: VpxEncoder,
    frame_count: u64,
    width: u32,
    height: u32,
}

impl Encoder {
    pub fn new(width: u32, height: u32, fps: u32, bitrate_kbps: u32) -> Result<Self> {
        let config = Config {
            width,
            height,
            timebase: [1, fps as i32],
            bitrate: bitrate_kbps,
            codec: Codec::VP8,
        };
        let inner = VpxEncoder::new(config)?;
        Ok(Self { inner, frame_count: 0, width, height })
    }

    pub fn encode(&mut self, frame: &Frame) -> Result<Vec<EncodedFrame>> {
        let i420 = frame.to_i420();
        let timestamp_us = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;

        let packets = self.inner.encode(
            self.frame_count as i64,
            &i420,
            vpx_encode::VPX_DL_REALTIME,
        )?;
        self.frame_count += 1;

        Ok(packets
            .iter()
            .map(|p| EncodedFrame {
                data: p.data.to_vec(),
                timestamp_us,
                width: self.width,
                height: self.height,
                is_keyframe: p.key,
            })
            .collect())
    }
}
