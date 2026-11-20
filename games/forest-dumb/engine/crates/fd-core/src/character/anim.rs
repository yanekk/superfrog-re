//! The character's animation frame pickers (T51): the routines that set `char_anim_ptr`, the frame
//! `char_sprite_copy` copies into sprites 0-3. Ported from `char_step_4be6`, `h0_4c28`, `h0_4c6a`,
//! `h0_4d32` and `h0_4d54` (`h0+$4be6`..`$4d74`). docs/09-play-measured.md, "The animation
//! pickers and the joystick read", lists what each reads and why none can be stopped partway.
//!
//! A frame is a pointer the original reads from tables in its own hunk 0 (`h0+$4edc`..`$4f63`), so
//! [`AnimTables`] holds those bytes as the program was loaded: a frame's value is whatever the
//! loader relocated it to, and the engine copies it as the original does. Loaded at the oracle's
//! bases (`fd-conform`'s `HUNK_BASES`), `char_anim_ptr` compares equal with the original's.

use core::ops::Range;

/// The bytes of hunk 0 the pickers read, by hunk 0 offset. The walk tables are indexed by
/// `char_walk_t >> 3`, which reaches 31 for a counter the game never holds (`$a8`..`$ff`), so the
/// window runs to `h0+$4f30 + 31 * 4 + 4`, past the tables into the code after them, where the
/// original would read too.
pub const TABLES: Range<u32> = 0x4edc..0x4fb0;

/// The walk tables: right (`h0_4c28`) and left (`char_step_4be6`).
pub const WALK_RIGHT: u32 = 0x4edc;
pub const WALK_LEFT: u32 = 0x4f30;

/// The pointer tables of hunk 0, as loaded (see the module note).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AnimTables {
    bytes: Vec<u8>,
}

impl AnimTables {
    /// The tables from hunk 0's bytes as loaded, `hunk0[0]` being `h0+$0`. An error if hunk 0 is too
    /// short to hold them.
    pub fn from_hunk0(hunk0: &[u8]) -> Result<AnimTables, String> {
        let (from, to) = (TABLES.start as usize, TABLES.end as usize);
        hunk0
            .get(from..to)
            .map(|b| AnimTables { bytes: b.to_vec() })
            .ok_or_else(|| {
                format!(
                    "hunk 0 is {:#x} bytes; the animation tables need {to:#x}",
                    hunk0.len()
                )
            })
    }

    /// The long at hunk 0 offset `off`, big-endian. `off` is inside [`TABLES`] for every counter
    /// a byte can hold.
    pub fn long(&self, off: u32) -> u32 {
        let i = (off - TABLES.start) as usize;
        u32::from_be_bytes([
            self.bytes[i],
            self.bytes[i + 1],
            self.bytes[i + 2],
            self.bytes[i + 3],
        ])
    }
}

/// The animation state the pickers read and write, at the original's widths.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Anim {
    /// `char_walk_t` (`h0+$4be4`): the walk frame counter, 0 to `$27` in play.
    pub walk_t: u8,
    /// `char_facing` (`h0+$4ece`): `$ff` left, 1 right. The pickers test its sign only.
    pub facing: u8,
    /// `char_probe_1` (`h0+$4ed1`), which `char_probe_flags` writes.
    pub probe_1: u8,
    /// `char_probe_2` (`h0+$4ed2`); the pickers test bits 0 and 1.
    pub probe_2: u8,
    /// `char_anim_ptr` (`h0+$4ed8`): the frame, as the tables hold it.
    pub ptr: u32,
}

impl Anim {
    fn left(&self) -> bool {
        self.facing & 0x80 != 0
    }

    /// One walk step: face, count, and take the counter's eighth from `table`.
    fn walk(&mut self, facing: u8, table: u32, t: &AnimTables) {
        self.facing = facing;
        self.walk_t = self.walk_t.wrapping_add(1);
        // `cmpi.b #$28; bmi`: the counter is kept while the byte difference is negative, which is
        // not "below $28" unsigned: $a8..$ff are kept too. The game only counts up from 0, so it
        // wraps at $28.
        if self.walk_t.wrapping_sub(0x28) & 0x80 == 0 {
            self.walk_t = 0;
        }
        self.ptr = t.long(table + 4 * (self.walk_t >> 3) as u32);
    }

    /// `char_step_4be6`: a walk step facing left.
    pub fn walk_left(&mut self, t: &AnimTables) {
        self.walk(0xff, WALK_LEFT, t);
    }

    /// `h0_4c28`: a walk step facing right.
    pub fn walk_right(&mut self, t: &AnimTables) {
        self.walk(1, WALK_RIGHT, t);
    }

