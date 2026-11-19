//! The translated program as an engine (translator T28, DESIGN §2.14, §3.3): the generated tree
//! built in at compile time ([`tree`]), run on `pcode-machine` from power-on or from a snapshot of
//! the original, calling back at every frame boundary. See README.md.
//!
//! The engine drives: [`TranslatedGame::run`] runs the program and calls `each` at each frame's end
//! with the frame, taking the next frame's input back. The foreground (`main_loop`, the title's
//! loop) is nested Rust calls a pure crate cannot suspend, so a shell needing a pull-style step runs
//! `run` on a thread of its own and hands frames and inputs across a channel (fd-conform's
//! `translated_engine`, later fd-app).
//!
//! Starting from a snapshot: the original's PC and the return addresses on its stack are any
//! instruction, mid-function and mid-block. The run enters the function holding the PC there
//! (`tree::resume`, every instruction an entry); when it returns, there is no Rust caller to go back
//! to, so the run resumes again at the address the `rts` or `rte` popped, until the program stops.
//! A function returning to its real Rust caller never reaches that loop: only the frames the
//! snapshot was taken inside do.

use std::cell::Cell;
use std::sync::Arc;

use pcode_machine::mem::SLOW_BASE;
use pcode_machine::{Config, Disks, Input, Machine, Picture};
use pcode_rt::{Bus, Image, Mounted, PC, Regs, SR, Stop, ops};

pub mod seam;
pub use seam::{Level1, Seam};

/// The tree from `build/translator/tree/` or `FD_TRANSLATED`, else a stub with no entries
/// (`buildscripts/fd_translated.rs`).
#[allow(
    unused_imports,
    dead_code,
    unused_variables,
    unused_mut,
    unreachable_code,
    clippy::all
)]
pub mod tree {
    include!(concat!(env!("OUT_DIR"), "/tree.rs"));
}

/// A translated entry, as the tree's `entry` gives it.
pub type Routine = fn(&mut Regs, &mut dyn Bus, &Image) -> Result<(), Stop>;
/// A translated function entered at an instruction, as the tree's `resume` gives it.
pub type Resume = fn((u32, u32), &mut Regs, &mut dyn Bus, &Image) -> Result<(), Stop>;

/// The stack [`TranslatedGame::run`] needs: every translated function is a Rust frame, up to
/// `pcode_rt::DEPTH_CAP` of them, measured up to 49 KB each unoptimised (fd-conform's judge,
/// translator T14). A shell runs `run` on a thread with this much stack.
pub const RUN_STACK: usize = 64 << 20;

/// Where the stack starts at power-on: slow RAM below hunk 0, which nothing the program loads
/// reaches (translator T26, `os_run`).
pub const POWER_ON_STACK: u32 = 0xC0_7000;

/// The supervisor stack at power-on: Kickstart 1.3's exec keeps it at the top of slow RAM (the
/// oracle's SSP `$C7FFFA` inside an interrupt at the entry's step, translator T32). The program
/// runs in user mode on [`POWER_ON_STACK`], and the first interrupt it takes (play's first level 3,
/// since the title takes none) switches to this one.
pub const POWER_ON_SUPERVISOR_STACK: u32 = 0xC8_0000;

/// CIA-A's interrupt mask the program leaves set in play: the keyboard's serial bit (translator
/// T01, T25). The original's snapshot holds no CIA state, so a snapshot start sets it.
const CIA_A_MASK: u8 = 0x08;

/// INTREQ's PORTS bit number, which CIA-A's interrupt line sets.
const PORTS_BIT: u16 = 3;

/// A translated program: its instructions' bytes, each hunk's length, and its functions by entry
/// and by instruction. [`Program::BUILT_IN`] is the tree; a test hands a made-up one.
#[derive(Clone, Copy)]
pub struct Program {
    pub code: &'static [(u32, u32, u32)],
    pub hunk_sizes: &'static [u32],
    pub entry: fn(&str) -> Option<Routine>,
    pub resume: fn((u32, u32)) -> Option<Resume>,
}

impl Program {
    /// The tree built in.
    pub const BUILT_IN: Program = Program {
        code: tree::CODE,
        hunk_sizes: tree::HUNK_SIZES,
        entry: tree::entry,
        resume: tree::resume,
    };
    /// No translation at all, as a build without a tree has.
    pub const STUB: Program = Program {
        code: &[],
        hunk_sizes: &[],
        entry: |_| None,
        resume: |_| None,
    };
}

