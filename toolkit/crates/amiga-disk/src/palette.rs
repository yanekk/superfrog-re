//! The OCS palette: 32 colour registers of 12-bit `0x0RGB`.

use crate::error::ReadError;
use crate::read::Reader;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Palette(pub [u16; 32]);

impl Palette {
    /// 32 big-endian words at `at`, as COLOR00..31 take them.
    pub fn read(bytes: &[u8], at: usize) -> Result<Palette, ReadError> {
        let r = Reader::new(bytes);
        let mut p = [0u16; 32];
        for (k, c) in p.iter_mut().enumerate() {
            *c = r.u16(at + 2 * k)?;
        }
        Ok(Palette(p))
    }

    /// Colour `index` as 8-bit RGB (nibble × 17).
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
