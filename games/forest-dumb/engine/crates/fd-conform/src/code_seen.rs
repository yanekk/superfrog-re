//! Code seen running on the original (translator DESIGN §2.3, T02): every instruction start the
//! original executes, reduced to the entries a static export must seed, written to
//! `games/forest-dumb/re/entries.toml`.
//!
//! Three kinds of run, unioned: level 1's cases on the original from the level sync point,
//! unfrozen ([`crate::code_writes::run_case_seen_in`]); the power-on run through the title into
//! level 1 on `boot-to-level1`; and each world 2 to 6 start: level 1 ended by the cheat with
//! `world_index` set so the game's own `next_world` loads that world, run into its play. Every even address of
//! the program's hunks is watched (`Amiga::watch_pcs`).
//!
//! An executed address counts only when its bytes are the loaded program's: before the program
//! loads, the crack's own programs run in the same memory, and they are out of scope (§8). Those are
//! counted as `foreign` and dropped.
//!
//! The reduction ([`run_starts`]): a start is an executed instruction that no other executed
//! instruction reaches by falling through or by a direct branch, jump or call (a target decoded
//! from the instruction's own bytes), and then, since a loop entered only by a computed jump is
//! reached by its own back-branch, the lowest executed address still unreached from the starts and
//! symbols, repeated until every executed instruction is reached ([`closure`]). What is left is what static analysis cannot find from its
//! neighbours: computed jump and call targets, interrupt entries, code entered from the OS. Direct
//! call targets are not starts, because the export follows a direct call itself; keeping them out
//! keeps the file small. Every executed `func` of `re/symbols.toml` is listed as `symbol` whether or
//! not it is a start, the program entry `h0+$0` always, and a start some autovector (`$64..$7c`)
//! pointed at, at a frame boundary, as `vector`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use amiga_conform::script;
use amiga_oracle::{Amiga, AmigaMemory, Cpu68k, FrameInput, Joy, OracleConfig, read_only};

use crate::case::Case;
use crate::code_writes::{MAX_LEN, read, run_case_seen_in};
use crate::routine::{HunkSpans, PROGRAM, h, load_program};
use crate::{address, conform_dir, disks, sync};
use fd_core::{Width, layout_var};

/// Where an entry came from, in the order one is preferred when several apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Source {
    ProgramEntry,
    Symbol,
    Vector,
    Trace,
}

impl Source {
    pub fn name(self) -> &'static str {
        match self {
            Source::ProgramEntry => "program-entry",
            Source::Symbol => "symbol",
            Source::Vector => "vector",
            Source::Trace => "trace",
        }
    }

    pub const ALL: [Source; 4] = [
        Source::ProgramEntry,
        Source::Symbol,
        Source::Vector,
        Source::Trace,
    ];
}

/// How control leaves one instruction, from its bytes alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Flow {
    /// The next instruction can run after it (a conditional branch, a call that returns).
    pub falls: bool,
    /// A target written in the instruction: `Bcc`, `BRA`, `BSR`, `DBcc`, `JMP`/`JSR` to an
    /// absolute or PC-relative address. Computed targets (`(a0)`, `(d8,pc,xn)`) are `None`.
    pub direct: Option<u32>,
}

fn word(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes([*b.get(at)?, *b.get(at + 1)?]))
}

/// [`Flow`] of the instruction `insn` at `addr`.
pub fn flow(addr: u32, insn: &[u8]) -> Flow {
    let Some(op) = word(insn, 0) else {
        return Flow {
            falls: true,
            direct: None,
        };
    };
    let rel = |d: i32| addr.wrapping_add(2).wrapping_add(d as u32);
    match op {
        // RTS, RTE, RTR, ILLEGAL.
        0x4e75 | 0x4e73 | 0x4e77 | 0x4afc => {
            return Flow {
                falls: false,
                direct: None,
            };
        }
        _ => {}
    }
    let jmp = op & 0xffc0 == 0x4ec0;
    if jmp || op & 0xffc0 == 0x4e80 {
        let (mode, reg) = ((op >> 3) & 7, op & 7);
        let direct = match (mode, reg) {
            (7, 0) => word(insn, 2).map(|w| w as i16 as i32 as u32),
            (7, 1) => word(insn, 2)
                .zip(word(insn, 4))
                .map(|(hi, lo)| (hi as u32) << 16 | lo as u32),
            (7, 2) => word(insn, 2).map(|w| rel(w as i16 as i32)),
            _ => None,
        };
        return Flow {
            falls: !jmp,
            direct,
        };
    }
    if op & 0xf000 == 0x6000 {
        let d8 = op as u8 as i8;
        let disp = if d8 == 0 {
            word(insn, 2).map(|w| w as i16 as i32)
        } else {
            Some(d8 as i32)
        };
        return Flow {
            falls: (op >> 8) & 0xf != 0,
            direct: disp.map(rel),
        };
    }
    if op & 0xf0f8 == 0x50c8 {
        return Flow {
            falls: true,
            direct: word(insn, 2).map(|w| rel(w as i16 as i32)),
        };
    }
    Flow {
        falls: true,
        direct: None,
    }
}