/// The 68000's registers at a snapshot, as the oracle gives them at a frame boundary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Regs68k {
    pub d: [u32; 8],
    /// `a[7]` is the stack pointer SR selects.
    pub a: [u32; 8],
    /// The next instruction to run.
    pub pc: u32,
    pub sr: u16,
    pub usp: u32,
    pub ssp: u32,
}

/// Where a run starts.
pub enum Start<'a> {
    /// The program file loaded at the hunk bases as Kickstart would and entered at `h0+$0`, the
    /// stack at [`POWER_ON_STACK`]. The program waits for fire at the disk 2 prompt (`h0+$7b70`):
    /// the first frames' input should hold it.
    PowerOn { program: &'a [u8], disks: Disks },
    /// The original's state: chip RAM then slow RAM, the registers, the custom registers by
    /// `reg / 2` (VPOSR and VHPOSR place the beam, INTENAR, INTREQR and DMACONR are those
    /// registers' state, as `Machine::from_snapshot` reads them).
    Snapshot {
        mem: &'a [u8],
        regs: Regs68k,
        custom: &'a [u16; 256],
        disks: Disks,
    },
}

/// What `each` answers at a frame boundary.
pub enum Control {
    /// Run the next frame on this input.
    Next(Input),
    /// End the run here. The game cannot be run again: the frames it was inside are gone.
    Stop,
}

/// One frame as the translated program made it.
pub struct FrameOut<'a> {
    /// The frame's picture; on a stop, the last whole frame's.
    pub picture: Picture,
    /// Every custom-register write of the frame, the CPU's and the copper's, in order: (cycle,
    /// register offset from `$DFF000`, word).
    pub writes: Vec<(u64, u16, u16)>,
    /// The CPU's of `writes` alone, in order, tagged as each was made: the oracle keeps the CPU's
    /// only (`Amiga::cpu_writes`), so the frame bar compares these (translator T45).
    pub cpu_writes: Vec<(u64, u16, u16)>,
    /// Indices into `cpu_writes` of the writes a read of a write-only register made, whose word
    /// is the bus's leftover (`pcode_rt::Bus::ww_leftover`): the frame bar compares them by
    /// register only (translator T47).
    pub cpu_leftover: Vec<usize>,
    /// Paula's writes of the frame, in order: (cycle, register, word).
    pub paula: Vec<(u64, u16, u16)>,
    /// Indices into `paula` of the writes made with a leftover bus word, as `cpu_leftover`.
    pub paula_leftover: Vec<usize>,
    /// Set on the last frame of a run the program stopped: how it stopped.
    pub stop: Option<Stop>,
    /// Chip RAM then slow RAM at each CPU write of the frame that [`TranslatedGame::watch_write`]
    /// names, in order: where the program marks its own work's end (translator T30).
    pub watched: Vec<Vec<u8>>,
    /// Chip RAM and slow RAM at the frame's end, for a shell to read the state from.
    pub chip: &'a [u8],
    pub slow: &'a [u8],
    /// The watched routines ([`TranslatedGame::watch_returns`]) that returned in the frame, in
    /// order, each as (hunk, offset) of its entry.
    pub returns: Vec<(u32, u32)>,
    /// The watched routines still running at the frame's end, outermost first.
    pub inside: Vec<(u32, u32)>,
    /// How many [`TranslatedGame::write_at`] writes the program has not reached by the frame's end.
    pub writes_at_pending: usize,
}

/// A write waiting for the program to enter a routine ([`TranslatedGame::write_at`]).
#[derive(Clone, Debug, PartialEq, Eq)]
struct WriteAt {
    /// The routine's entry, (hunk, offset).
    at: (u32, u32),
    addr: u32,
    bytes: Vec<u8>,
}

