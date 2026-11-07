//! The sprite sheets a world loads (`docs/06-graphics-and-rendering.md`, "Sprite sets"). Frames sit
//! side by side in a five-plane, line-interleaved bitmap, like a screen. What each frame is used for
//! is the porting tasks' to find; this only reads and checks the sheets.

use crate::error::AssetError;
use crate::level::sized;
use crate::{Files, PLANES};

/// One sheet: its picture and, when it has one, its mask, the same size with one plane repeated
/// five times. The world-specific pictures carry their world's background and have no mask.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sheet {
    pub file: String,
    pub picture: Vec<u8>,
    pub mask: Option<Vec<u8>>,
    /// Bytes a line of one plane.
    pub line_bytes: u16,
    pub frame_width: u16,
    pub frame_height: u16,
}

impl Sheet {
    /// Lines in the sheet.
    pub fn lines(&self) -> usize {
        self.picture.len() / (self.line_bytes as usize * PLANES)
    }

    /// The colour number (0 to 31) of pixel (`x`, `y`) of the whole sheet.
    pub fn pixel(&self, x: usize, y: usize) -> u8 {
        let line = self.line_bytes as usize;
        let mut colour = 0;
        for plane in 0..PLANES {
            let at = (y * PLANES + plane) * line + x / 8;
            colour |= (self.picture[at] >> (7 - x % 8) & 1) << plane;
        }
        colour
    }
}

/// The sheets of one world, by the file each comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sprites {
    /// `StworkiNew.rawm`, `.mask`: 32 x 32 frames, 70 in rows of 10.
    pub creatures: Sheet,
    /// `_ELEMENTY.RAWM`, `.MASK`: 16 x 16 frames, two rows of 20.
    pub elements: Sheet,
    /// `_PlomienN`: two 32 x 32 frames.
    pub flame: Sheet,
    /// `_ZNIKAJKAN`: six 32 x 16 frames.
    pub vanishing: Sheet,
    /// `_WYBIJACZKA`, `2`, `3`: one 32 x 32 frame (loaded twice by the game; one copy here).
    pub knocker: Sheet,
}

/// The world-specific set's number: 1 in worlds 1 and 2, 2 in 3 and 4, 3 in 5 and 6.
fn set(world: u8) -> u8 {
    world.saturating_sub(1) / 2 + 1
}

/// The sprite files world `world` loads, picture then mask where it has one.
pub fn sprite_file_names(world: u8) -> [String; 7] {
    let n = set(world);
    let knocker = if n == 1 { String::new() } else { n.to_string() };
    [
        "StworkiNew.rawm".into(),
        "StworkiNew.mask".into(),
        "_ELEMENTY.RAWM".into(),
        "_ELEMENTY.MASK".into(),
        format!("_Plomien{n}"),
        format!("_ZNIKAJKA{n}"),
        format!("_WYBIJACZKA{knocker}"),
    ]
}

/// The sheets of world `world` (1 to 6), from `files`. Each must be exactly its size: the frame
/// count chapter 06 gives, and the size of the game's buffer it loads into.
pub fn read_sprites(world: u8, files: &Files) -> Result<Sprites, AssetError> {
    if !(1..=6).contains(&world) {
        return Err(AssetError::missing(
            &format!("world {world}"),
            "there are worlds 1 to 6",
        ));
    }
    let [cr, crm, el, elm, fl, va, kn] = sprite_file_names(world);
    // (bytes a line, frame width, frame height, lines in the sheet)
    let sheet = |file: &str, mask: Option<&str>, dims: (u16, u16, u16, usize)| {
        let (line_bytes, frame_width, frame_height, lines) = dims;
        let size = line_bytes as usize * PLANES * lines;
        let picture = files(file)?;
        sized(file, &picture, size)?;
        let mask = match mask {
            Some(m) => {
                let bytes = files(m)?;
                sized(m, &bytes, size)?;
                Some(bytes)
            }
            None => None,
        };
        Ok::<_, AssetError>(Sheet {
            file: file.to_string(),
            picture,
            mask,
            line_bytes,
            frame_width,
            frame_height,
        })
    };
    Ok(Sprites {
        creatures: sheet(&cr, Some(&crm), (40, 32, 32, 7 * 32))?,
        elements: sheet(&el, Some(&elm), (40, 16, 16, 2 * 16))?,
        flame: sheet(&fl, None, (8, 32, 32, 32))?,
        vanishing: sheet(&va, None, (24, 32, 16, 16))?,
        knocker: sheet(&kn, None, (4, 32, 32, 32))?,
    })
}