/// The executed instructions (start address to bytes) no other executed instruction reaches by
/// falling through or by a direct target.
pub fn local_starts(insns: &BTreeMap<u32, Vec<u8>>) -> BTreeSet<u32> {
    let mut reached = BTreeSet::new();
    for (&a, bytes) in insns {
        let f = flow(a, bytes);
        if f.falls {
            reached.insert(a + bytes.len() as u32);
        }
        if let Some(t) = f.direct {
            reached.insert(t & 0xff_ffff);
        }
    }
    let local: BTreeSet<u32> = insns
        .keys()
        .copied()
        .filter(|a| !reached.contains(a))
        .collect();
    local
}

/// [`local_starts`] and the starts [`closure`] adds to them: from these, falling through and
/// direct targets reach every executed instruction.
pub fn run_starts(insns: &BTreeMap<u32, Vec<u8>>) -> BTreeSet<u32> {
    let local = local_starts(insns);
    let extra = closure(insns, &local);
    local.into_iter().chain(extra).collect()
}

/// The starts to add to `roots` so every instruction of `insns` is reached from one by falling
/// through and direct targets. A loop entered only by a computed jump is reached by its own
/// back-branch, so no instruction of it is a local start; the lowest unreached address is taken,
/// and again until none is left.
pub fn closure(insns: &BTreeMap<u32, Vec<u8>>, roots: &BTreeSet<u32>) -> BTreeSet<u32> {
    let mut reached = BTreeSet::new();
    let mut extra = BTreeSet::new();
    let walk = |from: u32, reached: &mut BTreeSet<u32>| {
        let mut todo = vec![from];
        while let Some(a) = todo.pop() {
            let Some(bytes) = insns.get(&a) else { continue };
            if !reached.insert(a) {
                continue;
            }
            let f = flow(a, bytes);
            if f.falls {
                todo.push(a + bytes.len() as u32);
            }
            if let Some(t) = f.direct {
                todo.push(t & 0xff_ffff);
            }
        }
    };
    for &r in roots {
        walk(r, &mut reached);
    }
    while let Some(&a) = insns.keys().find(|a| !reached.contains(a)) {
        extra.insert(a);
        walk(a, &mut reached);
    }
    extra
}

/// The entries to record: every run start, every executed `func` symbol, and `program_entry`, each
/// with its [`Source`].
pub fn entries(
    insns: &BTreeMap<u32, Vec<u8>>,
    funcs: &BTreeSet<u32>,
    vectors: &BTreeSet<u32>,
    program_entry: u32,
) -> BTreeMap<u32, Source> {
    // Closed from the local starts and the symbols together, so a symbol that reaches a computed
    // loop spares it a trace entry.
    let mut at: BTreeSet<u32> = local_starts(insns);
    at.extend(funcs.iter().filter(|a| insns.contains_key(a)));
    at.insert(program_entry);
    let extra = closure(insns, &at);
    at.extend(extra);
    at.into_iter()
        .map(|a| {
            let s = if a == program_entry {
                Source::ProgramEntry
            } else if funcs.contains(&a) {
                Source::Symbol
            } else if vectors.contains(&a) {
                Source::Vector
            } else {
                Source::Trace
            };
            (a, s)
        })
        .collect()
}

/// `hN+$off`, else `$xxxxxx`.
pub fn name(hunks: &[(u32, u32)], a: u32) -> String {
    match hunks.iter().position(|&(b, n)| a >= b && a < b + n) {
        Some(i) => format!("h{i}+${:x}", a - hunks[i].0),
        None => format!("${a:06x}"),
    }
}

