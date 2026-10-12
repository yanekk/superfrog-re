//! The game's state: every original variable the engine models, as typed Rust, plus the
//! conversions to and from the original's memory that [`LAYOUT`] drives.
//!
//! What the level's picture is drawn from (T13, docs/12) is kept in the original's own form: the
//! copper list, the panel's bitplanes and the two display buffers. The renderer reads them as
//! Denise would, so a transplanted state draws the original's frame, and a mechanic task writes
//! them as the original does (its tile, animation and bob blits; owner, 2026-10-11).

use std::collections::BTreeMap;

use crate::audio::AudioDriver;
use crate::layout::{LAYOUT, MAP_ROW_CELLS, MAP_ROWS, Ram, VarId, def};
use crate::rng::Rng;

/// What the original keeps in memory, as far as the engine models it. A power-on state is all
/// zero, as the oracle's RAM is (FINDINGS 2026-10-11: RAM zeroed).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct State {
    pub rng: Rng,
    /// `camera_x`, `camera_y`: the level pixel the camera follows (docs/12 "Scroll").
    pub camera: Camera,
    /// `scroll_column`: the vertical blank's last scroll, as the address of the shown column.
    pub scroll_column: u32,
    /// `map_rows`: one row above `level_map` in the original's memory.
    pub map_rows: u32,
    /// `world`: 1..6, 7 the bonus level.
    pub world: u32,
    /// `level_map`: [`MAP_ROWS`] rows of [`MAP_ROW_CELLS`] cells, `tile << 6 | marker`.
    pub map: Vec<u16>,
    /// `playfield_buffer_0` and `_1`: the display buffers' bitplanes, as the original's bytes.
    pub buffers: [Vec<u8>; 2],
    /// `level_copper`: the level's copper list, as the original's bytes.
    pub copper: Vec<u8>,
    /// `panel_bitmap`: the status panel's five bitplanes.
    pub panel: Vec<u8>,
    /// The runs of memory kept as the original's bytes ([`VarId::is_raw_region`], the front end's
    /// variables and copper lists), in [`LAYOUT`] order; the sound driver's are in `audio`.
    pub ram: Vec<Vec<u8>>,
    /// The sound driver (docs/11), its variables the original's own.
    pub audio: AudioDriver,
    /// Bytes the engine wrote outside every modelled variable, by address: kept so a read after
    /// such a write sees it, never compared. A test asserts the front end leaves it empty.
    pub scratch: BTreeMap<u32, u8>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Camera {
    pub x: u32,
    pub y: u32,
}

impl Default for State {
    fn default() -> State {
        let len = |id| def(id).width.len() as usize;
        State {
            rng: Rng::default(),
            camera: Camera::default(),
            scroll_column: 0,
            map_rows: 0,
            world: 0,
            map: vec![0; (MAP_ROWS * MAP_ROW_CELLS) as usize],
            buffers: [
                vec![0; len(VarId::PlayfieldBuffer0)],
                vec![0; len(VarId::PlayfieldBuffer1)],
            ],
            copper: vec![0; len(VarId::LevelCopper)],
            panel: vec![0; len(VarId::PanelBitmap)],
            ram: raw_regions()
                .map(|d| vec![0; d.width.len() as usize])
                .collect(),
            audio: AudioDriver::blank(),
            scratch: BTreeMap::new(),
        }
    }
}

/// The [`LAYOUT`] entries kept in `State::ram`, in order.
fn raw_regions() -> impl Iterator<Item = &'static crate::VarDef> {
    LAYOUT
        .iter()
        .filter(|d| d.id.is_raw_region() && audio_var(d.id).is_none())
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

impl State {
    /// The state the original's memory holds: every [`LAYOUT`] entry read from `ram`.
    pub fn transplant(ram: &dyn Ram) -> State {
        let mut s = State::default();
        for d in LAYOUT {
            let bytes: Vec<u8> = (0..d.width.len()).map(|k| ram.byte(d.addr + k)).collect();
            s.set_bytes(d.id, &bytes);
        }
        s
    }

