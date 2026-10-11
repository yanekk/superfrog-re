//! Bob and sprite sheets (docs/10 "Sheets"): plane-major bitmaps 320 pixels wide that the game
//! cuts its frames from. Which rectangle is which frame is in the executable's object and animation
//! tables, decoded by the mechanic tasks that use them.

use crate::error::{AssetError, Reader};
use crate::gamedata::World;
use crate::picture::Bitmap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SpriteId {
    /// `HERO` (d1_c04): the frog, 320 × 384, 4 planes.
    Hero,
    /// `LnBO`: a world's bobs, 320 × 320, 5 planes (W1..W6, the bonus and the final level).
    Bobs(World),
    /// `FRUC` (d1_c12): the slot machine's reels, 320 × 384, 4 planes.
    SlotReels,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpriteBank {
    pub id: SpriteId,
    pub sheet: Bitmap,
}

impl SpriteBank {
    pub(crate) fn read(r: &Reader, id: SpriteId) -> Result<SpriteBank, AssetError> {
        let (h, d) = match id {
            SpriteId::Hero | SpriteId::SlotReels => (384, 4),
            SpriteId::Bobs(_) => (320, 5),
        };
        r.size(Bitmap::planar_len(320, h, d))?;
        Ok(SpriteBank {
            id,
            sheet: Bitmap::read(r, 0, 320, h, d)?,
        })
    }
}
