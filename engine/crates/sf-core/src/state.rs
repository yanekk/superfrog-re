//! The game's state: every original variable the engine models, kept as the original's bytes at the
//! original's address, as [`LAYOUT`] lists them.
//!
//! The 68000 code keeps pointers to its own tables in its variables, reads a long where it wrote
//! two words and a byte of a word it wrote whole; a byte-exact copy keeps all of that, so the port
//! reads and writes the original's addresses ([`State::byte`], [`State::write_byte`]) and names them
//! with accessors. What the level's picture is drawn from (T13, docs/12) is kept the same way: the
//! copper list, the panel's bitplanes and the two display buffers, which the renderer reads as
//! Denise would and the mechanics write as the original's blits do (owner, 2026-10-11).
//!
//! Only [`LAYOUT`]'s variables live here. What the game reads but never writes (its tables, the
//! graphics it loaded) comes from the disks ([`crate::DisplayData`]); [`crate::mem::Ram`] puts the
//! two together.
//!
//! The sound driver's variables (`crate::audio::vars`) are the exception: the driver keeps them
//! itself ([`State::audio`]), and the state reads and writes them there. A write the front end makes
//! outside every variable is kept in [`State::scratch`] (T20), never compared.

use std::borrow::Cow;
use std::collections::BTreeMap;

use crate::audio::AudioDriver;
use crate::layout::{LAYOUT, MAP_ROW_CELLS, MAP_ROWS, Ram, VarId, def, h0, index_of};
use crate::rng::Rng;

/// What the original keeps in memory, as far as the engine models it. A power-on state is all
/// zero, as the oracle's RAM is (FINDINGS 2026-10-11: RAM zeroed), but for `ExecBase` at address
/// 4, which Kickstart sets before the game starts.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct State {
    /// Each [`LAYOUT`] entry's bytes, in table order (empty for the sound driver's, in `audio`).
    vars: Vec<Vec<u8>>,
    /// The sound driver (docs/11), its variables the original's own.
    pub audio: AudioDriver,
    /// Bytes the front end wrote outside every modelled variable, by address: kept so a read
    /// after such a write sees it, never compared. A test asserts the front end leaves it empty.
    pub scratch: BTreeMap<u32, u8>,
    /// Where `pause` (h0+$7442) is while a level is paused: the original keeps it in its program
    /// counter and `d0` (T191).
    pub pause: Option<crate::level::pass::Pause>,
    /// In a level, CIA-A's serial-port flag: a keyboard byte has come in that `kbd_poll` has not
    /// handled; and, once it has, the register holds the `$ff` the poll wrote back (h0+$16038,
    /// T191).
    pub kbd_flag: bool,
    pub kbd_acked: bool,
}

/// `camera_x`, `camera_y`: the level pixel the camera follows (docs/12 "Scroll").
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Camera {
    pub x: u32,
    pub y: u32,
}

/// The sound driver's variable block a [`VarId`] names: its index in `audio::vars::ALL`.
fn audio_var(id: VarId) -> Option<usize> {
    Some(match id {
        VarId::AudioSfx => 0,
        VarId::AudioMusicOn => 1,
        VarId::AudioP41 => 2,
        VarId::AudioShadow => 3,
        VarId::AudioStream => 4,
        VarId::AudioSfxTables => 5,
        VarId::AudioModuleAddr => 6,
        VarId::AudioHeaderBackup => 7,
        _ => return None,
    })
}

/// `ExecBase` (address 4) as Kickstart 1.3 leaves it with slow RAM, the oracle's machine.
pub const EXEC_BASE: u32 = 0x00c0_0276;
/// The level 1 interrupt's autovector (address `$64`) as Kickstart 1.3 leaves it (the oracle's).
pub const KICKSTART_LEVEL1: u32 = 0x00fc_0c8e;

impl Default for State {
    fn default() -> State {
        let mut s = State {
            vars: LAYOUT
                .iter()
                .map(|d| match audio_var(d.id) {
                    Some(_) => Vec::new(),
                    None => vec![0; d.width.len() as usize],
                })
                .collect(),
            audio: AudioDriver::blank(),
            scratch: BTreeMap::new(),
            pause: None,
            kbd_flag: false,
            kbd_acked: false,
        };
        s.set_l(4, EXEC_BASE);
        s.set_l(0x64, KICKSTART_LEVEL1);
        s
    }
}