    /// `h0_4c6a`: a frame chosen by the probe bytes and the facing. When neither the right bit nor
    /// value of `probe_1` matches, the frame is left as it was.
    pub fn frame_from_probes(&mut self, t: &AnimTables) {
        let (p1, p2) = (self.probe_1, self.probe_2);
        let a = p2 & 2 != 0 || p1 == 2;
        let b = p2 & 1 != 0 || p1 == 3;
        // Facing right `a` is tested first, facing left `b`.
        let frame = match (p1 == 1, self.left()) {
            (true, false) => Some(0x4f04),
            (true, true) => Some(0x4f58),
            (false, false) if a => Some(0x4f08),
            (false, false) if b => Some(0x4f0c),
            (false, true) if b => Some(0x4f5c),
            (false, true) if a => Some(0x4f60),
            (false, _) => None,
        };
        if let Some(off) = frame {
            self.ptr = t.long(off);
        }
    }

    /// `h0_4d32`: frame `h0+$4ef0`, or `h0+$4f44` facing left.
    pub fn frame_4d32(&mut self, t: &AnimTables) {
        self.ptr = t.long(if self.left() { 0x4f44 } else { 0x4ef0 });
    }

    /// `h0_4d54`: frame `h0+$4ef4`, or `h0+$4f48` facing left.
    pub fn frame_4d54(&mut self, t: &AnimTables) {
        self.ptr = t.long(if self.left() { 0x4f48 } else { 0x4ef4 });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tables whose long at each offset is the offset itself, so a test reads which entry was taken.
    fn tables() -> AnimTables {
        let mut h0 = vec![0; TABLES.end as usize];
        for off in (TABLES.start..TABLES.end).step_by(4) {
            h0[off as usize..off as usize + 4].copy_from_slice(&off.to_be_bytes());
        }
        AnimTables::from_hunk0(&h0).unwrap()
    }

    #[test]
    fn a_short_hunk_is_an_error() {
        let e = AnimTables::from_hunk0(&[0; 0x4f00]).unwrap_err();
        assert!(e.contains("0x4fb0"), "{e}");
    }

    #[test]
    fn the_walk_counter_wraps_at_0x28_and_keeps_a8_to_ff() {
        let t = tables();
        let step = |walk_t: u8| {
            let mut a = Anim {
                walk_t,
                ..Anim::default()
            };
            a.walk_right(&t);
            (a.walk_t, a.ptr, a.facing)
        };
        assert_eq!(step(0), (1, WALK_RIGHT, 1));
        assert_eq!(step(0x0f), (0x10, WALK_RIGHT + 8, 1));
        assert_eq!(step(0x27), (0, WALK_RIGHT, 1));
        assert_eq!(step(0x50), (0, WALK_RIGHT, 1));
        // N set after the compare: kept, and read 31 entries in.
        assert_eq!(step(0xfe), (0xff, WALK_RIGHT + 31 * 4, 1));
        let mut a = Anim::default();
        a.walk_left(&t);
        assert_eq!((a.walk_t, a.ptr, a.facing), (1, WALK_LEFT, 0xff));
    }

    #[test]
    fn the_probe_frame_follows_the_listing_order() {
        let t = tables();
        let pick = |p1: u8, p2: u8, facing: u8| {
            let mut a = Anim {
                probe_1: p1,
                probe_2: p2,
                facing,
                ptr: 7,
                ..Anim::default()
            };
            a.frame_from_probes(&t);
            a.ptr
        };
        assert_eq!(pick(1, 3, 1), 0x4f04);
        assert_eq!(pick(1, 3, 0xff), 0x4f58);
        // Both bits: right takes bit 1's frame, left bit 0's.
        assert_eq!(pick(0, 3, 1), 0x4f08);
        assert_eq!(pick(0, 3, 0x80), 0x4f5c);
        assert_eq!(pick(3, 0, 0), 0x4f0c);
        assert_eq!(pick(2, 0, 0xff), 0x4f60);
        // Nothing matches: the frame stays.
        assert_eq!(pick(0, 0, 1), 7);
        assert_eq!(pick(4, 0xfc, 0xff), 7);
    }

    #[test]
    fn the_two_poses_test_the_sign_of_facing() {
        let t = tables();
        for (facing, a, b) in [
            (0, 0x4ef0, 0x4ef4),
            (0x7f, 0x4ef0, 0x4ef4),
            (0x80, 0x4f44, 0x4f48),
        ] {
            let mut s = Anim {
                facing,
                ..Anim::default()
            };
            s.frame_4d32(&t);
            assert_eq!(s.ptr, a);
            s.frame_4d54(&t);
            assert_eq!(s.ptr, b);
        }
    }
}
