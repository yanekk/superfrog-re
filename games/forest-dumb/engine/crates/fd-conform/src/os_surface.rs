//! Where the program leans on the Amiga's OS, measured on the original (plan forest-dumb-translator
//! DESIGN §2.8, T01), so the translated program's machine answers exactly that and guesses nothing.
//!
//! The original runs from power-on through the title, Start, level 1's load and its first frames
//! of play, and, for each later world asked for, from just before Start with that world chosen,
//! through its load. Every even address of the program's hunks is watched (`Amiga::watch_pcs`),
//! and so is every slot of every library, device and resource jump table in ExecBase's lists.
//! From the program's executed instructions, decoded here ([`decode`]):
//!
//! - **Library calls**: a `jsr`/`jmp` through a negative displacement off an address register.
//!   Which library is not in the instruction (the program calls dos.library through `a5`), so
//!   it is measured: a library stays a candidate for a site only while, in every step the site
//!   ran `n` times, that library's slot at the same offset ran at least `n` times.
//! - **OS memory**: absolute addresses outside the hunks, the custom chips and the CIAs (`$4`, the
//!   exception vectors), and reads off a pointer the OS handed over (ExecBase, a library base, a
//!   call's result), followed along the straight line of executed code after it.
//! - **CIA accesses** by absolute address, or a long immediate pointing at a CIA register.
//! - **Interrupts** taken (`Amiga::irqs`), each given to the program when its autovector points
//!   into the hunks and the handler there ran, else to the OS.
//! - **Other exits**: every executed `jsr`/`jmp` whose target is outside the hunks or not known
//!   statically and is not a library call.
//!
//! Each step is given a phase ([`Phase`]) by the program code that ran in it.
//!
//! What it cannot see is [`MISSES`]; the doc is `games/forest-dumb/docs/15-os-surface.md`.

pub mod decode;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use amiga_conform::script;
use amiga_oracle::{Amiga, AmigaMemory, Cpu68k, OracleConfig, kickstart_path, read_only};

use crate::routine::{PROGRAM, load_program};
use crate::{HUNK_BASES, disks, sync};
use decode::{Access, Ea, Flow, Insn};

/// What the survey cannot see, written into the file.
pub const MISSES: &str = "OS code reached by a return (rts/rte) rather than a jsr/jmp; an OS pointer \
     kept anywhere but a register or an absolute long variable; paths the run does not take (game \
     over, the end screen, worlds not asked for, a failed open, keys other than one ignored key in \
     play)";

/// The frames of play after the level sync point, by default.
pub const DEFAULT_FRAMES: u32 = 300;

/// The step before which a later world's run starts: the idle title, before `boot-to-level1`'s
/// Start at 5000.
pub const WORLD_FORK: u32 = 4990;

/// Program code the phases are told by (`re/symbols.toml`).
const ENTRY: u32 = HUNK_BASES[0];
const SYSTEM_ON: u32 = HUNK_BASES[0] + 0x3cc;
const SYSTEM_OFF: u32 = HUNK_BASES[0] + 0x440;
const GAME_FRAME: u32 = HUNK_BASES[0] + 0x1fec;
const TITLE_PASS: u32 = HUNK_BASES[0] + 0x6e6e;
const FILE_ROUTINES: [u32; 3] = [
    HUNK_BASES[0] + 0x8ec4, // load_file
    HUNK_BASES[0] + 0x8f58, // load_file_raw
    HUNK_BASES[0] + 0x8fd8, // read_marker
];
/// `title_code_world`'s first `move.l #$0, world_index`, run when the code digits hold their
/// start value `$50506` (measured: `boot-to-level1` enters no code). Its immediate is what a later
/// world's run changes: `world_index` itself cannot be poked between this and `load_world`,
/// which run in the same step.
const NO_CODE_WORLD: u32 = HUNK_BASES[0] + 0x778c;
/// `load_list_world_1` to `_6`, in hunk 4.
const WORLD_LISTS: [u32; 6] = [
    HUNK_BASES[4] + 0x1c29c,
    HUNK_BASES[4] + 0x1a260,
    HUNK_BASES[4] + 0x1a8d4,
    HUNK_BASES[4] + 0x1af46,
    HUNK_BASES[4] + 0x1b5b8,
    HUNK_BASES[4] + 0x1bc2a,
];

/// Where the Kickstart is mapped.
const ROM_BASE: u32 = 0xf8_0000;

/// A key the game ignores (L, `$28`), held for [`KEY_FRAMES`] frames from [`KEY_AT`] frames into
/// play, so the program's own keyboard handler (`level2_handler`) runs and its CIA accesses show.
const KEY: &str = "0: K28\n";
const KEY_AT: u32 = 10;
const KEY_FRAMES: u32 = 10;

/// The custom chips, which are not the OS.
const CUSTOM: std::ops::Range<u32> = 0xdf_f000..0xe0_0000;
/// Both CIAs' register space.
const CIA: std::ops::Range<u32> = 0xbf_d000..0xbf_f000;

/// What the program is doing in a step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Phase {
    /// None of the others: start-up before the first load, the SCENA screen, the prompts.
    Other,
    /// `system_on`, `system_off` or a file routine ran, or the OS holds the machine outside the
    /// title and play.
    Load,
    /// `title_pass` ran.
    Title,
    /// `game_frame` ran.
    Play,
}

impl Phase {
    fn name(self) -> &'static str {
        match self {
            Phase::Other => "other",
            Phase::Load => "load",
            Phase::Title => "title",
            Phase::Play => "play",
        }
    }
}

/// A library, device or resource in ExecBase's lists: its name, base and jump table size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lib {
    pub name: String,
    pub base: u32,
    /// `lib_NegSize`: the jump table is the `neg / 6` slots below `base`.
    pub neg: u16,
}

impl Lib {
    /// Every slot's address.
    pub fn slots(&self) -> impl Iterator<Item = u32> + '_ {
        (1..=self.neg as u32 / 6).map(move |k| self.base.wrapping_sub(6 * k))
    }
}