impl State {
    /// The state the original's memory holds: every [`LAYOUT`] entry read from `ram`.
    pub fn transplant(ram: &dyn Ram) -> State {
        let mut s = State::default();
        for (i, d) in LAYOUT.iter().enumerate() {
            let bytes: Vec<u8> = (0..d.width.len()).map(|k| ram.byte(d.addr + k)).collect();
            match audio_var(d.id) {
                Some(k) => s.audio.set_var_block(k, &bytes),
                None => s.vars[i] = bytes,
            }
        }
        s
    }

    /// Every [`LAYOUT`] entry's value, in table order, one pair per [`crate::VarDef::pieces`] piece.
    pub fn probe(&self) -> Vec<(VarId, u32)> {
        LAYOUT
            .iter()
            .flat_map(|d| {
                let bytes = self.var(d.id);
                d.pieces()
                    .map(|p| {
                        let at = (p.addr - d.addr) as usize;
                        let v = bytes[at..at + p.width.len() as usize]
                            .iter()
                            .fold(0u32, |v, &b| v << 8 | b as u32);
                        (d.id, v)
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// Overwrite the bytes at `addr..addr + bytes.len()` that modelled variables cover, as a write
    /// to the original's memory would; bytes no variable covers are ignored. Conformance tooling
    /// uses it to plant a known divergence.
    pub fn poke(&mut self, addr: u32, bytes: &[u8]) {
        for (k, &b) in bytes.iter().enumerate() {
            let a = addr + k as u32;
            if index_of(a).is_some() {
                self.put(a, b);
            }
        }
    }

    /// `len` bytes at `addr`, when one variable covers them all (not the sound driver's).
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

    /// The sound driver's block covering `addr`: (its index, the offset in it).
    fn audio_at(addr: u32) -> Option<(usize, usize)> {
        crate::audio::vars::ALL
            .iter()
            .enumerate()
            .find_map(|(k, &(off, len))| {
                let o = addr.checked_sub(h0(off))? as usize;
                (o < len).then_some((k, o))
            })
    }

    /// The byte at `addr`, when a variable covers it.
    pub fn byte(&self, addr: u32) -> Option<u8> {
        let addr = addr & 0xFF_FFFF;
        self.raw(addr, 1).map(|b| b[0]).or_else(|| {
            let (k, o) = State::audio_at(addr)?;
            Some(self.audio.var_block(k)[o])
        })
    }

    /// Write the byte at `addr` into the variable covering it; false when none does.
    fn put(&mut self, addr: u32, v: u8) -> bool {
        if let Some(b) = self.raw_mut(addr, 1) {
            b[0] = v;
        } else if let Some((k, o)) = State::audio_at(addr) {
            let mut block = self.audio.var_block(k);
            block[o] = v;
            self.audio.set_var_block(k, &block);
        } else {
            return false;
        }
        true
    }

    /// Write the byte at `addr`, as the original's code does.
    ///
    /// # Panics
    /// When no [`LAYOUT`] variable covers `addr`: the port writes a variable the table does not
    /// map, which would leave it out of every transplant and comparison.
    pub fn write_byte(&mut self, addr: u32, v: u8) {
        let addr = addr & 0xFF_FFFF;
        if !self.put(addr, v) {
            panic!("write to ${addr:06x}, which no LAYOUT variable covers");
        }
    }

    // --- The original's memory, by address, as the front end's code reads and writes it. ---

    /// The byte at `addr`: a variable's, else what the front end last wrote there, else 0.
    pub fn b(&self, addr: u32) -> u8 {
        self.byte(addr)
            .or_else(|| self.scratch.get(&addr).copied())
            .unwrap_or(0)
    }

    pub fn w(&self, addr: u32) -> u16 {
        u16::from_be_bytes([self.b(addr), self.b(addr.wrapping_add(1))])
    }

    pub fn l(&self, addr: u32) -> u32 {
        (self.w(addr) as u32) << 16 | self.w(addr.wrapping_add(2)) as u32
    }

    /// Write the byte at `addr`: into its variable, else into [`State::scratch`].
    pub fn set_b(&mut self, addr: u32, v: u8) {
        if !self.put(addr, v) {
            self.scratch.insert(addr, v);
        }
    }

    pub fn set_w(&mut self, addr: u32, v: u16) {
        let [a, b] = v.to_be_bytes();
        self.set_b(addr, a);
        self.set_b(addr.wrapping_add(1), b);
    }

    pub fn set_l(&mut self, addr: u32, v: u32) {
        self.set_w(addr, (v >> 16) as u16);
        self.set_w(addr.wrapping_add(2), v as u16);
    }

    /// `bytes` written from `addr` on.
    pub fn write(&mut self, addr: u32, bytes: &[u8]) {
        match self.raw_mut(addr, bytes.len() as u32) {
            Some(b) => b.copy_from_slice(bytes),
            None => {
                for (k, &v) in bytes.iter().enumerate() {
                    self.set_b(addr + k as u32, v);
                }
            }
        }
    }

    /// `len` bytes from `addr` on.
    pub fn read(&self, addr: u32, len: u32) -> Vec<u8> {
        match self.raw(addr, len) {
            Some(b) => b.to_vec(),
            None => (0..len).map(|k| self.b(addr + k)).collect(),
        }
    }

    /// A whole variable's bytes.
    pub fn var(&self, id: VarId) -> Cow<'_, [u8]> {
        match audio_var(id) {
            Some(k) => Cow::Owned(self.audio.var_block(k)),
            None => Cow::Borrowed(&self.vars[id as usize]),
        }
    }

    /// A whole variable's bytes, to write (not the sound driver's: the driver writes those).
    pub fn var_mut(&mut self, id: VarId) -> &mut [u8] {
        assert!(audio_var(id).is_none(), "{id:?} is the sound driver's");
        &mut self.vars[id as usize]
    }

    /// The long at the start of `id`.
    fn long(&self, id: VarId) -> u32 {
        let b = &self.vars[id as usize];
        u32::from_be_bytes([b[0], b[1], b[2], b[3]])
    }

    fn set_long(&mut self, id: VarId, v: u32) {
        self.var_mut(id)[..4].copy_from_slice(&v.to_be_bytes());
    }

    pub fn rng(&self) -> Rng {
        Rng::from_bytes(self.vars[VarId::RngState as usize][..].try_into().expect("8 bytes"))
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

    /// `map_rows`: one row above the in-RAM map.
    pub fn map_rows(&self) -> u32 {
        self.long(VarId::MapRows)
    }

    pub fn set_map_rows(&mut self, v: u32) {
        self.set_long(VarId::MapRows, v);
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
        &self.vars[VarId::LevelMap as usize]
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
    pub fn bytes_of(&self, id: VarId) -> (u32, Cow<'_, [u8]>) {
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
        assert_eq!(s.byte(at + 4), Some(9));
        assert_eq!(s.byte(0xF0_0000), None);
    }

    #[test]
    fn the_sound_driver_s_variables_are_read_and_written_in_the_driver() {
        let mut s = State::default();
        let at = h0(crate::audio::vars::MUSIC_ON.0);
        s.write_byte(at + 1, 0x5a);
        assert_eq!(s.byte(at + 1), Some(0x5a));
        assert_eq!(s.audio.var_block(1)[1], 0x5a);
        assert!(s.raw(at, 1).is_none());
        assert!(s.scratch.is_empty());
    }

    #[test]
    #[should_panic(expected = "no LAYOUT variable covers")]
    fn a_write_outside_the_table_is_refused() {
        State::default().write_byte(0xF0_0000, 1);
    }

    #[test]
    fn a_front_end_write_outside_the_table_is_kept_aside() {
        let mut s = State::default();
        s.set_w(0x100, 0x1234);
        assert_eq!(s.w(0x100), 0x1234);
        assert_eq!(s.scratch.len(), 2);
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