/// The translated program on its machine.
pub struct TranslatedGame {
    m: Machine,
    r: Regs,
    prog: Program,
    bases: Vec<u32>,
    sizes: Vec<u32>,
    /// Where the next run enters; `None` once a run has ended.
    at: Option<u32>,
    /// Why the game cannot start (a program file the machine could not load), or how it stopped.
    stopped: Option<Stop>,
    input: Input,
    picture: Picture,
    /// The hand-written routines run in place of translated ones (DESIGN §2.9).
    seam: Seam,
    /// The routines whose returns each frame reports ([`FrameOut::returns`]).
    watched: Vec<(u32, u32)>,
    /// Writes waiting for their routine's entry ([`TranslatedGame::write_at`]).
    writes_at: Vec<WriteAt>,
    /// Level 1's hand engine for the frame mount ([`TranslatedGame::mount_level_1`]), handed to it
    /// through `Bus::mount_data`.
    level_1: Option<Arc<Level1>>,
}

impl TranslatedGame {
    /// The built-in tree on `start`, the program at the original's hunk bases, translated alone:
    /// nothing mounted until [`TranslatedGame::mount_level_1`].
    pub fn new(start: Start) -> Self {
        Self::with_program(Program::BUILT_IN, &fd_core::HUNK_BASES, start)
    }

    /// Level 1's hand engine mounted (DESIGN §2.9, T36): every routine `fd_core::PORTED` marks
    /// mounted run through the seam from the next run on, `game_frame` among them, which the hand
    /// engine plays from `data` while the program is in world 1. A mounted routine with no adapter
    /// stops the run at once, naming it.
    pub fn mount_level_1(&mut self, data: Arc<fd_core::GameData>) {
        match Seam::from_ported(fd_core::PORTED) {
            Ok(seam) => {
                for k in seam.addresses() {
                    let f = seam.get(k).expect("listed");
                    self.seam.mount((k.0 as u8, k.1), f);
                }
            }
            Err(e) => {
                self.stopped.get_or_insert(Stop::Unsupported {
                    op: e,
                    at: "start".into(),
                });
            }
        }
        self.level_1 = Some(Arc::new(Level1::new(data)));
    }

    /// Whether a tree is built in (false on the stub).
    pub fn generated() -> bool {
        !tree::CODE.is_empty()
    }

    /// `prog` with its hunks at `bases`, on `start`.
    pub fn with_program(prog: Program, bases: &[u32], start: Start) -> Self {
        let cfg = Config::a500(bases.to_vec());
        let mut r = Regs::new();
        let mut stopped = None;
        let (mut m, at, sizes) = match start {
            Start::PowerOn { program, disks } => {
                let m = match Machine::new(cfg.clone(), program) {
                    Ok(mut m) => {
                        m.insert(disks);
                        m
                    }
                    Err(e) => {
                        stopped = Some(Stop::Unsupported {
                            op: format!("loading the program: {e}"),
                            at: "power-on".into(),
                        });
                        let mem = vec![0; (cfg.chip + cfg.slow) as usize];
                        Machine::from_snapshot(cfg, &mem, &[0; 256])
                    }
                };
                r.set_a(7, POWER_ON_STACK);
                r.set(ops::ISP, 4, POWER_ON_SUPERVISOR_STACK as u64);
                let sizes = if m.hunk_sizes().is_empty() {
                    prog.hunk_sizes.to_vec()
                } else {
                    m.hunk_sizes().to_vec()
                };
                (m, bases[0], sizes)
            }
            Start::Snapshot {
                mem,
                regs,
                custom,
                disks,
            } => {
                let mut m = Machine::from_snapshot(cfg, mem, custom);
                m.insert(disks);
                m.cia_a_mut().set_icr_mask(CIA_A_MASK);
                for n in 0..8 {
                    r.set_d(n, regs.d[n]);
                    r.set_a(n, regs.a[n]);
                }
                // SR as written, not through `sr_set`, which would swap A7 for the other stack.
                r.set(SR, 2, (regs.sr & 0xa71f) as u64);
                ops::ccr_set(&mut r, regs.sr as u64);
                let other = if regs.sr & 0x2000 != 0 {
                    (ops::USP, regs.usp)
                } else {
                    (ops::ISP, regs.ssp)
                };
                r.set(other.0, 4, other.1 as u64);
                (m, regs.pc & 0xFF_FFFF, prog.hunk_sizes.to_vec())
            }
        };
        m.set_entry(prog.entry);
        TranslatedGame {
            m,
            r,
            prog,
            bases: bases.to_vec(),
            sizes,
            at: Some(at),
            stopped,
            input: Input::default(),
            picture: Picture::blank(0, 0, 0, 0),
            seam: Seam::new(),
            watched: Vec::new(),
            writes_at: Vec::new(),
            level_1: None,
        }
    }

