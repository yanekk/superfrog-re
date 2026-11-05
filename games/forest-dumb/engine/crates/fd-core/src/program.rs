//! The original's own tables, read from its main program as loaded (plans/forest-dumb-level-1 T08),
//! and the machine leftovers a ported routine reproduces for the routine rig.
//!
//! The character's routines index step tables in hunk 0 (`h0+$40ac`, `$42ec`, `$431e`, `$43ba`,
//! `$44d4`, `$49d0`, `$4f88`) and the collision map's row table (`collision_rows`, `h0+$37b08`) with
//! counters the game keeps small but a generated state may set to any long. The original then reads
//! whatever the address reaches; [`Program`] reads the same byte, wrapping on the 68000's 24-bit bus,
//! when it holds all of memory (the rig) and 0 past hunk 0 when it holds hunk 0 alone (play, where
//! the counters never get there).

use std::sync::Arc;

/// Where the oracle loads each hunk of the main program (measured, T00). `fd-conform`'s
/// `HUNK_BASES` is this; a pointer the engine copies out of a table equals the original's only when
/// hunk 0 is relocated against these.
pub const HUNK_BASES: [u32; 5] = [0xc0_77d8, 0x1_0178, 0x1_1400, 0x1_2c38, 0x4_d668];

/// The main program's file on disk 1.
pub const PROGRAM_FILE: &str = "FOREST_DUMB_FOREVER";

/// The address of `hN+$off` where the oracle loads it.
pub const fn h(hunk: usize, off: u32) -> u32 {
    HUNK_BASES[hunk] + off
}

/// Bytes the original reads as data, addressed by hunk 0 offset.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Program {
    mem: Arc<Vec<u8>>,
    /// Index of `h0+$0` in `mem`.
    at: u32,
}

impl Program {
    /// Hunk 0 alone, relocated against [`HUNK_BASES`] (`fd_assets::loaded_hunk`).
    pub fn hunk0(bytes: Vec<u8>) -> Program {
        Program {
            mem: Arc::new(bytes),
            at: 0,
        }
    }

    /// All of a 24-bit address space, hunk 0 at [`HUNK_BASES`]`[0]`: the routine rig's memory.
    pub fn memory(mem: Arc<Vec<u8>>) -> Program {
        Program {
            mem,
            at: HUNK_BASES[0],
        }
    }

    /// The byte at `h0+off`.
    pub fn byte(&self, off: u32) -> u8 {
        let i = self.at.wrapping_add(off) & 0xff_ffff;
        self.mem.get(i as usize).copied().unwrap_or(0)
    }

    /// The big-endian long at `h0+off`.
    pub fn long(&self, off: u32) -> u32 {
        (0..4).fold(0, |v, k| v << 8 | self.byte(off.wrapping_add(k)) as u32)
    }

    /// Hunk 0's bytes from offset 0, for tables kept apart (`character::AnimTables`).
    pub fn bytes(&self) -> &[u8] {
        &self.mem[self.at as usize..]
    }
}

/// What a routine leaves on the machine besides the game's variables: the registers (d0-d7,
/// a0-a6), the custom-chip writes in order (offset from `$dff000`, word), and how many times it has
/// read the beam position. Play reads none of it; the routine rig holds a port to it, because a
/// test of memory alone lets a wrong return value through (porting-routines §3). A caller of the
/// original reads d0 back in places (`h0_4718` subtracts the run step `h0_47e2` leaves there), so
/// the engine returns such values as results; the copy here is the register as the original
/// leaves it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scratch {
    pub d: [u32; 8],
    pub a: [u32; 8],
    pub hw: Vec<(u16, u16)>,
    /// The beam byte the next VHPOSR read returns: each read returns one more, as on the rig.
    pub beam: u8,
}

impl Scratch {
    /// `move.b v, dn`: the low byte only.
    pub fn set_b(&mut self, n: usize, v: u8) {
        self.d[n] = self.d[n] & !0xff | v as u32;
    }

    /// `move.w v, dn`: the low word only.
    pub fn set_w(&mut self, n: usize, v: u16) {
        self.d[n] = self.d[n] & !0xffff | v as u32;
    }

    /// One read of VHPOSR's high byte.
    pub fn vhposr(&mut self) -> u8 {
        let v = self.beam;
        self.beam = v.wrapping_add(1);
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hunk0_alone_reads_0_past_its_end_and_memory_wraps_at_24_bits() {
        let p = Program::hunk0(vec![1, 2, 3, 4, 5]);
        assert_eq!(p.long(1), 0x0203_0405);
        assert_eq!(p.long(3), 0x0405_0000);
        let mut m = vec![0; 1 << 24];
        m[0] = 0xaa;
        m[HUNK_BASES[0] as usize + 2] = 0x55;
        let p = Program::memory(Arc::new(m));
        assert_eq!(p.byte(2), 0x55);
        assert_eq!(p.byte((1 << 24) - HUNK_BASES[0]), 0xaa);
        assert_eq!(p.byte(0u32.wrapping_sub(HUNK_BASES[0])), 0xaa);
    }

    #[test]
    fn byte_and_word_moves_keep_the_rest_of_the_register() {
        let mut s = Scratch::default();
        s.d[0] = 0x1234_5678;
        s.set_b(0, 0xff);
        assert_eq!(s.d[0], 0x1234_56ff);
        s.set_w(0, 0xabcd);
        assert_eq!(s.d[0], 0x1234_abcd);
        s.beam = 0xff;
        assert_eq!((s.vhposr(), s.vhposr()), (0xff, 0));
    }
}
