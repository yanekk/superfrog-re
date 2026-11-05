//! `collision_map` (`h3+$fa50`): what the character's probes read to tell solid from empty
//! (games/forest-dumb/docs/09-play-measured.md, "Are the display buffers game state?"). One bit a
//! pixel, rows of `$2e` bytes (23 cells of 16 pixels), 256 rows, row `y & $ff`: a ring the view
//! scrolls through vertically. Horizontally it does not wrap: the edge redraws write a cell's shape at
//! `(col * 2)` bytes from its band's start, so as the camera moves right the writes march on past the
//! map's `$2e00` bytes into the slack before the tiles at `h3+$13540` (T55; `level/redraw.rs`).
//!
//! Shared with T08, whose probes read it. Built incrementally as the view scrolls, by the edge redraws
//! (T55) and by `game_frame`'s animated cells (T25), so the engine keeps it as the original does.

/// Bytes a row.
pub const ROW_BYTES: u32 = 0x2e;
/// Rows in the ring.
pub const ROWS: u32 = 256;
/// The map's hunk 3 offset.
pub const ORIGIN: u32 = 0xfa50;
/// Bytes kept before [`ORIGIN`]: the left edge redraw writes its first cell's word 2 bytes before its
/// band (`redraw_edge_c`'s `suba.l #2`), which for band 0 is before the map.
const BEFORE: u32 = 0x40;
/// Bytes kept from [`ORIGIN`]: the map and the slack after it up to the tiles (`h3+$13540`). A
/// redraw at the camera's largest x (`$17cf`, column 380) writes up to `$311a` bytes in.
pub const KEPT: u32 = 0x13540 - ORIGIN;

/// `collision_map` and the slack around it, as bytes. A write outside what is kept is dropped: the
/// original would land it on the tiles or before the map, which only positions past the camera's
/// clamps reach (the routine rig's adapters write those straight into the original's memory).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CollisionMap {
    bytes: Vec<u8>,
}

impl Default for CollisionMap {
    fn default() -> CollisionMap {
        CollisionMap {
            bytes: vec![0; (BEFORE + KEPT) as usize],
        }
    }
}

impl CollisionMap {
    /// Where `at` (bytes from [`ORIGIN`], wrapping: `-2` is `$ffff_fffe`) sits in `bytes`.
    fn index(at: u32) -> Option<usize> {
        let i = at.wrapping_add(BEFORE);
        (i < BEFORE + KEPT).then_some(i as usize)
    }

    /// The byte at `at` bytes from [`ORIGIN`]; 0 outside what is kept.
    pub fn byte(&self, at: u32) -> u8 {
        Self::index(at).map_or(0, |i| self.bytes[i])
    }

    /// Set the byte at `at`, if kept.
    pub fn set_byte(&mut self, at: u32, v: u8) {
        if let Some(i) = Self::index(at) {
            self.bytes[i] = v;
        }
    }

    /// Write a big-endian word at `at`, byte by byte as kept.
    pub fn set_word(&mut self, at: u32, v: u16) {
        let [hi, lo] = v.to_be_bytes();
        self.set_byte(at, hi);
        self.set_byte(at.wrapping_add(1), lo);
    }

    /// Every kept byte with its offset from [`ORIGIN`] (wrapping), for a transplant or a test that
    /// loads or compares the whole map.
    pub fn offsets() -> impl Iterator<Item = u32> {
        (0..BEFORE + KEPT).map(|i| i.wrapping_sub(BEFORE))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_land_big_endian_and_outside_is_dropped() {
        let mut m = CollisionMap::default();
        m.set_word(0, 0x1234);
        assert_eq!((m.byte(0), m.byte(1)), (0x12, 0x34));
        // Two bytes before the map: kept.
        m.set_word(0u32.wrapping_sub(2), 0xabcd);
        assert_eq!(m.byte(0u32.wrapping_sub(2)), 0xab);
        // On the tiles: dropped, reads 0.
        m.set_word(KEPT, 0xffff);
        assert_eq!(m.byte(KEPT), 0);
        assert_eq!(CollisionMap::offsets().count(), (BEFORE + KEPT) as usize);
    }
}
