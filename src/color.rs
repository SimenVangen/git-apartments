//! Colors.

use std::fmt;
use std::str::FromStr;

use crate::Error;

/// An 8-bit RGB color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    /// Builds a color from a `0xRRGGBB` literal.
    pub const fn hex(v: u32) -> Self {
        Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
    }

    /// Perceived brightness from 0 (black) to 255 (white).
    pub fn luma(self) -> u8 {
        let Rgb(r, g, b) = self;
        ((r as u32 * 299 + g as u32 * 587 + b as u32 * 114) / 1000) as u8
    }

    /// Scales each channel by `factor` (below 1 darkens, above 1 lightens).
    pub fn scale(self, factor: f32) -> Self {
        let f = |c: u8| (c as f32 * factor).round().clamp(0.0, 255.0) as u8;
        Rgb(f(self.0), f(self.1), f(self.2))
    }

    /// Darkens a color until it stands out against snow.
    ///
    /// Language colors can be close to white (some are pale yellows or
    /// greys), which would vanish against a snowy street.
    pub fn against_snow(self) -> Self {
        let mut c = self;
        while c.luma() > 185 {
            c = c.scale(0.85);
        }
        c
    }
}

impl FromStr for Rgb {
    type Err = Error;

    /// Parses `#rrggbb` or `rrggbb`.
    fn from_str(s: &str) -> Result<Self, Error> {
        let h = s.trim().trim_start_matches('#');
        if h.len() != 6 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(Error::Parse(format!("not a #rrggbb color: {s:?}")));
        }
        let v = u32::from_str_radix(h, 16).map_err(|e| Error::Parse(e.to_string()))?;
        Ok(Rgb::hex(v))
    }
}

impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_prints() {
        let c: Rgb = "#DEA584".parse().unwrap();
        assert_eq!(c, Rgb(0xde, 0xa5, 0x84));
        assert_eq!(c.to_string(), "#dea584");
        assert!("#12345".parse::<Rgb>().is_err());
        assert!("zzzzzz".parse::<Rgb>().is_err());
    }

    #[test]
    fn pale_colors_are_darkened() {
        let pale = Rgb(250, 250, 240);
        assert!(pale.against_snow().luma() <= 185);
        let red = Rgb(200, 30, 30);
        assert_eq!(red.against_snow(), red);
    }
}