/// ExecBase's library, device and resource lists, read through `peek`. A broken list stops at 64
/// nodes or a null.
pub fn read_libs(peek: &dyn Fn(u32, usize) -> Vec<u8>) -> Vec<Lib> {
    let long = |a: u32| u32::from_be_bytes(peek(a, 4).try_into().unwrap_or([0; 4]));
    let word = |a: u32| u16::from_be_bytes(peek(a, 2).try_into().unwrap_or([0; 2]));
    let exec = long(4);
    let mut out = Vec::new();
    if exec == 0 {
        return out;
    }
    // ResourceList, DeviceList, LibList.
    for list in [0x150, 0x15e, 0x17a] {
        let mut node = long(exec + list);
        for _ in 0..64 {
            let succ = long(node);
            if node == 0 || succ == 0 {
                break;
            }
            let name = c_string(peek, long(node + 10));
            out.push(Lib {
                name,
                base: node,
                neg: word(node + 16),
            });
            node = succ;
        }
    }
    out.sort_by_key(|l| l.base);
    out
}

fn c_string(peek: &dyn Fn(u32, usize) -> Vec<u8>, at: u32) -> String {
    if at == 0 {
        return String::new();
    }
    peek(at, 64)
        .into_iter()
        .take_while(|&b| b != 0)
        .map(|b| {
            if b.is_ascii_graphic() || b == b' ' {
                b as char
            } else {
                '?'
            }
        })
        .collect()
}

/// One step as the survey sees it.
pub struct Observation<'a> {
    pub phase: Phase,
    /// `(pc, count)` for the step, program addresses and jump table slots alike; zero counts
    /// allowed.
    pub hits: &'a [(u32, u32)],
    /// The tables the slots were watched for.
    pub libs: &'a [Lib],
    /// The level of each interrupt taken.
    pub irqs: &'a [u8],
    /// Each autovector (levels 1 to 7) before and after the step.
    pub vectors: [(u32, u32); 7],
    /// Reads the machine's memory, for decoding an instruction the first time it runs.
    pub peek: &'a dyn Fn(u32, usize) -> Vec<u8>,
}

/// One executed instruction.
struct Seen {
    insn: Option<Insn>,
    /// The C string a `lea abs.l,a1` points at, for naming the library `OpenLibrary` opens.
    string: Option<String>,
    counts: BTreeMap<Phase, u64>,
}

