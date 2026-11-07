//! The routine rig (plans/forest-dumb-level-1 DESIGN §4, §4.1): one routine of the original against
//! its port, over many generated states.
//!
//! The original runs alone on Moira ([`Cpu68k`]) over the main program loaded at the oracle's hunk
//! bases ([`crate::HUNK_BASES`]), in hardware mode: its custom-chip writes are recorded in order, its
//! blits land in memory, and its raster waits end. A [`Generator`] lays a base once (screens, a map)
//! and makes each case's [`Setup`]: entry registers, bytes changed, what hardware bytes read. The port
//! is a closure that takes the `Setup` and returns its [`Effects`]; [`Rig::check`] requires the same
//! registers back (d0-d7, a0-a6; condition codes and a7 are not compared), the same bytes changed, and
//! the same custom-chip writes in the same order (`toolkit/docs/porting-routines.md` §3: memory alone
//! lets a wrong return value through). Bytes on the rig's stack are not compared: they are the return
//! addresses of inner calls.
//!
//! A state the original does not return from is counted, never compared and never a pass: a hang
//! (`max_instr` instructions ran) in [`RigReport::hangs`], a fault (an exception) in
//! [`RigReport::faults`]. A check that compared nothing fails.
//!
//! What the port may read is [`Rig::memory_for`]: the program and the generator's base, under which
//! the port lays the case's pokes itself.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::ops::Range;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

pub use amiga_oracle::Gen;
use amiga_oracle::{AmigaMemory, CallError, CallResult, Cpu68k, MEM_SIZE, Regs};

use crate::HUNK_BASES;

/// The main program on disk 1, as `fd_tools::game_file` names it.
pub const PROGRAM: &str = "disk1/FOREST_DUMB_FOREVER";
/// The stack the calls run on, far above anything the program uses (it ends near `$6a000` in chip
/// RAM and `$c7xxxx` in slow RAM); the routine oracle's memory is flat 16 MB.
pub const STACK: u32 = 0x7F_0000;
/// Bytes on the call's stack are not compared: return addresses of inner calls and saved registers.
pub const STACK_AREA: Range<u32> = 0x7E_0000..0x7F_0000;
/// `move.b $dff006.l,d0; rts`: reads the shim's VHPOSR counter, which runs on across calls, so the
/// port can start from where the original's raster reads start.
const PROBE: u32 = 0x70_0000;
const PROBE_CODE: [u8; 8] = [0x10, 0x39, 0x00, 0xdf, 0xf0, 0x06, 0x4e, 0x75];
/// A routine still running after this many instructions is counted as a hang. Raster waits end in
/// at most 256 VHPOSR reads; the spike's largest routine took a few thousand.
pub const MAX_INSTR: u64 = 200_000;
/// How many differences one failing case lists.
const SHOWN: usize = 24;

/// The address of `hN+$off` in the listing, where the oracle loads it.
pub const fn h(hunk: usize, off: u32) -> u32 {
    HUNK_BASES[hunk] + off
}

/// One generated state: the registers a routine is entered with, the bytes changed before the call,
/// and what hardware bytes read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Setup {
    pub d: [u32; 8],
    /// `a[7]` is ignored: every call runs on [`STACK`].
    pub a: [u32; 8],
    /// Bytes written before the call, in order (a later poke over an earlier one wins).
    pub pokes: Vec<(u32, Vec<u8>)>,
    /// What a read of a hardware byte (a custom register's or a CIA's) returns, by address; every
    /// other hardware byte reads 0 and VHPOSR steps.
    pub hw_reads: Vec<(u32, u8)>,
    /// Set by the rig before the port runs, never by a generator: the byte the routine's first
    /// VHPOSR (`$dff006`/`$dff007`) read returns; each later read returns one more.
    pub beam: u8,
}

impl Setup {
    /// Every data register and a0-a6 random (address registers even), as the spike's judge began
    /// each state: a routine that reads a register it was not meant to shows up.
    pub fn random(g: &mut Gen) -> Setup {
        let mut s = Setup::default();
        for i in 0..8 {
            s.d[i] = g.next() as u32;
        }
        for i in 0..7 {
            s.a[i] = g.next() as u32 & !1;
        }
        s
    }

