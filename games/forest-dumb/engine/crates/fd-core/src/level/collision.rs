//! The collision plane: `collision_map` (`h3+$fa50`), one bit a pixel, 256 rows of `$2e` bytes, row
//! `y & $ff` at the offset `collision_rows` (`h0+$37b08`) gives (docs/09-play-measured.md, "Are the
//! display buffers game state?"). The foreground's edge redraws write it (T55) and objects change it
//! (T25), so it is play state, not the level file's collision map.
//!
//! A pixel's byte is its row's offset plus `(x >> 4) << 1` plus the half word, with no wrap at the
//! row's end: `x` is a level coordinate, so past 368 pixels a read lands in later rows, and near the
//! bottom row past the map. The original reads there; so does [`CollisionPlane`], which holds the
//! map and [`TAIL`] bytes after it, enough for every `x` a level reaches (`$1800` and less).

use std::sync::Arc;

use crate::program::{HUNK_BASES, Program};

/// `collision_map`'s hunk 3 offset and size (`re/symbols.toml`).
pub const MAP: u32 = 0xfa50;
pub const MAP_BYTES: u32 = 0x2e00;
/// Bytes after the map a probe reaches with `x` below `$1800`: row 255's offset `$2dd2` plus
/// `($1800 + $24 - $10) >> 4 << 1` plus a long, past `$2e00`.
pub const TAIL: u32 = 0x400;
/// `collision_rows`' hunk 0 offset.
pub const ROWS: u32 = 0x3_7b08;

/// The plane's bytes and the row table, as the original holds them.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CollisionPlane {
    mem: Arc<Vec<u8>>,
    /// Index of `collision_map`'s first byte in `mem`.
    at: u32,
    rows: Arc<[u32; 256]>,
}

impl CollisionPlane {
    /// An empty plane (every pixel clear), rows from the program's `collision_rows`.
    pub fn new(p: &Program) -> CollisionPlane {
        CollisionPlane {
            mem: Arc::new(vec![0; (MAP_BYTES + TAIL) as usize]),
            at: 0,
            rows: Arc::new(rows(p)),
        }
    }

    /// Over all of a 24-bit address space with the map at `h3+$fa50`: the routine rig's memory.
    pub fn memory(mem: Arc<Vec<u8>>, p: &Program) -> CollisionPlane {
        CollisionPlane {
            mem,
            at: HUNK_BASES[3] + MAP,
            rows: Arc::new(rows(p)),
        }
    }

    /// The plane's bytes from the map's first, [`MAP_BYTES`] + [`TAIL`] of them, to be laid from
    /// the original (a transplant) or by the edge redraws.
    pub fn bytes_mut(&mut self) -> &mut [u8] {
        let at = self.at as usize;
        let n = (MAP_BYTES + TAIL) as usize;
        &mut Arc::make_mut(&mut self.mem)[at..at + n]
    }

    fn byte(&self, off: u32) -> u8 {
        let i = self.at.wrapping_add(off) & 0xff_ffff;
        self.mem.get(i as usize).copied().unwrap_or(0)
    }

    /// The row table's entry for row `y & $ff`.
    pub fn row(&self, y: u32) -> u32 {
        self.rows[(y & 0xff) as usize]
    }

    /// Where the original reads the long holding pixel (`x`, `y`): `rows[y & $ff] + (x >> 4 << 1)`,
    /// from the map's first byte.
    pub fn offset(&self, x: u32, y: u32) -> u32 {
        self.row(y).wrapping_add((x >> 4) << 1)
    }

    /// The probe's window: the long at [`CollisionPlane::offset`] shifted left by `x & 15`, its
    /// high word. Bit 15 is pixel `x`, bit 15 - k pixel `x + k`.
    pub fn window(&self, x: u32, y: u32) -> u16 {
        let off = self.offset(x, y);
        let long = (0..4).fold(0u32, |v, k| v << 8 | self.byte(off.wrapping_add(k)) as u32);
        ((long << (x & 15)) >> 16) as u16
    }
}

/// Whether pixel (`x`, `y`) of the plane is solid, as the probe reads it.
pub fn solid(c: &CollisionPlane, x: u32, y: u32) -> bool {
    c.window(x, y) & 0x8000 != 0
}

fn rows(p: &Program) -> [u32; 256] {
    std::array::from_fn(|r| p.long(ROWS + 4 * r as u32))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A program whose row table is `r * $2e`, as the original's is.
    fn program() -> Program {
        let mut h0 = vec![0; (ROWS + 0x400) as usize];
        for r in 0..256u32 {
            let at = (ROWS + 4 * r) as usize;
            h0[at..at + 4].copy_from_slice(&(r * 0x2e).to_be_bytes());
        }
        Program::hunk0(h0)
    }

    #[test]
    fn a_pixel_is_its_rows_bit_and_rows_wrap_at_256() {
        let mut c = CollisionPlane::new(&program());
        // Row 3, x 20: byte 3 * $2e + 2, bit 7 - 4.
        c.bytes_mut()[3 * 0x2e + 2] = 0x08;
        assert!(solid(&c, 20, 3));
        assert!(solid(&c, 20, 259));
        assert!(!solid(&c, 19, 3) && !solid(&c, 21, 3));
        assert_eq!(c.window(16, 3), 0x0800);
        assert_eq!(c.window(19, 3), 0x4000);
        // x past the row runs on into the next one.
        c.bytes_mut()[4 * 0x2e] = 0x80;
        assert!(solid(&c, 0x2e * 8, 3));
    }

    #[test]
    fn the_window_spans_the_half_words() {
        let mut c = CollisionPlane::new(&program());
        c.bytes_mut()[2] = 0x01; // pixel 23 of row 0
        assert_eq!(c.window(15, 0), 0x0080);
        assert_eq!(c.window(31, 0) & 0x8000, 0);
        assert!(solid(&c, 23, 0));
    }
}