/// The survey in progress: fed one [`Observation`] a step, read out as a [`Report`].
pub struct Survey {
    hunks: Vec<(u32, u32)>,
    /// Decodes lengths with Moira, to check [`decode::decode`].
    cpu: Cpu68k,
    seen: BTreeMap<u32, Seen>,
    /// A library call site's candidate libraries, by name; absent until it first runs.
    cands: BTreeMap<u32, BTreeSet<String>>,
    /// `(level, owner, phase)` to count.
    irqs: BTreeMap<(u8, &'static str, Phase), u64>,
    /// `(level, owner, phase)` to the steps that autovector ended pointing there.
    vectors: BTreeMap<(u8, &'static str, Phase), u64>,
    steps: BTreeMap<Phase, u64>,
}

impl Survey {
    /// A survey of a program whose hunks are at `hunks` (`(base, size)`).
    pub fn new(hunks: Vec<(u32, u32)>) -> Survey {
        Survey {
            hunks,
            cpu: Cpu68k::new(AmigaMemory {
                chip: vec![],
                slow: vec![],
            }),
            seen: BTreeMap::new(),
            cands: BTreeMap::new(),
            irqs: BTreeMap::new(),
            vectors: BTreeMap::new(),
            steps: BTreeMap::new(),
        }
    }

    fn in_hunks(&self, a: u32) -> bool {
        self.hunks.iter().any(|&(b, s)| a >= b && a < b + s)
    }

    /// `hN+$off`, or `$addr` outside the hunks.
    pub fn name(&self, a: u32) -> String {
        match self.hunks.iter().position(|&(b, s)| a >= b && a < b + s) {
            Some(n) => format!("h{n}+${:x}", a - self.hunks[n].0),
            None => format!("${a:x}"),
        }
    }

    /// Every address watched for the program: each even address of each hunk.
    pub fn program_pcs(&self) -> Vec<u32> {
        self.hunks
            .iter()
            .flat_map(|&(b, s)| (b..b + s).step_by(2))
            .collect()
    }

    fn decode_at(&mut self, pc: u32, peek: &dyn Fn(u32, usize) -> Vec<u8>) -> Seen {
        let head = peek(pc, 10);
        self.cpu.write(pc, &head);
        let moira = self.cpu.insn_len(pc);
        let insn = decode::decode(&head, pc).filter(|i| i.len == moira);
        let string = match &insn {
            Some(i) if matches!(i.ops[..], [(Ea::AbsL(_), Access::Address), (Ea::A(1), _)]) => {
                Some(c_string(peek, i.ops[0].0.absolute().unwrap_or(0)))
            }
            _ => None,
        };
        Seen {
            insn,
            string,
            counts: BTreeMap::new(),
        }
    }

    /// Add one step.
    pub fn observe(&mut self, o: &Observation) {
        *self.steps.entry(o.phase).or_default() += 1;
        for (k, &(_, after)) in o.vectors.iter().enumerate() {
            let owner = if self.in_hunks(after) {
                "program"
            } else {
                "os"
            };
            *self
                .vectors
                .entry((k as u8 + 1, owner, o.phase))
                .or_default() += 1;
        }
        let mut slots: BTreeMap<u32, u32> = BTreeMap::new();
        let mut program: Vec<(u32, u32)> = Vec::new();
        for &(pc, n) in o.hits {
            if n == 0 {
                continue;
            }
            if self.in_hunks(pc) {
                program.push((pc, n));
            } else {
                slots.insert(pc, n);
            }
        }
        for &(pc, n) in &program {
            if !self.seen.contains_key(&pc) {
                let s = self.decode_at(pc, o.peek);
                self.seen.insert(pc, s);
            }
            let s = self.seen.get_mut(&pc).unwrap();
            *s.counts.entry(o.phase).or_default() += n as u64;
            let Some(d) = library_offset(s.insn.as_ref()) else {
                continue;
            };
            let ok: BTreeSet<String> = o
                .libs
                .iter()
                .filter(|l| d.unsigned_abs() as u32 <= l.neg as u32)
                .filter(|l| {
                    slots
                        .get(&l.base.wrapping_add(d as i32 as u32))
                        .copied()
                        .unwrap_or(0)
                        >= n
                })
                .map(|l| l.name.clone())
                .collect();
            let c = self.cands.entry(pc).or_insert_with(|| ok.clone());
            c.retain(|x| ok.contains(x));
        }
        let hits = |pc: u32| -> u64 {
            program
                .binary_search_by_key(&pc, |&(p, _)| p)
                .map_or(0, |i| program[i].1 as u64)
        };
        let mut levels: BTreeMap<u8, u64> = BTreeMap::new();
        for &l in o.irqs {
            *levels.entry(l).or_default() += 1;
        }
        for (&level, &n) in &levels {
            let (before, after) = o.vectors[(level as usize).clamp(1, 7) - 1];
            let mut handlers: Vec<u32> = [before, after]
                .into_iter()
                .filter(|&v| self.in_hunks(v))
                .collect();
            handlers.dedup();
            let ran: u64 = handlers.iter().map(|&h| hits(h)).sum();
            let program = n.min(ran);
            if program > 0 {
                *self.irqs.entry((level, "program", o.phase)).or_default() += program;
            }
            if n > program {
                *self.irqs.entry((level, "os", o.phase)).or_default() += n - program;
            }
        }
    }

    /// The survey so far.
    pub fn report(&self) -> Report {
        let mut r = Report {
            steps: self.steps.clone(),
            vectors: self.vectors.clone(),
            executed: self.seen.len() as u64,
            ..Report::default()
        };
        let followed = self.follow();
        let total = |s: &Seen| s.counts.values().sum::<u64>();
        let phases = |s: &Seen| s.counts.keys().copied().collect::<BTreeSet<Phase>>();
        for (&pc, s) in &self.seen {
            let Some(insn) = &s.insn else {
                r.undecoded.push(self.name(pc));
                continue;
            };
            let site = Site {
                at: self.name(pc),
                count: total(s),
                when: phases(s),
            };
            if let Some(d) = library_offset(Some(insn)) {
                let base = match insn.ops[0].0 {
                    Ea::Disp(n, _) => format!("a{n}"),
                    _ => unreachable!(),
                };
                let libs = self.cands.get(&pc).cloned().unwrap_or_default();
                if libs.is_empty() {
                    r.add_jump("no library vector", site);
                } else {
                    // Several left and none followed: all of them, so the file shows the doubt.
                    let lib = followed
                        .libs
                        .get(&pc)
                        .cloned()
                        .unwrap_or_else(|| libs.into_iter().collect::<Vec<_>>().join("|"));
                    r.add_call(&lib, d, &base, site);
                }
                continue;
            }
            for &(ea, acc) in &insn.ops {
                if acc == Access::Control {
                    let kind = match ea {
                        Ea::AbsW(a) | Ea::AbsL(a) | Ea::Pc(a) if self.in_hunks(a) => continue,
                        Ea::AbsW(_) | Ea::AbsL(_) | Ea::Pc(_) => "absolute outside the hunks",
                        _ => "register-indirect",
                    };
                    r.add_jump(kind, site.clone());
                    continue;
                }
                let a = match (ea, insn.size) {
                    (Ea::AbsW(a) | Ea::AbsL(a), _) => a,
                    (Ea::Imm(a), 4) if CIA.contains(&a) => {
                        r.add_cia(a, Access::Address, site.clone());
                        continue;
                    }
                    _ => continue,
                };
                if self.in_hunks(a) || CUSTOM.contains(&a) {
                    continue;
                }
                if CIA.contains(&a) {
                    r.add_cia(a, acc, site.clone());
                } else {
                    r.add_mem(os_memory(a), &format!("${a:x}"), acc, site.clone());
                }
            }
        }
        for (what, off, acc, site) in followed.reads {
            r.add_mem(what, &off, acc, site);
        }
        for (&(level, owner, phase), &count) in &self.irqs {
            r.irqs.push(IrqUse {
                level,
                owner,
                when: phase,
                count,
            });
        }
        r
    }

    /// Which library each call site calls: the one candidate the counts left, or, where several
    /// are left (exec's own slots at the same offsets run more often than the program's call), the
    /// library whose base the straight line put in the base register.
    fn resolve(&self, pc: u32, held: Option<&String>) -> Option<String> {
        let c = self.cands.get(&pc)?;
        if c.len() == 1 {
            return c.iter().next().cloned();
        }
        let lib = match held?.as_str() {
            "ExecBase" => "exec.library",
            h => h.strip_suffix(" base")?,
        };
        c.contains(lib).then(|| lib.to_string())
    }

    /// Reads off a pointer the OS handed over, and each call site's library, along each straight
    /// line of executed code: from `movea.l $4,An` (ExecBase) or a library call (its base register
    /// keeps the library's base; `d0` is its result), through long register-to-register moves and
    /// long stores to and loads from an absolute address, until the register is written otherwise
    /// or the line ends. Two sweeps, so a store later in address order than its load is seen.
    fn follow(&self) -> Followed {
        let mut out = Followed::default();
        for _ in 0..2 {
            let vars = std::mem::take(&mut out.vars);
            out = Followed::default();
            self.sweep(&vars, &mut out);
        }
        out
    }

    fn sweep(&self, vars: &BTreeMap<u32, String>, out: &mut Followed) {
        // Register n is dn, 8 + n is an.
        let mut held: [Option<String>; 16] = Default::default();
        let mut string: Option<String> = None;
        let mut next = 0u32;
        let reg = |ea: Ea| match ea {
            Ea::D(n) => Some(n as usize),
            Ea::A(n) => Some(8 + n as usize),
            _ => None,
        };
        for (&pc, s) in &self.seen {
            if pc != next {
                held = Default::default();
                string = None;
            }
            let Some(insn) = &s.insn else {
                next = 0;
                continue;
            };
            next = pc + insn.len;
            let site = Site {
                at: self.name(pc),
                count: s.counts.values().sum(),
                when: s.counts.keys().copied().collect(),
            };
            if let Some(d) = library_offset(Some(insn)) {
                let base = insn.ops[0].0.through().unwrap() as usize;
                let lib = self.resolve(pc, held[8 + base].as_ref());
                let result = match &lib {
                    Some(l) if l == "exec.library" && (d == -408 || d == -552) => {
                        string.clone().map(|s| format!("{s} base"))
                    }
                    Some(l) => Some(format!("{l} {} result", lvo_name(l, d))),
                    None => None,
                };
                if let Some(l) = lib {
                    out.libs.insert(pc, l);
                }
                let kept = held[8 + base].clone();
                // d0, d1, a0 and a1 are scratch across a library call.
                for k in [0, 1, 8, 9] {
                    held[k] = None;
                }
                held[8 + base] = kept;
                held[0] = result;
                continue;
            }
            for &(ea, acc) in &insn.ops {
                if let Some(n) = ea.through()
                    && let Some(what) = &held[8 + n as usize]
                    && matches!(acc, Access::Read | Access::Modify | Access::Write)
                {
                    let off = match ea {
                        Ea::Disp(_, d) if d < 0 => format!("-${:x}", -(d as i32)),
                        Ea::Disp(_, d) => format!("+${d:x}"),
                        Ea::Idx(_) => "+index".to_string(),
                        _ => "+$0".to_string(),
                    };
                    out.reads.push((what.clone(), off, acc, site.clone()));
                }
            }
            // Then what this instruction leaves in the registers.
            let mut moved: Option<(usize, Option<String>)> = None;
            if insn.size == 4
                && let [(src, Access::Read), (dst, Access::Write)] = insn.ops[..]
            {
                match (reg(src), reg(dst), src.absolute(), dst.absolute()) {
                    (Some(from), Some(to), _, _) => moved = Some((to, held[from].clone())),
                    (Some(from), None, _, Some(addr)) => {
                        if let Some(h) = &held[from] {
                            out.vars.insert(addr, h.clone());
                        }
                    }
                    (None, Some(to), Some(4), _) => moved = Some((to, Some("ExecBase".into()))),
                    (None, Some(to), Some(addr), _) => moved = Some((to, vars.get(&addr).cloned())),
                    _ => {}
                }
            }
            for &(ea, acc) in &insn.ops {
                if let Some(k) = reg(ea)
                    && matches!(acc, Access::Write | Access::Modify)
                {
                    held[k] = None;
                }
            }
            for (k, h) in held.iter_mut().enumerate() {
                if insn.clobbers & (1 << k) != 0 {
                    *h = None;
                }
            }
            if let Some((to, what)) = moved {
                held[to] = what;
            }
            if s.string.is_some() {
                string = s.string.clone();
            }
            if matches!(insn.flow, Flow::Jump | Flow::End | Flow::Call) {
                next = 0;
            }
        }
    }
}

/// What [`Survey::follow`] found.
#[derive(Default)]
struct Followed {
    /// Each library call site's library.
    libs: BTreeMap<u32, String>,
    /// `(what, offset, access, site)` for each read or write off a held pointer.
    reads: Vec<(String, String, Access, Site)>,
    /// Program variables a held pointer was stored to.
    vars: BTreeMap<u32, String>,
}

/// The displacement of a library call: `jsr`/`jmp` through a negative 16-bit displacement off an
/// address register.
fn library_offset(insn: Option<&Insn>) -> Option<i16> {
    match insn?.ops[..] {
        [(Ea::Disp(_, d), Access::Control)] if d < 0 => Some(d),
        _ => None,
    }
}

/// What OS memory an absolute address is.
fn os_memory(a: u32) -> String {
    match a {
        4 => "ExecBase".to_string(),
        0x64..=0x7c if a.is_multiple_of(4) => format!("level {} autovector", (a - 0x60) / 4),
        0x08..=0xbc if a.is_multiple_of(4) => "exception vector".to_string(),
        0xf8_0000.. => "ROM".to_string(),
        _ => "OS memory".to_string(),
    }
}

/// A site and how often it ran.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Site {
    pub at: String,
    pub count: u64,
    pub when: BTreeSet<Phase>,
}

/// Sites grouped under one key.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Group {
    pub sites: Vec<String>,
    pub count: u64,
    pub when: BTreeSet<Phase>,
}

