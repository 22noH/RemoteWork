pub mod capturer;
pub mod encoder;
pub mod frame;

pub use capturer::{Capturer, MonitorInfo};
pub use encoder::Encoder;
pub use frame::Frame;

/// A captured and VP8-encoded frame ready for WebRTC transmission
pub struct EncodedFrame {
    pub data: Vec<u8>,
    pub timestamp_us: u64,
    pub width: u32,
    pub height: u32,
    pub is_keyframe: bool,
}
