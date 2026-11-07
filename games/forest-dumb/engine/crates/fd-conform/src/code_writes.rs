//! Writes into code on the original (plans/forest-dumb-translator-trial DESIGN §2.6, T05).
//!
//! A static translator cannot follow a program that rewrites instructions it runs, so the trial's
//! gate (§2.7) asks whether level 1's real path does. A level case is stepped on the oracle from
//! the level sync point on its own input, and every frame the bytes of each instruction the game
//! has executed are compared with what they were.
//!
//! What is watched: every even address of the program's five hunks (T31's counter,
//! `Amiga::watch_pcs`), not only the instruction starts the listing knows, so a jump into the
//! middle of a listed instruction or into a data hunk is seen too and no listing is needed. An
//! address that ran is an instruction start; its length is Moira's disassembler's
//! (`Cpu68k::insn_len`) over the bytes it ran with.
//!
//! Two comparisons, both reported as a change:
//! - `since = "load"`: the first time an instruction runs, its bytes against the program file
//!   relocated at [`crate::HUNK_BASES`] (§2.6's "loaded values"). Catches a write that landed
//!   before the instruction first ran, the sync point's boot included.
//! - `since = "run"`: every later frame, its bytes against the last bytes seen.
//!
//! What it cannot see, and the report says so ([`MISSES`]): a write restored within one frame,
//! code run outside the program's hunks, and paths the cases do not take. The cases run without
//! their `freeze`: the freeze is the harness patching code, and the check is about the game's own
//! path. Their `set` writes (data) are applied, since a case's path depends on them.

use std::path::Path;

use amiga_oracle::{Amiga, AmigaMemory, Cpu68k, SLOW_BASE, kickstart_path};
use fd_core::{Width, layout_var};

use crate::case::Case;
use crate::routine::{PROGRAM, load_program};
use crate::{address, sync};

/// What a run cannot see, printed on the total line.
pub const MISSES: &str = "a write restored within one frame; code run outside the program's hunks; paths the cases do not take";

/// The longest 68000 instruction.
const MAX_LEN: usize = 10;

/// One change to the bytes of an executed instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    /// `hN+$off`.
    pub at: String,
    /// The oracle frame of the step after which it was seen (its input is `Inputs::at(frame)`).
    pub frame: u32,
    pub from: Vec<u8>,
    pub to: Vec<u8>,
    /// `"load"` against the program file at first execution, `"run"` against the last frame.
    pub since: &'static str,
}

/// One case's result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub case: String,
    /// Steps run from the level sync point.
    pub frames: u32,
    /// Distinct instructions executed.
    pub executed: usize,
    /// Their bytes, summed.
    pub executed_bytes: usize,
    pub changed: Vec<Change>,
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

impl Report {
    /// The case's JSON line.
    pub fn json(&self) -> String {
        let changed: Vec<String> = self
            .changed
            .iter()
            .map(|c| {
                format!(
                    r#"{{"at":"{}","frame":{},"from":"{}","to":"{}","since":"{}"}}"#,
                    c.at,
                    c.frame,
                    hex(&c.from),
                    hex(&c.to),
                    c.since
                )
            })
            .collect();
        format!(
            r#"{{"case":"{}","frames":{},"executed":{},"executed_bytes":{},"changed":[{}]}}"#,
            self.case,
            self.frames,
            self.executed,
            self.executed_bytes,
            changed.join(",")
        )
    }
}

/// The total line over `reports`.
pub fn total_json(reports: &[Report]) -> String {
    let frames: u64 = reports.iter().map(|r| r.frames as u64).sum();
    let changed: usize = reports.iter().map(|r| r.changed.len()).sum();
    format!(
        r#"{{"total":{{"cases":{},"frames":{},"changed":{},"misses":"{}"}}}}"#,
        reports.len(),
        frames,
        changed,
        MISSES
    )
}

/// The executed instructions of one run and their bytes as last seen.
struct Watch {
    /// The program as loaded, `MEM_SIZE` bytes.
    loaded: Vec<u8>,
    /// Each hunk's `(base, size)`.
    hunks: Vec<(u32, u32)>,
    /// Decodes an instruction's length; only the bytes at the address asked about are written.
    cpu: Cpu68k,
    /// Every executed instruction start, its bytes as last seen; ascending by address.
    seen: std::collections::BTreeMap<u32, Vec<u8>>,
}

fn read(mem: &AmigaMemory, addr: u32, len: usize) -> Vec<u8> {
    (0..len as u32)
        .map(|k| {
            let a = (addr + k) as usize & 0xFF_FFFF;
            let slow = SLOW_BASE as usize;
            if a < mem.chip.len() {
                mem.chip[a]
            } else if a >= slow && a - slow < mem.slow.len() {
                mem.slow[a - slow]
            } else {
                0
            }
        })
        .collect()
}

impl Watch {
    fn new(exe: &[u8]) -> Result<Watch, String> {
        let (loaded, hunks) = load_program(exe)?;
        Ok(Watch {
            loaded,
            hunks,
            cpu: Cpu68k::new(AmigaMemory {
                chip: vec![],
                slow: vec![],
            }),
            seen: Default::default(),
        })
    }