/// `re/entries.toml`'s text: a header, then one `[[entry]]` per entry in hunk and offset order.
pub fn render(entries: &BTreeMap<u32, Source>, hunks: &[(u32, u32)]) -> String {
    let key = |a: u32| {
        hunks
            .iter()
            .position(|&(b, n)| a >= b && a < b + n)
            .map_or((usize::MAX, a), |i| (i, a - hunks[i].0))
    };
    let mut list: Vec<(u32, Source)> = entries.iter().map(|(&a, &s)| (a, s)).collect();
    list.sort_by_key(|&(a, _)| key(a));
    let mut out = String::from(
        "# Where the original was seen running: the entries the whole-program export seeds (translator\n\
         # DESIGN §2.3). Written by `fd-conform code-seen` (fd-conform/src/code_seen.rs); do not edit.\n\
         # Addresses and sources only. source: program-entry | symbol (a func of re/symbols.toml that\n\
         # ran) | vector (an autovector pointed at it) | trace (a run start nothing falls or branches to\n\
         # directly: a computed target, an interrupt entry, code entered from the OS).\n",
    );
    for (a, s) in list {
        out.push_str(&format!(
            "\n[[entry]]\nat = \"{}\"\nsource = \"{}\"\n",
            name(hunks, a),
            s.name()
        ));
    }
    out
}

/// One `[[sym]]` of `re/symbols.toml`: the fields this module reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sym {
    pub hunk: usize,
    pub offset: u32,
    pub name: String,
    pub kind: String,
}

/// `re/symbols.toml`'s symbols, by hand as the crate's cases are (no `toml` crate, Cargo.toml).
pub fn parse_symbols(text: &str) -> Result<Vec<Sym>, String> {
    let mut out = Vec::new();
    let mut cur: Option<(Option<usize>, Option<u32>, Option<String>, Option<String>)> = None;
    let finish = |c: (Option<usize>, Option<u32>, Option<String>, Option<String>),
                  out: &mut Vec<Sym>|
     -> Result<(), String> {
        match c {
            (Some(hunk), Some(offset), Some(name), Some(kind)) => {
                out.push(Sym {
                    hunk,
                    offset,
                    name,
                    kind,
                });
                Ok(())
            }
            c => Err(format!("symbols: an incomplete [[sym]] {c:?}")),
        }
    };
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if line == "[[sym]]" {
            if let Some(c) = cur.take() {
                finish(c, &mut out)?;
            }
            cur = Some((None, None, None, None));
            continue;
        }
        let Some(c) = cur.as_mut() else { continue };
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let (k, v) = (k.trim(), v.trim());
        let string = || {
            v.strip_prefix('"')
                .and_then(|r| r.split_once('"'))
                .map(|(s, _)| s.to_string())
        };
        let int = || -> Result<u32, String> {
            let r = match v.strip_prefix("0x") {
                Some(h) => u32::from_str_radix(h, 16),
                None => v.parse(),
            };
            r.map_err(|e| format!("symbols line {}: {e}", n + 1))
        };
        match k {
            "hunk" => c.0 = Some(int()? as usize),
            "offset" => c.1 = Some(int()?),
            "name" => c.2 = string(),
            "kind" => c.3 = string(),
            _ => {}
        }
    }
    if let Some(c) = cur.take() {
        finish(c, &mut out)?;
    }
    Ok(out)
}

/// `games/forest-dumb/re/`.
pub fn re_dir() -> PathBuf {
    conform_dir().join("../../re")
}

