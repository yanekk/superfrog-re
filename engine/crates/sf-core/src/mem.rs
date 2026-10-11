//! What the ported 68000 code sees: the engine's variables ([`State`]) over the executable as the
//! game's loader leaves it in memory (hunk 0 relocated to [`H0`]). The game's own tables (an
//! animation's frame list, a jump table) are read from the executable on the player's disks; its
//! variables from the state. Big-endian, at the original's addresses, as the 68000 does.

use sf_assets::{ExeImage, Hunk};

use crate::layout::{H0, H1, H2, H3};
use crate::state::State;

/// Where the bootstrap puts each hunk of `d1_c03` (docs/08 "Memory map").
pub const HUNK_BASES: [u32; 4] = [H0, H1, H2, H3];

/// Hunk 0 of the game as loaded: its file contents at their declared size, every relocated long
/// with its target hunk's base added. Empty when `game` has no hunk 0 (a test's empty data).
pub fn loaded_hunk0(game: &ExeImage) -> Vec<u8> {
    game.hunks.first().map(loaded).unwrap_or_default()
}

/// One hunk as loaded: contents at the declared size, relocated to [`HUNK_BASES`].
fn loaded(h: &Hunk) -> Vec<u8> {
    let mut b = h.data.clone();
    b.resize(h.size.max(h.data.len() as u32) as usize, 0);
    for r in &h.relocs {
        let base = HUNK_BASES.get(r.target as usize).copied().unwrap_or(0);
        for &o in &r.offsets {
            let o = o as usize;
            if let Some(l) = b.get_mut(o..o + 4) {
                let v = u32::from_be_bytes([l[0], l[1], l[2], l[3]]).wrapping_add(base);
                l.copy_from_slice(&v.to_be_bytes());
            }
        }
    }
    b
}

/// The 68000's view of memory for the ported routines.
pub struct Bus<'a> {
    pub st: &'a mut State,
    /// Hunk 0 as loaded ([`loaded_hunk0`]): what an address no variable covers reads, inside it.
    pub h0: &'a [u8],
}

impl<'a> Bus<'a> {
    pub fn new(st: &'a mut State, h0: &'a [u8]) -> Bus<'a> {
        Bus { st, h0 }
    }

    /// The byte at `addr`: the state's variable there, else the loaded executable, else 0.
    pub fn b(&self, addr: u32) -> u8 {
        let addr = addr & 0xFF_FFFF;
        self.st.read(addr).unwrap_or_else(|| {
            addr.checked_sub(H0)
                .and_then(|o| self.h0.get(o as usize).copied())
                .unwrap_or(0)
        })
    }

    pub fn w(&self, addr: u32) -> u16 {
        u16::from_be_bytes([self.b(addr), self.b(addr.wrapping_add(1))])
    }

    pub fn l(&self, addr: u32) -> u32 {
        (self.w(addr) as u32) << 16 | self.w(addr.wrapping_add(2)) as u32
    }

    /// Write a byte (see [`State::write`]: only a [`crate::LAYOUT`] variable may be written).
    pub fn set_b(&mut self, addr: u32, v: u8) {
        self.st.write(addr, v);
    }

    pub fn set_w(&mut self, addr: u32, v: u16) {
        let [a, b] = v.to_be_bytes();
        self.st.write(addr, a);
        self.st.write(addr.wrapping_add(1), b);
    }

    pub fn set_l(&mut self, addr: u32, v: u32) {
        self.set_w(addr, (v >> 16) as u16);
        self.set_w(addr.wrapping_add(2), v as u16);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{VarId, def};
    use sf_assets::{HunkKind, Reloc};

    #[test]
    fn hunk0_is_relocated_to_each_target_s_base() {
        let b = loaded(&Hunk {
                kind: HunkKind::Code,
                mem: 0,
                size: 16,
                data: vec![0, 0, 0, 8, 0, 0, 0, 4, 9, 9],
                relocs: vec![
                    Reloc {
                        target: 0,
                        offsets: vec![0],
                    },
                    Reloc {
                        target: 3,
                        offsets: vec![4],
                    },
                ],
                present: true,
        });
        assert_eq!(b.len(), 16);
        assert_eq!(&b[..4], &(H0 + 8).to_be_bytes());
        assert_eq!(&b[4..8], &(H3 + 4).to_be_bytes());
        assert_eq!(&b[8..], &[9, 9, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn variables_shadow_the_executable() {
        let mut h0 = vec![0u8; 0x20000];
        let world = def(VarId::World).addr;
        h0[(world - H0) as usize + 3] = 5;
        h0[0x100] = 0x77;
        let mut st = State::default();
        let mut bus = Bus::new(&mut st, &h0);
        assert_eq!(bus.l(world), 0, "the variable, not the file");
        assert_eq!(bus.b(H0 + 0x100), 0x77, "the file where no variable is");
        assert_eq!(bus.b(0x10), 0, "outside both");
        bus.set_l(world, 0x0102_0304);
        assert_eq!(bus.w(world + 1), 0x0203);
        assert_eq!(st.world(), 0x0102_0304);
    }
}