impl Group {
    fn add(&mut self, s: Site) {
        if !self.sites.contains(&s.at) {
            self.sites.push(s.at);
            self.count += s.count;
        }
        self.when.extend(s.when);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrqUse {
    pub level: u8,
    pub owner: &'static str,
    pub when: Phase,
    pub count: u64,
}

/// The survey's result.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// Steps observed in each phase.
    pub steps: BTreeMap<Phase, u64>,
    /// `(level, owner, phase)` to the steps each autovector ended pointing at that owner.
    pub vectors: BTreeMap<(u8, &'static str, Phase), u64>,
    /// Distinct instructions executed.
    pub executed: u64,
    /// `(library, lvo, base register)`.
    pub calls: BTreeMap<(String, i16, String), Group>,
    /// `(what, address or offset, access)`; access is `read`, `write`, `modify` or `address`.
    pub mem: BTreeMap<(String, String, &'static str), Group>,
    /// `(register, access)`.
    pub cia: BTreeMap<(String, &'static str), Group>,
    /// `kind` to sites.
    pub jumps: BTreeMap<&'static str, Group>,
    pub irqs: Vec<IrqUse>,
    /// Executed instructions the decoder does not know, or whose length Moira disagrees with.
    pub undecoded: Vec<String>,
}

fn access_name(a: Access) -> &'static str {
    match a {
        Access::Read => "read",
        Access::Write => "write",
        Access::Modify => "modify",
        Access::Address => "address",
        Access::Control => "control",
    }
}

impl Report {
    fn add_call(&mut self, lib: &str, lvo: i16, base: &str, s: Site) {
        self.calls
            .entry((lib.to_string(), lvo, base.to_string()))
            .or_default()
            .add(s);
    }

    fn add_mem(&mut self, what: String, addr: &str, acc: Access, s: Site) {
        self.mem
            .entry((what, addr.to_string(), access_name(acc)))
            .or_default()
            .add(s);
    }

    fn add_cia(&mut self, a: u32, acc: Access, s: Site) {
        self.cia
            .entry((cia_name(a), access_name(acc)))
            .or_default()
            .add(s);
    }

    fn add_jump(&mut self, kind: &'static str, s: Site) {
        self.jumps.entry(kind).or_default().add(s);
    }

    /// The calls named `name`, as `(library, sites)`.
    pub fn calls_named(&self, name: &str) -> Vec<(String, Vec<String>)> {
        self.calls
            .iter()
            .filter(|((l, d, _), _)| lvo_name(l, *d) == name)
            .map(|((l, _, _), g)| (l.clone(), g.sites.clone()))
            .collect()
    }

    /// `re/os-calls.toml`'s text: sorted and stable, two runs of the same original give the same
    /// bytes.
    pub fn toml(&self, frames: u32, worlds: &[u32]) -> String {
        fn list<T: AsRef<str>>(xs: impl IntoIterator<Item = T>) -> String {
            let v: Vec<String> = xs
                .into_iter()
                .map(|x| format!("\"{}\"", x.as_ref()))
                .collect();
            format!("[{}]", v.join(", "))
        }
        let when = |w: &BTreeSet<Phase>| list(w.iter().map(|p| p.name()));
        let mut t = String::new();
        t.push_str(
            "# Where FOREST_DUMB_FOREVER leans on the OS, measured on the original by\n\
             # `fd-conform os-surface` (plans/forest-dumb-translator T01). Generated: do not edit.\n\
             # Read with games/forest-dumb/docs/15-os-surface.md. Sites are hN+$off; counts are\n\
             # executions over the run; `when` is load, title, play or other.\n\n",
        );
        let worlds: Vec<String> = worlds.iter().map(u32::to_string).collect();
        let steps: Vec<String> = self
            .steps
            .iter()
            .map(|(p, n)| format!("{} = {n}", p.name()))
            .collect();
        t.push_str(&format!(
            "[run]\nframes = {frames}\nworlds = [{}]\nsteps = {{ {} }}\nexecuted = {}\nmisses = \"{MISSES}\"\n",
            worlds.join(", "),
            steps.join(", "),
            self.executed
        ));
        for ((lib, lvo, base), g) in &self.calls {
            t.push_str(&format!(
                "\n[[call]]\nlibrary = \"{lib}\"\nlvo = {lvo}\nname = \"{}\"\nbase = \"{base}\"\nsites = {}\ncount = {}\nwhen = {}\n",
                lvo_name(lib, *lvo),
                list(&g.sites),
                g.count,
                when(&g.when)
            ));
        }
        for ((what, addr, acc), g) in &self.mem {
            let table = if *acc == "read" || *acc == "address" {
                "read"
            } else {
                "write"
            };
            t.push_str(&format!(
                "\n[[{table}]]\nwhat = \"{what}\"\naddr = \"{addr}\"\naccess = \"{acc}\"\nsites = {}\ncount = {}\nwhen = {}\n",
                list(&g.sites),
                g.count,
                when(&g.when)
            ));
        }
        for i in &self.irqs {
            t.push_str(&format!(
                "\n[[irq]]\nlevel = {}\nowner = \"{}\"\nwhen = \"{}\"\ncount = {}\n",
                i.level,
                i.owner,
                i.when.name(),
                i.count
            ));
        }
        for (&(level, owner, phase), &n) in &self.vectors {
            t.push_str(&format!(
                "\n[[vector]]\nlevel = {level}\nowner = \"{owner}\"\nwhen = \"{}\"\nsteps = {n}\n",
                phase.name()
            ));
        }
        for ((reg, acc), g) in &self.cia {
            t.push_str(&format!(
                "\n[[cia]]\nreg = \"{reg}\"\naccess = \"{acc}\"\nsites = {}\ncount = {}\nwhen = {}\n",
                list(&g.sites),
                g.count,
                when(&g.when)
            ));
        }
        for (kind, g) in &self.jumps {
            t.push_str(&format!(
                "\n[[jump]]\nkind = \"{kind}\"\nsites = {}\ncount = {}\nwhen = {}\n",
                list(&g.sites),
                g.count,
                when(&g.when)
            ));
        }
        if !self.undecoded.is_empty() {
            t.push_str(&format!(
                "\n[undecoded]\nsites = {}\n",
                list(&self.undecoded)
            ));
        }
        t
    }
}

/// `ciaa_pra` for `$bfe001`, `ciab_prb` for `$bfd100`; an address off the register grid by its
/// address.
pub fn cia_name(a: u32) -> String {
    const REGS: [&str; 16] = [
        "pra", "prb", "ddra", "ddrb", "talo", "tahi", "tblo", "tbhi", "todlow", "todmid", "todhi",
        "unused", "sdr", "icr", "cra", "crb",
    ];
    let (chip, odd) = match a & 0xf000 {
        0xe000 => ("ciaa", 1),
        _ => ("ciab", 0),
    };
    if a & 0xff != odd {
        return format!("${a:x}");
    }
    format!("{chip}_{}", REGS[((a >> 8) & 0xf) as usize])
}

/// The function at `lvo` in `lib` (Kickstart 1.3's tables), or `lvo-N` for one not listed.
pub fn lvo_name(lib: &str, lvo: i16) -> String {
    const STANDARD: [&str; 4] = ["Open", "Close", "Expunge", "Reserved"];
    const EXEC: [&str; 88] = [
        "Supervisor",
        "ExitIntr",
        "Schedule",
        "Reschedule",
        "Switch",
        "Dispatch",
        "Exception",
        "InitCode",
        "InitStruct",
        "MakeLibrary",
        "MakeFunctions",
        "FindResident",
        "InitResident",
        "Alert",
        "Debug",
        "Disable",
        "Enable",
        "Forbid",
        "Permit",
        "SetSR",
        "SuperState",
        "UserState",
        "SetIntVector",
        "AddIntServer",
        "RemIntServer",
        "Cause",
        "Allocate",
        "Deallocate",
        "AllocMem",
        "AllocAbs",
        "FreeMem",
        "AvailMem",
        "AllocEntry",
        "FreeEntry",
        "Insert",
        "AddHead",
        "AddTail",
        "Remove",
        "RemHead",
        "RemTail",
        "Enqueue",
        "FindName",
        "AddTask",
        "RemTask",
        "FindTask",
        "SetTaskPri",
        "SetSignal",
        "SetExcept",
        "Wait",
        "Signal",
        "AllocSignal",
        "FreeSignal",
        "AllocTrap",
        "FreeTrap",
        "AddPort",
        "RemPort",
        "PutMsg",
        "GetMsg",
        "ReplyMsg",
        "WaitPort",
        "FindPort",
        "AddLibrary",
        "RemLibrary",
        "OldOpenLibrary",
        "CloseLibrary",
        "SetFunction",
        "SumLibrary",
        "AddDevice",
        "RemDevice",
        "OpenDevice",
        "CloseDevice",
        "DoIO",
        "SendIO",
        "CheckIO",
        "WaitIO",
        "AbortIO",
        "AddResource",
        "RemResource",
        "OpenResource",
        "RawIOInit",
        "RawMayGetChar",
        "RawPutChar",
        "RawDoFmt",
        "GetCC",
        "TypeOfMem",
        "Procure",
        "Vacate",
        "OpenLibrary",
    ];
    const DOS: [&str; 33] = [
        "Open",
        "Close",
        "Read",
        "Write",
        "Input",
        "Output",
        "Seek",
        "DeleteFile",
        "Rename",
        "Lock",
        "UnLock",
        "DupLock",
        "Examine",
        "ExNext",
        "Info",
        "CreateDir",
        "CurrentDir",
        "IoErr",
        "CreateProc",
        "Exit",
        "LoadSeg",
        "UnLoadSeg",
        "GetPacket",
        "QueuePacket",
        "DeviceProc",
        "SetComment",
        "SetProtection",
        "DateStamp",
        "Delay",
        "WaitForChar",
        "ParentDir",
        "IsInteractive",
        "Execute",
    ];
    const GRAPHICS: [&str; 73] = [
        "BltBitMap",
        "BltTemplate",
        "ClearEOL",
        "ClearScreen",
        "TextLength",
        "Text",
        "SetFont",
        "OpenFont",
        "CloseFont",
        "AskSoftStyle",
        "SetSoftStyle",
        "AddBob",
        "AddVSprite",
        "DoCollision",
        "DrawGList",
        "InitGels",
        "InitMasks",
        "RemIBob",
        "RemVSprite",
        "SetCollision",
        "SortGList",
        "AddAnimOb",
        "Animate",
        "GetGBuffers",
        "InitGMasks",
        "GelsFuncE",
        "GelsFuncF",
        "LoadRGB4",
        "InitRastPort",
        "InitVPort",
        "MrgCop",
        "MakeVPort",
        "LoadView",
        "WaitBlit",
        "SetRast",
        "Move",
        "Draw",
        "AreaMove",
        "AreaDraw",
        "AreaEnd",
        "WaitTOF",
        "QBlit",
        "InitArea",
        "SetRGB4",
        "QBSBlit",
        "BltClear",
        "RectFill",
        "BltPattern",
        "ReadPixel",
        "WritePixel",
        "Flood",
        "PolyDraw",
        "SetAPen",
        "SetBPen",
        "SetDrMd",
        "InitView",
        "CBump",
        "CMove",
        "CWait",
        "VBeamPos",
        "InitBitMap",
        "ScrollRaster",
        "WaitBOVP",
        "GetSprite",
        "FreeSprite",
        "ChangeSprite",
        "MoveSprite",
        "LockLayerRom",
        "UnlockLayerRom",
        "SyncSBitMap",
        "CopySBitMap",
        "OwnBlitter",
        "DisownBlitter",
    ];
    let k = -(lvo as i32);
    if k % 6 != 0 || k <= 0 {
        return format!("lvo{lvo}");
    }
    let i = (k / 6) as usize;
    let first = |t: &[&str]| t.get(i - 5).map(|s| s.to_string());
    let name = if i <= 4 {
        Some(STANDARD[i - 1].to_string())
    } else {
        match lib {
            "exec.library" => first(&EXEC),
            "dos.library" => first(&DOS),
            "graphics.library" => first(&GRAPHICS),
            _ => None,
        }
    };
    name.unwrap_or_else(|| format!("lvo{lvo}"))
}

/// `games/forest-dumb/re/os-calls.toml`, where the survey is committed.
pub fn committed_path() -> std::path::PathBuf {
    crate::conform_dir().join("../../re/os-calls.toml")
}

/// What to run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// Frames of play after the level sync point.
    pub frames: u32,
    /// The worlds whose load is run, 1 to 6; world 1 is the run from power-on.
    pub worlds: Vec<u32>,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            frames: DEFAULT_FRAMES,
            worlds: vec![1, 2],
        }
    }
}

