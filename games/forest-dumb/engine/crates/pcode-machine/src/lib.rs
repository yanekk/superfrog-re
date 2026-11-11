//! The machine translated 68000 code runs on (translator DESIGN §2.8): memory laid out as the
//! oracle's A500, the program's hunks at given bases, a `pcode_rt::Bus` that routes custom-register
//! and CIA addresses to devices, and the beam from counted cycles. The chips, interrupts, sound,
//! input and OS plug in as [`Device`]s and through `poll` (translator T23 to T26).
//!
//! Pure: no clock, I/O, floats or hash maps. Names no game: the program's bytes, its hunk bases and
//! the RAM sizes are input.

use std::fmt;
use std::ops::Range;

use pcode_rt::{Bus, Image, Regs, Stop};

pub mod beam;
pub mod custom;
pub mod mem;

pub use custom::Cia;
use mem::{Region, SLOW_BASE};

/// The machine's shape.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    /// Chip RAM in bytes: a power of two, at most 2 MB. Mirrored up to `$200000`.
    pub chip: u32,
    /// Slow RAM at `$C00000`, in bytes, at most 1.5 MB.
    pub slow: u32,
    /// Where each hunk of the program is loaded, as the program's own loader placed it.
    pub hunk_bases: Vec<u32>,
    /// Lines in a frame: 313 for a long PAL frame.
    pub frame_lines: u16,
}

impl Config {
    /// The oracle's A500 (vAmiga `A500_OCS_1MB`): 512 KB chip, 512 KB slow, long PAL frames.
    pub fn a500(hunk_bases: Vec<u32>) -> Config {
        Config {
            chip: 0x8_0000,
            slow: 0x8_0000,
            hunk_bases,
            frame_lines: amiga_clock::FRAME_LINES,
        }
    }

    fn check(&self) -> Result<(), LoadError> {
        let bad = |why: String| Err(LoadError::Config(why));
        if !self.chip.is_power_of_two() || self.chip > mem::MAX_CHIP {
            return bad(format!(
                "chip RAM ${:x}: not a power of two up to 2 MB",
                self.chip
            ));
        }
        if self.slow > mem::MAX_SLOW {
            return bad(format!("slow RAM ${:x}: over 1.5 MB", self.slow));
        }
        if self.frame_lines == 0 {
            return bad("a frame of 0 lines".into());
        }
        Ok(())
    }
}

/// Why a program could not be loaded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoadError {
    /// The configuration itself is impossible.
    Config(String),
    /// The hunk file does not parse.
    Hunk(String),
    /// The file's hunks and the configured bases differ in number.
    HunkCount { file: usize, bases: usize },
    /// A hunk at its base does not lie wholly in chip or slow RAM.
    Placement { hunk: usize, base: u32, size: u32 },
    /// A relocation names a hunk the file lacks, or patches a long outside its hunk.
    Reloc { hunk: usize, why: String },
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Config(why) => write!(f, "machine configuration: {why}"),
            LoadError::Hunk(why) => write!(f, "hunk file: {why}"),
            LoadError::HunkCount { file, bases } => {
                write!(
                    f,
                    "hunk file has {file} hunks, the configuration {bases} bases"
                )
            }
            LoadError::Placement { hunk, base, size } => write!(
                f,
                "hunk {hunk} (${size:x} bytes) at ${base:06x} is not inside chip or slow RAM"
            ),
            LoadError::Reloc { hunk, why } => write!(f, "hunk {hunk}'s relocations: {why}"),
        }
    }
}

impl std::error::Error for LoadError {}

/// A chip behind some custom registers, or a CIA. `reg` is the custom register's even offset from
/// `$DFF000`, or the CIA's register 0 to 15 (a CIA reads and writes the low byte). `at` is the
/// cycle count at the access.
///
/// `Send` so a host can run the machine on a thread of its own (DESIGN §3.3).
pub trait Device: Send {
    fn read(&mut self, reg: u16, at: u64) -> u16;
    fn write(&mut self, reg: u16, v: u16, at: u64);
}

