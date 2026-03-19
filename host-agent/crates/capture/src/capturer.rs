use anyhow::Result;
use xcap::Monitor;
use super::Frame;

pub struct Capturer {
    monitor: Monitor,
}

impl Capturer {
    pub fn new() -> Result<Self> {
        let monitors = Monitor::all()?;
        let monitor = monitors
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;
        Ok(Self { monitor })
    }

    pub fn capture_frame(&self) -> Result<Frame> {
        let image = self.monitor.capture_image()?;
        Ok(Frame {
            width: image.width(),
            height: image.height(),
            rgba_data: image.into_raw(),
        })
    }

    pub fn width(&self) -> u32 {
        self.monitor.width()
    }

    pub fn height(&self) -> u32 {
        self.monitor.height()
    }
}
