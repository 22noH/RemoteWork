use anyhow::Result;
use enigo::{Enigo, Settings};
use proto::remote_work::{InputEvent, input_event::Event};
use crate::{mouse, keyboard};
use std::sync::{Arc, Mutex};

/// Geometry of the monitor currently being shared. Shared so the input mapping
/// tracks the viewer switching monitors mid-session.
#[derive(Debug, Clone, Copy)]
pub struct MonitorGeom {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

pub struct InputHandler {
    enigo: Enigo,
    geom: Arc<Mutex<MonitorGeom>>,
}

impl InputHandler {
    pub fn new(geom: Arc<Mutex<MonitorGeom>>) -> Result<Self> {
        let enigo = Enigo::new(&Settings::default())?;
        Ok(Self { enigo, geom })
    }

    pub fn handle(&mut self, event: InputEvent) -> Result<()> {
        let g = *self.geom.lock().unwrap();
        match event.event {
            Some(Event::MouseMove(mv)) => {
                mouse::move_mouse(
                    &mut self.enigo,
                    mv.x,
                    mv.y,
                    g.x,
                    g.y,
                    g.width,
                    g.height,
                )?;
            }
            Some(Event::MouseButton(btn)) => {
                use proto::remote_work::mouse_button::Button;
                let button = Button::try_from(btn.button).unwrap_or(Button::Left);
                mouse::click(&mut self.enigo, button, btn.pressed)?;
            }
            Some(Event::MouseScroll(scroll)) => {
                mouse::scroll(&mut self.enigo, scroll.delta_x, scroll.delta_y)?;
            }
            Some(Event::KeyEvent(key)) => {
                keyboard::key_event(&mut self.enigo, key.key_code, key.pressed)?;
            }
            None => {}
        }
        Ok(())
    }
}
