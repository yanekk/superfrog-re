//! The seam's judge (translator DESIGN §2.9, T35; `fd-judge --seam`): a hand-written routine mounted
//! in place of a translated one, against that translated routine, on the same states.
//!
//! The states are the routine oracle's: a registered generator's, seeded by its name, laid over the
//! program as [`super::translated::check`] lays them. Each state runs the replaced routine (the
//! translated entry) and the replacing one (`fd_translated::seam`'s adapter, entered as the seam
//! enters it, with a return address pushed) on two copies of the same [`Mem`], and compares every
//! byte written off the rig's stack ([`STACK_AREA`]), every custom-chip write in order, where each returned to (`PC`) and the stack
//! pointer after, and the registers `regs` names. Registers are not compared by default: a port
//! keeps no scratch register, and every caller of a mounted routine reloads or restores them
//! (translator T00: 487 of 2 000 `game_frame` states differ only in d2, d3, d7).
//!
//! Comparing with the translated routine, not the original, is the design's choice: the translated
//! routine is already held to the original (the whole-tree gate), and what a mount has to prove is
//! that the program it is mounted into sees the same thing. A state on which the replaced routine
//! stops is not judged and is counted, since there is nothing to hold the replacement to.

use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use amiga_oracle::RETURN_SENTINEL;
use pcode_rt::{Image, Mounted, PC, Regs};

use super::Mem;
use super::translated::{Entry, on_run_stack};
use crate::HUNK_BASES;
use crate::routine::{Gen, Generator, RegsCompared, Rig, STACK, STACK_AREA, seed};

/// How many differences the first failing state lists.
const SHOWN: usize = 12;

/// The verdict of a replacement against the routine it replaces.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SeamReport {
    pub cases: usize,
    /// States both ran on with the same effects.
    pub passed: usize,
    /// States they differed on (or the replacing routine stopped or panicked).
    pub failures: usize,
    /// States the replaced routine stopped on: not judged.
    pub replaced_stopped: usize,
    /// The first failing state and its differences.
    pub first: String,
    /// The first stop of the replaced routine.
    pub first_replaced_stop: String,
}

impl SeamReport {
    /// Passes when every judged state matched and at least one was judged.
    pub fn pass(&self) -> bool {
        self.failures == 0 && self.passed > 0
    }
}

/// What one side did on a state.
struct Ran {
    stop: Option<String>,
    regs: Regs,
    writes: Vec<(u32, u8)>,
    hw_writes: Vec<(u16, u16)>,
}

fn run(
    f: impl FnOnce(&mut Regs, &mut Mem, &Image) -> Result<(), pcode_rt::Stop> + Send,
    mut r: Regs,
    before: &Mem,
    img: &Image,
) -> Ran {
    let mut mem = before.clone();
    let ran = catch_unwind(AssertUnwindSafe(|| {
        on_run_stack(|| f(&mut r, &mut mem, img).map_err(|s| s.to_string()))
    }));
    let stop = match ran {
        Ok(Ok(())) => None,
        Ok(Err(s)) => Some(s),
        Err(e) => Some(format!(
            "panicked: {}",
            e.downcast_ref::<String>()
                .cloned()
                .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default()
        )),
    };
    // Off the stack, as the routine oracle compares: what a routine pushes for its own calls is
    // below the caller's stack pointer once it returns, and a port pushes nothing.
    let mut writes = mem.changed_since(before);
    writes.retain(|(a, _)| !STACK_AREA.contains(a));
    Ran {
        stop,
        regs: r,
        writes,
        hw_writes: mem.hw_writes,
    }
}

