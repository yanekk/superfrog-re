//! What the front end reads from the disks, built once: the game's hunk 0 as the loader leaves it
//! in memory (relocated), the files the title loads, the packed pictures and the overlay programs.

use sf_assets::sound::SoundData;
use sf_assets::{ExeId, ExeImage, GameData};

use crate::layout::{H0, H1, H2, H3, h3};

/// Where the Accessing-disk overlay program is unpacked and run (`h0+$b6f0` holds it).
pub const OVERLAY_AT: u32 = h3(0x3a560);

/// The front end's disk content.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FrontData {
    /// Hunk 0 of the game as loaded at [`H0`]: code and data, every pointer relocated.
    pub h0: Vec<u8>,
    /// The Accessing-disk overlay (`ovl_accessing_disk`), relocated to [`OVERLAY_AT`].
    pub overlay: Vec<u8>,
    /// Files by the game's name, decompressed as `load_file` leaves them: `MPIC`, `HPIC`, ...
    pub files: Vec<(String, Vec<u8>)>,
    /// Everything the sound driver reads.
    pub sound: Option<SoundData>,
}

impl FrontData {
    pub fn new(data: &GameData) -> FrontData {
        let exe = |id| data.exes.iter().find(|e: &&ExeImage| e.id == id);
        let h0 = exe(ExeId::Game)
            .map(|g| relocate(g, 0, &[H0, H1, H2, H3]))
            .unwrap_or_default();
        // `hunk_relocate` (h0+$cfba) moves the hunk down to the buffer's start over the unpacked
        // file, whose tail stays behind it.
        let mut overlay = exe(ExeId::AccessingDiskOverlay)
            .map(|o| relocate(o, 0, &[OVERLAY_AT]))
            .unwrap_or_default();
        if let Some(file) = data.raw.get("h0@36a04") {
            overlay.extend(file.get(overlay.len()..).unwrap_or(&[]));
        }
        let files = data
            .raw
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        FrontData {
            h0,
            overlay,
            files,
            sound: SoundData::from_game(data).ok(),
        }
    }

    /// A file's bytes, empty when the data lacks it (a test's empty `GameData`).
    pub fn file(&self, name: &str) -> &[u8] {
        self.files
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| &v[..])
            .unwrap_or(&[])
    }

    /// The byte at `h0+$off` as loaded, 0 past the file's data (BSS).
    pub fn h0_b(&self, off: u32) -> u8 {
        self.h0.get(off as usize).copied().unwrap_or(0)
    }

    pub fn h0_w(&self, off: u32) -> u16 {
        u16::from_be_bytes([self.h0_b(off), self.h0_b(off + 1)])
    }

    pub fn h0_l(&self, off: u32) -> u32 {
        (self.h0_w(off) as u32) << 16 | self.h0_w(off + 2) as u32
    }

    /// `len` bytes at `h0+$off` as loaded.
    pub fn h0_bytes(&self, off: u32, len: u32) -> Vec<u8> {
        (off..off + len).map(|a| self.h0_b(a)).collect()
    }
}

/// Hunk `hunk` of `exe` as the loader leaves it: its data padded to its size, each relocated long
/// plus its target hunk's base from `bases`.
pub(crate) fn relocate(exe: &ExeImage, hunk: usize, bases: &[u32]) -> Vec<u8> {
    let Some(h) = exe.hunks.get(hunk) else {
        return Vec::new();
    };
    let mut out = h.data.clone();
    out.resize(out.len().max(h.size as usize), 0);
    for r in &h.relocs {
        let base = bases.get(r.target as usize).copied().unwrap_or(0);
        for &o in &r.offsets {
            let o = o as usize;
            if let Some(b) = out.get_mut(o..o + 4) {
                let v = u32::from_be_bytes([b[0], b[1], b[2], b[3]]).wrapping_add(base);
                b.copy_from_slice(&v.to_be_bytes());
            }
        }
    }
    out
}
