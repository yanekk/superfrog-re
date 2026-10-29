//! Amiga hunk executables (`0x3F3`): the game's code and the tables in it (docs/08). The reader is
//! the toolkit's (`amiga_disk::hunk`); this names the executables.
//!
//! Hunks are kept as the file has them: contents unrelocated, so a long that the loader relocates
//! reads as an offset into its target hunk, which is how `re/symbols.toml` names addresses
//! (`hN+$off`). Hunks the header declares but the file does not carry (`d1_c03`, `d3_c04`: hunks 2
//! and 3) are kept as absent, sized from the header, because the game's loader still gives them
//! room (docs/08 "Memory map").

pub use amiga_disk::hunk::{Hunk, HunkKind, Reloc};

use crate::error::{AssetError, Reader};
use crate::gamedata::ChunkId;

/// Which executable (docs/08 "The five executables", and the two the game carries packed inside).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExeId {
    /// `d1_c01`: loads and starts the game.
    Bootstrap,
    /// `d1_c03`: the game.
    Game,
    /// `d3_c01`: an unused copy of the bootstrap.
    BootstrapCopy,
    /// `d3_c02`: disk 3's boot, which starts the story intro.
    Disk3Boot,
    /// `d3_c04`: the story intro.
    Intro,
    /// Packed in `d1_c03` at h0+$35fee (`ovl_insert_disk`): the insert-disk prompt.
    InsertDiskOverlay,
    /// Packed in `d1_c03` at h0+$36a04 (`ovl_accessing_disk`): the Accessing-disk overlay.
    AccessingDiskOverlay,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExeImage {
    pub id: ExeId,
    /// The disk chunk it is, or for a packed one, the chunk that carries it.
    pub chunk: ChunkId,
    pub hunks: Vec<Hunk>,
}

impl ExeImage {
    /// `len` bytes of hunk `hunk` from offset `at`, as in the file (unrelocated).
    pub fn bytes(&self, hunk: usize, at: u32, len: usize, what: &str) -> Result<&[u8], AssetError> {
        let data = self.hunks.get(hunk).map(|h| &h.data[..]).unwrap_or(&[]);
        Reader::new(data, self.chunk, what).bytes(at as usize, len)
    }

    pub(crate) fn reader(&self, hunk: usize, what: &str) -> Reader<'_> {
        let data = self.hunks.get(hunk).map(|h| &h.data[..]).unwrap_or(&[]);
        Reader::new(data, self.chunk, what)
    }
}

/// Parse a hunk executable (`amiga_disk::hunk::parse`). Never panics; anything unexpected is an
/// [`AssetError`].
pub fn parse_exe(id: ExeId, chunk: ChunkId, bytes: &[u8]) -> Result<ExeImage, AssetError> {
    let hunks = amiga_disk::hunk::parse(bytes)
        .map_err(|e| AssetError::read(chunk, format!("{id:?} executable"), e))?;
    Ok(ExeImage { id, chunk, hunks })
}