/// Run `replaced` (the translated entry at `entry`; `None` when the tree has none, which fails every
/// state) and `replacing` over `cases` states of `generator`, seeded by `name`, both with
/// `mount_data` as what the bus hands a mounted routine beside memory (level 1's hand engine for
/// the frame mount, T36).
#[allow(clippy::too_many_arguments)]
pub fn check(
    rig: &Rig,
    name: &str,
    cases: usize,
    entry: u32,
    generator: &Generator,
    replaced: Option<Entry>,
    replacing: Mounted,
    regs: RegsCompared,
    mount_data: Option<Arc<dyn Any + Send + Sync>>,
) -> SeamReport {
    let base = rig.memory_for(generator);
    let sizes = rig.hunk_sizes();
    let at = rig.name(entry);
    let img = Image {
        bases: &HUNK_BASES,
        sizes: &sizes,
        code: &[],
    };
    let mut rep = SeamReport {
        cases,
        ..SeamReport::default()
    };
    let mut g = Gen::new(seed(name));
    for case in 0..cases {
        let s = (generator.case)(&mut g);
        let mut before = Mem::new(base.clone(), &s.pokes);
        before.mount_data = mount_data.clone();
        before.beam.set(s.beam as u16);
        before.hw_reads = s.hw_reads.iter().copied().collect();
        // Entered as a call is: the return address pushed below the stack, a7 on it.
        let sp = STACK - 4;
        before.wl(sp, RETURN_SENTINEL);
        let mut r = Regs::new();
        for n in 0..8 {
            r.set_d(n, s.d[n]);
            r.set_a(n, s.a[n]);
        }
        r.set_a(7, sp);
        let old = match replaced {
            Some(f) => run(|r, m, img| f(r, m, img), r.clone(), &before, &img),
            None => Ran {
                stop: Some(format!("untranslated: {at}")),
                regs: r.clone(),
                writes: Vec::new(),
                hw_writes: Vec::new(),
            },
        };
        if let Some(stop) = &old.stop {
            if replaced.is_some() {
                rep.replaced_stopped += 1;
                if rep.first_replaced_stop.is_empty() {
                    rep.first_replaced_stop = format!("state {case}: {stop}");
                }
                continue;
            }
        }
        let new = run(|r, m, img| replacing(r, m, img), r, &before, &img);
        let diffs = differences(rig, &old, &new, regs);
        if diffs.is_empty() {
            rep.passed += 1;
        } else {
            rep.failures += 1;
            if rep.first.is_empty() {
                rep.first = format!("state {case} of {name} at {at}:\n  {}", diffs.join("\n  "));
            }
        }
    }
    rep
}

/// What `new` (the replacing routine) did differently from `old` (the replaced), one line each,
/// the first [`SHOWN`].
fn differences(rig: &Rig, old: &Ran, new: &Ran, regs: RegsCompared) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(s) = &old.stop {
        out.push(format!("the replaced routine stopped: {s}"));
    }
    if let Some(s) = &new.stop {
        out.push(format!("the replacing routine stopped: {s}"));
        return out;
    }
    let (ra, rb) = (&old.regs, &new.regs);
    let pc = |r: &Regs| r.get(PC, 4) as u32;
    if pc(ra) != pc(rb) {
        out.push(format!(
            "returned to: replaced ${:06x}, replacing ${:06x}",
            pc(ra),
            pc(rb)
        ));
    }
    if ra.a(7) != rb.a(7) {
        out.push(format!(
            "a7: replaced ${:08x}, replacing ${:08x}",
            ra.a(7),
            rb.a(7)
        ));
    }
    for n in 0..8 {
        if regs.d >> n & 1 != 0 && ra.d(n) != rb.d(n) {
            out.push(format!(
                "d{n}: replaced ${:08x}, replacing ${:08x}",
                ra.d(n),
                rb.d(n)
            ));
        }
    }
    for n in 0..7 {
        if regs.a >> n & 1 != 0 && ra.a(n) != rb.a(n) {
            out.push(format!(
                "a{n}: replaced ${:08x}, replacing ${:08x}",
                ra.a(n),
                rb.a(n)
            ));
        }
    }
    // Both lists are sorted by address: walk them together.
    let (mut i, mut j) = (0, 0);
    let (a, b) = (&old.writes, &new.writes);
    while i < a.len() || j < b.len() {
        let (x, y) = (a.get(i), b.get(j));
        let line = match (x, y) {
            (Some(&(p, u)), Some(&(q, v))) if p == q => {
                i += 1;
                j += 1;
                (u != v).then(|| {
                    format!(
                        "{}: replaced wrote ${u:02x}, replacing ${v:02x}",
                        rig.name(p)
                    )
                })
            }
            (Some(&(p, u)), Some(&(q, _))) if p < q => {
                i += 1;
                Some(format!(
                    "{}: replaced wrote ${u:02x}, replacing nothing",
                    rig.name(p)
                ))
            }
            (Some(&(p, u)), None) => {
                i += 1;
                Some(format!(
                    "{}: replaced wrote ${u:02x}, replacing nothing",
                    rig.name(p)
                ))
            }
            (_, Some(&(q, v))) => {
                j += 1;
                Some(format!(
                    "{}: replacing wrote ${v:02x}, replaced nothing",
                    rig.name(q)
                ))
            }
            (None, None) => unreachable!(),
        };
        out.extend(line);
    }
    if old.hw_writes != new.hw_writes {
        let k = old
            .hw_writes
            .iter()
            .zip(&new.hw_writes)
            .take_while(|(x, y)| x == y)
            .count();
        let show = |w: Option<&(u16, u16)>| {
            w.map_or("none".to_string(), |&(reg, v)| {
                format!("${reg:03x}=${v:04x}")
            })
        };
        out.push(format!(
            "custom write {k}: replaced {}, replacing {} ({} against {} writes)",
            show(old.hw_writes.get(k)),
            show(new.hw_writes.get(k)),
            old.hw_writes.len(),
            new.hw_writes.len()
        ));
    }
    if out.len() > SHOWN {
        let more = out.len() - SHOWN;
        out.truncate(SHOWN);
        out.push(format!("and {more} more"));
    }
    out
}
