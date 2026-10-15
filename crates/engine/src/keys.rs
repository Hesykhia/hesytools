//! Key and mouse-button identifiers, with stable human-readable names.
//!
//! Names are what get written to config/macro files and shown in the UI, so
//! they must round-trip: `Key::from_name(k.name()) == Some(k)`.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// A keyboard key, identified by its Windows virtual-key code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Key(pub u16);

const NAMED: &[(u16, &str)] = &[
    (0x08, "Backspace"),
    (0x09, "Tab"),
    (0x0D, "Enter"),
    (0x10, "Shift"),
    (0x11, "Ctrl"),
    (0x12, "Alt"),
    (0x13, "Pause"),
    (0x14, "CapsLock"),
    (0x1B, "Escape"),
    (0x20, "Space"),
    (0x21, "PageUp"),
    (0x22, "PageDown"),
    (0x23, "End"),
    (0x24, "Home"),
    (0x25, "Left"),
    (0x26, "Up"),
    (0x27, "Right"),
    (0x28, "Down"),
    (0x2C, "PrintScreen"),
    (0x2D, "Insert"),
    (0x2E, "Delete"),
    (0x5B, "LWin"),
    (0x5C, "RWin"),
    (0x5D, "Menu"),
    (0x6A, "NumMultiply"),
    (0x6B, "NumAdd"),
    (0x6D, "NumSubtract"),
    (0x6E, "NumDecimal"),
    (0x6F, "NumDivide"),
    (0x90, "NumLock"),
    (0x91, "ScrollLock"),
    (0xA0, "LShift"),
    (0xA1, "RShift"),
    (0xA2, "LCtrl"),
    (0xA3, "RCtrl"),
    (0xA4, "LAlt"),
    (0xA5, "RAlt"),
    (0xBA, "Semicolon"),
    (0xBB, "Equals"),
    (0xBC, "Comma"),
    (0xBD, "Minus"),
    (0xBE, "Period"),
    (0xBF, "Slash"),
    (0xC0, "Backtick"),
    (0xDB, "LBracket"),
    (0xDC, "Backslash"),
    (0xDD, "RBracket"),
    (0xDE, "Quote"),
];

impl Key {
    pub const SHIFT: Key = Key(0x10);
    pub const CTRL: Key = Key(0x11);
    pub const ALT: Key = Key(0x12);
    pub const ESCAPE: Key = Key(0x1B);

    pub fn name(self) -> String {
        let vk = self.0;
        match vk {
            0x30..=0x39 | 0x41..=0x5A => (vk as u8 as char).to_string(),
            0x60..=0x69 => format!("Num{}", vk - 0x60),
            0x70..=0x87 => format!("F{}", vk - 0x6F),
            _ => NAMED
                .iter()
                .find(|(v, _)| *v == vk)
                .map(|(_, n)| n.to_string())
                .unwrap_or_else(|| format!("VK{vk:02X}")),
        }
    }

    pub fn from_name(name: &str) -> Option<Key> {
        let n = name.trim();
        if n.len() == 1 {
            let c = n.chars().next()?.to_ascii_uppercase();
            if c.is_ascii_alphanumeric() {
                return Some(Key(c as u16));
            }
        }
        if n.len() == 4 && n.get(..3).is_some_and(|p| p.eq_ignore_ascii_case("num")) {
            if let Ok(d) = n[3..].parse::<u16>() {
                return Some(Key(0x60 + d));
            }
        }
        if let Some(rest) = n.strip_prefix('F').or_else(|| n.strip_prefix('f')) {
            if let Ok(d) = rest.parse::<u16>() {
                return (1..=24).contains(&d).then_some(Key(0x6F + d));
            }
        }
        if let Some(hex) = n.strip_prefix("VK") {
            return u16::from_str_radix(hex, 16).ok().map(Key);
        }
        NAMED
            .iter()
            .find(|(_, s)| s.eq_ignore_ascii_case(n))
            .map(|(v, _)| Key(*v))
    }

    /// Which modifier (if any) this key is. Left/right variants collapse.
    pub fn modifier(self) -> Option<Modifier> {
        match self.0 {
            0x10 | 0xA0 | 0xA1 => Some(Modifier::Shift),
            0x11 | 0xA2 | 0xA3 => Some(Modifier::Ctrl),
            0x12 | 0xA4 | 0xA5 => Some(Modifier::Alt),
            0x5B | 0x5C => Some(Modifier::Win),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Modifier {
    Ctrl,
    Shift,
    Alt,
    Win,
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name())
    }
}

impl Serialize for Key {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.name())
    }
}

impl<'de> Deserialize<'de> for Key {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Key::from_name(&s).ok_or_else(|| serde::de::Error::custom(format!("unknown key '{s}'")))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MouseButton {
    #[serde(rename = "LeftMouse")]
    Left,
    #[serde(rename = "RightMouse")]
    Right,
    #[serde(rename = "MiddleMouse")]
    Middle,
    #[serde(rename = "Mouse4")]
    X1,
    #[serde(rename = "Mouse5")]
    X2,
}

impl MouseButton {
    pub fn name(self) -> &'static str {
        match self {
            MouseButton::Left => "LeftMouse",
            MouseButton::Right => "RightMouse",
            MouseButton::Middle => "MiddleMouse",
            MouseButton::X1 => "Mouse4",
            MouseButton::X2 => "Mouse5",
        }
    }

    pub fn from_name(name: &str) -> Option<MouseButton> {
        [Self::Left, Self::Right, Self::Middle, Self::X1, Self::X2]
            .into_iter()
            .find(|b| b.name().eq_ignore_ascii_case(name.trim()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_vk_round_trips() {
        for vk in 1..=0xFEu16 {
            let k = Key(vk);
            assert_eq!(Key::from_name(&k.name()), Some(k), "vk {vk:#x} -> {}", k.name());
        }
    }

    #[test]
    fn friendly_names() {
        assert_eq!(Key::from_name("a"), Some(Key(0x41)));
        assert_eq!(Key::from_name("F24"), Some(Key(0x87)));
        assert_eq!(Key::from_name("num5"), Some(Key(0x65)));
        assert_eq!(Key::from_name("space"), Some(Key(0x20)));
        assert_eq!(MouseButton::from_name("mouse4"), Some(MouseButton::X1));
    }
}