    /// `bytes` at `a`. Addresses wrap to the 24-bit bus, as the 68000's do; a poke running past the
    /// end of memory is dropped.
    pub fn put(&mut self, a: u32, bytes: Vec<u8>) {
        let a = a & 0xFF_FFFF;
        if a as usize + bytes.len() <= MEM_SIZE {
            self.pokes.push((a, bytes));
        }
    }

    pub fn b(&mut self, a: u32, v: u8) {
        self.put(a, vec![v]);
    }

    pub fn w(&mut self, a: u32, v: u16) {
        self.put(a, v.to_be_bytes().to_vec());
    }

    pub fn l(&mut self, a: u32, v: u32) {
        self.put(a, v.to_be_bytes().to_vec());
    }

    /// The byte at `a` once the pokes are laid over `mem`: what the port reads.
    pub fn byte(&self, mem: &[u8], a: u32) -> u8 {
        let a = a & 0xFF_FFFF;
        self.pokes
            .iter()
            .rev()
            .find(|(at, b)| (*at..*at + b.len() as u32).contains(&a))
            .map(|(at, b)| b[(a - at) as usize])
            .unwrap_or(mem[a as usize])
    }
}

/// How one routine's states are made. One per routine, registered by name in [`crate::generators`]; the
/// translation service's judge (T33) and the engine port's rig test use the same one.
#[derive(Clone, Copy)]
pub struct Generator {
    /// Laid over memory once before all cases: screens and tiles, a map. Deterministic.
    pub base: Option<fn(&mut [u8])>,
    pub case: fn(&mut Gen) -> Setup,
}

/// What the port did, as the original's [`CallResult`]: the registers it returns with, the bytes it
/// wrote (address, new value; any order, a later write to an address wins, a byte written with the
/// value it held is not a change), and the custom-chip writes in order as (offset from `$dff000`,
/// word), a byte write as that byte on both halves.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Effects {
    pub regs: Regs,
    pub writes: Vec<(u32, u8)>,
    pub hw_writes: Vec<(u16, u16)>,
}

/// Which returned registers a check compares: bit `n` of `d` for `dn`, of `a` for `an`. `a7`, the
/// stack pointer, is never compared. A literal port is checked on [`RegsCompared::ALL`]; an engine
/// port on typed state models only the registers a caller reads (a scroll step's `d0`, say), and
/// names those, so the rest, scratch the original leaves behind, are not held against it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegsCompared {
    pub d: u8,
    pub a: u8,
}

impl RegsCompared {
    pub const ALL: RegsCompared = RegsCompared { d: 0xff, a: 0x7f };
    /// Memory and custom-chip writes only: a routine whose every caller saves and restores the
    /// registers around it, or reloads them before reading.
    pub const NONE: RegsCompared = RegsCompared { d: 0, a: 0 };

    /// The data registers `ds` only.
    pub const fn d(ds: &[usize]) -> RegsCompared {
        let (mut d, mut i) = (0u8, 0);
        while i < ds.len() {
            d |= 1 << ds[i];
            i += 1;
        }
        RegsCompared { d, a: 0 }
    }
}

/// What a check ran.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RigReport {
    /// States the original returned from and the port matched.
    pub passed: usize,
    /// Distinct instruction counts the original took over the states it returned from: a proxy for
    /// the branches the generator reaches.
    pub paths: usize,
    /// States the original had not returned from after `max_instr` instructions.
    pub hangs: usize,
    /// States the original took an exception on (an address error from a pointer read off a
    /// table's end, say).
    pub faults: usize,
    /// States the original returned from after starting a blit the routine oracle does not model:
    /// line mode or fill mode (BLTCON1 bit 0, 3 or 4 at the BLTSIZE write). The shim runs neither,
    /// so a port agreeing on such a state proves nothing (T33 answers `no-oracle`).
    pub unmodelled_blits: usize,
}

/// One conditional branch of a routine or its callees (`Bcc` other than `BRA`/`BSR`, `DBcc`), by
/// address, and over how many states it went each way. A `DBcc` loop's exit is not taken. A branch
/// with either count 0 is uncovered: no state took it that way.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Branch {
    pub at: u32,
    pub taken: u32,
    pub not_taken: u32,
}

impl Branch {
    pub fn covered(&self) -> bool {
        self.taken > 0 && self.not_taken > 0
    }
}

