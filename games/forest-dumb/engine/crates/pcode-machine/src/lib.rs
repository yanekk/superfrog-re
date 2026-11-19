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

pub(crate) mod audio;
pub mod beam;
pub mod blitter;
pub mod cia;
pub mod collide;
pub mod copper;
pub mod custom;
mod dma;
pub mod input;
pub mod irq;
pub mod journal;
pub mod mem;
pub mod os;
pub mod paula;
mod video;

pub use amiga_chips::Picture;
pub use cia::CiaA;
pub use custom::Cia;
pub use input::Input;
pub use irq::{Entry, Routine};
use mem::{Region, SLOW_BASE};
pub use os::{Disks, Library};
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
    /// The CPU's of `writes` alone, in order: what the oracle keeps (`Amiga::cpu_writes`), so the
    /// frame bar compares like with like (translator T45).
    pub cpu_writes: Vec<(u64, u16, u16)>,
    /// Indices into `cpu_writes` of the writes a read of a write-only register made, with the
    /// bus's leftover word ([`pcode_rt::Bus::ww_leftover`], translator T47).
    pub cpu_leftover: Vec<usize>,
    /// Chip RAM then slow RAM at each CPU write [`Machine::watch_write`] names, in order.
    pub watched: Vec<Vec<u8>>,
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
    /// The cycle count up to which the bus's slots have been run (`dma`): every slot before it has
    /// its owner, and a blit has got as far as those slots let it.
    dma_at: u64,
    /// `Regs`' cycles and cycles off the bus at the last `poll_time`, with the CPU's waits for the
    /// bus added: the next block's work is what the counts have grown by since.
    cpu_mark: Option<(u64, u64)>,
    /// Of the block's trail, the instructions `sync` has fitted onto the bus already, their cycles
    /// and cycles off the bus, and of the next one its accesses made and whether its leading idle
    /// cycles are spent.
    synced: (usize, u64, u64, (u64, bool)),
    /// The sprite channels' DMA as the bus has run it (`dma::Sprites`).
    sprites: dma::Sprites,
    /// The audio channels' DMA slots (`audio`, T52).
    audio: audio::Audio,
    /// The last word on the data bus (vAmiga `Memory::dataBus`).
    bus: u16,
    fault: Option<Stop>,
    /// The operating system's answers, its structures and the disk (`os`, translator T26).
    os: os::Os,
    /// INTENA and INTREQ as Paula holds them (bit 15 never set).
    intena: u16,
    intreq: u16,
    /// Requests raised for a cycle count not yet reached: (INTREQ bit, at).
    raised: Vec<(u16, u64)>,
    /// No interrupt can become due before this cycle count unless the CPU changes INTENA, INTREQ,
    /// SR or a chip first ([`Bus::insn`]'s cheap test, translator T50): the next copper write, a
    /// raised request, the keyboard's next byte, the frame's end; 0 while a blit that may
    /// interrupt runs. Set by `poll_time`, cleared by every CPU write to a chip and by one to a
    /// word of chip RAM the copper has yet to read.
    irq_at: u64,
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
    /// The CPU's of them, tagged as each is made: a register filter could not tell a copper MOVE
    /// from the CPU's write to the same register.
    cpu_log: Vec<(u64, u16, u16)>,
    /// Indices into `cpu_log` of the writes made with a leftover bus word.
    cpu_leftover: Vec<usize>,
    /// The write being made is a read's leftover-bus write (`Bus::ww_leftover`, or a read of a
    /// write-only register).
    leftover: bool,
    /// The CPU write whose memory is kept ([`Machine::watch_write`]), and the copies this frame.
    watch_write: Option<(u16, u16)>,
    watched: Vec<Vec<u8>>,
    /// The registers and DMACON as the current frame found them.
    frame_custom: [u16; 256],
    frame_dmacon: u16,
    /// CLXDAT's collision bits gathered since its last read (bit 15 not held).
    clx: u16,
    /// The current frame's first line not yet gathered into `clx`: Denise sets the bits as it
    /// finishes each line, so a read part way down sees the lines above the beam.
    clx_from: u16,
}

