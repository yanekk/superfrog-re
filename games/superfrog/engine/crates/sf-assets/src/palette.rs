//! The OCS palette: 32 colour registers of 12-bit `0x0RGB` (games/superfrog/docs/06 "Palettes"). The type is the
//! toolkit's (`amiga_disk::palette`); `Palette::rgb888` uses the expansion `games/superfrog/tools/render_levels.py`
//! uses (nibble × 17).

pub use amiga_disk::palette::Palette;

use crate::error::{AssetError, Reader};

/// 32 big-endian words at `at`, as the game keeps them (and as COLOR00..31 take them).
pub(crate) fn read_palette(r: &Reader, at: usize) -> Result<Palette, AssetError> {
    Palette::read(r.data, at).map_err(|e| r.read_err(e))
}
