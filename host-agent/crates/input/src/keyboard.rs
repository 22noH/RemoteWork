use anyhow::Result;
use enigo::{Enigo, Keyboard, Direction, Key};

pub fn key_event(enigo: &mut Enigo, key_code: u32, pressed: bool) -> Result<()> {
    if let Some(key) = hid_to_enigo_key(key_code) {
        let direction = if pressed { Direction::Press } else { Direction::Release };
        enigo.key(key, direction)?;
    }
    Ok(())
}

/// Map USB HID keycodes to enigo Keys (common subset)
fn hid_to_enigo_key(hid_code: u32) -> Option<Key> {
    match hid_code {
        0x04..=0x1D => Some(Key::Unicode((b'a' + (hid_code - 0x04) as u8) as char)),
        0x1E..=0x26 => Some(Key::Unicode((b'1' + (hid_code - 0x1E) as u8) as char)),
        0x27 => Some(Key::Unicode('0')),
        0x28 => Some(Key::Return),
        0x29 => Some(Key::Escape),
        0x2A => Some(Key::Backspace),
        0x2B => Some(Key::Tab),
        0x2C => Some(Key::Space),
        0x2D => Some(Key::Unicode('-')),
        0x2E => Some(Key::Unicode('=')),
        0x4F => Some(Key::RightArrow),
        0x50 => Some(Key::LeftArrow),
        0x51 => Some(Key::DownArrow),
        0x52 => Some(Key::UpArrow),
        0x4A => Some(Key::Home),
        0x4D => Some(Key::End),
        0x4B => Some(Key::PageUp),
        0x4E => Some(Key::PageDown),
        0x4C => Some(Key::Delete),
        0xE0 | 0xE4 => Some(Key::Control),
        0xE1 | 0xE5 => Some(Key::Shift),
        0xE2 | 0xE6 => Some(Key::Alt),
        0xE3 | 0xE7 => Some(Key::Meta),
        _ => None,
    }
}