/// The machine: memory, the custom registers' last written values, the beam and the devices.
pub struct Machine {
    cfg: Config,
    chip: Vec<u8>,
    slow: Vec<u8>,
    /// Each custom register's last word written (by `reg / 2`), and what a readable one without a
    /// device reads. Writes to read-only registers do not land here.
    custom: [u16; 256],
    devices: Vec<Box<dyn Device>>,
    /// Per custom register (`reg / 2`), its device's index plus one; 0 for none.
    route: [u8; 256],
    /// CIA-A's and CIA-B's device index plus one; 0 for none.
    cia_route: [u8; 2],
    /// What a CIA register without a device reads: its last byte written.
    cia_latch: [[u8; 16]; 2],
    hunk_sizes: Vec<u32>,
    /// The cycle count as of the last `poll` (or `set_cycles`): the time of every access until the
    /// next. The bus is not handed the registers, so an access inside a block is timed at the
    /// block's start, at most one block early (the frame bar admits it, DESIGN §2.7, §2.8).
    now: u64,
    /// The beam's offset into a frame at cycle count 0.
    phase: u64,
    /// The last word on the data bus (vAmiga `Memory::dataBus`).
    bus: u16,
    fault: Option<Stop>,
}

impl Machine {
    fn empty(cfg: Config) -> Machine {
        Machine {
            chip: vec![0; cfg.chip as usize],
            slow: vec![0; cfg.slow as usize],
            cfg,
            custom: [0; 256],
            devices: Vec::new(),
            route: [0; 256],
            cia_route: [0; 2],
            cia_latch: [[0; 16]; 2],
            hunk_sizes: Vec::new(),
            now: 0,
            phase: 0,
            bus: 0,
            fault: None,
        }
    }

    /// `program` (an AmigaDOS hunk file) with hunk `n` at `cfg.hunk_bases[n]`, relocated, in
    /// otherwise zeroed RAM; the beam at line 0's start at cycle 0.
    pub fn new(cfg: Config, program: &[u8]) -> Result<Machine, LoadError> {
        cfg.check()?;
        let hunks = amiga_disk::hunk::parse(program).map_err(|e| LoadError::Hunk(e.to_string()))?;
        if hunks.len() != cfg.hunk_bases.len() {
            return Err(LoadError::HunkCount {
                file: hunks.len(),
                bases: cfg.hunk_bases.len(),
            });
        }
        let mut m = Machine::empty(cfg);
        for (n, hk) in hunks.iter().enumerate() {
            let base = m.cfg.hunk_bases[n];
            let size = hk.size.max(hk.data.len() as u32);
            let Some(ram) = m.ram_mut(base, size) else {
                return Err(LoadError::Placement {
                    hunk: n,
                    base,
                    size,
                });
            };
            ram[..hk.data.len()].copy_from_slice(&hk.data);
            m.hunk_sizes.push(size);
        }
        for (n, hk) in hunks.iter().enumerate() {
            let (base, size) = (m.cfg.hunk_bases[n], m.hunk_sizes[n]);
            for r in &hk.relocs {
                let target =
                    *m.cfg
                        .hunk_bases
                        .get(r.target as usize)
                        .ok_or_else(|| LoadError::Reloc {
                            hunk: n,
                            why: format!("to hunk {}, which the file lacks", r.target),
                        })?;
                for &off in &r.offsets {
                    if off.checked_add(4).is_none_or(|e| e > size) {
                        return Err(LoadError::Reloc {
                            hunk: n,
                            why: format!("a long at ${off:x}, outside the hunk"),
                        });
                    }
                    let at = m.ram_mut(base + off, 4).expect("inside the placed hunk");
                    let v = u32::from_be_bytes([at[0], at[1], at[2], at[3]]).wrapping_add(target);
                    at.copy_from_slice(&v.to_be_bytes());
                }
            }
        }
        Ok(m)
    }

