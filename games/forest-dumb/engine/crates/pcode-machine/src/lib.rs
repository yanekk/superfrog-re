//! The machine translated 68000 code runs on (translator DESIGN §2.8): memory laid out as the
//! oracle's A500, the program's hunks at given bases, a `pcode_rt::Bus` that routes custom-register
//! and CIA addresses to devices, and the beam from counted cycles. Interrupts are taken in `poll`,
//! at a translated block's start (`irq`). The blitter, the copper and the picture are the machine's
//! own (T24: [`blitter`], [`copper`], `video`, frames through [`Machine::end_frame`]); sound,
//! input and the OS plug in as [`Device`]s and raise requests with [`Machine::raise`] (T25, T26).
//!
//! Pure: no clock, I/O, floats or hash maps. Names no game: the program's bytes, its hunk bases and
//! the RAM sizes are input.

use std::fmt;
use std::ops::Range;

use amiga_chips::copper::Fetch;
use pcode_rt::{Bus, Image, PC, Regs, Stop, ops};

use blitter::Blitter;
use journal::{Journal, Watch};

pub mod beam;
pub mod blitter;
pub mod cia;
pub mod copper;
pub mod custom;
pub mod input;
pub mod irq;
pub mod journal;
pub mod mem;
pub mod paula;
mod video;

pub use amiga_chips::Picture;
pub use cia::CiaA;
pub use custom::Cia;
pub use input::Input;
pub use irq::{Entry, Routine};
use mem::{Region, SLOW_BASE};
pub use paula::PaulaLog;

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

