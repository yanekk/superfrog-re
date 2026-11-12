//! The judge's translated mode (translator-trial T04, DESIGN §2.2, §2.5, §3.2): any entry of one
//! translated tree, run on `pcode-rt` over [`Mem`], against the original over a generator's states,
//! compared exactly as a Muse port is ([`super::check`]): registers d0-d7 and a0-a6, every byte
//! written off the stack, every custom-chip write in order.
//!
//! The tree is [`tree`], put in by `build.rs` from the file `FD_TRANSLATED` names
//! (`build/translator/tree.rs`, never committed). One build serves every routine of it, where
//! `FD_PORT` takes one build per routine. Without `FD_TRANSLATED` the stub has no entries, so every
//! state stops as untranslated: this mode's negative check.
//!
//! A [`Stop`] fails its state and is counted by kind; the rig reports it as "the port stopped: ..".
//! A state on which the original changes a byte of the tree's `CODE` is not judged: the rig counts
//! it as `self_modifying` (DESIGN §2.5, translator T15), so a translated store into code fails only
//! where the original does not write there.
//!
//! Each state runs on a thread of its own with a [`RUN_STACK`]-byte stack, so a run that nests
//! `pcode_rt::DEPTH_CAP` translated frames stops as `hang (depth)` instead of overflowing the stack,
//! which aborts the process (translator T14).

use std::panic::resume_unwind;
use std::thread;

use amiga_oracle::{RETURN_SENTINEL, Regs};
use pcode_rt::{Bus, Image, Stop};

use super::Mem;
use crate::HUNK_BASES;
use crate::routine::{
    Effects, Generator, PortStop, RegsCompared, Rig, RigFailure, RigReport, STACK,
};

/// The stack each judged state runs on. Every translated function is a Rust frame, and a run nests
/// at most `pcode_rt::DEPTH_CAP` of them. Measured (translator T14, 2026-11-11, the trial's tree in
/// the judge's unoptimised build): one frame up to 49 KB (`h0+$9b06`), 66 KB at the deepest run
/// (`game_frame`, 6 deep). 64 MB is 2 MB a frame at the cap; the pages are touched only as used.
pub const RUN_STACK: usize = 64 << 20;

/// A translated entry, as `tools/pcode_rust.py` emits each function and `tree::entry` returns it.
pub type Entry = fn(&mut pcode_rt::Regs, &mut dyn Bus, &Image) -> Result<(), Stop>;

/// The tree built in from `FD_TRANSLATED`, or the stub: `CODE`, the `(hunk, start, end)` ranges of
/// its translated instructions, and `entry("hN+$off")`.
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

impl Bus for Mem {
    fn rb(&mut self, a: u32) -> u8 {
        Mem::rb(self, a)
    }
    fn rw(&mut self, a: u32) -> u16 {
        Mem::rw(self, a)
    }
    fn rl(&mut self, a: u32) -> u32 {
        Mem::rl(self, a)
    }
    fn wb(&mut self, a: u32, v: u8) {
        Mem::wb(self, a, v)
    }
    fn ww(&mut self, a: u32, v: u16) {
        Mem::ww(self, a, v)
    }
    fn wl(&mut self, a: u32, v: u32) {
        Mem::wl(self, a, v)
    }
}

/// The stops a judged entry made, one per failed state, by kind; `first` names the first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stops {
    pub untranslated: usize,
    pub write_into_code: usize,
    pub unsupported: usize,
    /// `Stop::Hang`: the step or depth cap (`first` says which), apart from the original's hangs the
    /// report counts.
    pub hangs: usize,
    /// `Stop::Trap`: an exception the translated code ends on (a division by zero).
    pub traps: usize,
    /// `Stop::Outside`: an access to unmapped memory. The judge's own memory never makes one; the
    /// machine's does (pcode-machine).
    pub outside: usize,
    pub first: String,
}

