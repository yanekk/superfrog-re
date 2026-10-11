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

/// What the game's memory holds that never changes once it is set up: hunk 0 as loaded, with the
/// tables the boot builds in it, and the files each world loads (its tiles). Built once from the
/// disks; the ported routines read it where no [`crate::LAYOUT`] variable is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rom {
    /// Hunk 0 ([`loaded_hunk0`]) with the boot's tables in it ([`boot_tables`]).
    pub h0: Vec<u8>,
    /// Other regions the boot fills, by address.
    pub regions: Vec<(u32, Vec<u8>)>,
    /// Per `world` value (index 0 for world 1 … 6, then the bonus level, then the final level):
    /// the regions that world's files fill.
    pub worlds: Vec<Vec<(u32, Vec<u8>)>>,
}

/// `tile_ptrs` (h1+$30fe0): the boot's table of 896 pointers to the tiles, `$a0` bytes apart from
/// h1+0, where each world's `LnBM` is loaded (h0+$172ea).
pub const TILE_PTRS: u32 = H1 + 0x30fe0;

/// The tables the boot builds once (`rng_seed_from_cia`'s calls at h0+$7734..$7746), each run
/// until the end mark the executable holds after it: the map's row pointers (h0+$1730c), the
/// display ring's row pointers (h0+$174a0) and its row offsets (h0+$176a2), all in hunk 0; the
/// tile pointers ([`TILE_PTRS`], h0+$172ea) are returned as a region of hunk 1.
pub fn boot_tables(h0: &mut [u8]) -> Vec<(u32, Vec<u8>)> {
    fn get(h: &[u8], a: u32) -> Option<u32> {
        let o = a.checked_sub(H0)? as usize;
        Some(u32::from_be_bytes(h.get(o..o + 4)?.try_into().ok()?))
    }
    fn put(h: &mut [u8], a: u32, v: &[u8]) {
        let o = (a - H0) as usize;
        h[o..o + v.len()].copy_from_slice(v);
    }
    if h0.len() < 0x17800 {
        return Vec::new();
    }
    // h0+$1730c: row pointers from one row above the map's first, `$260` apart.
    let mut a = H0 + 0x17330;
    let mut v = H1 + 0x21680 - 0x260;
    while get(h0, a).is_some_and(|x| (x as i32) >= 0) {
        put(h0, a, &v.to_be_bytes());
        a += 4;
        v += 0x260;
    }
    // h0+$174a0: the ring's row pointers into playfield_buffer_0, `$300` apart, wrapping after
    // `$3900`.
    let buffer0 = H3 + 0x10600;
    let mut a = H0 + 0x174d6;
    let mut v = buffer0;
    while get(h0, a).is_some_and(|x| (x as i32) >= 0) {
        put(h0, a, &v.to_be_bytes());
        a += 4;
        v += 0x300;
        if v == buffer0 + 0x3900 {
            v = buffer0;
        }
    }
    // h0+$176a2: words 0, $10, … $120, repeating.
    let mut a = H0 + 0x176c4;
    let mut v = 0u16;
    while get(h0, a).is_some_and(|x| (x as i32 >> 16) >= 0) {
        put(h0, a, &v.to_be_bytes());
        a += 2;
        v += 0x10;
        if v == 0x130 {
            v = 0;
        }
    }
    // h0+$172ea: 896 tile pointers.
    let tiles: Vec<u8> = (0..0x380u32)
        .flat_map(|k| (H1 + k * 0xa0).to_be_bytes())
        .collect();
    vec![(TILE_PTRS, tiles)]
}

impl Rom {
    pub fn new(data: &sf_assets::GameData) -> Rom {
        let Some(game) = data.exes.iter().find(|e| e.id == sf_assets::ExeId::Game) else {
            return Rom::default();
        };
        let mut h0 = loaded_hunk0(game);
        let regions = boot_tables(&mut h0);
        let order = [
            sf_assets::World::W1,
            sf_assets::World::W2,
            sf_assets::World::W3,
            sf_assets::World::W4,
            sf_assets::World::W5,
            sf_assets::World::W6,
            sf_assets::World::Bonus,
            sf_assets::World::Final,
        ];
        let worlds = order
            .iter()
            .map(|&w| {
                data.worlds
                    .iter()
                    .find(|g| g.world == w)
                    .map(|g| vec![(H1, g.tiles.encode())])
                    .unwrap_or_default()
            })
            .collect();
        Rom {
            h0,
            regions,
            worlds,
        }
    }

    /// The byte at `addr` with `world` loaded, if this image holds it.
    pub fn byte(&self, world: u32, addr: u32) -> Option<u8> {
        let within = |regions: &[(u32, Vec<u8>)]| {
            regions.iter().find_map(|(at, b)| {
                let o = addr.checked_sub(*at)? as usize;
                b.get(o).copied()
            })
        };
        if let Some(o) = addr.checked_sub(H0)
            && let Some(&v) = self.h0.get(o as usize)
        {
            return Some(v);
        }
        within(&self.regions).or_else(|| {
            let w = (world as usize).checked_sub(1)?;
            within(self.worlds.get(w)?)
        })
    }
}

/// The 68000's view of memory for the ported routines.
pub struct Bus<'a> {
    pub st: &'a mut State,
    pub rom: &'a Rom,
}

impl<'a> Bus<'a> {
    pub fn new(st: &'a mut State, rom: &'a Rom) -> Bus<'a> {
        Bus { st, rom }
    }

    /// The byte at `addr`: the state's variable there, else the fixed image, else 0.
    pub fn b(&self, addr: u32) -> u8 {
        let addr = addr & 0xFF_FFFF;
        self.st
            .read(addr)
            .or_else(|| self.rom.byte(self.st.world(), addr))
            .unwrap_or(0)
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
        let mut rom = Rom {
            h0: vec![0u8; 0x20000],
            ..Rom::default()
        };
        let world = def(VarId::World).addr;
        rom.h0[(world - H0) as usize + 3] = 5;
        rom.h0[0x100] = 0x77;
        let mut st = State::default();
        let mut bus = Bus::new(&mut st, &rom);
        assert_eq!(bus.l(world), 0, "the variable, not the file");
        assert_eq!(bus.b(H0 + 0x100), 0x77, "the file where no variable is");
        assert_eq!(bus.b(0x10), 0, "outside both");
        bus.set_l(world, 0x0102_0304);
        assert_eq!(bus.w(world + 1), 0x0203);
        assert_eq!(st.world(), 0x0102_0304);
    }
}
