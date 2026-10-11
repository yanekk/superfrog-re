//! Fonts (docs/10 "Fonts").
//!
//! The hall-of-talent and credits text is printed from one 8 × 8 one-plane font in the game
//! executable: the glyphs side by side as a 320-pixel-wide strip of 8 rows (40 bytes a row) at
//! h0+$378ca, in the order of the character string at h0+$322e. The printer (h0+$30fc) clears a
//! cell for the string's first character, a space, and copies glyph `k - 1` for the character at
//! position `k`. Other text (the status panel's digits, the menus) is drawn from the bob sheets and
//! pictures, not from a font.

use crate::error::{AssetError, Why};
use crate::exe::ExeImage;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FontId {
    /// The hall of talent and credits font.
    Hall,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Font {
    pub id: FontId,
    /// The character each glyph draws, in glyph order (ASCII). A space draws nothing.
    pub charset: Vec<u8>,
    /// One per character: 8 rows, bit 7 leftmost.
    pub glyphs: Vec<[u8; 8]>,
}

impl Font {
    /// The glyph for `c`, or `None` (space, or a character the font lacks).
    pub fn glyph(&self, c: u8) -> Option<&[u8; 8]> {
        self.charset
            .iter()
            .position(|&x| x == c)
            .map(|k| &self.glyphs[k])
    }

    pub(crate) fn hall(game: &ExeImage) -> Result<Font, AssetError> {
        let what = "hall-of-talent font";
        let r = game.reader(0, what);
        let mut at = 0x322e;
        if r.u8(at)? != b' ' {
            return Err(r.err(Why::Malformed(
                "the font's character string does not start with a space".into(),
            )));
        }
        let mut charset = Vec::new();
        loop {
            at += 1;
            match r.u8(at)? {
                0 => break,
                c if charset.len() < 40 => charset.push(c),
                _ => {
                    return Err(r.err(Why::Malformed(
                        "font string longer than its 40 glyphs".into(),
                    )));
                }
            }
        }
        let strip = r.bytes(0x378ca, 8 * 40)?;
        let glyphs = (0..charset.len())
            .map(|k| core::array::from_fn(|y| strip[y * 40 + k]))
            .collect();
        Ok(Font {
            id: FontId::Hall,
            charset,
            glyphs,
        })
    }
}