/// One frame as the machine ran it ([`Machine::end_frame`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// What the frame showed.
    pub picture: Picture,
    /// Every custom-register write of the frame, the CPU's and the copper's, in order: (cycle,
    /// register offset from `$DFF000`, word). A copper interrupt is its MOVE to INTREQ here.
    pub writes: Vec<(u64, u16, u16)>,
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
    /// What a CIA-B register without a device reads: its last byte written. (Index 0, CIA-A's,
    /// is unused: without a device, CIA-A is [`CiaA`].)
    cia_latch: [[u8; 16]; 2],
    /// CIA-A's model, used when no device is attached to it.
    cia_a: CiaA,
    /// Paula's writes this frame.
    paula: PaulaLog,
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
    /// INTENA and INTREQ as Paula holds them (bit 15 never set).
    intena: u16,
    intreq: u16,
    /// Requests raised for a cycle count not yet reached: (INTREQ bit, at).
    raised: Vec<(u16, u64)>,
    /// The translated program's `entry`, where `poll` finds a vector's handler.
    entry: Option<Entry>,
    /// DMACON as its set/clear writes leave it (DMACONR, with the blitter's BBUSY and BZERO).
    dmacon: u16,
    blitter: Blitter,
    /// The current frame's copper; `None` until the first access to time starts it.
    copper: Option<copper::Copper>,
    /// The current frame's chip writes, and the chip words the copper's run reads.
    journal: Journal,
    watch: Watch,
    /// The current frame's custom writes, in order.
    log: Vec<(u64, u16, u16)>,
    /// The registers and DMACON as the current frame found them.
    frame_custom: [u16; 256],
    frame_dmacon: u16,
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
            cia_a: CiaA::default(),
            paula: PaulaLog::default(),
            hunk_sizes: Vec::new(),
            now: 0,
            phase: 0,
            bus: 0,
            fault: None,
            intena: 0,
            intreq: 0,
            raised: Vec::new(),
            entry: None,
            dmacon: 0,
            blitter: Blitter::default(),
            copper: None,
            journal: Journal::default(),
            watch: Watch::default(),
            log: Vec::new(),
            frame_custom: [0; 256],
            frame_dmacon: 0,
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
    /// the beam at cycle 0; INTENAR and INTREQR (`custom[0x0E]`, `custom[0x0F]`) are INTENA's and
    /// INTREQ's state; the rest are the registers' values. Hunk sizes are unknown here.
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
        // DMACONR gives DMACON's state; BBUSY and BZERO are the blitter's, with no blit running here.
        m.dmacon = custom[custom::DMACONR as usize >> 1] & 0x07FF;
        m.intena = custom[irq::INTENAR as usize >> 1] & 0x7FFF;
        m.intreq = custom[irq::INTREQR as usize >> 1] & 0x7FFF;
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
        self.advance(cycles);
        self.now = cycles;
    }

    /// Hand the machine the translated program's `entry`, so `poll` can call the handler an
    /// interrupt vector points at. Without one, a taken interrupt stops the run as untranslated.
    pub fn set_entry(&mut self, entry: Entry) {
        self.entry = Some(entry);
    }

    /// INTENA as Paula holds it (what INTENAR reads).
    pub fn intena(&self) -> u16 {
        self.intena
    }

    /// INTREQ as Paula holds it, with every request raised for a count up to `cycles()`.
    pub fn intreq(&mut self) -> u16 {
        self.settle();
        self.intreq
    }

    /// A device sets INTREQ bit `bit` (0 to 13) at cycle count `at`: seen from the first INTREQR
    /// read, INTREQ write or block start timed at `at` or later. Panics on a bit that raises no interrupt.
    pub fn raise(&mut self, bit: u16, at: u64) {
        irq::level_of(bit);
        self.raised.push((bit, at));
    }

    /// The level to take now, with the status register `sr`: INTENA & INTREQ with the master
    /// enable on, above SR's mask.
    fn pending(&self, sr: u16) -> Option<u8> {
        irq::above_mask(irq::ipl(self.intena, self.intreq), sr)
    }

    /// Move into INTREQ the requests raised for a count up to now.
    fn settle(&mut self) {
        let now = self.now;
        let mut due = 0;
        self.raised.retain(|&(bit, at)| {
            let d = at <= now;
            if d {
                due |= 1 << bit;
            }
            !d
        });
        self.intreq |= due;
    }

    /// Take a level `level` interrupt as the 68000 does: SR copied, S set, T cleared and the mask
    /// raised (A7 becomes the system stack if it was the user's), PC and the old SR pushed, then
    /// the handler at the vector's address called. Its `rte` pops the frame; anything else coming
    /// back stops the run, since control cannot follow it.
    fn take(&mut self, level: u8, r: &mut Regs, img: &Image) -> Result<(), Stop> {
        let old = ops::sr_get(r) as u16;
        // The interrupted code's PC register is not the 68000's PC (generated code keeps that in
        // a local), but it holds a call's return address between `rts` and its check: keep it.
        let pc = r.get(PC, 4);
        ops::sr_set(r, irq::handler_sr(old, level) as u64);
        r.tick(irq::IRQ_CYCLES);
        self.now = r.cycles();
        let sp = r.a(7).wrapping_sub(6);
        pcode_rt::store(self, img, sp.wrapping_add(2), 4, irq::IRQ_RETURN as u64)?;
        pcode_rt::store(self, img, sp, 2, old as u64)?;
        r.set_a(7, sp);
        let at = self.rl(irq::vector(level)) & 0xFF_FFFF;
        let handler = img
            .hunk_of(at)
            .and(self.entry)
            .and_then(|entry| entry(&img.describe(at)))
            .ok_or_else(|| Stop::Untranslated(img.describe(at)))?;
        handler(r, self, img)?;
        let back = r.get(PC, 4) as u32;
        if back != irq::IRQ_RETURN {
            return Err(Stop::Unsupported {
                op: format!(
                    "interrupt handler returned to {}, not by rte",
                    img.describe(back)
                ),
                at: img.describe(at),
            });
        }
        r.set(PC, 4, pc);
        self.now = r.cycles();
        Ok(())
    }

    /// DMACON as its set/clear writes leave it.
    pub fn dmacon(&self) -> u16 {
        self.dmacon
    }

    /// The cycle the current frame ends at: the host calls [`Machine::end_frame`] when the CPU's
    /// count reaches it. Until then the copper's writes past it wait.
    pub fn frame_end(&mut self) -> u64 {
        self.ensure_frame();
        let c = self.copper.as_ref().expect("started");
        (c.frame_start + beam::frame_cycles(self.cfg.frame_lines) as i64).max(0) as u64
    }

    /// End the current frame: the copper's and the blitter's events up to its end happen, its
    /// picture is drawn, and the next frame starts (the copper restarted at COP1LC). Returns the
    /// picture and the frame's custom writes. A picture the machine cannot draw (HAM, dual
    /// playfield) is empty, and the run stops at the next `poll`.
    pub fn end_frame(&mut self) -> Frame {
        let end = self.frame_end();
        let saved = self.now;
        self.advance(end.saturating_sub(1));
        self.now = saved;
        let c = self.copper.as_ref().expect("started");
        let history = self.journal.history();
        let drawn = video::draw(&video::FrameIn {
            chip: &self.chip,
            history: &history,
            frame_start: c.frame_start,
            frame_lines: self.cfg.frame_lines,
            start: &self.frame_custom,
            dmacon: self.frame_dmacon,
            writes: &self.log,
        });
        let picture = drawn.unwrap_or_else(|op| {
            self.unsupported(op);
            Picture::blank(0, 0, 0, 0)
        });
        let writes = std::mem::take(&mut self.log);
        self.start_frame(end as i64, end as i64);
        Frame { picture, writes }
    }

    /// Start the frame the beam is in now, if none is: a machine's first access to time.
    fn ensure_frame(&mut self) {
        if self.copper.is_none() {
            let into = self.phase.wrapping_add(self.now) % beam::frame_cycles(self.cfg.frame_lines);
            self.start_frame(self.now as i64 - into as i64, self.now as i64);
        }
    }

    /// Make every chip event due by cycle `t`, in time order: the copper's writes and the end of a
    /// blit. Leaves the access time at `t`'s caller's to set.
    fn advance(&mut self, t: u64) {
        self.ensure_frame();
        loop {
            if self.watch.dirty {
                self.copper_rerun();
            }
            let cop = self
                .copper
                .as_ref()
                .and_then(|c| c.next_at())
                .filter(|&c| c <= t as i64);
            let blit = self.blitter.busy_until.filter(|&b| b <= t);
            // The earlier first; a blit ending on a copper write's cycle ends before it.
            match (cop, blit) {
                (Some(c), Some(b)) if c < b as i64 => self.copper_deliver(),
                (_, Some(b)) => {
                    self.now = b;
                    self.blitter.busy_until = None;
                    self.raise(blitter::INT_BLIT.trailing_zeros() as u16, b);
                }
                (Some(_), None) => self.copper_deliver(),
                (None, None) => break,
            }
        }
    }

    /// The bitplane set-up the registers hold now.
    pub(crate) fn fetch(&self) -> Fetch {
        let g = |reg: u16| self.custom[reg as usize >> 1];
        use amiga_chips::copper::reg;
        Fetch {
            bplcon0: g(reg::BPLCON0),
            diwstrt: g(reg::DIWSTRT),
            diwstop: g(reg::DIWSTOP),
            ddfstrt: g(reg::DDFSTRT),
            ddfstop: g(reg::DDFSTOP),
        }
    }

    /// Stop the run at the next `poll`: the machine does not do `op`.
    pub(crate) fn unsupported(&mut self, op: &str) {
        if self.fault.is_none() {
            self.fault = Some(Stop::Unsupported {
                op: op.into(),
                at: format!("cycle {}", self.now),
            });
        }
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
            Region::Chip(i) => {
                // Journalled for the copper and the picture, which read chip RAM as of a beam
                // position; an unchanged byte changes no reading.
                let old = self.chip[i];
                if old != v {
                    self.journal.record(self.now, i as u32, old);
                    self.watch.written(i as u32, self.now);
                    self.chip[i] = v;
                }
            }
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
        let routed = self.route[reg as usize >> 1] != 0;
        let v = match reg {
            // A device given these registers answers them; otherwise the machine's own state.
            custom::DMACONR if !routed => {
                let busy = if self.blit_busy() { 0x4000 } else { 0 };
                let zero = if self.blitter.zero { 0x2000 } else { 0 };
                self.dmacon | busy | zero
            }
            irq::INTENAR => self.intena,
            irq::INTREQR => self.intreq(),
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

    /// A CPU write: on the data bus, then the register.
    fn custom_write(&mut self, reg: u16, v: u16) {
        self.bus = v;
        self.reg_write(reg, v, true);
    }

    /// A write to custom register `reg` by the CPU (`cpu`) or the copper, at the access time.
    pub(crate) fn reg_write(&mut self, reg: u16, v: u16, cpu: bool) {
        self.log.push((self.now, reg, v));
        if custom::read_only(reg) {
            return;
        }
        self.paula.note(reg, v, self.now, self.beam(self.now));
        self.custom[reg as usize >> 1] = v;
        match reg {
            custom::DMACON => {
                let old = self.dmacon;
                self.dmacon = irq::set_clear(old, v) & 0x07FF;
                let on = |d: u16| d & copper::COPPER_DMA == copper::COPPER_DMA;
                match (on(old), on(self.dmacon)) {
                    (false, true) => self.copper_on(),
                    (true, false) => self.copper_stop(),
                    _ => {}
                }
            }
            irq::INTENA => self.intena = irq::set_clear(self.intena, v),
            irq::INTREQ => {
                // A request raised before this write is cleared by it like any other.
                self.settle();
                self.intreq = irq::set_clear(self.intreq, v);
            }
            blitter::BLTSIZE => self.start_blit(),
            copper::COPJMP1 | copper::COPJMP2 if cpu => {
                let lc = if reg == copper::COPJMP1 {
                    copper::COP1LC
                } else {
                    copper::COP2LC
                };
                self.copper_strobe(lc);
            }
            // A list that jumps is not followed (`run_bus` reads on past the MOVE).
            copper::COPJMP1 | copper::COPJMP2 => self.unsupported("copper jump"),
            _ => {}
        }
        if let id @ 1.. = self.route[reg as usize >> 1] {
            self.devices[id as usize - 1].write(reg, v, self.now);
        }
    }

    fn cia_read(&mut self, cia: Cia, reg: u16) -> u8 {
        match self.cia_route[cia as usize] {
            0 if cia == Cia::A => {
                let v = self.cia_a.read(reg, self.now);
                self.cia_fault();
                v
            }
            0 => self.cia_latch[cia as usize][reg as usize],
            id => self.devices[id as usize - 1].read(reg, self.now) as u8,
        }
    }

    fn cia_write(&mut self, cia: Cia, reg: u16, v: u8) {
        match self.cia_route[cia as usize] {
            0 if cia == Cia::A => {
                self.cia_a.write(reg, v, self.now);
                self.cia_fault();
            }
            0 => self.cia_latch[cia as usize][reg as usize] = v,
            id => self.devices[id as usize - 1].write(reg, v as u16, self.now),
        }
    }
}

impl Machine {
    /// A timer touched on CIA-A: the run stops at the next block, as for an access outside memory.
    fn cia_fault(&mut self) {
        if let (None, Some(reg)) = (&self.fault, self.cia_a.fault()) {
            self.fault = Some(Stop::Unsupported {
                op: "CIA-A timer".into(),
                at: format!("${:06x}", 0xBF_E001 | (reg as u32) << 8),
            });
        }
    }

    /// The next frame's input, at the current cycle count: JOY1DAT reads `i.joy1`, CIA-A's PRA
    /// the fire buttons, and the keyboard queues a byte for each key gone down or up, sending the
    /// first at once if the program acknowledged the last (`cia`).
    pub fn set_input(&mut self, i: &Input) {
        self.custom[custom::JOY1DAT as usize >> 1] = i.joy1;
        self.cia_a.fire = i.fire;
        self.cia_a.mouse_left = i.mouse_left;
        self.cia_a.set_keys(i.key_set(), self.now);
    }

    /// Paula's writes this frame, in order: the cycle count each was made at, and the write with
    /// its beam position.
    pub fn paula_writes(&self) -> &[(u64, amiga_paula::PaulaWrite)] {
        self.paula.writes()
    }

    /// The frame's Paula writes, emptying the list for the next frame.
    pub fn take_paula_writes(&mut self) -> Vec<(u64, amiga_paula::PaulaWrite)> {
        self.paula.take()
    }

    /// The INTREQ bits the CIAs have asked for since the last call: PORTS (`$0008`) when CIA-A's
    /// interrupt line went down. The interrupt logic sets them in INTREQ.
    pub fn take_cia_interrupts(&mut self) -> u16 {
        if self.cia_a.take_interrupt() {
            INTREQ_PORTS
        } else {
            0
        }
    }

    /// CIA-A's model (when no device is attached to it): at a snapshot, set its ICR mask as the
    /// program left it.
    pub fn cia_a(&self) -> &CiaA {
        &self.cia_a
    }
    pub fn cia_a_mut(&mut self) -> &mut CiaA {
        &mut self.cia_a
    }
}

/// INTREQ's PORTS bit: level 2, CIA-A's interrupt line.
pub const INTREQ_PORTS: u16 = 0x0008;

impl Bus for Machine {
    /// Times the block's accesses, ends the run on an access outside memory made since, and takes
    /// every interrupt pending above SR's mask, highest first: a handler's own blocks poll too, so
    /// a higher level nests inside it, and one left pending after its `rte` is taken here at once,
    /// as a 68000 takes it before the next instruction.
    fn poll(&mut self, r: &mut Regs, img: &Image) -> Result<(), Stop> {
        self.advance(r.cycles());
        self.now = r.cycles();
        // The keyboard's next byte, if its time has come since the last block.
        self.cia_a.tick(self.now);
        if let Some(f) = &self.fault {
            return Err(f.clone());
        }
        loop {
            self.settle();
            match self.pending(ops::sr_get(r) as u16) {
                Some(level) => self.take(level, r, img)?,
                None => return Ok(()),
            }
            if let Some(f) = &self.fault {
                return Err(f.clone());
            }
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
#[cfg(test)]
mod tests_chips;
