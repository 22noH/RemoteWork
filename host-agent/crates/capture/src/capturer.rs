use anyhow::Result;
use xcap::Monitor;
use super::Frame;

/// Lightweight description of one monitor, sent to the viewer for the picker.
#[derive(Debug, Clone)]
pub struct MonitorInfo {
    pub index: usize,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub is_primary: bool,
}

pub struct Capturer {
    monitor: Monitor,
}

impl Capturer {
    pub fn new() -> Result<Self> {
        Self::for_index(Self::primary_index()?)
    }

    /// Capture a specific monitor by its index in `list()`.
    pub fn for_index(index: usize) -> Result<Self> {
        let monitor = Monitor::all()?
            .into_iter()
            .nth(index)
            .ok_or_else(|| anyhow::anyhow!("Monitor index {} out of range", index))?;
        Ok(Self { monitor })
    }

    /// Enumerate the connected monitors (stable order for a session).
    pub fn list() -> Result<Vec<MonitorInfo>> {
        Ok(Monitor::all()?
            .iter()
            .enumerate()
            .map(|(i, m)| MonitorInfo {
                index: i,
                name: m.name().to_string(),
                width: m.width(),
                height: m.height(),
                x: m.x(),
                y: m.y(),
                is_primary: m.is_primary(),
            })
            .collect())
    }

    /// Index of the primary monitor (0 if none reports as primary).
    pub fn primary_index() -> Result<usize> {
        Ok(Monitor::all()?
            .iter()
            .position(|m| m.is_primary())
            .unwrap_or(0))
    }

    /// Virtual-desktop offset of this monitor's top-left corner.
    pub fn x(&self) -> i32 {
        self.monitor.x()
    }
    pub fn y(&self) -> i32 {
        self.monitor.y()
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