    /// Every even address of every hunk.
    fn pcs(&self) -> Vec<u32> {
        self.hunks
            .iter()
            .flat_map(|&(base, size)| (base..base + size).step_by(2))
            .collect()
    }

    fn name(&self, addr: u32) -> String {
        match self
            .hunks
            .iter()
            .position(|&(b, s)| addr >= b && addr < b + s)
        {
            Some(n) => format!("h{n}+${:x}", addr - self.hunks[n].0),
            None => format!("${addr:x}"),
        }
    }

    /// After the step that started at `frame`: add the instructions `hits` says ran, compare every
    /// instruction seen with `mem`.
    fn after_step(&mut self, frame: u32, hits: &[(u32, u32)], mem: &AmigaMemory) -> Vec<Change> {
        let mut changes = Vec::new();
        // Compare what was already seen first, so an instruction first run in this step is not
        // also compared with itself.
        for (&addr, bytes) in self.seen.iter_mut() {
            let now = read(mem, addr, bytes.len());
            if now != *bytes {
                changes.push((addr, std::mem::replace(bytes, now.clone()), now, "run"));
            }
        }
        for &(addr, n) in hits {
            if n == 0 || self.seen.contains_key(&addr) {
                continue;
            }
            let head = read(mem, addr, MAX_LEN);
            self.cpu.write(addr, &head);
            let len = (self.cpu.insn_len(addr) as usize).clamp(2, MAX_LEN);
            let now = head[..len].to_vec();
            let file = self.loaded[addr as usize..addr as usize + len].to_vec();
            if now != file {
                changes.push((addr, file, now.clone(), "load"));
            }
            self.seen.insert(addr, now);
        }
        changes
            .into_iter()
            .map(|(addr, from, to, since)| Change {
                at: self.name(addr),
                frame,
                from,
                to,
                since,
            })
            .collect()
    }
}

/// [`run_case_in`] on the Kickstart, disks and cache the oracle finds itself.
pub fn run_case(
    case: &Case,
    max_frames: Option<u32>,
    between: impl FnMut(&mut Amiga, u32),
) -> Result<Report, String> {
    let kickstart = kickstart_path().map_err(|e| e.to_string())?;
    let disks = fd_tools::disk_dir()?;
    run_case_in(
        &kickstart,
        &disks,
        &sync::cache_dir(),
        case,
        max_frames,
        between,
    )
}

/// Run level case `case` on the original from the level sync point to its last frame (or
/// `max_frames` steps, whichever is fewer), calling `between(amiga, frame)` before each step:
/// tests poke code there. A case that does not start at level 1 is an error.
pub fn run_case_in(
    kickstart: &Path,
    disk_dir: &Path,
    cache: &Path,
    case: &Case,
    max_frames: Option<u32>,
    mut between: impl FnMut(&mut Amiga, u32),
) -> Result<Report, String> {
    if !case.is_level() {
        return Err(format!("{}: does not start at level 1", case.name));
    }
    let inputs = crate::Inputs::load(case)?;
    let lead = case.sync_frame - sync::LEVEL_1_SYNC;
    let steps = max_frames.map_or(lead + case.frames, |m| m.min(lead + case.frames));
    let mut am = sync::reach_level_1_in(kickstart, disk_dir, cache, steps)?;
    let exe = fd_tools::game_file(disk_dir, PROGRAM)?;
    let mut watch = Watch::new(&exe)?;
    am.watch_pcs(&watch.pcs());
    let mut changed = Vec::new();
    for k in 0..steps {
        let frame = sync::LEVEL_1_SYNC + k;
        if frame == case.sync_frame {
            for (name, value) in &case.set {
                let v = layout_var(name)
                    .ok_or_else(|| format!("`{name}` is not in fd-core's LAYOUT"))?;
                let bytes = match v.width {
                    Width::Byte => vec![*value as u8],
                    Width::Word => (*value as u16).to_be_bytes().to_vec(),
                    Width::Long => value.to_be_bytes().to_vec(),
                };
                am.poke(address(name)?, &bytes);
            }
        }
        between(&mut am, frame);
        am.step(inputs.at(frame)).map_err(|e| e.to_string())?;
        changed.extend(watch.after_step(frame, am.pc_hits(), &am.memory()));
    }
    Ok(Report {
        case: case.name.clone(),
        frames: steps,
        executed: watch.seen.len(),
        executed_bytes: watch.seen.values().map(Vec::len).sum(),
        changed,
    })
}

/// `*` and `?` over a case's file stem.
pub fn glob_match(pattern: &str, name: &str) -> bool {
    fn go(p: &[u8], n: &[u8]) -> bool {
        match (p.first(), n.first()) {
            (None, None) => true,
            (Some(b'*'), _) => go(&p[1..], n) || (!n.is_empty() && go(p, &n[1..])),
            (Some(b'?'), Some(_)) => go(&p[1..], &n[1..]),
            (Some(a), Some(b)) if a == b => go(&p[1..], &n[1..]),
            _ => false,
        }
    }
    go(pattern.as_bytes(), name.as_bytes())
}