    /// A case's sync point: `mem` is chip RAM then slow RAM, as the oracle's `AmigaMemory` holds
    /// them, `custom` the registers by `reg / 2`. VPOSR and VHPOSR (`custom[2]`, `custom[3]`) place
    /// the beam at cycle 0; the rest are the registers' values. Hunk sizes are unknown here.
    ///
    /// Panics on an impossible `cfg` or on `mem` not `cfg.chip + cfg.slow` bytes: both are the
    /// host's own facts, not the program's.
    pub fn from_snapshot(cfg: Config, mem: &[u8], custom: &[u16; 256]) -> Machine {
        if let Err(e) = cfg.check() {
            panic!("{e}");
        }
        let chip = cfg.chip as usize;
        assert_eq!(
            mem.len(),
            chip + cfg.slow as usize,
            "a snapshot is chip RAM then slow RAM"
        );
        let mut m = Machine::empty(cfg);
        m.chip.copy_from_slice(&mem[..chip]);
        m.slow.copy_from_slice(&mem[chip..]);
        m.custom = *custom;
        let (line, h) = beam::decode(custom[2], custom[3]);
        m.sync_beam(0, line, h);
        m
    }

    pub fn config(&self) -> &Config {
        &self.cfg
    }

    /// Each hunk's size as loaded (empty for a snapshot): with `config().hunk_bases`, what a
    /// `pcode_rt::Image` needs.
    pub fn hunk_sizes(&self) -> &[u32] {
        &self.hunk_sizes
    }

    pub fn chip(&self) -> &[u8] {
        &self.chip
    }
    pub fn chip_mut(&mut self) -> &mut [u8] {
        &mut self.chip
    }
    pub fn slow(&self) -> &[u8] {
        &self.slow
    }
    pub fn slow_mut(&mut self) -> &mut [u8] {
        &mut self.slow
    }

    /// The custom registers' last written words, by `reg / 2`.
    pub fn custom(&self) -> &[u16; 256] {
        &self.custom
    }

    /// The first access outside memory, if any: the stop the next `poll` returns.
    pub fn fault(&self) -> Option<&Stop> {
        self.fault.as_ref()
    }

    /// The cycle count accesses are timed at: set by every `poll`, or by a host stepping the
    /// machine itself.
    pub fn cycles(&self) -> u64 {
        self.now
    }
    pub fn set_cycles(&mut self, cycles: u64) {
        self.now = cycles;
    }

    /// The beam's `(line, h)` at cycle count `cycles`: h in DMA slots (colour clocks), 227 a line.
    pub fn beam(&self, cycles: u64) -> (u16, u16) {
        beam::position(self.phase.wrapping_add(cycles), self.cfg.frame_lines)
    }

    /// Make the beam be at `(line, h)` when the cycle count is `cycles`.
    pub fn sync_beam(&mut self, cycles: u64, line: u16, h: u16) {
        let frame = beam::frame_cycles(self.cfg.frame_lines);
        let target = beam::offset(line, h) % frame;
        self.phase = (target + frame - cycles % frame) % frame;
    }

    /// Route the custom registers `regs` (even offsets from `$DFF000`, end exclusive) to `dev`.
    /// VPOSR and VHPOSR stay the beam's. Panics on a register already routed.
    pub fn attach(&mut self, regs: Range<u16>, dev: Box<dyn Device>) {
        let id = self.add(dev);
        for reg in regs.step_by(2) {
            let slot = &mut self.route[(reg as usize >> 1) & 0xFF];
            assert_eq!(*slot, 0, "custom register ${reg:03x} already has a device");
            *slot = id;
        }
    }