/// The case whose cheat and HELP end level 1 (`main_loop` then runs `next_world`, frame 6525): a
/// world start is this case with `world_index` set to the world before the one wanted, so the
/// game's own `next_world` and `load_world` load it. Poking `world_index` before the title's Start
/// does not hold: the game sets it to 0 after Start (measured 2026-11-11).
pub const LEVEL_END_CASE: &str = "level-end.toml";
/// Frames of play each world start runs once `game_running` is set again.
pub const WORLD_PLAY_FRAMES: u32 = 300;
/// A world start that has not reached play this many frames after its case's last frame is an
/// error. World 2's SCENA screen waited from about 1 000 frames after `next_world` (09-play-measured).
pub const WORLD_REACH_CAP: u32 = 4000;
/// After the case's frames, fire is held [`FIRE_HELD`] frames in every [`FIRE_EVERY`] until play
/// starts, since how long each world takes to load is not known and its SCENA screen waits for fire.
const FIRE_EVERY: u32 = 60;
const FIRE_HELD: u32 = 11;
/// The default power-on run: to the level sync point and 699 frames of level 1 on.
pub const POWER_ON_FRAMES: u32 = 7000;
/// The autovectors, levels 1 to 7.
const VECTORS: std::ops::RangeInclusive<u32> = 0x64..=0x7c;

/// What the runs saw.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Seen {
    /// Executed instruction starts with the loaded program's bytes.
    pub insns: BTreeMap<u32, Vec<u8>>,
    /// Program addresses an autovector held at a frame boundary.
    pub vectors: BTreeSet<u32>,
    /// Executed addresses whose bytes were not the program's when they ran (and never were later).
    pub foreign: BTreeSet<u32>,
    /// One line per run: its name, steps and the instructions it executed.
    pub runs: Vec<(String, u32, usize)>,
}

impl Seen {
    fn merge(&mut self, other: Seen) {
        for (a, b) in other.insns {
            self.insns.entry(a).or_insert(b);
        }
        self.vectors.extend(other.vectors);
        self.foreign.extend(other.foreign);
        self.runs.extend(other.runs);
        let insns = &self.insns;
        self.foreign.retain(|a| !insns.contains_key(a));
    }
}

/// The loaded program, its hunks, and the length decoder, shared by the collectors of a run.
struct Program {
    loaded: Vec<u8>,
    hunks: HunkSpans,
}

impl Program {
    fn pcs(&self) -> Vec<u32> {
        self.hunks
            .iter()
            .flat_map(|&(base, size)| (base..base + size).step_by(2))
            .collect()
    }

    fn inside(&self, a: u32) -> bool {
        self.hunks.iter().any(|&(b, n)| a >= b && a < b + n)
    }

    fn vectors(&self, mem: &AmigaMemory, into: &mut BTreeSet<u32>) {
        for v in VECTORS.step_by(4) {
            let b = read(mem, v, 4);
            let a = u32::from_be_bytes([b[0], b[1], b[2], b[3]]) & 0xff_ffff;
            if self.inside(a) {
                into.insert(a);
            }
        }
    }
}

/// One run's watch over a live machine.
struct Collector<'p> {
    prog: &'p Program,
    cpu: Cpu68k,
    seen: Seen,
}

impl<'p> Collector<'p> {
    fn new(prog: &'p Program) -> Collector<'p> {
        Collector {
            prog,
            cpu: Cpu68k::new(AmigaMemory {
                chip: vec![],
                slow: vec![],
            }),
            seen: Seen::default(),
        }
    }

    /// The instruction at `addr` as `mem` holds it, if it is the loaded program's.
    fn program_insn(&mut self, mem: &AmigaMemory, addr: u32) -> Option<Vec<u8>> {
        let head = read(mem, addr, MAX_LEN);
        self.cpu.write(addr, &head);
        let len = (self.cpu.insn_len(addr) as usize).clamp(2, MAX_LEN);
        let file = &self.prog.loaded[addr as usize..addr as usize + len];
        (head[..len] == *file).then(|| file.to_vec())
    }

    fn after_step(&mut self, am: &Amiga) {
        let mem = am.memory();
        for &(addr, n) in am.pc_hits() {
            if n == 0 || self.seen.insns.contains_key(&addr) {
                continue;
            }
            match self.program_insn(&mem, addr) {
                Some(b) => {
                    self.seen.foreign.remove(&addr);
                    self.seen.insns.insert(addr, b);
                }
                None => {
                    self.seen.foreign.insert(addr);
                }
            }
        }
        self.prog.vectors(&mem, &mut self.seen.vectors);
    }
}

/// What to run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// Every level case here is run; others (`first-case`) are skipped.
    pub cases: PathBuf,
    pub power_on_frames: u32,
    pub worlds: Vec<u32>,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            cases: conform_dir().join("cases"),
            power_on_frames: POWER_ON_FRAMES,
            worlds: vec![2, 3, 4, 5, 6],
        }
    }
}