    /// Run `f` in place of the translated routine at `addr` (hunk, offset) from the next run on.
    pub fn mount(&mut self, addr: (u8, u32), f: Mounted) {
        self.seam.mount(addr, f);
    }

    /// Report each return from the routines entered at `addrs` (hunk, offset) in the frame it
    /// happened in ([`FrameOut::returns`]): a case lines frames up at a load's return
    /// (translator T32). Only a call through the routine's entry counts, and a routine mounted
    /// through the seam is not seen.
    pub fn watch_returns(&mut self, addrs: &[(u8, u32)]) {
        self.watched = addrs.iter().map(|&(h, o)| (h as u32, o)).collect();
    }

    /// Write `bytes` into RAM at `addr` the moment the program enters the routine at `at` (hunk,
    /// offset), before its first instruction reads anything; once, then dropped. The oracle's
    /// `Amiga::poke_at` at that routine's first instruction, for a state the program writes and
    /// reads within one frame (a case's `set` with `when`, translator T33). Only an entry through
    /// the routine's own function counts, as with [`TranslatedGame::watch_returns`], mounted or
    /// not; bytes outside chip and slow RAM are dropped, as the oracle's poke drops them.
    pub fn write_at(&mut self, at: (u8, u32), addr: u32, bytes: &[u8]) {
        self.writes_at.push(WriteAt {
            at: (at.0 as u32, at.1),
            addr,
            bytes: bytes.to_vec(),
        });
    }

    /// Start the run with the beam at `line`, so the first frame is only the rest of it: a power-on
    /// case starts the program on the line the original's entry ran on (translator T32). Before
    /// the first run only; a snapshot's beam is its VPOSR and VHPOSR.
    pub fn start_at_line(&mut self, line: u16) {
        self.m.sync_beam(self.r.cycles(), line, 0);
    }

    /// What is mounted.
    pub fn seam(&self) -> &Seam {
        &self.seam
    }

    /// Level 1's hand engine, once [`TranslatedGame::mount_level_1`] mounted it.
    pub fn level_1(&self) -> Option<&Level1> {
        self.level_1.as_deref()
    }

    /// Keep RAM at every CPU write of `w` (register offset from `$DFF000`, word) in
    /// `FrameOut::watched` ([`Machine::watch_write`]).
    pub fn watch_write(&mut self, w: Option<(u16, u16)>) {
        self.m.watch_write(w);
    }

    /// The input of the frame that ended where a [`Start::Snapshot`] was taken: its keys were
    /// already sent by the original's keyboard, so the first frame sends only what changed.
    pub fn held_at_start(&mut self, input: &Input) {
        self.m.keys_held(input);
    }

    /// The first frame's input: the frames after it take theirs from `run`'s `each`.
    pub fn set_input(&mut self, input: Input) {
        self.input = input;
    }

    pub fn machine(&self) -> &Machine {
        &self.m
    }

    pub fn regs(&self) -> &Regs {
        &self.r
    }

