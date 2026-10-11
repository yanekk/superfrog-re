//! The OCS palette: 32 colour registers of 12-bit `0x0RGB` (docs/06 "Palettes").

use crate::error::AssetError;
use crate::error::Reader;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Palette(pub [u16; 32]);

impl Palette {
    /// 32 big-endian words at `at`, as the game keeps them (and as COLOR00..31 take them).
    pub(crate) fn read(r: &Reader, at: usize) -> Result<Palette, AssetError> {
        let mut p = [0u16; 32];
        for (k, c) in p.iter_mut().enumerate() {
            *c = r.u16(at + 2 * k)?;
        }
        Ok(Palette(p))
    }

    /// Colour `index` as 8-bit RGB: each nibble times 17, the expansion `tools/render_levels.py` uses.
    pub fn rgb888(&self, index: u8) -> [u8; 3] {
        rgb888(self.0[index as usize & 31])
    }
}

/// A 12-bit `0x0RGB` colour as 8-bit RGB (nibble × 17).
pub fn rgb888(c: u16) -> [u8; 3] {
    [
        (c >> 8 & 15) as u8 * 17,
        (c >> 4 & 15) as u8 * 17,
        (c & 15) as u8 * 17,
    ]
}