/// What [`Rig::coverage`] measured: the survey of the same states, every branch, every byte read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Coverage {
    pub report: RigReport,
    /// The static walk's branches from the entry and every branch executed (one reached only
    /// through an indirect jump), each once, ascending. One never executed has both counts 0.
    pub branches: Vec<Branch>,
    /// Every byte the original read in data space on some state it returned from, off the rig's
    /// stack ([`STACK_AREA`]): its operands, tables it indexes, hardware registers. Instruction
    /// fetches and PC-relative operands are not here.
    pub reads: BTreeSet<u32>,
}

impl Coverage {
    /// The branches some way of which no state took.
    pub fn uncovered(&self) -> impl Iterator<Item = &Branch> {
        self.branches.iter().filter(|b| !b.covered())
    }
}

/// What a traced run gathers, state by state.
#[derive(Default)]
struct Tally {
    branches: BTreeMap<u32, Branch>,
    reads: BTreeSet<u32>,
}

/// A check that failed: how many states differed, the first one in full, and the counts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RigFailure {
    pub name: String,
    /// States the port did not match.
    pub failures: usize,
    pub report: RigReport,
    /// The first failing state's inputs and differences, or why nothing was compared.
    pub first: String,
}

impl fmt::Display for RigFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let r = &self.report;
        write!(
            f,
            "{}: {} state(s) differ, {} passed, {} paths, {} hang(s), {} fault(s)\n{}",
            self.name, self.failures, r.passed, r.paths, r.hangs, r.faults, self.first
        )
    }
}

impl std::error::Error for RigFailure {}

/// The original routine on Moira over the loaded program, and the comparison with a port.
pub struct Rig {
    cpu: Cpu68k,
    /// Memory before any generator's base: the program and the VHPOSR probe.
    pristine: Arc<Vec<u8>>,
    /// Each hunk's base and size, for naming addresses; empty for a rig over raw memory.
    hunks: Vec<(u32, u32)>,
    max_instr: u64,
    last_regs: Option<Regs>,
}

/// Each hunk's `(base, size)` on the oracle.
pub type HunkSpans = Vec<(u32, u32)>;

/// `exe`'s hunks at [`crate::HUNK_BASES`], relocated, in a `MEM_SIZE` image, and each hunk's
/// `(base, size)`: memory as the oracle holds the program once loaded.
pub fn load_program(exe: &[u8]) -> Result<(Vec<u8>, HunkSpans), String> {
    let hunks = amiga_disk::hunk::parse(exe).map_err(|e| format!("{PROGRAM}: {e}"))?;
    if hunks.len() != HUNK_BASES.len() {
        return Err(format!(
            "{PROGRAM}: {} hunks, not the {} the oracle measured",
            hunks.len(),
            HUNK_BASES.len()
        ));
    }
    let mut mem = vec![0u8; MEM_SIZE];
    for (hk, &base) in hunks.iter().zip(&HUNK_BASES) {
        mem[base as usize..base as usize + hk.data.len()].copy_from_slice(&hk.data);
    }
    for (hk, &base) in hunks.iter().zip(&HUNK_BASES) {
        for r in &hk.relocs {
            let target = *HUNK_BASES
                .get(r.target as usize)
                .ok_or_else(|| format!("{PROGRAM}: a relocation to hunk {}", r.target))?;
            for &off in &r.offsets {
                let at = (base + off) as usize;
                let v =
                    u32::from_be_bytes(mem[at..at + 4].try_into().unwrap()).wrapping_add(target);
                mem[at..at + 4].copy_from_slice(&v.to_be_bytes());
            }
        }
    }
    let spans = hunks
        .iter()
        .zip(&HUNK_BASES)
        .map(|(hk, &b)| (b, hk.size))
        .collect();
    Ok((mem, spans))
}

impl Rig {
    /// The main program from the owner's disks, loaded at the oracle's hunk bases.
    pub fn new() -> Result<Rig, String> {
        let dir = fd_tools::disk_dir()?;
        let exe = fd_tools::game_file(&dir, PROGRAM)?;
        Rig::from_program(&exe)
    }

    /// `exe`'s hunks at [`crate::HUNK_BASES`], relocated: the loader the game's own `LoadSeg` would
    /// be. `Cpu68k::load_exe` places hunks back to back and the oracle did not.
    pub fn from_program(exe: &[u8]) -> Result<Rig, String> {
        let (mem, hunks) = load_program(exe)?;
        let mut rig = Rig::from_memory(mem);
        rig.hunks = hunks;
        Ok(rig)
    }

