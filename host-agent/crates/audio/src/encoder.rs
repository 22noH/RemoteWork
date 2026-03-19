use anyhow::Result;
use opus::{Encoder, Application, Channels};

const SAMPLE_RATE: u32 = 48000;

pub struct OpusEncoder {
    encoder: Encoder,
}

impl OpusEncoder {
    pub fn new() -> Result<Self> {
        let encoder = Encoder::new(SAMPLE_RATE, Channels::Mono, Application::Voip)?;
        Ok(Self { encoder })
    }

    pub fn encode_f32(&mut self, pcm: &[f32]) -> Result<Vec<u8>> {
        let pcm_i16: Vec<i16> = pcm.iter().map(|&s| (s * i16::MAX as f32) as i16).collect();
        let mut output = vec![0u8; 4000];
        let len = self.encoder.encode(&pcm_i16, &mut output)?;
        output.truncate(len);
        Ok(output)
    }
}