impl Stops {
    fn count(&mut self, s: &Stop) {
        match s {
            Stop::Untranslated(_) => self.untranslated += 1,
            Stop::WriteIntoCode(_) => self.write_into_code += 1,
            Stop::Unsupported { .. } => self.unsupported += 1,
            Stop::Hang { .. } => self.hangs += 1,
            Stop::Trap { .. } => self.traps += 1,
            Stop::Outside(_) => self.outside += 1,
            // The judge's memory has no OS; a machine's unanswered OS call is a thing not carried out.
            Stop::Os(_) => self.unsupported += 1,
        }
        if self.first.is_empty() {
            self.first = s.to_string();
        }
    }
}

/// A translated entry's verdict and the stops behind it.
pub struct Judged {
    pub verdict: Result<RigReport, RigFailure>,
    pub stops: Stops,
}

/// Run `run` (an entry of a translated tree whose instructions are `code`; `None` when the tree has
/// no entry at `entry`) against the original at `entry` over `cases` states of `generator`, seeded
/// by `name`, as [`super::check`] runs a port.
///
/// The state's registers are copied into `pcode-rt`'s, the entry is called as the oracle calls
/// the original (a7 four bytes below the rig's stack, the oracle's return address there), with the
/// program at [`HUNK_BASES`], and the registers are copied back.
pub fn check(
    rig: &mut Rig,
    name: &str,
    cases: usize,
    entry: u32,
    generator: &Generator,
    run: Option<Entry>,
    code: &[(u32, u32, u32)],
) -> Judged {
    let base = rig.memory_for(generator);
    let sizes = rig.hunk_sizes();
    let at = rig.name(entry);
    let mut stops = Stops::default();
    // A state on which the original itself changes a translated instruction's byte is out of the
    // game's reach (DESIGN §2.5): the rig counts it as self-modifying and runs no port on it.
    rig.set_code(
        code.iter()
            .map(|&(hunk, start, end)| {
                let b = HUNK_BASES[hunk as usize];
                (b + start, b + end)
            })
            .collect(),
    );
    let verdict = rig.check_regs(name, cases, entry, generator, RegsCompared::ALL, |s| {
        let mut before = Mem::new(base.clone(), &s.pokes);
        before.beam.set(s.beam as u16);
        before.hw_reads = s.hw_reads.iter().copied().collect();
        // The oracle pushes its return address below the stack it is given and enters with a7 on
        // it (`amiga_oracle::Cpu68k::call`), so stack-relative reads see what the original's do.
        // Written before the snapshot, so it is no write of the routine's. A top-level `rts` pops it.
        let sp = STACK - 4;
        before.wl(sp, RETURN_SENTINEL);
        let mut mem = before.clone();
        let mut r = pcode_rt::Regs::new();
        for n in 0..8 {
            r.set_d(n, s.d[n]);
            r.set_a(n, s.a[n]);
        }
        r.set_a(7, sp);
        let img = Image {
            bases: &HUNK_BASES,
            sizes: &sizes,
            code,
        };
        let ran = match run {
            Some(f) => thread::scope(|scope| {
                let t = thread::Builder::new()
                    .stack_size(RUN_STACK)
                    .spawn_scoped(scope, || f(&mut r, &mut mem, &img))
                    .expect("a thread for a judged state");
                // A panic in the run goes on unwinding here, where the rig catches it.
                t.join().unwrap_or_else(|e| resume_unwind(e))
            }),
            None => Err(Stop::Untranslated(at.clone())),
        };
        if let Err(stop) = ran {
            stops.count(&stop);
            // Not a panic: no hook runs, and the rig names it as a stop.
            resume_unwind(Box::new(PortStop(stop.to_string())));
        }
        Effects {
            regs: Regs {
                d: std::array::from_fn(|n| r.d(n)),
                a: std::array::from_fn(|n| r.a(n)),
                ..Regs::default()
            },
            writes: mem.changed_since(&before),
            hw_writes: mem.hw_writes,
        }
    });
    rig.set_code(Vec::new());
    Judged { verdict, stops }
}
