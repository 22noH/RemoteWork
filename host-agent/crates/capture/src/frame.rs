/// Raw captured frame in RGBA format
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgba_data: Vec<u8>,
}

impl Frame {
    /// Convert RGBA to I420 (YUV420p) for VP8 encoding
    pub fn to_i420(&self) -> Vec<u8> {
        let w = self.width as usize;
        let h = self.height as usize;
        let y_plane = w * h;
        let uv_plane = w * h / 4;
        let mut yuv = vec![0u8; y_plane + 2 * uv_plane];

        for row in 0..h {
            for col in 0..w {
                let idx = (row * w + col) * 4;
                let r = self.rgba_data[idx] as f32;
                let g = self.rgba_data[idx + 1] as f32;
                let b = self.rgba_data[idx + 2] as f32;

                // BT.601 coefficients
                yuv[row * w + col] = (16.0 + 65.481 * r / 255.0 + 128.553 * g / 255.0 + 24.966 * b / 255.0) as u8;

                if row % 2 == 0 && col % 2 == 0 {
                    let u = (128.0 - 37.797 * r / 255.0 - 74.203 * g / 255.0 + 112.0 * b / 255.0) as u8;
                    let v = (128.0 + 112.0 * r / 255.0 - 93.786 * g / 255.0 - 18.214 * b / 255.0) as u8;
                    let uv_idx = y_plane + (row / 2) * (w / 2) + (col / 2);
                    yuv[uv_idx] = u;
                    yuv[uv_idx + uv_plane] = v;
                }
            }
        }
        yuv
    }
}