impl Machine {
    fn empty(cfg: Config) -> Machine {
        let lines = cfg.frame_lines;
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
            dma_at: 0,
            cpu_mark: None,
            synced: (0, 0, 0, (0, false)),
            sprites: dma::Sprites::default(),
            audio: audio::Audio::new(lines),
            bus: 0,
            fault: None,
            os: os::Os::new(),
            intena: 0,
            intreq: 0,
            raised: Vec::new(),
            irq_at: 0,
            entry: None,
            dmacon: 0,
            blitter: Blitter::default(),
            copper: None,
            journal: Journal::default(),
            watch: Watch::default(),
            log: Vec::new(),
            cpu_log: Vec::new(),
            cpu_leftover: Vec::new(),
            leftover: false,
            watch_write: None,
            watched: Vec::new(),
            frame_custom: [0; 256],
            frame_dmacon: 0,
            clx: 0,
            clx_from: 0,
        }
    }

    /// `program` (an AmigaDOS hunk file) with hunk `n` at `cfg.hunk_bases[n]`, relocated, in
    /// otherwise zeroed RAM but for what Kickstart leaves a program it starts (ExecBase at `$4`, the
    /// autovectors on the OS's handlers, `os`); the beam at line 0's start at cycle 0.
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
        m.install_os();
        Ok(m)
    }

    /// A case's sync point: `mem` is chip RAM then slow RAM, as the oracle's `AmigaMemory` holds
    /// them, `custom` the registers by `reg / 2`. VPOSR and VHPOSR (`custom[2]`, `custom[3]`) place
    /// the beam at cycle 0; INTENAR and INTREQR (`custom[0x0E]`, `custom[0x0F]`) are INTENA's and
    /// INTREQ's state; CLXDAT (`custom[7]`) the collisions gathered and not yet read, as a read
    /// would return them (the oracle's `Amiga::clxdat`); the rest are the registers' values. Hunk sizes are unknown here. `$4` is set
    /// to the machine's ExecBase (`os`): the oracle's would lead into a Kickstart not here.
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
        m.clx = custom[collide::CLXDAT as usize >> 1] & 0x7FFF;
        let (line, h) = beam::decode(custom[2], custom[3]);
        m.sync_beam(0, line, h);
        m.set_exec_base();
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
        self.dma_idle(cycles);
        self.cpu_mark = None;
        self.synced = (0, 0, 0, (0, false));
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

    /// Take a level `level` interrupt as the 68000 does: [`Machine::enter_interrupt`], the handler
    /// called on this machine, [`Machine::leave_interrupt`].
    fn take(&mut self, level: u8, r: &mut Regs, img: &Image) -> Result<(), Stop> {
        let taken = self.enter_interrupt(level, r, img)?;
        (taken.handler)(r, self, img)?;
        self.leave_interrupt(&taken, r, img)
    }

    /// The first half of taking a level `level` interrupt as the 68000 does: SR copied, S set, T
    /// cleared and the mask raised (A7 becomes the system stack if it was the user's), 44 cycles
    /// counted, PC and the old SR pushed, and the handler the vector points at found. The host calls
    /// it with its own bus and then [`Machine::leave_interrupt`]: a host wrapping the machine runs
    /// the handler on its wrapper, so it sees the handler's blocks too (translator T28).
    pub fn enter_interrupt(&mut self, level: u8, r: &mut Regs, img: &Image) -> Result<Taken, Stop> {
        let old = ops::sr_get(r) as u16;
        // The interrupted code's PC register is not the 68000's PC (generated code keeps that in
        // a local), but it holds a call's return address between `rts` and its check: keep it.
        let pc = r.get(PC, 4);
        ops::sr_set(r, irq::handler_sr(old, level) as u64);
        r.tick(irq::IRQ_CYCLES);
        r.off_bus(irq::IRQ_OFF_BUS);
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
            .ok_or_else(|| os::interrupt_stop(at, img))?;
        Ok(Taken { handler, at, pc })
    }

    /// The second half, once the handler has returned: its `rte` popped the frame; anything else
    /// coming back stops the run, since control cannot follow it.
    pub fn leave_interrupt(&mut self, taken: &Taken, r: &mut Regs, img: &Image) -> Result<(), Stop> {
        let back = r.get(PC, 4) as u32;
        if back != irq::IRQ_RETURN {
            return Err(Stop::Unsupported {
                op: format!(
                    "interrupt handler returned to {}, not by rte",
                    img.describe(back)
                ),
                at: img.describe(taken.at),
            });
        }
        r.set(PC, 4, taken.pc);
        self.now = r.cycles();
        Ok(())
    }

    /// What [`Bus::poll`] does before taking an interrupt: fit the CPU's work since the last call
    /// into the bus slots the chips leave it, ticking `r` by the CPU's waits (`dma`), time the
    /// block's accesses at `r`'s count, run the chips up to it, tick the keyboard, and end the run
    /// on an access outside memory made since.
    pub fn poll_time(&mut self, r: &mut Regs) -> Result<(), Stop> {
        self.contend(r);
        self.advance(r.cycles());
        self.now = r.cycles();
        // The keyboard's next byte, if its time has come since the last block.
        self.cia_a.tick(self.now);
        self.arm();
        match &self.fault {
            Some(f) => Err(f.clone()),
            None => Ok(()),
        }
    }

    /// Set [`Machine::irq_at`](Machine) from the chips as they stand at `now`.
    fn arm(&mut self) {
        self.settle();
        let mut at = self.frame_end();
        if let Some(c) = self.copper.as_ref().and_then(|c| c.next_at()) {
            at = at.min(c.max(0) as u64);
        }
        if let Some(&r) = self.raised.iter().map(|(_, t)| t).min() {
            at = at.min(r);
        }
        if let Some(k) = self.cia_a.next_at() {
            at = at.min(k);
        }
        // A blit's end is raised as the bus runs it (`contend`), so it has no time ahead.
        let blit_irq = irq::INTEN | blitter::INT_BLIT;
        if self.watch.dirty || (self.intena & blit_irq == blit_irq && self.blit_busy()) {
            at = 0;
        }
        self.irq_at = at;
    }

    /// Whether no interrupt can be taken at `r`'s count: before [`Machine::arm`]'s bound and none
    /// pending above SR's mask (an INTENA or INTREQ write, or SR lowered by `rte`, `move to SR`).
    #[inline]
    pub fn quiet(&self, r: &Regs) -> bool {
        r.cycles() < self.irq_at && self.pending(r.get(pcode_rt::SR, 2) as u16).is_none()
    }

    /// The interrupt level to take now under `r`'s SR, with every request due by now in INTREQ.
    pub fn due(&mut self, r: &Regs) -> Option<u8> {
        self.settle();
        self.pending(ops::sr_get(r) as u16)
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
        let picture = match self.draw(self.cfg.frame_lines) {
            Ok(d) => {
                self.clx |= d.clx;
                d.picture
            }
            Err(op) => {
                self.unsupported(op);
                Picture::blank(0, 0, 0, 0)
            }
        };
        self.clx_from = 0;
        let writes = std::mem::take(&mut self.log);
        let cpu_writes = std::mem::take(&mut self.cpu_log);
        let cpu_leftover = std::mem::take(&mut self.cpu_leftover);
        let watched = std::mem::take(&mut self.watched);
        self.audio.end_frame();
        self.start_frame(end as i64, end as i64);
        Frame {
            picture,
            writes,
            cpu_writes,
            cpu_leftover,
            watched,
        }
    }

    /// The current frame drawn from its start up to line `upto` (exclusive), its collisions
    /// gathered from `clx_from`.
    fn draw(&self, upto: u16) -> Result<video::Drawn, &'static str> {
        let c = self.copper.as_ref().expect("started");
        let history = self.journal.history_with(&self.blit_unwritten());
        video::draw(&video::FrameIn {
            chip: &self.chip,
            history: &history,
            frame_start: c.frame_start,
            frame_lines: self.cfg.frame_lines,
            start: &self.frame_custom,
            dmacon: self.frame_dmacon,
            writes: &self.log,
            clx_lines: self.clx_from..upto,
        })
    }

    /// A CLXDAT read: the lines the beam has finished since the last read are drawn and their
    /// collisions gathered; the bits are returned with bit 15 set and cleared (vAmiga
    /// `peekCLXDAT`). The line the beam is on is gathered by the next read.
    fn read_clxdat(&mut self) -> u16 {
        let saved = self.now;
        self.advance(saved);
        self.now = saved;
        let start = self.copper.as_ref().expect("started").frame_start;
        let line = ((saved as i64 - start).max(0) / amiga_clock::LINE_CYCLES as i64)
            .min(self.cfg.frame_lines as i64) as u16;
        if line > self.clx_from {
            match self.draw(line) {
                Ok(d) => self.clx |= d.clx,
                Err(op) => self.unsupported(op),
            }
            self.clx_from = line;
        }
        collide::CLXDAT_NONE | std::mem::take(&mut self.clx)
    }

    /// Start the frame the beam is in now, if none is: a machine's first access to time. Its
    /// lines above the beam are not the machine's (a snapshot taken part way down a frame holds
    /// no record of them), so their collisions are not gathered.
    fn ensure_frame(&mut self) {
        if self.copper.is_none() {
            self.audio.start(self.dmacon);
            let into = self.phase.wrapping_add(self.now) % beam::frame_cycles(self.cfg.frame_lines);
            self.start_frame(self.now as i64 - into as i64, self.now as i64);
            self.clx_from = (into / amiga_clock::LINE_CYCLES as u64) as u16;
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
            // A blit's end is the bus's (`contend`), which raises its interrupt for its cycle.
            match cop {
                Some(_) => self.copper_deliver(),
                None => break,
            }
        }
    }

    /// The CPU's work since the last call, fitted into the bus slots the chips leave it (`dma`):
    /// `r` is ticked by what the CPU waited. A first call, or a count gone back (a new `Regs`),
    /// only marks where the work starts.
    fn contend(&mut self, r: &mut Regs) {
        let (c, o) = (r.cycles(), r.off_bus_cycles());
        match self.cpu_mark {
            Some((c0, o0)) if c >= c0 && o >= o0 => {
                // Slots a host's jump of the count left unrun are the chips' alone.
                self.dma_idle(c0);
                let (done, done_c, done_o, part) = self.synced;
                let begin = self.dma_at.max(c0 + done_c);
                let mut work = [(0u32, 0u32, 0u32); pcode_rt::TRAIL];
                let n = match Self::trail_since(r, c0 + done_c, o0 + done_o, done, &mut work) {
                    Some(n) => n,
                    // Something ticked past the trail: the rest of the block is one lump.
                    None => {
                        let lump = ((c - c0 - done_c) as u32, (o - o0).saturating_sub(done_o) as u32);
                        work[0] = (lump.0, lump.1, 0);
                        1
                    }
                };
                let skip = part;
                let end = self.with_slots(|s, beam| s.cpu(begin, &work[..n], skip, beam));
                self.dma_at = end;
                let late = end.saturating_sub(c);
                if late > 0 {
                    r.tick(late as u32);
                }
            }
            _ => self.dma_idle(c),
        }
        r.clear_trail();
        self.synced = (0, 0, 0, (0, false));
        self.cpu_mark = Some((r.cycles(), r.off_bus_cycles()));
        self.blit_settle();
    }

    /// The block's trail from instruction `from` on, as (cycles, off the bus, of those first), into
    /// `work`; its length, if the trail holds the whole block since the mark one instruction at a
    /// time (it does unless something ticked past it). `(c0, o0)` is the mark moved past the
    /// instructions before `from`, so only the rest is summed: `sync` runs before every access.
    fn trail_since(
        r: &Regs,
        c0: u64,
        o0: u64,
        from: usize,
        work: &mut [(u32, u32, u32); pcode_rt::TRAIL],
    ) -> Option<usize> {
        let trail = r.trail();
        if trail.len() >= pcode_rt::TRAIL || from > trail.len() {
            return None;
        }
        let (mut sc, mut so) = (0u64, 0u64);
        for (k, (&(c, o), &l)) in trail[from..].iter().zip(&r.trail_lead()[from..]).enumerate() {
            work[k] = (c, o, l);
            sc += c as u64;
            so += o as u64;
        }
        ((sc, so) == (r.cycles().wrapping_sub(c0), r.off_bus_cycles().wrapping_sub(o0)))
            .then_some(trail.len() - from)
    }

    /// An instruction that reads or writes memory is about to: the block's instructions before it
    /// are fitted onto the bus, and its accesses are timed at its start, the CPU's waits so far
    /// included (`Bus::sync`, translator T49). The waits reach `Regs` at the next `poll`.
    pub fn sync(&mut self, r: &Regs) {
        let Some((c0, o0)) = self.cpu_mark else {
            return;
        };
        if r.cycles() < c0 || r.off_bus_cycles() < o0 {
            return;
        }
        let (done, done_c, done_o, part) = self.synced;
        let mut work = [(0u32, 0u32, 0u32); pcode_rt::TRAIL];
        let Some(len) = Self::trail_since(r, c0 + done_c, o0 + done_o, done, &mut work) else {
            return;
        };
        // The last is the instruction about to access memory: its own time is still to come.
        let Some(n) = len.checked_sub(1) else {
            return;
        };
        let (cur, cur_off, cur_lead) = work[n];
        let (bc, bo) = work[..n]
            .iter()
            .fold((0u64, 0u64), |(a, b), &(x, y, _)| (a + x as u64, b + y as u64));
        let begin = self.dma_at.max(c0 + done_c);
        // Moira makes an instruction's data accesses before its last bus cycle, the next
        // instruction's prefetch: its leading idle cycles and its accesses up to there are fitted
        // too, and the access is timed at the start of the last of them.
        let own = (cur.saturating_sub(cur_off) / 4) as u64;
        let ahead = own.saturating_sub(1);
        // What of the current instruction a sync before this one fitted already.
        let (skip, led) = if n > 0 { (0, false) } else { part };
        let lead = cur_lead.min(cur_off) as u64;
        let (end, at) = self.with_slots(|s, beam| {
            let t = s.cpu(begin, &work[..n], if n > 0 { part } else { (0, false) }, beam);
            let t = if led { t } else { s.cpu_idle(t, t + lead, beam) };
            let end = s.accesses(t, ahead.saturating_sub(skip), beam);
            (end, if ahead > skip { end - 4 } else { t })
        });
        self.dma_at = end;
        self.synced = (done + n, done_c + bc, done_o + bo, (skip.max(ahead), true));
        self.blit_settle();
        if at > self.now {
            self.advance(at);
            self.now = at;
        }
    }

    /// Run the bus's slots up to cycle count `t` with no CPU access.
    fn dma_idle(&mut self, t: u64) {
        if t > self.dma_at {
            let from = self.dma_at;
            self.with_slots(|s, beam| s.idle(from, t, beam));
            self.dma_at = t;
            self.blit_settle();
        }
    }

    /// The bus as the registers hold it now, handed to `f` with the beam at a cycle count.
    fn with_slots<R>(&mut self, f: impl FnOnce(&mut dma::Slots, &dyn Fn(u64) -> (u16, u16)) -> R) -> R {
        let (phase, lines) = (self.phase, self.cfg.frame_lines);
        let beam = move |t: u64| beam::position(phase.wrapping_add(t), lines);
        let fetch = self.fetch();
        let mut slots = dma::Slots {
            fetch,
            dmacon: self.dmacon,
            blit: self.blitter.run.as_mut(),
            frame_lines: lines,
            copper: self.copper.as_ref().map(|c| (c, c.frame_start)),
            sprites: &mut self.sprites,
            audio: &mut self.audio,
            chip: &self.chip,
            custom: &self.custom,
        };
        f(&mut slots, &beam)
    }

    /// A blit whose BLTDONE the bus has run: its interrupt a slot later, and the blitter free.
    fn blit_settle(&mut self) {
        self.blit_journal(None);
        if let Some(done) = self.blitter.run.as_ref().and_then(|b| b.done_at) {
            self.blitter.run = None;
            self.raise(blitter::INT_BLIT.trailing_zeros() as u16, done + 2);
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
                    // A word the copper has yet to read: its re-run may write sooner than
                    // `arm`'s bound, an INTREQ write among them.
                    if self.watch.dirty {
                        self.irq_at = 0;
                    }
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
            // On the reference a DMA slot before the read may have left that word; the machine
            // has no DMA slots, so the write is marked (T47).
            let w = self.bus;
            self.leftover = true;
            self.custom_write(reg, w);
            self.leftover = false;
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
            collide::CLXDAT if !routed => self.read_clxdat(),
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
        if cpu {
            self.irq_at = 0;
            if self.leftover {
                self.cpu_leftover.push(self.cpu_log.len());
            }
            self.cpu_log.push((self.now, reg, v));
            if self.watch_write == Some((reg, v)) {
                let mut m = self.chip.clone();
                m.extend_from_slice(&self.slow);
                self.watched.push(m);
            }
        }
        if custom::read_only(reg) {
            return;
        }
        self.paula.note(reg, v, self.now, self.beam(self.now));
        if paula::is_paula(reg) {
            self.audio.write(reg, v, self.beam(self.now));
        }
        if self.leftover && cpu && paula::is_paula(reg) {
            self.paula.mark_last();
        }
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
            custom::ADKCON => {
                let r = custom::ADKCONR as usize >> 1;
                self.custom[r] = irq::set_clear(self.custom[r], v) & 0x7FFF;
            }
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
        self.irq_at = 0;
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
    /// The keys `i` holds were already down, and sent, when this machine's state was taken (a
    /// snapshot between frames): the next [`Machine::set_input`] sends only what changed from them.
    /// Its other inputs are not edges and are set by `set_input` alone.
    pub fn keys_held(&mut self, i: &Input) {
        self.cia_a.set_held(i.key_set());
    }

    pub fn set_input(&mut self, i: &Input) {
        self.irq_at = 0;
        self.custom[custom::JOY1DAT as usize >> 1] = i.joy1;
        self.cia_a.fire = i.fire;
        self.cia_a.mouse_left = i.mouse_left;
        self.cia_a.set_keys(i.key_set(), self.now);
    }

    /// The current frame's custom-register writes so far, the CPU's and the copper's, in order:
    /// what [`Machine::end_frame`] will return in `Frame::writes`. A host running code that is not
    /// a frame (the routine judge, translator T27) reads them here without ending one.
    pub fn custom_writes(&self) -> &[(u64, u16, u16)] {
        &self.log
    }

    /// Keep chip RAM then slow RAM as it is at every CPU write of `w` (register offset from
    /// `$DFF000`, word), as the oracle's `Amiga::watch_write` does, so a host can compare state
    /// where the program marks the end of its work (the game's pass end, translator T30), not
    /// where a frame happens to end. `None` keeps nothing.
    pub fn watch_write(&mut self, w: Option<(u16, u16)>) {
        self.watch_write = w;
    }

    /// The copies [`Machine::watch_write`] kept since the frame started, emptying the list.
    pub fn take_watched(&mut self) -> Vec<Vec<u8>> {
        std::mem::take(&mut self.watched)
    }

    /// The CPU's of [`Machine::custom_writes`] alone, in order: what `end_frame` will return in
    /// `Frame::cpu_writes`.
    pub fn cpu_writes(&self) -> &[(u64, u16, u16)] {
        &self.cpu_log
    }

    /// Indices into [`Machine::cpu_writes`] of the writes made with a leftover bus word: what
    /// `end_frame` will return in `Frame::cpu_leftover`.
    pub fn cpu_leftover(&self) -> &[usize] {
        &self.cpu_leftover
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

    /// [`Machine::take_paula_writes`], with the indices of those made with a leftover bus word.
    pub fn take_paula_writes_marked(&mut self) -> (Vec<(u64, amiga_paula::PaulaWrite)>, Vec<usize>) {
        self.paula.take_marked()
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

/// An interrupt taken by [`Machine::enter_interrupt`]: the handler to call, where it is, and the
/// interrupted code's PC register, which [`Machine::leave_interrupt`] puts back.
#[derive(Clone, Copy, Debug)]
pub struct Taken {
    pub handler: Routine,
    pub at: u32,
    pc: u64,
}

/// INTREQ's PORTS bit: level 2, CIA-A's interrupt line.
pub const INTREQ_PORTS: u16 = 0x0008;

impl Bus for Machine {
    /// Times the block's accesses, ends the run on an access outside memory made since, and takes
    /// every interrupt pending above SR's mask, highest first: a handler's own blocks poll too, so
    /// a higher level nests inside it, and one left pending after its `rte` is taken here at once,
    /// as a 68000 takes it before the next instruction.
    fn poll(&mut self, r: &mut Regs, img: &Image) -> Result<(), Stop> {
        self.poll_time(r)?;
        while let Some(level) = self.due(r) {
            self.take(level, r, img)?;
            if let Some(f) = &self.fault {
                return Err(f.clone());
            }
        }
        Ok(())
    }

    /// Polls only once an interrupt may be due ([`Machine::quiet`]).
    #[inline]
    fn insn(&mut self, r: &mut Regs, img: &Image) -> Result<(), Stop> {
        if self.quiet(r) {
            return Ok(());
        }
        self.poll(r, img)
    }

    fn sync(&mut self, r: &Regs) {
        Machine::sync(self, r)
    }

    /// A library vector: the OS's answer (`os`).
    fn call_outside(&mut self, a: u32, r: &mut Regs, img: &Image) -> Result<(), Stop> {
        self.os_call(a, r, img)
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
            Region::Rom(off) => match self.os.rom_byte(off) {
                Some(v) => {
                    self.bus = v as u16;
                    v
                }
                None => {
                    self.outside(a);
                    0
                }
            },
            Region::Outside => {
                self.outside(a);
                0
            }
        }
    }

    fn rw(&mut self, a: u32) -> u16 {
        match self.region(a) {
            Region::Chip(_) | Region::Slow(_) | Region::Rom(_) => {
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
            // Kickstart's ROM: a write is the program's bug, and would change nothing there.
            Region::Rom(_) | Region::Outside => self.outside(a),
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
            Region::Rom(_) | Region::Outside => self.outside(a),
        }
    }

    fn wl(&mut self, a: u32, v: u32) {
        self.ww(a, (v >> 16) as u16);
        self.ww(a.wrapping_add(2), v as u16);
    }

    fn ww_leftover(&mut self, a: u32, v: u16) {
        self.leftover = true;
        self.ww(a, v);
        self.leftover = false;
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_chips;
#[cfg(test)]
mod tests_dma;