    /// Give CIA `cia` to `dev`. Panics if it already has one.
    pub fn attach_cia(&mut self, cia: Cia, dev: Box<dyn Device>) {
        assert_eq!(
            self.cia_route[cia as usize], 0,
            "{cia:?} already has a device"
        );
        self.cia_route[cia as usize] = self.add(dev);
    }

    fn add(&mut self, dev: Box<dyn Device>) -> u8 {
        self.devices.push(dev);
        u8::try_from(self.devices.len()).expect("at most 255 devices")
    }

    /// `len` bytes of RAM at `a`, if they lie wholly in chip RAM (not a mirror) or slow RAM.
    fn ram_mut(&mut self, a: u32, len: u32) -> Option<&mut [u8]> {
        let end = a.checked_add(len)?;
        if end <= self.cfg.chip {
            return Some(&mut self.chip[a as usize..end as usize]);
        }
        if a >= SLOW_BASE && end - SLOW_BASE <= self.cfg.slow {
            return Some(&mut self.slow[(a - SLOW_BASE) as usize..(end - SLOW_BASE) as usize]);
        }
        None
    }

    fn region(&self, a: u32) -> Region {
        mem::region(a, self.cfg.chip, self.cfg.slow)
    }

    fn outside(&mut self, a: u32) {
        if self.fault.is_none() {
            self.fault = Some(Stop::Outside(format!("${:06x}", a & 0xFF_FFFF)));
        }
    }

    fn ram_read(&mut self, a: u32) -> Option<u8> {
        match self.region(a) {
            Region::Chip(i) => Some(self.chip[i]),
            Region::Slow(i) => Some(self.slow[i]),
            _ => None,
        }
    }

    fn ram_write(&mut self, a: u32, v: u8) -> bool {
        match self.region(a) {
            Region::Chip(i) => self.chip[i] = v,
            Region::Slow(i) => self.slow[i] = v,
            _ => return false,
        }
        true
    }

    fn custom_read(&mut self, reg: u16) -> u16 {
        if custom::write_only(reg) {
            // The 68000 bus's rule: the register takes the last word on the bus, which is read.
            let w = self.bus;
            self.custom_write(reg, w);
            return w;
        }
        let v = match reg {
            custom::VPOSR => beam::vposr(self.beam(self.now).0, self.cfg.frame_lines),
            custom::VHPOSR => {
                let (line, h) = self.beam(self.now);
                beam::vhposr(line, h)
            }
            _ => match self.route[reg as usize >> 1] {
                0 => self.custom[reg as usize >> 1],
                id => self.devices[id as usize - 1].read(reg, self.now),
            },
        };
        self.bus = v;
        v
    }

    fn custom_write(&mut self, reg: u16, v: u16) {
        self.bus = v;
        if custom::read_only(reg) {
            return;
        }
        self.custom[reg as usize >> 1] = v;
        if let id @ 1.. = self.route[reg as usize >> 1] {
            self.devices[id as usize - 1].write(reg, v, self.now);
        }
    }

    fn cia_read(&mut self, cia: Cia, reg: u16) -> u8 {
        match self.cia_route[cia as usize] {
            0 => self.cia_latch[cia as usize][reg as usize],
            id => self.devices[id as usize - 1].read(reg, self.now) as u8,
        }
    }

    fn cia_write(&mut self, cia: Cia, reg: u16, v: u8) {
        match self.cia_route[cia as usize] {
            0 => self.cia_latch[cia as usize][reg as usize] = v,
            id => self.devices[id as usize - 1].write(reg, v as u16, self.now),
        }
    }
}

impl Bus for Machine {
    /// Times the block's accesses, and ends the run on an access outside memory made since.
    fn poll(&mut self, r: &mut Regs, _img: &Image) -> Result<(), Stop> {
        self.now = r.cycles();
        match &self.fault {
            Some(f) => Err(f.clone()),
            None => Ok(()),
        }
    }