/// The level cases in `dir`, by file name; `skipped` gets the others' names.
pub fn level_cases(dir: &Path, skipped: &mut Vec<String>) -> Result<Vec<Case>, String> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .collect();
    paths.sort();
    let mut out = Vec::new();
    for p in &paths {
        let c = Case::load(p)?;
        if c.is_level() {
            out.push(c);
        } else {
            skipped.push(c.name);
        }
    }
    Ok(out)
}

/// The power-on run on `boot-to-level1` for `frames` steps.
fn power_on(prog: &Program, cfg: &OracleConfig, frames: u32) -> Result<Seen, String> {
    let (_, sc) = sync::level_1_script()?;
    let mut am = Amiga::power_on(cfg).map_err(|e| e.to_string())?;
    am.watch_pcs(&prog.pcs());
    let mut col = Collector::new(prog);
    for frame in 0..frames {
        if frame == sync::DISK_2_SWAP {
            am.insert_disk(0, 1);
        }
        am.step(script::input_at(&sc.lines, frame))
            .map_err(|e| e.to_string())?;
        col.after_step(&am);
    }
    let n = col.seen.insns.len();
    col.seen.runs.push(("power-on".into(), frames, n));
    Ok(col.seen)
}

/// World `world`'s start: [`LEVEL_END_CASE`] from the level sync point with `world_index` set to
/// `world - 2` at the case's sync frame, then fire pressed until the new world's play starts
/// (`world_index` is `world - 1`, `irq_count` stopped and counting again), then
/// [`WORLD_PLAY_FRAMES`] frames of play.
fn world_start(prog: &Program, paths: (&Path, &Path, &Path), world: u32) -> Result<Seen, String> {
    let end = Case::load(&conform_dir().join("cases").join(LEVEL_END_CASE))?;
    let inputs = crate::Inputs::load(&end)?;
    let world_index = address("world_index")?;
    let case_end = end.sync_frame + end.frames;
    let room = case_end - sync::LEVEL_1_SYNC + WORLD_REACH_CAP + WORLD_PLAY_FRAMES;
    let mut am = sync::reach_level_1_in(paths.0, paths.1, paths.2, room)?;
    am.watch_pcs(&prog.pcs());
    let mut col = Collector::new(prog);
    let (mut left, mut played) = (false, None);
    let irq_count = address("irq_count")?;
    let mut last_irq = am.peek(irq_count, 2);
    let mut frame = sync::LEVEL_1_SYNC;
    loop {
        if frame == end.sync_frame {
            for (name, value) in &end.set {
                let v = layout_var(name)
                    .ok_or_else(|| format!("`{name}` is not in fd-core's LAYOUT"))?;
                let bytes = match v.width {
                    Width::Byte => vec![*value as u8],
                    Width::Word => (*value as u16).to_be_bytes().to_vec(),
                    Width::Long => value.to_be_bytes().to_vec(),
                };
                am.poke(address(name)?, &bytes);
            }
            am.poke(world_index, &(world - 2).to_be_bytes());
        }
        let input = if frame < case_end {
            inputs.at(frame)
        } else if played.is_none() && (frame - case_end) % FIRE_EVERY < FIRE_HELD {
            FrameInput::joy(Joy::FIRE)
        } else {
            FrameInput::NONE
        };
        am.step(input).map_err(|e| e.to_string())?;
        col.after_step(&am);
        frame += 1;
        // `game_running` stays set through the load; `irq_count` stands still while the OS has the
        // machine and the SCENA screen waits, and counts again once play runs (measured 2026-11-11).
        let irq = am.peek(irq_count, 2);
        let running = irq != last_irq;
        last_irq = irq;
        let wi = am.peek(world_index, 4) == (world - 1).to_be_bytes();
        match played {
            None if !left => left = wi && !running,
            None if running => {
                if !wi {
                    return Err(format!("world {world}: play started in another world"));
                }
                played = Some(frame);
            }
            None if frame >= case_end + WORLD_REACH_CAP => {
                return Err(format!(
                    "world {world}: play not started {WORLD_REACH_CAP} frames after the level's end"
                ));
            }
            Some(p) if frame - p >= WORLD_PLAY_FRAMES => break,
            _ => {}
        }
    }
    let n = col.seen.insns.len();
    col.seen
        .runs
        .push((format!("world-{world}"), frame - sync::LEVEL_1_SYNC, n));
    Ok(col.seen)
}