    /// A rig over `mem` (padded to 16 MB), for routines assembled by hand in tests. The probe's
    /// eight bytes at `$700000` are overwritten.
    pub fn from_memory(mut mem: Vec<u8>) -> Rig {
        mem.resize(MEM_SIZE, 0);
        mem[PROBE as usize..PROBE as usize + PROBE_CODE.len()].copy_from_slice(&PROBE_CODE);
        let mut cpu = Cpu68k::new(AmigaMemory {
            chip: vec![],
            slow: vec![],
        });
        cpu.memory_mut().copy_from_slice(&mem);
        cpu.set_hardware(true);
        Rig {
            cpu,
            pristine: Arc::new(mem),
            hunks: Vec::new(),
            max_instr: MAX_INSTR,
            last_regs: None,
        }
    }

    /// Instructions after which a state counts as a hang ([`MAX_INSTR`] unless set).
    pub fn set_max_instr(&mut self, n: u64) {
        self.max_instr = n;
    }

    /// The original's registers when it last returned, in the last check or survey.
    pub fn last_regs(&self) -> Option<Regs> {
        self.last_regs
    }

    /// The memory every case of `generator` starts from, before its pokes: what a port reads.
    pub fn memory_for(&self, generator: &Generator) -> Arc<Vec<u8>> {
        match generator.base {
            None => self.pristine.clone(),
            Some(lay) => {
                let mut m = self.pristine.to_vec();
                lay(&mut m);
                Arc::new(m)
            }
        }
    }

    /// `hN+$off` for an address inside the program, else `$xxxxxx`.
    pub fn name(&self, a: u32) -> String {
        for (i, &(b, n)) in self.hunks.iter().enumerate() {
            if a >= b && a < b + n {
                return format!("h{i}+${:x}", a - b);
            }
        }
        format!("${a:06x}")
    }

    /// Run the original at `entry` and `port` from `cases` states of `generator`, seeded by `name`, and
    /// compare. `Ok` only when every state the original returned from matched and there was at
    /// least one.
    pub fn check(
        &mut self,
        name: &str,
        cases: usize,
        entry: u32,
        generator: &Generator,
        port: impl FnMut(&Setup) -> Effects,
    ) -> Result<RigReport, RigFailure> {
        self.check_regs(name, cases, entry, generator, RegsCompared::ALL, port)
    }

    /// [`Rig::check`], comparing only the returned registers `regs` names.
    pub fn check_regs(
        &mut self,
        name: &str,
        cases: usize,
        entry: u32,
        generator: &Generator,
        regs: RegsCompared,
        mut port: impl FnMut(&Setup) -> Effects,
    ) -> Result<RigReport, RigFailure> {
        self.run(name, cases, entry, generator, regs, Some(&mut port), None)
    }

    /// The original alone over `cases` states: how many it returns from (`passed`), its paths, its
    /// hangs and faults. What a generator task shows before any port exists, and what tells a
    /// routine the oracle cannot run (it returns on no state) from one it can.
    pub fn survey(
        &mut self,
        name: &str,
        cases: usize,
        entry: u32,
        generator: &Generator,
    ) -> RigReport {
        match self.run(name, cases, entry, generator, RegsCompared::ALL, None, None) {
            Ok(r) => r,
            Err(f) => f.report,
        }
    }

    /// The original alone over the same `cases` states as [`Rig::check`] and [`Rig::survey`] with
    /// the same `name`, traced: every conditional branch of the routine and its callees, over how
    /// many returning states it went each way, and every byte those states read. A generator that
    /// leaves a branch uncovered does not exercise that way of the routine. The report is the
    /// survey's. Tracing costs time (`toolkit/docs/architecture/verification.md`), so `check` and
    /// `survey` run without it.
    pub fn coverage(
        &mut self,
        name: &str,
        cases: usize,
        entry: u32,
        generator: &Generator,
    ) -> Coverage {
        let mut tally = Tally::default();
        for at in self.cpu.conditional_branches(entry) {
            tally.branches.insert(
                at,
                Branch {
                    at,
                    ..Branch::default()
                },
            );
        }
        self.cpu.set_trace(true);
        let report = match self.run(
            name,
            cases,
            entry,
            generator,
            RegsCompared::ALL,
            None,
            Some(&mut tally),
        ) {
            Ok(r) => r,
            Err(f) => f.report,
        };
        self.cpu.set_trace(false);
        Coverage {
            report,
            branches: tally.branches.into_values().collect(),
            reads: tally.reads,
        }
    }

