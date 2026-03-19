/// Modifier key bitmask (matches proto definition)
pub struct Modifiers(pub u32);

impl Modifiers {
    pub const CTRL: u32 = 1;
    pub const SHIFT: u32 = 2;
    pub const ALT: u32 = 4;
    pub const META: u32 = 8;

    pub fn ctrl(&self) -> bool { self.0 & Self::CTRL != 0 }
    pub fn shift(&self) -> bool { self.0 & Self::SHIFT != 0 }
    pub fn alt(&self) -> bool { self.0 & Self::ALT != 0 }
    pub fn meta(&self) -> bool { self.0 & Self::META != 0 }
}
