use anyhow::Result;
use opus::{Decoder, Channels};

const SAMPLE_RATE: u32 = 48000;
const FRAME_SIZE: usize = 960;

pub struct OpusDecoder {
    decoder: Decoder,
}

impl OpusDecoder {
    pub fn new() -> Result<Self> {
        let decoder = Decoder::new(SAMPLE_RATE, Channels::Mono)?;
        Ok(Self { decoder })
    }

    pub fn decode(&mut self, data: &[u8]) -> Result<Vec<f32>> {
        let mut pcm_i16 = vec![0i16; FRAME_SIZE];
        let samples = self.decoder.decode(data, &mut pcm_i16, false)?;
        let pcm_f32: Vec<f32> = pcm_i16[..samples]
            .iter()
            .map(|&s| s as f32 / i16::MAX as f32)
            .collect();
        Ok(pcm_f32)
    }
}