/// One level case through `code_writes`' runner, its instructions kept where they are the
/// program's.
fn case_run(prog: &Program, paths: (&Path, &Path, &Path), case: &Case) -> Result<Seen, String> {
    let mut seen = Seen::default();
    let vectors = &mut seen.vectors;
    let (report, insns) = run_case_seen_in(paths.0, paths.1, paths.2, case, None, |am, _| {
        prog.vectors(&am.memory(), vectors)
    })?;
    for (a, b) in insns {
        let at = a as usize;
        if prog.loaded[at..at + b.len()] == *b {
            seen.insns.insert(a, b);
        } else {
            seen.foreign.insert(a);
        }
    }
    seen.runs
        .push((case.name.clone(), report.frames, seen.insns.len()));
    Ok(seen)
}

/// Every run [`Options`] names, on the Kickstart, disks and level sync cache given, in parallel
/// after the power-on run (each its own oracle), merged in a fixed order.
pub fn collect_in(
    kickstart: &Path,
    disk_dir: &Path,
    cache: &Path,
    opts: &Options,
    skipped: &mut Vec<String>,
) -> Result<(Seen, HunkSpans), String> {
    let rom_check = read_only(kickstart).map_err(|e| e.to_string())?;
    drop(rom_check);
    let disks = disks::read(disk_dir)?;
    let exe = fd_tools::game_file(disk_dir, PROGRAM)?;
    let (loaded, hunks) = load_program(&exe)?;
    let prog = Program {
        loaded,
        hunks: hunks.clone(),
    };
    let cases = level_cases(&opts.cases, skipped)?;
    let cfg = OracleConfig::new(kickstart.to_path_buf(), disks, opts.power_on_frames + 10);
    let mut seen = power_on(&prog, &cfg, opts.power_on_frames)?;

    enum Job<'c> {
        World(u32),
        Case(&'c Case),
    }
    let jobs: Vec<Job> = opts
        .worlds
        .iter()
        .map(|&w| Job::World(w))
        .chain(cases.iter().map(Job::Case))
        .collect();
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get());
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results: Vec<std::sync::Mutex<Option<Result<Seen, String>>>> =
        jobs.iter().map(|_| std::sync::Mutex::new(None)).collect();
    std::thread::scope(|s| {
        for _ in 0..workers.min(jobs.len()) {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(job) = jobs.get(i) else { break };
                    let r = match job {
                        Job::World(w) => world_start(&prog, (kickstart, disk_dir, cache), *w),
                        Job::Case(c) => case_run(&prog, (kickstart, disk_dir, cache), c),
                    };
                    *results[i].lock().unwrap() = Some(r);
                }
            });
        }
    });
    for r in results {
        seen.merge(r.into_inner().unwrap().unwrap()?);
    }
    Ok((seen, hunks))
}

/// The executed `func` symbols' addresses, and the program entry, from `re/symbols.toml`.
pub fn funcs(symbols: &[Sym]) -> BTreeSet<u32> {
    symbols
        .iter()
        .filter(|s| s.kind == "func")
        .map(|s| h(s.hunk, s.offset))
        .collect()
}

/// Write `text` to `path` through a temporary file and a rename.
pub fn write_atomic(path: &Path, text: &str) -> Result<(), String> {
    let tmp = path.with_extension("toml.tmp");
    std::fs::write(&tmp, text).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}