    fn run(
        &mut self,
        name: &str,
        cases: usize,
        entry: u32,
        generator: &Generator,
        compared: RegsCompared,
        mut port: Option<&mut dyn FnMut(&Setup) -> Effects>,
        mut tally: Option<&mut Tally>,
    ) -> Result<RigReport, RigFailure> {
        if let Some(lay) = generator.base {
            lay(self.cpu.memory_mut());
        }
        let mut g = Gen::new(seed(name));
        let mut report = RigReport::default();
        let mut paths = BTreeSet::new();
        let (mut failures, mut first) = (0usize, None::<String>);
        for case in 0..cases {
            let mut s = (generator.case)(&mut g);
            let saved: Vec<(u32, Vec<u8>)> = s
                .pokes
                .iter()
                .map(|(a, b)| (*a, self.cpu.read(*a, b.len()).to_vec()))
                .collect();
            for (a, b) in &s.pokes {
                self.cpu.write(*a, b);
            }
            for &(a, v) in &s.hw_reads {
                self.cpu.set_hardware_read(a, v);
            }
            let probe = Regs {
                a: [0, 0, 0, 0, 0, 0, 0, STACK],
                ..Regs::default()
            };
            let beam = self
                .cpu
                .call(PROBE, probe, 10)
                .expect("the VHPOSR probe")
                .regs
                .d[0] as u8;
            s.beam = beam.wrapping_add(1);
            let mut regs = Regs {
                d: s.d,
                a: s.a,
                ..Regs::default()
            };
            regs.a[7] = STACK;
            let res = self.cpu.call(entry, regs, self.max_instr);
            let diffs = match &res {
                Err(CallError::Timeout) => {
                    report.hangs += 1;
                    None
                }
                Err(_) => {
                    report.faults += 1;
                    None
                }
                Ok(r) => {
                    paths.insert(r.instr);
                    if let Some(t) = tally.as_mut() {
                        let trace = self.cpu.last_trace();
                        for w in trace.branches {
                            let b = t.branches.entry(w.at).or_insert(Branch {
                                at: w.at,
                                ..Branch::default()
                            });
                            b.taken += w.taken as u32;
                            b.not_taken += w.not_taken as u32;
                        }
                        t.reads
                            .extend(trace.reads.into_iter().filter(|a| !STACK_AREA.contains(a)));
                    }
                    if unmodelled_blit(&r.hw_writes) {
                        report.unmodelled_blits += 1;
                    }
                    self.last_regs = Some(r.regs);
                    match port.as_mut() {
                        None => Some(Vec::new()),
                        Some(p) => {
                            let ran = catch_unwind(AssertUnwindSafe(|| p(&s)));
                            Some(match ran {
                                Err(e) => vec![format!("the port panicked: {}", panic_text(&e))],
                                Ok(eff) => self.compare(r, &eff, compared),
                            })
                        }
                    }
                }
            };
            for (a, b) in saved.iter().rev() {
                self.cpu.write(*a, b);
            }
            for &(a, _) in &s.hw_reads {
                self.cpu.set_hardware_read(a, 0);
            }
            match diffs {
                None => {}
                Some(d) if d.is_empty() => report.passed += 1,
                Some(d) => {
                    failures += 1;
                    if first.is_none() {
                        first = Some(format!(
                            "case {case}:\n{}differences ({} in all):\n  {}",
                            self.describe(&s),
                            d.len(),
                            d[..d.len().min(SHOWN)].join("\n  ")
                        ));
                    }
                }
            }
        }
        if generator.base.is_some() {
            self.cpu.memory_mut().copy_from_slice(&self.pristine);
        }
        report.paths = paths.len();
        let fail = |failures, first| RigFailure {
            name: name.to_string(),
            failures,
            report,
            first,
        };
        if failures > 0 {
            return Err(fail(failures, first.unwrap_or_default()));
        }
        if report.passed == 0 {
            return Err(fail(
                0,
                format!(
                    "compared nothing: the original returned on none of {cases} states \
                     ({} hang(s), {} fault(s))",
                    report.hangs, report.faults
                ),
            ));
        }
        Ok(report)
    }

