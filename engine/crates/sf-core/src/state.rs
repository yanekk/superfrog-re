//! The game's state: every original variable the engine models, kept as the original's bytes at the
//! original's address, as [`LAYOUT`] lists them.
//!
//! The 68000 code keeps pointers to its own tables in its variables, reads a long where it wrote
//! two words and a byte of a word it wrote whole; a byte-exact copy keeps all of that, so the port
//! reads and writes the original's addresses ([`State::read`], [`State::write`]) and names them
//! with accessors. What the level's picture is drawn from (T13, docs/12) is kept the same way: the
//! copper list, the panel's bitplanes and the two display buffers, which the renderer reads as
//! Denise would and the mechanics write as the original's blits do (owner, 2026-10-11).
//!
//! Only [`LAYOUT`]'s variables live here. What the game reads but never writes (its tables, the
//! graphics it loaded) comes from the disks ([`crate::DisplayData`]); [`crate::mem::Ram`] puts the
//! two together.

use crate::layout::{LAYOUT, MAP_ROW_CELLS, MAP_ROWS, Ram, VarId, def, index_of};
use crate::rng::Rng;

/// What the original keeps in memory, as far as the engine models it. A power-on state is all
/// zero, as the oracle's RAM is (FINDINGS 2026-10-11: RAM zeroed).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct State {
    /// Each [`LAYOUT`] entry's bytes, in table order.
    vars: Vec<Vec<u8>>,
}

/// `camera_x`, `camera_y`: the level pixel the camera follows (docs/12 "Scroll").
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Camera {
    pub x: u32,
    pub y: u32,
}

impl Default for State {
    fn default() -> State {
        State {
            vars: LAYOUT
                .iter()
                .map(|d| vec![0; d.width.len() as usize])
                .collect(),
        }
    }
}

impl State {
    /// The state the original's memory holds: every [`LAYOUT`] entry read from `ram`.
    pub fn transplant(ram: &dyn Ram) -> State {
        State {
            vars: LAYOUT
                .iter()
                .map(|d| (0..d.width.len()).map(|k| ram.byte(d.addr + k)).collect())
                .collect(),
        }
    }

    /// Every [`LAYOUT`] entry's value, in table order, one pair per [`crate::VarDef::pieces`] piece.
    pub fn probe(&self) -> Vec<(VarId, u32)> {
        LAYOUT
            .iter()
            .zip(&self.vars)
            .flat_map(|(d, bytes)| {
                d.pieces().map(move |p| {
                    let at = (p.addr - d.addr) as usize;
                    let v = bytes[at..at + p.width.len() as usize]
                        .iter()
                        .fold(0u32, |v, &b| v << 8 | b as u32);
                    (d.id, v)
                })
            })
            .collect()
    }

    /// Overwrite the bytes at `addr..addr + bytes.len()` that modelled variables cover, as a write
    /// to the original's memory would; bytes no variable covers are ignored. Conformance tooling
    /// uses it to plant a known divergence.
    pub fn poke(&mut self, addr: u32, bytes: &[u8]) {
        for (k, &b) in bytes.iter().enumerate() {
            let a = addr + k as u32;
            if let Some(i) = index_of(a) {
                self.vars[i][(a - LAYOUT[i].addr) as usize] = b;
            }
        }
    }

    /// `len` bytes at `addr`, when one variable covers them all.
    pub fn raw(&self, addr: u32, len: u32) -> Option<&[u8]> {
        let i = index_of(addr)?;
        let at = (addr - LAYOUT[i].addr) as usize;
        self.vars[i].get(at..at + len as usize)
    }

    /// [`State::raw`], to write.
    pub fn raw_mut(&mut self, addr: u32, len: u32) -> Option<&mut [u8]> {
        let i = index_of(addr)?;
        let at = (addr - LAYOUT[i].addr) as usize;
        self.vars[i].get_mut(at..at + len as usize)
    }

    /// The byte at `addr`, when a variable covers it.
    pub fn read(&self, addr: u32) -> Option<u8> {
        self.raw(addr & 0xFF_FFFF, 1).map(|b| b[0])
    }

    /// Write the byte at `addr`, as the original's code does.
    ///
    /// # Panics
    /// When no [`LAYOUT`] variable covers `addr`: the port writes a variable the table does not
    /// map, which would leave it out of every transplant and comparison.
    pub fn write(&mut self, addr: u32, v: u8) {
        let addr = addr & 0xFF_FFFF;
        match self.raw_mut(addr, 1) {
            Some(b) => b[0] = v,
            None => panic!("write to ${addr:06x}, which no LAYOUT variable covers"),
        }
    }

    /// A whole variable's bytes.
    pub fn var(&self, id: VarId) -> &[u8] {
        &self.vars[id as usize]
    }

    pub fn var_mut(&mut self, id: VarId) -> &mut [u8] {
        &mut self.vars[id as usize]
    }

    /// The long at the start of `id`.
    fn long(&self, id: VarId) -> u32 {
        let b = self.var(id);
        u32::from_be_bytes([b[0], b[1], b[2], b[3]])
    }

    fn set_long(&mut self, id: VarId, v: u32) {
        self.var_mut(id)[..4].copy_from_slice(&v.to_be_bytes());
    }

    pub fn rng(&self) -> Rng {
        Rng::from_bytes(self.var(VarId::RngState).try_into().expect("8 bytes"))
    }

    pub fn set_rng(&mut self, r: Rng) {
        self.var_mut(VarId::RngState).copy_from_slice(&r.to_bytes());
    }

    pub fn camera(&self) -> Camera {
        Camera {
            x: self.long(VarId::CameraX),
            y: self.long(VarId::CameraY),
        }
    }

