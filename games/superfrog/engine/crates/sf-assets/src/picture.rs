//! Planar bitmaps and the full-screen pictures (games/superfrog/docs/10 "Pictures").
//!
//! Every bitmap the game keeps is *plane-major* (`amiga_disk::planar`, whose [`Bitmap`] this is):
//! all of plane 0's rows, then plane 1's. The pictures are raw planes and a palette, not IFF.

use crate::error::{AssetError, Reader, Why};
use crate::gamedata::World;
use crate::palette::{Palette, read_palette};

pub use amiga_disk::planar::Bitmap;

/// A plane-major bitmap of `width × height × depth` at `at` in `r`.
pub(crate) fn read_bitmap(
    r: &Reader,
    at: usize,
    width: u16,
    height: u16,
    depth: u8,
) -> Result<Bitmap, AssetError> {
    let len = Bitmap::planar_len(width, height, depth);
    let data = r.bytes(at, len)?;
    Bitmap::from_planar(data, width, height, depth).ok_or_else(|| {
        r.err(Why::Malformed(format!(
            "not a {width}x{height}x{depth} bitmap"
        )))
    })
}

/// Which full-screen picture (games/superfrog/docs/10). Named after the game's files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PictureId {
    /// `MPIC` (d1_c06): the title and its menu. 6 planes; the palette is the copper's.
    Title,
    /// `HPIC` (d1_c07): the hall of talent, 288 lines.
    HallOfTalent,
    /// `AWA1` (d1_c08): the features table.
    Awards1,
    /// `AWA2` (d1_c09): the bonus table.
    Awards2,
    /// `LnLP`: the world loading picture, 352 lines (W1..W6 and the bonus level).
    Loading(World),
    /// `LnET`: the end-of-world picture (W1..W6).
    EndOfWorld(World),
    /// `FRUT` (d1_c11): the slot machine. 6 planes; the palette is the copper's.
    SlotMachine,
    /// `FRUM` (d1_c13): one plane over the slot machine picture: its lit areas.
    SlotMachineMask,
    /// Packed in `d1_c03` at h0+$37a0a (`pic_progress_map`): Superfrog's progress map.
    ProgressMap,
}

/// A full-screen picture: its bitmap, the palette it carries (if the file has one), and any bytes
/// after the palette the format does not explain yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    pub bitmap: Bitmap,
    pub palette: Option<Palette>,
    pub extra: Vec<u8>,
}

impl Picture {
    /// `width × height × depth` plane-major, then (if `palette`) 32 colours, then `extra` bytes,
    /// which must be all that is left.
    pub(crate) fn read(
        r: &Reader,
        width: u16,
        height: u16,
        depth: u8,
        palette: bool,
        extra: usize,
    ) -> Result<Picture, AssetError> {
        let len = Bitmap::planar_len(width, height, depth);
        r.size(len + if palette { 64 } else { 0 } + extra)?;
        Ok(Picture {
            bitmap: read_bitmap(r, 0, width, height, depth)?,
            palette: if palette {
                Some(read_palette(r, len)?)
            } else {
                None
            },
            extra: r.bytes(len + if palette { 64 } else { 0 }, extra)?.to_vec(),
        })
    }
}
