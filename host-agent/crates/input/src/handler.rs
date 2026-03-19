use anyhow::Result;
use enigo::{Enigo, Settings};
use proto::remote_work::{InputEvent, input_event::Event};
use crate::{mouse, keyboard};

pub struct InputHandler {
    enigo: Enigo,
    screen_width: u32,
    screen_height: u32,
}

impl InputHandler {
    pub fn new(screen_width: u32, screen_height: u32) -> Result<Self> {
        let enigo = Enigo::new(&Settings::default())?;
        Ok(Self { enigo, screen_width, screen_height })
    }

    pub fn handle(&mut self, event: InputEvent) -> Result<()> {
        match event.event {
            Some(Event::MouseMove(mv)) => {
                mouse::move_mouse(
                    &mut self.enigo,
                    mv.x,
                    mv.y,
                    self.screen_width,
                    self.screen_height,
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