    /// Every [`LAYOUT`] entry's value, in table order, one pair per [`crate::VarDef::pieces`] piece.
    pub fn probe(&self) -> Vec<(VarId, u32)> {
        LAYOUT
            .iter()
            .flat_map(|d| {
                let bytes = self.bytes(d.id);
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
        for d in LAYOUT {
            let mut cur = self.bytes(d.id);
            let mut hit = false;
            for (k, &b) in bytes.iter().enumerate() {
                let a = addr + k as u32;
                if (d.addr..d.addr + d.width.len()).contains(&a) {
                    cur[(a - d.addr) as usize] = b;
                    hit = true;
                }
            }
            if hit {
                self.set_bytes(d.id, &cur);
            }
        }
    }

    /// The bytes of a variable kept in the original's own form, by address: `len` bytes at
    /// `addr` when one such variable covers them all. The renderer reads the copper list, the
    /// panel and the display buffers through this, as the hardware reads memory.
    pub fn raw(&self, addr: u32, len: u32) -> Option<&[u8]> {
        let (id, at) = LAYOUT
            .iter()
            .find(|d| addr >= d.addr && addr + len <= d.addr + d.width.len())
            .map(|d| (d.id, (addr - d.addr) as usize))?;
        let bytes: &[u8] = match id {
            VarId::PlayfieldBuffer0 => &self.buffers[0],
            VarId::PlayfieldBuffer1 => &self.buffers[1],
            VarId::LevelCopper => &self.copper,
            VarId::PanelBitmap => &self.panel,
            id if id.is_raw_region() && audio_var(id).is_none() => {
                &self.ram[raw_regions().position(|d| d.id == id)?]
            }
            _ => return None,
        };
        Some(&bytes[at..at + len as usize])
    }

    /// The modelled bytes covering `addr..addr + len`, writable: a display buffer, the level's
    /// copper list, the panel or a run in `ram`.
    fn raw_mut(&mut self, addr: u32, len: u32) -> Option<&mut [u8]> {
        let (id, at) = LAYOUT
            .iter()
            .find(|d| addr >= d.addr && addr + len <= d.addr + d.width.len())
            .map(|d| (d.id, (addr - d.addr) as usize))?;
        let bytes: &mut [u8] = match id {
            VarId::PlayfieldBuffer0 => &mut self.buffers[0],
            VarId::PlayfieldBuffer1 => &mut self.buffers[1],
            VarId::LevelCopper => &mut self.copper,
            VarId::PanelBitmap => &mut self.panel,
            id if id.is_raw_region() && audio_var(id).is_none() => {
                let k = raw_regions().position(|d| d.id == id)?;
                &mut self.ram[k]
            }
            _ => return None,
        };
        Some(&mut bytes[at..at + len as usize])
    }

    // --- The original's memory, by address, as the front end's code reads and writes it. ---

    /// The byte at `addr`: a modelled run's, else what the engine last wrote there, else 0.
    pub fn b(&self, addr: u32) -> u8 {
        if let Some(b) = self.raw(addr, 1) {
            return b[0];
        }
        // The sound driver's variables, read only (the driver writes them itself).
        for (k, &(off, len)) in crate::audio::vars::ALL.iter().enumerate() {
            let at = crate::layout::h0(off);
            if (at..at + len as u32).contains(&addr) {
                return self.audio.var_block(k)[(addr - at) as usize];
            }
        }
        self.scratch.get(&addr).copied().unwrap_or(0)
    }

    pub fn w(&self, addr: u32) -> u16 {
        u16::from_be_bytes([self.b(addr), self.b(addr.wrapping_add(1))])
    }

    pub fn l(&self, addr: u32) -> u32 {
        (self.w(addr) as u32) << 16 | self.w(addr.wrapping_add(2)) as u32
    }

    pub fn set_b(&mut self, addr: u32, v: u8) {
        match self.raw_mut(addr, 1) {
            Some(b) => b[0] = v,
            None => {
                self.scratch.insert(addr, v);
            }
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

    /// `id`'s value as the original stores it (big-endian, its [`crate::VarDef`]'s width).
    fn bytes(&self, id: VarId) -> Vec<u8> {
        let long = |v: u32| v.to_be_bytes().to_vec();
        match id {
            VarId::RngState => self.rng.to_bytes().to_vec(),
            VarId::CameraY => long(self.camera.y),
            VarId::CameraX => long(self.camera.x),
            VarId::ScrollColumn => long(self.scroll_column),
            VarId::MapRows => long(self.map_rows),
            VarId::World => long(self.world),
            VarId::LevelMap => self.map.iter().flat_map(|c| c.to_be_bytes()).collect(),
            VarId::PlayfieldBuffer0 => self.buffers[0].clone(),
            VarId::PlayfieldBuffer1 => self.buffers[1].clone(),
            VarId::LevelCopper => self.copper.clone(),
            VarId::PanelBitmap => self.panel.clone(),
            id => match audio_var(id) {
                Some(k) => self.audio.var_block(k),
                None => self.ram[raw_regions()
                    .position(|d| d.id == id)
                    .expect("every raw region is in ram")]
                .clone(),
            },
        }
    }

    fn set_bytes(&mut self, id: VarId, b: &[u8]) {
        let long = |b: &[u8]| u32::from_be_bytes(b[..4].try_into().expect("4 bytes"));
        match id {
            VarId::RngState => self.rng = Rng::from_bytes(b.try_into().expect("8 bytes")),
            VarId::CameraY => self.camera.y = long(b),
            VarId::CameraX => self.camera.x = long(b),
            VarId::ScrollColumn => self.scroll_column = long(b),
            VarId::MapRows => self.map_rows = long(b),
            VarId::World => self.world = long(b),
            VarId::LevelMap => {
                self.map = b
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|c| u16::from_be_bytes([c[0], c[1]]))
                    .collect()
            }
            VarId::PlayfieldBuffer0 => self.buffers[0] = b.to_vec(),
            VarId::PlayfieldBuffer1 => self.buffers[1] = b.to_vec(),
            VarId::LevelCopper => self.copper = b.to_vec(),
            VarId::PanelBitmap => self.panel = b.to_vec(),
            id => match audio_var(id) {
                Some(k) => self.audio.set_var_block(k, b),
                None => {
                    let k = raw_regions()
                        .position(|d| d.id == id)
                        .expect("every raw region is in ram");
                    self.ram[k] = b.to_vec();
                }
            },
        }
    }

    /// The map cell at (`col`, `row`); `None` outside the map's 300 × 84 cells.
    pub fn cell(&self, col: i32, row: i32) -> Option<u16> {
        ((0..300).contains(&col) && (0..MAP_ROWS as i32).contains(&row))
            .then(|| self.map[(row * MAP_ROW_CELLS as i32 + col) as usize])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{AmigaMemory, SLOW_BASE, def};

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
            s.rng,
            Rng {
                lo: 0x0102_0304,
                hi: 0x0506_0708
            }
        );
        let p = s.probe();
        assert_eq!(
            &p[..2],
            &[
                (VarId::RngState, 0x0102_0304),
                (VarId::RngState, 0x0506_0708)
            ]
        );
        assert!(p[2..].iter().all(|&(_, v)| v == 0));
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
        assert_eq!(s.rng, Rng { lo: 0, hi: 0xAABB });
        s.poke(0x100, &[1]);
        assert_eq!(s.rng, Rng { lo: 0, hi: 0xAABB });
    }

    #[test]
    fn raw_reads_only_whole_runs_inside_one_raw_variable() {
        let mut s = State::default();
        let at = def(VarId::LevelCopper).addr;
        s.copper[4] = 9;
        assert_eq!(s.raw(at + 4, 2), Some(&[9, 0][..]));
        assert_eq!(s.raw(at + 0x3ff, 2), None);
        assert_eq!(s.raw(def(VarId::CameraX).addr, 4), None);
    }

    #[test]
    fn cells_outside_the_map_are_none() {
        let mut s = State::default();
        s.map[MAP_ROW_CELLS as usize + 2] = 0x3E09;
        assert_eq!(s.cell(2, 1), Some(0x3E09));
        assert_eq!(s.cell(-1, 1), None);
        assert_eq!(s.cell(300, 1), None);
        assert_eq!(s.cell(0, 84), None);
    }
}
