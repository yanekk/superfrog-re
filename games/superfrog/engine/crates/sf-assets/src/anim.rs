//! IFF ANIM: the end sequence's `END1` and `END2` (d3_c09, d3_c10; games/superfrog/docs/10 "ANIM"). The decoder is
//! the toolkit's (`amiga_disk::anim`: the first frame a ByteRun1 picture, then operation-5 deltas
//! into two alternating buffers); this names the two files and keeps the chunks as T7MP-style tags.

pub use amiga_disk::anim::AnimFrame;

use crate::error::AssetError;
use crate::gamedata::ChunkId;
use crate::palette::Palette;
use crate::t7mp::Tag;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnimId {
    /// `END1` (d3_c09).
    End1,
    /// `END2` (d3_c10).
    End2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anim {
    pub id: AnimId,
    /// From `CMAP`, each 8-bit component's top nibble (the file's low nibbles are zero).
    pub palette: Palette,
    /// The first frame's chunks other than `BMHD`, `CMAP` and `BODY` (`DPPS`, `CRNG`, `DPAN`,
    /// `CAMG`), as stored.
    pub chunks: Vec<(Tag, Vec<u8>)>,
    pub frames: Vec<AnimFrame>,
}

/// Decode a `FORM ANIM`. Never panics; anything outside what the two files use is refused.
pub fn decode_anim(id: AnimId, chunk_id: ChunkId, bytes: &[u8]) -> Result<Anim, AssetError> {
    let a = amiga_disk::anim::anim(bytes)
        .map_err(|e| AssetError::read(chunk_id, format!("{id:?} animation"), e))?;
    Ok(Anim {
        id,
        palette: a.palette,
        chunks: a.chunks.into_iter().map(|(t, p)| (Tag(t), p)).collect(),
        frames: a.frames,
    })
}