    pub fn set_camera(&mut self, c: Camera) {
        self.set_long(VarId::CameraX, c.x);
        self.set_long(VarId::CameraY, c.y);
    }

    /// `scroll_column`: the vertical blank's last scroll, as the address of the shown column.
    pub fn scroll_column(&self) -> u32 {
        self.long(VarId::ScrollColumn)
    }

    pub fn set_scroll_column(&mut self, v: u32) {
        self.set_long(VarId::ScrollColumn, v);
    }

    /// `world`: 1..6, 7 the bonus level.
    pub fn world(&self) -> u32 {
        self.long(VarId::World)
    }

    pub fn set_world(&mut self, w: u32) {
        self.set_long(VarId::World, w);
    }

    /// `level_map`: [`MAP_ROWS`] rows of [`MAP_ROW_CELLS`] cells, `tile << 6 | marker`.
    pub fn map(&self) -> &[u8] {
        self.var(VarId::LevelMap)
    }

    /// The map cell at (`col`, `row`); `None` outside the map's 300 × 84 cells.
    pub fn cell(&self, col: i32, row: i32) -> Option<u16> {
        ((0..300).contains(&col) && (0..MAP_ROWS as i32).contains(&row)).then(|| {
            let at = 2 * (row * MAP_ROW_CELLS as i32 + col) as usize;
            let m = self.map();
            u16::from_be_bytes([m[at], m[at + 1]])
        })
    }

    pub fn set_cell(&mut self, col: i32, row: i32, v: u16) {
        assert!((0..MAP_ROW_CELLS as i32).contains(&col) && (0..MAP_ROWS as i32).contains(&row));
        let at = 2 * (row * MAP_ROW_CELLS as i32 + col) as usize;
        self.var_mut(VarId::LevelMap)[at..at + 2].copy_from_slice(&v.to_be_bytes());
    }

    /// The bytes of `id` as [`LAYOUT`] places them: for checks that walk the table.
    pub fn bytes_of(&self, id: VarId) -> (u32, &[u8]) {
        (def(id).addr, self.var(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{AmigaMemory, SLOW_BASE};

    fn memory_with_rng(bytes: [u8; 8]) -> AmigaMemory {
        let mut m = AmigaMemory {
            chip: vec![0; 512 * 1024],
            slow: vec![0; 512 * 1024],
        };
        let at = (def(VarId::RngState).addr - SLOW_BASE) as usize;
        m.slow[at..at + 8].copy_from_slice(&bytes);
        m
    }

    #[test]
    fn transplant_then_probe_gives_back_the_memory() {
        let s = State::transplant(&memory_with_rng([1, 2, 3, 4, 5, 6, 7, 8]));
        assert_eq!(
            s.rng(),
            Rng {
                lo: 0x0102_0304,
                hi: 0x0506_0708
            }
        );
        let p = s.probe();
        let at = LAYOUT
            .iter()
            .take_while(|d| d.id != VarId::RngState)
            .flat_map(|d| d.pieces())
            .count();
        assert_eq!(
            &p[at..at + 2],
            &[
                (VarId::RngState, 0x0102_0304),
                (VarId::RngState, 0x0506_0708)
            ]
        );
        assert_eq!(p.iter().filter(|&&(_, v)| v != 0).count(), 2);
    }

    #[test]
    fn every_variable_round_trips_through_memory() {
        let mut m = memory_with_rng([0; 8]);
        let mut n = 0u8;
        for d in LAYOUT {
            for k in 0..d.width.len() {
                n = n.wrapping_mul(31).wrapping_add(7);
                let a = d.addr + k;
                if a >= SLOW_BASE {
                    m.slow[(a - SLOW_BASE) as usize] = n;
                } else {
                    m.chip[a as usize] = n;
                }
            }
        }
        let s = State::transplant(&m);
        let pieces: Vec<_> = LAYOUT.iter().flat_map(|d| d.pieces()).collect();
        for (p, (id, v)) in pieces.iter().zip(s.probe()) {
            assert_eq!((p.id, p.read(&m)), (id, v), "{}", p.name);
        }
    }

    #[test]
    fn poke_changes_only_covered_bytes() {
        let mut s = State::default();
        let at = def(VarId::RngState).addr;
        s.poke(at + 6, &[0xAA, 0xBB, 0xCC]);
        assert_eq!(s.rng(), Rng { lo: 0, hi: 0xAABB });
        s.poke(0x100, &[1]);
        assert_eq!(s.rng(), Rng { lo: 0, hi: 0xAABB });
    }

    #[test]
    fn raw_reads_only_whole_runs_inside_one_variable() {
        let mut s = State::default();
        let at = def(VarId::LevelCopper).addr;
        s.var_mut(VarId::LevelCopper)[4] = 9;
        assert_eq!(s.raw(at + 4, 2), Some(&[9, 0][..]));
        assert_eq!(s.raw(at + 0x3ff, 2), None);
        assert_eq!(s.read(at + 4), Some(9));
        assert_eq!(s.read(0xF0_0000), None);
    }

    #[test]
    #[should_panic(expected = "no LAYOUT variable covers")]
    fn a_write_outside_the_table_is_refused() {
        State::default().write(0xF0_0000, 1);
    }

    #[test]
    fn cells_outside_the_map_are_none() {
        let mut s = State::default();
        s.set_cell(2, 1, 0x3E09);
        assert_eq!(s.cell(2, 1), Some(0x3E09));
        assert_eq!(s.cell(-1, 1), None);
        assert_eq!(s.cell(300, 1), None);
        assert_eq!(s.cell(0, 84), None);
    }
}
