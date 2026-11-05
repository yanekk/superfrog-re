# T60 — rig-branch-coverage

**Phase:** 5 · **Depends on:** T32 · **Weight:** medium

Added by the owner (2026-11-05, mid-run amendment): Muse writes generators (DESIGN §4.3), and a machine check of a generator needs to know which branches the original took.

## Goal
The routine rig reports, for a routine and a generator, every conditional branch in the routine and its callees and whether some generated state took it each way, so a generator can be judged by machine. Today `RigReport::paths` only counts distinct instruction counts, and T36 proved branch coverage with hand-written witness models (FINDINGS).
## Design sections this implements
§4.3.
## Reuses
T31's exec hook: patch `0006-exec-hook.patch` defines the hook pointer in `Moira/Moira.cpp`, which `build.rs`' `build_moira` also compiles for the routine runner (`amiga_moira`), so the routine runner can count executed addresses without a new vAmiga patch. `amiga-oracle/src/routine.rs`, T32's `Rig::survey`, `tools/call_tree.py`'s decoding of each reached address for where the branches are.
## Files
`toolkit/crates/amiga-oracle/src/routine.rs`, `toolkit/crates/amiga-oracle/shim/` (only if the routine runner needs a dispatch for the hook), `toolkit/crates/amiga-oracle/tests/routine_branches.rs`, `games/forest-dumb/engine/crates/fd-conform/src/routine.rs`, `fd-conform/tests/rig_branches.rs`, `toolkit/docs/architecture/verification.md`, `toolkit/smoke.toml`, `games/forest-dumb/smoke.toml`.
## Interface
```
pub struct Branch { pub at: u32, pub taken: u32, pub not_taken: u32 }   // a Bcc, DBcc or Scc-guarded jump, counted over the states
impl Rig { pub fn branches(&mut self, entry: u32, cases: usize, gen: &Generator) -> Result<Vec<Branch>> }
```
A branch with `taken == 0` or `not_taken == 0` is uncovered. `dbra` counts loop exit as not taken.
## Tests
- [ ] A made-up routine (toolkit test, no game) with two branches: a generator that sets one condition only leaves exactly the other uncovered.
- [ ] Counting is off when not asked for: `Rig::check` gives identical results and stays within 5% of its time with the hook compiled in.
- [ ] On a Forest Dumb routine with a hand-written generator (T36's `camera_update`), the uncovered list matches T36's witness notes in `docs/11-translation.md`.
## Done when
- [ ] `verification.md` documents `branches` and its cost.
- [ ] `tools/check_tier_names.py` passes: the toolkit part names no game.