    fn rb(&mut self, a: u32) -> u8 {
        match self.region(a) {
            Region::Chip(_) | Region::Slow(_) => {
                let v = self.ram_read(a).expect("RAM");
                self.bus = v as u16;
                v
            }
            Region::Custom(reg) => {
                let w = self.custom_read(reg);
                let v = if a & 1 == 0 { (w >> 8) as u8 } else { w as u8 };
                self.bus = v as u16;
                v
            }
            Region::Cia(a) => {
                let reg = custom::cia_reg(a);
                let v = match custom::cia_read8(a) {
                    Some(cia) => self.cia_read(cia, reg),
                    // No chip answers: the bus holds the instruction word being fetched.
                    None if a & 1 == 0 => (self.bus >> 8) as u8,
                    None => self.bus as u8,
                };
                // vAmiga `peek8<CIA>` puts the byte on the bus, as RAM and custom reads do.
                self.bus = v as u16;
                v
            }
            Region::Outside => {
                self.outside(a);
                0
            }
        }
    }

    fn rw(&mut self, a: u32) -> u16 {
        match self.region(a) {
            Region::Chip(_) | Region::Slow(_) => {
                let hi = self.rb(a);
                let lo = self.rb(a.wrapping_add(1));
                let v = (hi as u16) << 8 | lo as u16;
                self.bus = v;
                v
            }
            Region::Custom(reg) => self.custom_read(reg),
            Region::Cia(a) => {
                let reg = custom::cia_reg(a);
                let v = match (a >> 12) & 3 {
                    0 => {
                        (self.cia_read(Cia::B, reg) as u16) << 8 | self.cia_read(Cia::A, reg) as u16
                    }
                    1 => (self.cia_read(Cia::B, reg) as u16) << 8 | 0xFF,
                    2 => 0xFF00 | self.cia_read(Cia::A, reg) as u16,
                    _ => self.bus,
                };
                self.bus = v;
                v
            }
            Region::Outside => {
                self.outside(a);
                0
            }
        }
    }

    fn rl(&mut self, a: u32) -> u32 {
        let hi = self.rw(a);
        (hi as u32) << 16 | self.rw(a.wrapping_add(2)) as u32
    }

    fn wb(&mut self, a: u32, v: u8) {
        match self.region(a) {
            Region::Chip(_) | Region::Slow(_) => {
                self.ram_write(a, v);
                self.bus = v as u16;
            }
            // A byte reaches a custom register on both halves of the data bus.
            Region::Custom(reg) => self.custom_write(reg, (v as u16) << 8 | v as u16),
            Region::Cia(a) => {
                let (sel_a, sel_b) = custom::cia_write(a);
                let reg = custom::cia_reg(a);
                self.bus = v as u16;
                if sel_a {
                    self.cia_write(Cia::A, reg, v);
                }
                if sel_b {
                    self.cia_write(Cia::B, reg, v);
                }
            }
            Region::Outside => self.outside(a),
        }
    }

    fn ww(&mut self, a: u32, v: u16) {
        match self.region(a) {
            Region::Chip(_) | Region::Slow(_) => {
                self.wb(a, (v >> 8) as u8);
                self.wb(a.wrapping_add(1), v as u8);
                self.bus = v;
            }
            Region::Custom(reg) => self.custom_write(reg, v),
            Region::Cia(a) => {
                let (sel_a, sel_b) = custom::cia_write(a);
                let reg = custom::cia_reg(a);
                self.bus = v;
                if sel_a {
                    self.cia_write(Cia::A, reg, v as u8);
                }
                if sel_b {
                    self.cia_write(Cia::B, reg, (v >> 8) as u8);
                }
            }
            Region::Outside => self.outside(a),
        }
    }

    fn wl(&mut self, a: u32, v: u32) {
        self.ww(a, (v >> 16) as u16);
        self.ww(a.wrapping_add(2), v as u16);
    }
}

#[cfg(test)]
mod tests;
