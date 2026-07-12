use anyhow::Result;
use enigo::{Enigo, Mouse, Coordinate};
use enigo::Button as EnigoButton;
use proto::remote_work::mouse_button::Button;

pub fn move_mouse(
    enigo: &mut Enigo,
    x: f32,
    y: f32,
    offset_x: i32,
    offset_y: i32,
    screen_width: u32,
    screen_height: u32,
) -> Result<()> {
    // Normalized coords are relative to the captured monitor; add its
    // virtual-desktop offset so enigo's absolute move lands on that monitor.
    let abs_x = offset_x + (x * screen_width as f32) as i32;
    let abs_y = offset_y + (y * screen_height as f32) as i32;
    enigo.move_mouse(abs_x, abs_y, Coordinate::Abs)?;
    Ok(())
}

pub fn click(enigo: &mut Enigo, button: Button, pressed: bool) -> Result<()> {
    let enigo_btn = match button {
        Button::Left => EnigoButton::Left,
        Button::Right => EnigoButton::Right,
        Button::Middle => EnigoButton::Middle,
    };
    let direction = if pressed {
        enigo::Direction::Press
    } else {
        enigo::Direction::Release
    };
    enigo.button(enigo_btn, direction)?;
    Ok(())
}

pub fn scroll(enigo: &mut Enigo, delta_x: f32, delta_y: f32) -> Result<()> {
    if delta_y.abs() > 0.01 {
        enigo.scroll(delta_y as i32, enigo::Axis::Vertical)?;
    }
    if delta_x.abs() > 0.01 {
        enigo.scroll(delta_x as i32, enigo::Axis::Horizontal)?;
    }
    Ok(())
}
