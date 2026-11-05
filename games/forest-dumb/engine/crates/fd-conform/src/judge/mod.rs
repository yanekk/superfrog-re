//! The translation service's judge (plans/forest-dumb-level-1 T33, DESIGN §4.3): a literal port of
//! one routine, written against [`Cpu`] and [`Mem`] by a model, run on the routine rig against the
//! original over a registered generator's states.
//!
//! The port under judgement is [`port::port`], put in by `build.rs` from the file `FD_PORT` names
//! (a path under `build/`, never committed: a translation of the game's code derives from it).
//! Without `FD_PORT` a stub that changes nothing is built, so the judge fails every routine that
//! changes anything: its negative check.

mod mem;

pub use mem::{Cpu, Mem};

pub use crate::routine::h;
use crate::routine::{Effects, Generator, Rig, RigFailure, RigReport, STACK};
use amiga_oracle::Regs;

/// The port built in from `FD_PORT`, or the stub. A split routine's file carries its callees'
/// passing ports as sibling modules, each called as `port_hN_off(cpu)` (`tools/translate.py`).
#[allow(unused_imports, dead_code, unused_variables, unused_mut, clippy::all)]
pub mod port {
    use super::*;
    include!(concat!(env!("OUT_DIR"), "/port.rs"));
}

/// Run `port` against the original at `entry` over `cases` states of `generator`, seeded by `name`,
/// as [`Rig::check`] does: the same registers, the same bytes changed and the same custom-chip
/// writes in order, or the first failing state in full.
pub fn check(
    rig: &mut Rig,
    name: &str,
    cases: usize,
    entry: u32,
    generator: &Generator,
    port: fn(&mut Cpu),
) -> Result<RigReport, RigFailure> {
    let base = rig.memory_for(generator);
    rig.check(name, cases, entry, generator, |s| {
        let mut before = Mem::new(base.clone(), &s.pokes);
        // The rig set `beam` to the byte the original's first VHPOSR read returned.
        before.beam.set(s.beam as u16);
        before.hw_reads = s.hw_reads.iter().copied().collect();
        let mut a = s.a;
        a[7] = STACK;
        let mut cpu = Cpu {
            d: s.d,
            a,
            mem: before.clone(),
        };
        port(&mut cpu);
        Effects {
            regs: Regs {
                d: cpu.d,
                a: cpu.a,
                ..Regs::default()
            },
            writes: cpu.mem.changed_since(&before),
            hw_writes: cpu.mem.hw_writes,
        }
    })
}