/// [`run_in`] on the Kickstart and disks the oracle finds itself.
pub fn run(opts: &Options) -> Result<Report, String> {
    let kickstart = kickstart_path().map_err(|e| e.to_string())?;
    let disks = fd_tools::disk_dir()?;
    run_in(&kickstart, &disks, opts)
}

/// Where a step stands: whether the OS holds the machine (`system_on` ran, `system_off` not yet),
/// and whether the program has started.
#[derive(Debug, Clone, Copy, Default)]
struct State {
    started: bool,
    os_held: bool,
}

/// The step loop of one run: watch, step, observe.
struct Runner<'a> {
    am: Amiga,
    /// The Kickstart, for what ExecBase's lists point into ROM (`Amiga::peek` reads RAM only).
    rom: Vec<u8>,
    survey: &'a mut Survey,
    program: Vec<u32>,
    libs: Vec<Lib>,
    state: State,
}

impl Runner<'_> {
    fn watch(&mut self) {
        let mut pcs = self.program.clone();
        for l in &self.libs {
            pcs.extend(l.slots());
        }
        pcs.sort_unstable();
        pcs.dedup();
        self.am.watch_pcs(&pcs);
    }

    fn peek(am: &Amiga, rom: &[u8], a: u32, n: usize) -> Vec<u8> {
        (0..n as u32)
            .map(|k| {
                let a = a.wrapping_add(k) & 0xff_ffff;
                if a >= ROM_BASE && !rom.is_empty() {
                    // A 256 KB Kickstart is mirrored at $f80000 and $fc0000.
                    rom[(a - ROM_BASE) as usize % rom.len()]
                } else {
                    am.peek(a, 1)[0]
                }
            })
            .collect()
    }

    fn vectors(&self) -> [u32; 7] {
        let m = self.am.peek(0x64, 28);
        std::array::from_fn(|k| u32::from_be_bytes(m[4 * k..4 * k + 4].try_into().unwrap()))
    }

    /// One step on `input`; observed once the program's entry has run.
    fn step(&mut self, input: amiga_oracle::FrameInput) -> Result<(), String> {
        let before = self.vectors();
        self.am.step(input).map_err(|e| e.to_string())?;
        let hits = self.am.pc_hits();
        let ran = |pc: u32| {
            hits.binary_search_by_key(&pc, |&(p, _)| p)
                .is_ok_and(|i| hits[i].1 > 0)
        };
        if !self.state.started {
            self.state.started = ran(ENTRY);
            if !self.state.started {
                return Ok(());
            }
        }
        let (on, off) = (ran(SYSTEM_ON), ran(SYSTEM_OFF));
        // The title runs with the OS holding the machine (docs/07), so the title and play are
        // told by their own code first, and the OS's hold only names the steps between.
        let phase = if on || off || FILE_ROUTINES.iter().any(|&p| ran(p)) {
            Phase::Load
        } else if ran(GAME_FRAME) {
            Phase::Play
        } else if ran(TITLE_PASS) {
            Phase::Title
        } else if self.state.os_held {
            Phase::Load
        } else {
            Phase::Other
        };
        // A load list hands over, loads over many frames, and takes back: `system_off` is last.
        if off {
            self.state.os_held = false;
        } else if on {
            self.state.os_held = true;
        }
        let after = self.vectors();
        let irqs: Vec<u8> = self.am.irqs().iter().map(|i| i.level).collect();
        let (am, rom) = (&self.am, &self.rom);
        let peek = |a: u32, n: usize| Runner::peek(am, rom, a, n);
        let libs = read_libs(&peek);
        self.survey.observe(&Observation {
            phase,
            hits: am.pc_hits(),
            libs: &self.libs,
            irqs: &irqs,
            vectors: std::array::from_fn(|k| (before[k], after[k])),
            peek: &peek,
        });
        if libs != self.libs {
            self.libs = libs;
            self.watch();
        }
        Ok(())
    }
}

