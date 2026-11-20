# T60 — rig-branch-coverage

**Phase:** 5 · **Depends on:** T32 · **Weight:** medium

Added by the owner (2026-11-05, mid-run amendment): Muse writes generators (DESIGN §4.3), and a machine check of a generator needs to know which branches the original took.

## Goal
The routine rig reports, for a routine and a generator, every conditional branch in the routine and its callees and whether some generated state took it each way, so a generator can be judged by machine. Today `RigReport::paths` only counts distinct instruction counts, and T36 proved branch coverage with hand-written witness models (FINDINGS).
## Design sections this implements
§4.3.
## Reuses
The routine runner (`shim/moira_shim.cpp`) already steps one instruction at a time in its own loop, so it can record each instruction's address and the next without a vAmiga patch. T31's `vamiga_exec_hook` (`0006-exec-hook.patch`, compiled into the runner's Moira as `amiga_moira::vamiga_exec_hook`) is the other way in, but it is one process-wide pointer and runners run on several threads, so it needs a thread-local target. Moira's `disassemble` (compiled in from `MoiraDebugger.cpp`) gives instruction lengths for a static walk. `amiga-oracle/src/routine.rs`, T32's `Rig::survey`. `tools/call_tree.py` is a game tool and cannot be used by the toolkit.
## Files
`toolkit/crates/amiga-oracle/src/{routine.rs,ffi.rs}`, `toolkit/crates/amiga-oracle/shim/{moira_shim.cpp,moira_shim.h}`, `toolkit/crates/amiga-oracle/tests/routine_branches.rs`, `games/forest-dumb/engine/crates/fd-conform/src/routine.rs`, `fd-conform/tests/rig_branches.rs`, `toolkit/docs/architecture/verification.md`, `toolkit/smoke.toml`, `games/forest-dumb/smoke.toml`.
## Interface
Toolkit: `Cpu68k` gains an opt-in, off by default, that records per call each conditional branch executed (`Bcc` other than `BRA`/`BSR`, `DBcc`) with whether it was taken (next address not the one after it), and the addresses the call read; and a static walk from an entry over Moira's disassembler, following direct branches and calls, listing every conditional branch it meets. Indirect jumps are not followed; a branch reached only through one shows up once executed.
```
pub struct Branch { pub at: u32, pub taken: u32, pub not_taken: u32 }   // counted over the states
pub struct Coverage { pub report: RigReport, pub branches: Vec<Branch>, pub reads: BTreeSet<u32> }
impl Rig { pub fn coverage(&mut self, name: &str, cases: usize, entry: u32, gen: &Generator) -> Coverage }
```
Same states as `Rig::check` with the same `name`. `branches` is the static walk's branches plus any executed, each once; a branch never executed is listed with both counts 0. A branch with `taken == 0` or `not_taken == 0` is uncovered. `dbra` counts loop exit as not taken. `reads` is every address the original read on some state that returned.
## Tests
- [ ] A made-up routine (toolkit test, no game) with two branches in sequence and a third inside one arm: calls whose states set one condition only leave exactly the expected branches uncovered, the never-executed one with 0 and 0.
- [ ] With recording off, `Rig::check` and `Rig::survey` give identical results to before.
- [ ] On T36's `camera_update` with its generator, the report agrees with T36's table in `docs/11-translation.md` (28 branches, none uncovered), or the difference is explained in the doc.
- [ ] On T37's `h0_29ca` with its generator, the blitter wait T37 names as never taken is reported uncovered.
## Done when
- [ ] `verification.md` documents `coverage`, what the static walk misses, and its measured time cost with recording on and off (measured, not asserted in a test).
- [ ] `tools/check_tier_names.py` passes: the toolkit part names no game.