    /// Runs the program; at every frame boundary (inside `Bus::poll`) calls `each` with the frame
    /// and takes the next frame's input from it. Returns `None` when `each` says stop, the stop
    /// when the program stops (after handing `each` a last frame with `stop` set). Needs
    /// [`RUN_STACK`] bytes of stack for the real program.
    pub fn run(&mut self, each: &mut dyn FnMut(&FrameOut) -> Control) -> Option<Stop> {
        let Some(mut at) = self.at.take() else {
            return self.stopped.clone();
        };
        self.m.set_input(&self.input);
        let img = Image {
            bases: &self.bases,
            sizes: &self.sizes,
            code: self.prog.code,
        };
        let mut host = Host {
            m: &mut self.m,
            seam: &self.seam,
            level_1: self.level_1.as_deref(),
            each,
            picture: &mut self.picture,
            host_stop: false,
            watched: &self.watched,
            writes_at: &mut self.writes_at,
            write_due: Cell::new(None),
            entered: Cell::new(None),
            inside: Vec::new(),
            returns: Vec::new(),
        };
        let stop = match self.stopped.clone() {
            Some(s) => s,
            None => loop {
                let res = match img.hunk_of(at).and_then(self.prog.resume) {
                    Some(f) => f(
                        img.hunk_of(at).expect("in a hunk"),
                        &mut self.r,
                        &mut host,
                        &img,
                    ),
                    None => Err(Stop::Untranslated(img.describe(at))),
                };
                match res {
                    // Returned with no Rust caller: a frame the start was inside. Go on where its
                    // `rts` or `rte` went.
                    Ok(()) => at = self.r.get(PC, 4) as u32 & 0xFF_FFFF,
                    Err(s) => break s,
                }
            },
        };
        if host.host_stop {
            return None;
        }
        // The program has stopped: what it had returned from since the last block is over too.
        while let Some(&(k, depth)) = host.inside.last()
            && self.r.depth < depth
        {
            host.inside.pop();
            host.returns.push(k);
        }
        let (paula, paula_leftover) = host.paula();
        let out = FrameOut {
            picture: host.picture.clone(),
            writes: host.m.custom_writes().to_vec(),
            cpu_writes: host.m.cpu_writes().to_vec(),
            cpu_leftover: host.m.cpu_leftover().to_vec(),
            paula,
            paula_leftover,
            stop: Some(stop.clone()),
            watched: host.m.take_watched(),
            chip: host.m.chip(),
            slow: host.m.slow(),
            returns: std::mem::take(&mut host.returns),
            inside: host.inside.iter().map(|&(k, _)| k).collect(),
            writes_at_pending: host.writes_at.len(),
        };
        (host.each)(&out);
        self.stopped = Some(stop.clone());
        Some(stop)
    }
}

/// The machine as the generated code sees it during a run: the machine's bus, with the frame
/// boundary and the interrupts taken here, so a handler's blocks reach this `poll` too.
struct Host<'a, 'b> {
    m: &'a mut Machine,
    seam: &'a Seam,
    /// What the frame mount reads beside memory.
    level_1: Option<&'a Level1>,
    each: &'b mut dyn FnMut(&FrameOut) -> Control,
    picture: &'a mut Picture,
    /// `each` said stop: the stop unwinding the run is the host's, not the program's.
    host_stop: bool,
    /// Routines whose returns are reported, (hunk, offset).
    watched: &'a [(u32, u32)],
    /// Writes waiting for their routine's entry.
    writes_at: &'a mut Vec<WriteAt>,
    /// A routine with a waiting write was just entered: its writes are made at the first block's
    /// `poll`, before any instruction (`mounted` has `&self` only).
    write_due: Cell<Option<(u32, u32)>>,
    /// A function's entry asked `mounted` and its first block has not polled yet, with the
    /// routine if it is watched. `mounted` is the one hook every entry reaches, and it has no
    /// `Regs`; the first block's `poll` comes next, after `fn_entry` counted the function in.
    entered: Cell<Option<Option<(u32, u32)>>>,
    /// Watched routines running, each with `Regs::depth` inside it. One has returned once a block
    /// polls at a smaller depth, or a function is entered at its depth or above: a call is not a
    /// block's end, so `jsr a; jsr b` reaches `b`'s entry with no block of the caller between.
    inside: Vec<((u32, u32), u32)>,
    /// Watched returns since the last frame's end.
    returns: Vec<(u32, u32)>,
}

impl Host<'_, '_> {
    /// Every waiting write for the routine entered at `k`, in the order given, then dropped.
    fn make_writes_at(&mut self, k: (u32, u32)) {
        let (chip, slow) = (self.m.chip().len() as u32, self.m.slow().len() as u32);
        let mut k_writes = Vec::new();
        self.writes_at.retain(|w| {
            let hit = w.at == k;
            if hit {
                k_writes.push(w.clone());
            }
            !hit
        });
        for w in k_writes {
            for (n, &b) in w.bytes.iter().enumerate() {
                let a = w.addr.wrapping_add(n as u32) & 0xFF_FFFF;
                if a < chip {
                    self.m.chip_mut()[a as usize] = b;
                } else if (SLOW_BASE..SLOW_BASE + slow).contains(&a) {
                    self.m.slow_mut()[(a - SLOW_BASE) as usize] = b;
                }
            }
        }
    }

    fn paula(&mut self) -> (Vec<(u64, u16, u16)>, Vec<usize>) {
        let (writes, leftover) = self.m.take_paula_writes_marked();
        let writes = writes
            .into_iter()
            .map(|(at, w)| (at, w.reg, w.value))
            .collect();
        (writes, leftover)
    }