    /// The differences between the original's result and the port's effects, custom-chip writes
    /// first. The cpu's memory holds the case's state (the call rolled its own writes back).
    fn compare(&self, want: &CallResult, got: &Effects, regs: RegsCompared) -> Vec<String> {
        let mut diffs = Vec::new();
        let (ow, pw) = (&want.hw_writes, &got.hw_writes);
        if ow != pw {
            let k = ow.iter().zip(pw.iter()).take_while(|(x, y)| x == y).count();
            let show = |v: &[(u16, u16)]| {
                v.iter()
                    .skip(k)
                    .take(6)
                    .map(|(r, w)| format!("$dff{r:03x}=${w:04x}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            diffs.push(format!(
                "custom-chip writes: original made {}, port {}; the first {k} agree, then original [{}] port [{}]",
                ow.len(),
                pw.len(),
                show(ow),
                show(pw)
            ));
        }
        for i in (0..8).filter(|i| regs.d & 1 << i != 0) {
            let (o, p) = (want.regs.d[i], got.regs.d[i]);
            if o != p {
                diffs.push(format!("d{i}: original ${o:08x}, port ${p:08x}"));
            }
        }
        for i in (0..7).filter(|i| regs.a & 1 << i != 0) {
            let (o, p) = (want.regs.a[i], got.regs.a[i]);
            if o != p {
                diffs.push(format!("a{i}: original ${o:08x}, port ${p:08x}"));
            }
        }
        let before = |a: u32| self.cpu.read(a, 1)[0];
        let theirs: BTreeMap<u32, u8> = want
            .writes
            .iter()
            .filter(|w| !STACK_AREA.contains(&w.0))
            .map(|w| (w.0, w.2))
            .collect();
        let mut ours = BTreeMap::new();
        for &(a, v) in &got.writes {
            ours.insert(a & 0xFF_FFFF, v);
        }
        ours.retain(|&a, &mut v| !STACK_AREA.contains(&a) && before(a) != v);
        let addrs: BTreeSet<u32> = theirs.keys().chain(ours.keys()).copied().collect();
        for a in addrs {
            match (theirs.get(&a), ours.get(&a)) {
                (Some(o), Some(p)) if o != p => diffs.push(format!(
                    "byte {}: original wrote ${o:02x}, port wrote ${p:02x}",
                    self.name(a)
                )),
                (Some(o), None) => diffs.push(format!(
                    "byte {}: original wrote ${o:02x}, port left it ${:02x}",
                    self.name(a),
                    before(a)
                )),
                (None, Some(p)) => diffs.push(format!(
                    "byte {}: original left it ${:02x}, port wrote ${p:02x}",
                    self.name(a),
                    before(a)
                )),
                _ => {}
            }
        }
        diffs
    }

    fn describe(&self, s: &Setup) -> String {
        let mut t = format!("entry registers: d={:08x?} a0-a6={:08x?}\n", s.d, &s.a[..7]);
        t += "memory set before the call:\n";
        for (a, bytes) in &s.pokes {
            let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
            t += &format!("  {} = ${hex}\n", self.name(*a));
        }
        for (a, v) in &s.hw_reads {
            t += &format!("hardware byte ${a:06x} reads ${v:02x}\n");
        }
        t
    }
}

/// Whether `writes` start a blit in line or fill mode: BLTCON1 ($dff042) as last written before a
/// BLTSIZE ($dff058) write. A BLTCON1 not written in the call counts as 0.
fn unmodelled_blit(writes: &[(u16, u16)]) -> bool {
    let mut con1 = 0u16;
    for &(reg, v) in writes {
        match reg {
            0x42 => con1 = v,
            0x58 if con1 & 0x19 != 0 => return true,
            _ => {}
        }
    }
    false
}

/// A routine's seed from its name, so each generator's states are fixed and differ between routines.
/// The seed of the states a check, survey or coverage run named `name` makes: a generator's states
/// are reproduced from it (fd-judge's `--cover` poke check).
pub fn seed(name: &str) -> u64 {
    name.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
        (h ^ b as u64).wrapping_mul(0x100_0000_01b3)
    })
}

fn panic_text(e: &Box<dyn std::any::Any + Send>) -> String {
    e.downcast_ref::<String>()
        .cloned()
        .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_default()
}