/// Survey the original on the Kickstart at `kickstart` and the zips in `disk_dir`.
pub fn run_in(kickstart: &Path, disk_dir: &Path, opts: &Options) -> Result<Report, String> {
    if opts.worlds.is_empty() || opts.worlds.iter().any(|w| !(1..=6).contains(w)) {
        return Err(format!("worlds {:?}: each must be 1 to 6", opts.worlds));
    }
    let rom = read_only(kickstart).map_err(|e| e.to_string())?;
    let images = disks::read(disk_dir)?;
    let (_, sc) = sync::level_1_script()?;
    let key = script::parse(KEY).map_err(|e| e.to_string())?.lines[0].1;
    let press = sync::LEVEL_1_SYNC + KEY_AT..sync::LEVEL_1_SYNC + KEY_AT + KEY_FRAMES;
    let input = |frame: u32| {
        if press.contains(&frame) {
            key
        } else {
            script::input_at(&sc.lines, frame)
        }
    };
    let exe = fd_tools::game_file(disk_dir, PROGRAM)?;
    let (_, hunks) = load_program(&exe)?;
    let end = sync::LEVEL_1_SYNC + opts.frames;
    let cfg = OracleConfig::new(kickstart.to_path_buf(), images, end + 1);
    let mut survey = Survey::new(hunks);
    let program = survey.program_pcs();
    let mut run = Runner {
        am: Amiga::power_on(&cfg).map_err(|e| e.to_string())?,
        rom,
        survey: &mut survey,
        program,
        libs: Vec::new(),
        state: State::default(),
    };
    run.watch();
    let mut fork = None;
    for frame in 0..end {
        if frame == sync::DISK_2_SWAP {
            run.am.insert_disk(0, 1);
        }
        if frame == WORLD_FORK {
            fork = Some((run.am.snapshot(), run.state, run.libs.clone()));
        }
        run.step(input(frame))?;
    }
    if !run.state.started {
        return Err("the program's entry h0+$0 never ran".into());
    }
    let (snap, state, libs) = fork.ok_or("the run ended before the fork")?;
    for &world in opts.worlds.iter().filter(|&&w| w != 1) {
        run.am.restore(&snap);
        run.state = state;
        run.libs = libs.clone();
        run.watch();
        // `move.l #$0, world_index`'s immediate, put back once it has run.
        let imm = NO_CODE_WORLD + 2;
        run.am.poke(imm, &(world - 1).to_be_bytes());
        let list = WORLD_LISTS[world as usize - 1];
        let mut patched = true;
        let mut listed = false;
        for frame in WORLD_FORK..end {
            run.step(input(frame))?;
            let hits = run.am.pc_hits();
            let ran = |pc: u32| {
                hits.binary_search_by_key(&pc, |&(p, _)| p)
                    .is_ok_and(|i| hits[i].1 > 0)
            };
            listed |= ran(list);
            if patched && ran(NO_CODE_WORLD) {
                run.am.poke(imm, &[0, 0, 0, 0]);
                patched = false;
            }
        }
        if !listed {
            return Err(format!("world {world}'s load list never ran"));
        }
    }
    drop(run);
    Ok(survey.report())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A made-up program at `$1000` and an exec-like library at `$3000` with a 60-byte table:
    /// `movea.l $4.w,a6; jsr -$1e(a6); movea.l d0,a1; move.l $26(a1),d1; rts`.
    const PROGRAM_AT: u32 = 0x1000;
    const CODE: [u8; 14] = [
        0x2c, 0x78, 0x00, 0x04, // movea.l $4.w,a6
        0x4e, 0xae, 0xff, 0xe2, // jsr -$1e(a6)
        0x22, 0x40, // movea.l d0,a1
        0x22, 0x29, 0x00, 0x26, // move.l $26(a1),d1
    ];

    fn mem() -> Vec<u8> {
        let mut m = vec![0u8; 0x4000];
        m[PROGRAM_AT as usize..PROGRAM_AT as usize + CODE.len()].copy_from_slice(&CODE);
        m[PROGRAM_AT as usize + CODE.len()..][..2].copy_from_slice(&[0x4e, 0x75]);
        m
    }

    fn libs() -> Vec<Lib> {
        vec![
            Lib {
                name: "fake.library".into(),
                base: 0x3000,
                neg: 60,
            },
            Lib {
                name: "other.library".into(),
                base: 0x3800,
                neg: 60,
            },
        ]
    }

    fn observe(s: &mut Survey, phase: Phase, hits: &[(u32, u32)], irqs: &[u8], v3: u32) {
        let m = mem();
        let peek = move |a: u32, n: usize| {
            (0..n)
                .map(|k| m.get(a as usize + k).copied().unwrap_or(0))
                .collect()
        };
        let mut vectors = [(0xfc_0000, 0xfc_0000); 7];
        vectors[2] = (v3, v3);
        s.observe(&Observation {
            phase,
            hits,
            libs: &libs(),
            irqs,
            vectors,
            peek: &peek,
        });
    }

    fn line(n: u32) -> Vec<(u32, u32)> {
        [0, 4, 8, 10, 14]
            .iter()
            .map(|&o| (PROGRAM_AT + o, n))
            .collect()
    }

    #[test]
    fn a_call_through_a_fake_vector_table_is_found_with_its_lvo_and_site() {
        let mut s = Survey::new(vec![(PROGRAM_AT, 0x100)]);
        // Step 1: both libraries' -$1e slots ran; step 2: only fake's did, so fake it is.
        let mut h = line(1);
        h.push((0x3000 - 0x1e, 1));
        h.push((0x3800 - 0x1e, 3));
        observe(&mut s, Phase::Load, &h, &[], 0);
        let mut h = line(2);
        h.push((0x3000 - 0x1e, 2));
        observe(&mut s, Phase::Title, &h, &[], 0);
        let r = s.report();
        assert!(r.undecoded.is_empty(), "{:?}", r.undecoded);
        let g = &r.calls[&("fake.library".to_string(), -0x1e, "a6".to_string())];
        assert_eq!(g.sites, vec!["h0+$4"]);
        assert_eq!(g.count, 3);
        assert_eq!(g.when, [Phase::Load, Phase::Title].into());
        // `$4` read as ExecBase, and $26 read off the call's result.
        assert!(
            r.mem
                .contains_key(&("ExecBase".into(), "$4".into(), "read")),
            "{:?}",
            r.mem
        );
        let off = r
            .mem
            .keys()
            .find(|(_, a, _)| a == "+$26")
            .unwrap_or_else(|| panic!("{:?}", r.mem));
        // -30 is no standard vector, so an unlisted library names it by number.
        assert_eq!(off.0, "fake.library lvo-30 result");
        let t = r.toml(300, &[1]);
        assert!(
            t.contains("library = \"fake.library\"\nlvo = -30\nname = \"lvo-30\""),
            "{t}"
        );
        assert_eq!(t, s.report().toml(300, &[1]));
    }

    #[test]
    fn a_site_no_library_slot_matches_is_an_unknown_jump() {
        let mut s = Survey::new(vec![(PROGRAM_AT, 0x100)]);
        observe(&mut s, Phase::Play, &line(1), &[], 0);
        let r = s.report();
        assert!(r.calls.is_empty());
        assert_eq!(r.jumps["no library vector"].sites, vec!["h0+$4"]);
    }

    #[test]
    fn an_interrupt_is_the_programs_only_when_its_vector_points_in_and_the_handler_ran() {
        let mut s = Survey::new(vec![(PROGRAM_AT, 0x100)]);
        // Vector into the program, its handler (h0+$e) ran once for two level 3s: one each.
        observe(
            &mut s,
            Phase::Play,
            &[(PROGRAM_AT + 14, 1)],
            &[3, 3],
            PROGRAM_AT + 14,
        );
        // The OS's vector.
        observe(&mut s, Phase::Title, &[], &[3], 0xfc_0000);
        let r = s.report();
        let got: Vec<_> = r
            .irqs
            .iter()
            .map(|i| (i.level, i.owner, i.when, i.count))
            .collect();
        assert_eq!(
            got,
            vec![
                (3, "os", Phase::Title, 1),
                (3, "os", Phase::Play, 1),
                (3, "program", Phase::Play, 1)
            ]
        );
    }

    #[test]
    fn names() {
        assert_eq!(cia_name(0xbfe001), "ciaa_pra");
        assert_eq!(cia_name(0xbfed01), "ciaa_icr");
        assert_eq!(cia_name(0xbfec01), "ciaa_sdr");
        assert_eq!(cia_name(0xbfd100), "ciab_prb");
        assert_eq!(cia_name(0xbfe000), "$bfe000");
        assert_eq!(lvo_name("exec.library", -408), "OldOpenLibrary");
        assert_eq!(lvo_name("exec.library", -414), "CloseLibrary");
        assert_eq!(lvo_name("exec.library", -552), "OpenLibrary");
        assert_eq!(lvo_name("dos.library", -42), "Read");
        assert_eq!(lvo_name("dos.library", -36), "Close");
        assert_eq!(lvo_name("graphics.library", -72), "OpenFont");
        assert_eq!(lvo_name("graphics.library", -78), "CloseFont");
        // Known offsets either side of the two private slots before LoadRGB4.
        assert_eq!(lvo_name("graphics.library", -192), "LoadRGB4");
        assert_eq!(lvo_name("graphics.library", -270), "WaitTOF");
        assert_eq!(lvo_name("graphics.library", -414), "FreeSprite");
        assert_eq!(lvo_name("graphics.library", -456), "OwnBlitter");
        assert_eq!(lvo_name("dos.library", -198), "Delay");
        assert_eq!(lvo_name("exec.library", -198), "AllocMem");
        assert_eq!(lvo_name("x.library", -6), "Open");
        assert_eq!(lvo_name("x.library", -60), "lvo-60");
    }

    #[test]
    fn library_lists_are_read_from_execbase() {
        // ExecBase at $400, its LibList at $57a holding one node at $800 named at $900.
        let mut m = vec![0u8; 0x1000];
        let put = |m: &mut Vec<u8>, a: usize, v: u32| m[a..a + 4].copy_from_slice(&v.to_be_bytes());
        put(&mut m, 4, 0x400);
        put(&mut m, 0x400 + 0x17a, 0x800);
        put(&mut m, 0x800, 0x400 + 0x17e); // succ: the list's tail node
        put(&mut m, 0x80a, 0x900);
        m[0x810..0x812].copy_from_slice(&36u16.to_be_bytes());
        m[0x900..0x90c].copy_from_slice(b"dos.library\0");
        let peek = move |a: u32, n: usize| {
            (0..n)
                .map(|k| m.get(a as usize + k).copied().unwrap_or(0))
                .collect()
        };
        let libs = read_libs(&peek);
        assert_eq!(
            libs,
            vec![Lib {
                name: "dos.library".into(),
                base: 0x800,
                neg: 36
            }]
        );
        assert_eq!(
            libs[0].slots().collect::<Vec<_>>(),
            vec![0x7fa, 0x7f4, 0x7ee, 0x7e8, 0x7e2, 0x7dc]
        );
    }
}
