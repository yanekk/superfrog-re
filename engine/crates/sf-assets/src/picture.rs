//! Planar bitmaps and the full-screen pictures (docs/10 "Pictures").
//!
//! Every bitmap the game keeps is *plane-major*: all of plane 0's rows, then plane 1's, each row
//! `width / 8` bytes, most significant bit leftmost. A pixel's colour index has plane `p` as bit `p`.

use crate::error::{AssetError, Reader, Why};
use crate::gamedata::World;
use crate::palette::Palette;

/// Colour indices, one byte per pixel, row-major.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Bitmap {
    pub width: u16,
    pub height: u16,
    /// Bitplanes: indices are below `1 << depth`.
    pub depth: u8,
    pub pixels: Vec<u8>,
}

impl std::fmt::Debug for Bitmap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bitmap({}x{}x{})", self.width, self.height, self.depth)
    }
}

impl Bitmap {
    /// Bytes a plane-major bitmap of this size takes.
    pub fn planar_len(width: u16, height: u16, depth: u8) -> usize {
        width as usize / 8 * height as usize * depth as usize
    }

    /// Decode a plane-major bitmap from `data` (exactly [`Bitmap::planar_len`] bytes are read).
    pub fn from_planar(data: &[u8], width: u16, height: u16, depth: u8) -> Option<Bitmap> {
        let (w, h) = (width as usize, height as usize);
        let row = w / 8;
        if w % 8 != 0 || depth > 8 || data.len() < row * h * depth as usize {
            return None;
        }
        let mut pixels = vec![0u8; w * h];
        for p in 0..depth as usize {
            let plane = &data[p * row * h..(p + 1) * row * h];
            for (k, &b) in plane.iter().enumerate() {
                if b == 0 {
                    continue;
                }
                let base = k * 8;
                for bit in 0..8 {
                    if b & (0x80 >> bit) != 0 {
                        pixels[base + bit] |= 1 << p;
                    }
                }
            }
        }
        Some(Bitmap {
            width,
            height,
            depth,
            pixels,
        })
    }

    /// The plane-major bytes again: `from_planar(&b.to_planar(), ..) == b`.
    pub fn to_planar(&self) -> Vec<u8> {
        let (w, h) = (self.width as usize, self.height as usize);
        let row = w / 8;
        let mut out = vec![0u8; row * h * self.depth as usize];
        for p in 0..self.depth as usize {
            for (k, px) in self.pixels.iter().enumerate() {
                if px >> p & 1 != 0 {
                    out[p * row * h + k / 8] |= 0x80 >> (k % 8);
                }
            }
        }
        out
    }

    pub fn pixel(&self, x: u16, y: u16) -> u8 {
        self.pixels[y as usize * self.width as usize + x as usize]
    }

    pub(crate) fn read(
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
}

/// Which full-screen picture (docs/10). Named after the game's files.
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
            bitmap: Bitmap::read(r, 0, width, height, depth)?,
            palette: if palette {
                Some(Palette::read(r, len)?)
            } else {
                None
            },
            extra: r.bytes(len + if palette { 64 } else { 0 }, extra)?.to_vec(),
        })
    }
}
