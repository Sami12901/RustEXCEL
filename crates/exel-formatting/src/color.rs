use serde::{Deserialize, Serialize};
use std::fmt;

/// An 8-bit RGB color with optional alpha channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    pub const RED: Self = Self::rgb(255, 0, 0);
    pub const GREEN: Self = Self::rgb(0, 128, 0);
    pub const BLUE: Self = Self::rgb(0, 0, 255);
    pub const YELLOW: Self = Self::rgb(255, 255, 0);
    pub const GRAY: Self = Self::rgb(128, 128, 128);
    pub const LIGHT_GRAY: Self = Self::rgb(211, 211, 211);
    pub const DARK_GRAY: Self = Self::rgb(64, 64, 64);
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Parse a hex color string like "#RRGGBB", "#RGB", "RRGGBB", or "#RRGGBBAA".
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim().strip_prefix('#').unwrap_or(hex.trim());
        match hex.len() {
            3 => {
                let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
                let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
                let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
                Some(Self::rgb(r, g, b))
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Some(Self::rgb(r, g, b))
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
                Some(Self::rgba(r, g, b, a))
            }
            _ => None,
        }
    }

    /// Format as "#RRGGBB" (or "#RRGGBBAA" if not fully opaque).
    pub fn to_hex(&self) -> String {
        if self.a == 255 {
            format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
        } else {
            format!("#{:02X}{:02X}{:02X}{:02X}", self.r, self.g, self.b, self.a)
        }
    }

    /// Returns standard [r, g, b] array.
    pub fn to_rgb_array(&self) -> [u8; 3] {
        [self.r, self.g, self.b]
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::BLACK
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_hex() {
        assert_eq!(Color::from_hex("#FF00AA").unwrap(), Color::rgb(255, 0, 170));
        assert_eq!(Color::from_hex("00FF00").unwrap(), Color::rgb(0, 255, 0));
        assert_eq!(Color::from_hex("#F0A").unwrap(), Color::rgb(255, 0, 170));
        assert_eq!(Color::from_hex("#00000080").unwrap(), Color::rgba(0, 0, 0, 128));
        assert_eq!(Color::rgb(255, 128, 0).to_hex(), "#FF8000");
    }
}