    /// The frame that ended at `frame_end()`: drawn, handed to `each`, the next one's input taken.
    fn end_frame(&mut self, r: &mut Regs) -> Result<(), Stop> {
        let frame = self.m.end_frame();
        let (paula, paula_leftover) = self.paula();
        // One step count per frame (pcode-rt: a host running frame by frame resets it).
        r.steps = 0;
        let out = FrameOut {
            picture: frame.picture,
            writes: frame.writes,
            cpu_writes: frame.cpu_writes,
            cpu_leftover: frame.cpu_leftover,
            paula,
            paula_leftover,
            stop: None,
            watched: frame.watched,
            chip: self.m.chip(),
            slow: self.m.slow(),
            returns: std::mem::take(&mut self.returns),
            inside: self.inside.iter().map(|&(k, _)| k).collect(),
            writes_at_pending: self.writes_at.len(),
        };
        let next = (self.each)(&out);
        *self.picture = out.picture;
        match next {
            Control::Next(input) => {
                self.m.set_input(&input);
                Ok(())
            }
            Control::Stop => {
                self.host_stop = true;
                Err(Stop::Unsupported {
                    op: "host: stopped".into(),
                    at: format!("cycle {}", r.cycles()),
                })
            }
        }
    }
}

impl Bus for Host<'_, '_> {
    fn poll(&mut self, r: &mut Regs, img: &Image) -> Result<(), Stop> {
        if let Some(k) = self.write_due.take() {
            self.make_writes_at(k);
        }
        let entered = self.entered.take();
        // Running at depth `d`, everything deeper has returned; a function just entered at `d`
        // follows the return of whatever ran at `d` too.
        let running = r.depth - entered.is_some() as u32;
        while let Some(&(k, depth)) = self.inside.last()
            && depth > running
        {
            self.inside.pop();
            self.returns.push(k);
        }
        if let Some(Some(k)) = entered {
            self.inside.push((k, r.depth));
        }
        while r.cycles() >= self.m.frame_end() {
            self.end_frame(r)?;
        }
        self.m.poll_time(r)?;
        if self.m.take_cia_interrupts() != 0 {
            self.m.raise(PORTS_BIT, r.cycles());
        }
        while let Some(level) = self.m.due(r) {
            let taken = self.m.enter_interrupt(level, r, img)?;
            (taken.handler)(r, self, img)?;
            self.m.leave_interrupt(&taken, r, img)?;
            if let Some(f) = self.m.fault() {
                return Err(f.clone());
            }
        }
        Ok(())
    }
    /// The whole `poll` once an interrupt or the frame's end may be due ([`Machine::quiet`]); the
    /// block bookkeeping it does first has nothing to do between instructions.
    #[inline]
    fn insn(&mut self, r: &mut Regs, img: &Image) -> Result<(), Stop> {
        if self.m.quiet(r) {
            return Ok(());
        }
        self.poll(r, img)
    }
    fn call_outside(&mut self, a: u32, r: &mut Regs, img: &Image) -> Result<(), Stop> {
        self.m.call_outside(a, r, img)
    }
    fn sync(&mut self, r: &Regs) {
        self.m.sync(r)
    }
    fn mounted(&self, k: (u32, u32)) -> Option<Mounted> {
        if self.writes_at.iter().any(|w| w.at == k) {
            self.write_due.set(Some(k));
        }
        if !self.watched.is_empty() {
            self.entered
                .set(Some(self.watched.contains(&k).then_some(k)));
        }
        self.seam.get(k)
    }
    fn mount_data(&self) -> Option<&dyn std::any::Any> {
        self.level_1.map(|l| l as &dyn std::any::Any)
    }
    fn rb(&mut self, a: u32) -> u8 {
        self.m.rb(a)
    }
    fn rw(&mut self, a: u32) -> u16 {
        self.m.rw(a)
    }
    fn rl(&mut self, a: u32) -> u32 {
        self.m.rl(a)
    }
    fn wb(&mut self, a: u32, v: u8) {
        self.m.wb(a, v)
    }
    fn ww(&mut self, a: u32, v: u16) {
        self.m.ww(a, v)
    }
    fn ww_leftover(&mut self, a: u32, v: u16) {
        self.m.ww_leftover(a, v)
    }
    fn wl(&mut self, a: u32, v: u32) {
        self.m.wl(a, v)
    }
}