/// The summary line: counts by source, foreign addresses, and executed `func` symbols.
pub fn summary_json(seen: &Seen, entries: &BTreeMap<u32, Source>, funcs: &BTreeSet<u32>) -> String {
    let by: Vec<String> = Source::ALL
        .iter()
        .map(|s| {
            format!(
                r#""{}":{}"#,
                s.name(),
                entries.values().filter(|&&x| x == *s).count()
            )
        })
        .collect();
    let frames: u64 = seen.runs.iter().map(|r| r.1 as u64).sum();
    format!(
        r#"{{"total":{{"runs":{},"frames":{},"instructions":{},"entries":{},"by_source":{{{}}},"foreign":{},"funcs_executed":{},"funcs":{}}}}}"#,
        seen.runs.len(),
        frames,
        seen.insns.len(),
        entries.len(),
        by.join(","),
        seen.foreign.len(),
        funcs.iter().filter(|a| seen.insns.contains_key(a)).count(),
        funcs.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(list: &[(u32, &[u8])]) -> BTreeMap<u32, Vec<u8>> {
        list.iter().map(|&(a, b)| (a, b.to_vec())).collect()
    }

    #[test]
    fn flow_decodes_branches_jumps_calls_and_returns() {
        // bra.s +4, beq.w, bsr.s, dbf d0
        assert_eq!(
            flow(0x100, &[0x60, 0x04]),
            Flow {
                falls: false,
                direct: Some(0x106)
            }
        );
        assert_eq!(
            flow(0x100, &[0x67, 0x00, 0xff, 0xfe]),
            Flow {
                falls: true,
                direct: Some(0x100)
            }
        );
        assert_eq!(
            flow(0x100, &[0x61, 0x10]),
            Flow {
                falls: true,
                direct: Some(0x112)
            }
        );
        assert_eq!(
            flow(0x100, &[0x51, 0xc8, 0x00, 0x10]),
            Flow {
                falls: true,
                direct: Some(0x112)
            }
        );
        // jsr abs.l, jmp abs.w, jmp (d16,pc), jsr (a0), jmp (d8,pc,d0)
        assert_eq!(
            flow(0x100, &[0x4e, 0xb9, 0x00, 0x01, 0x23, 0x44]),
            Flow {
                falls: true,
                direct: Some(0x12344)
            }
        );
        assert_eq!(
            flow(0x100, &[0x4e, 0xf8, 0x10, 0x00]),
            Flow {
                falls: false,
                direct: Some(0x1000)
            }
        );
        assert_eq!(
            flow(0x100, &[0x4e, 0xfa, 0x00, 0x20]),
            Flow {
                falls: false,
                direct: Some(0x122)
            }
        );
        assert_eq!(
            flow(0x100, &[0x4e, 0x90]),
            Flow {
                falls: true,
                direct: None
            }
        );
        assert_eq!(
            flow(0x100, &[0x4e, 0xfb, 0x00, 0x02]),
            Flow {
                falls: false,
                direct: None
            }
        );
        for op in [[0x4e, 0x75], [0x4e, 0x73], [0x4e, 0x77]] {
            assert_eq!(
                flow(0x100, &op),
                Flow {
                    falls: false,
                    direct: None
                }
            );
        }
        assert_eq!(
            flow(0x100, &[0x70, 0x01]),
            Flow {
                falls: true,
                direct: None
            }
        );
    }

    /// A made-up executed set: an entry that calls a routine directly and one through `a0`, a loop,
    /// and an interrupt handler nothing in the program reaches.
    fn program() -> BTreeMap<u32, Vec<u8>> {
        set(&[
            // entry
            (0x1000, &[0x41, 0xf9, 0x00, 0x00, 0x10, 0x40]), // lea $1040, a0
            (0x1006, &[0x61, 0x18]),                         // bsr.s $1020 (direct call)
            (0x1008, &[0x4e, 0x90]),                         // jsr (a0) -> $1040 (computed)
            (0x100a, &[0x60, 0xfa]),                         // bra.s $1006
            // direct callee: a dbf loop
            (0x1020, &[0x70, 0x03]),             // moveq #3, d0
            (0x1022, &[0x51, 0xc8, 0xff, 0xfe]), // dbf d0, $1022
            (0x1026, &[0x4e, 0x75]),             // rts
            // computed callee
            (0x1040, &[0x52, 0x80]), // addq.l #1, d0
            (0x1042, &[0x4e, 0x75]), // rts
            // interrupt handler
            (0x1060, &[0x52, 0x79, 0x00, 0x00, 0x20, 0x00]), // addq.w #1, $2000
            (0x1066, &[0x4e, 0x73]),                         // rte
        ])
    }

    #[test]
    fn a_computed_call_and_an_interrupt_entry_are_the_starts() {
        let starts: Vec<u32> = run_starts(&program()).into_iter().collect();
        assert_eq!(starts, vec![0x1000, 0x1040, 0x1060]);
    }

    #[test]
    fn code_after_a_return_reached_only_by_a_computed_jump_is_a_start() {
        let mut p = program();
        // After the handler's rte, reached only through a jump table.
        p.insert(0x1068, vec![0x4e, 0x71]);
        assert!(run_starts(&p).contains(&0x1068));
        // Fall-through from an executed instruction covers it.
        p.insert(0x1066, vec![0x4e, 0x71]);
        assert!(!run_starts(&p).contains(&0x1068));
    }

    #[test]
    fn a_loop_entered_only_by_a_computed_jump_gets_one_start() {
        let mut p = program();
        // $1080: subq.l #1, d1; bne.s $1080; rts. Its back-branch reaches its head, so no
        // instruction of it is a local start, yet nothing outside it reaches it.
        p.insert(0x1080, vec![0x53, 0x81]);
        p.insert(0x1082, vec![0x66, 0xfc]);
        p.insert(0x1084, vec![0x4e, 0x75]);
        assert!(!local_starts(&p).contains(&0x1080));
        let starts: Vec<u32> = run_starts(&p).into_iter().collect();
        assert_eq!(starts, vec![0x1000, 0x1040, 0x1060, 0x1080]);
        let e = entries(&p, &BTreeSet::new(), &BTreeSet::new(), 0x1000);
        assert_eq!(e.get(&0x1080), Some(&Source::Trace));
        // A symbol on the loop covers it.
        let e = entries(&p, &[0x1080].into(), &BTreeSet::new(), 0x1000);
        assert_eq!(e.get(&0x1080), Some(&Source::Symbol));
    }

    #[test]
    fn entries_name_their_source() {
        let funcs: BTreeSet<u32> = [0x1000, 0x1020, 0x1060, 0x2000].into();
        let vectors: BTreeSet<u32> = [0x1060, 0x1040].into();
        let e = entries(&program(), &funcs, &vectors, 0x1000);
        let got: Vec<(u32, &str)> = e.iter().map(|(&a, s)| (a, s.name())).collect();
        // 0x1020, a symbol reached by a direct call, is listed; 0x2000, never run, is not.
        assert_eq!(
            got,
            vec![
                (0x1000, "program-entry"),
                (0x1020, "symbol"),
                (0x1040, "vector"),
                (0x1060, "symbol"),
            ]
        );
    }

    #[test]
    fn render_orders_by_hunk_and_offset() {
        // Hunk 1 sits below hunk 0 in memory.
        let hunks = [(0x2000, 0x100), (0x1000, 0x100)];
        let e: BTreeMap<u32, Source> =
            [(0x1004, Source::Trace), (0x2000, Source::ProgramEntry)].into();
        let t = render(&e, &hunks);
        let at: Vec<&str> = t.lines().filter(|l| l.starts_with("at = ")).collect();
        assert_eq!(at, vec![r#"at = "h0+$0""#, r#"at = "h1+$4""#]);
        assert!(t.contains(
            "source = \"program-entry\"\n\n[[entry]]\nat = \"h1+$4\"\nsource = \"trace\"\n"
        ));
    }

    #[test]
    fn symbols_parse_hunk_offset_name_and_kind() {
        let t = "# c\n[[sym]]\nexe = \"x\"\nhunk = 0\noffset = 0x1eda\nname = \"h\"\nkind = \"func\"\nnote = \"a = b\"\n\n[[sym]]\nhunk = 4\noffset = 16\nname = \"v\"\nkind = \"var\"\nsize = 2\n";
        let s = parse_symbols(t).unwrap();
        assert_eq!(
            s,
            vec![
                Sym {
                    hunk: 0,
                    offset: 0x1eda,
                    name: "h".into(),
                    kind: "func".into()
                },
                Sym {
                    hunk: 4,
                    offset: 16,
                    name: "v".into(),
                    kind: "var".into()
                },
            ]
        );
        assert!(parse_symbols("[[sym]]\nhunk = 0\n").is_err());
    }

    #[test]
    fn the_real_symbols_file_parses() {
        let s = parse_symbols(&std::fs::read_to_string(re_dir().join("symbols.toml")).unwrap())
            .unwrap();
        assert!(
            s.iter()
                .any(|s| s.name == "level3_handler" && s.kind == "func")
        );
        assert!(funcs(&s).contains(&h(0, 0x1eda)));
    }
}
