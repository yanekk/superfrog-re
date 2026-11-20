# T31 — code-ran-counter

**Phase:** 0 · **Depends on:** — · **Weight:** medium

## Goal
The oracle counts, per frame, how often the original executed each of a list of watched instruction addresses, so a case can show that a ported routine really ran in play (DESIGN §4.1). Today the oracle has no execution hook: `set_profile` samples one PC per raster line, which misses short routines. This is a toolkit task: it measures nothing about a game. T06 uses it and re-measures T00's rate answers with it.
## Design sections this implements
§4.1 (coverage), §7 (owner, 2026-11-03: a sixth vAmiga patch, accepting one rebuild of every sync cache).
## Reuses
The hook pattern of `toolkit/crates/amiga-oracle/patches/0005-irq-entry-hook.patch` (a null function pointer a client installs); the thread-local machine pointer `irqHook` dispatches through in `shim/shim.cpp`; `amiga_irqs`' per-frame collection in `shim/shim.cpp` and `src/amiga.rs` as the model for a per-frame list; `toolkit/tools/setup/oracle.sh` to build.
## Files
`toolkit/crates/amiga-oracle/patches/0006-exec-hook.patch`, `toolkit/crates/amiga-oracle/shim/{shim.h,shim.cpp}`, `toolkit/crates/amiga-oracle/src/{ffi.rs,amiga.rs}`, `toolkit/crates/amiga-oracle/tests/exec_counts.rs`, `toolkit/docs/architecture/verification.md` (owns the new API), `toolkit/smoke.toml`.
## Interface
```
impl Amiga {
    pub fn watch_pcs(&mut self, pcs: &[u32]);            // replaces the watched set; empty turns it off
    pub fn pc_hits(&self) -> &[(u32, u32)];              // (pc, count) for the last stepped frame, watched pcs only
}
```
Where the hook goes, from vAmiga at `VAMIGA_COMMIT`:
- The pointer is defined in `Moira/Moira.cpp`, inside `namespace vamiga`, not in `CPU.cpp` as 0005's is. `build.rs`' `build_moira` compiles the same patched `Moira.cpp` a second time for the routine runner, renamed to `amiga_moira` and without `CPU.cpp`, so a pointer in `CPU.cpp` leaves the runner unlinkable.
- It is called once per executed instruction in `Moira::execute`. In the fast path it goes before `reg.pc += 2`; in the slow path it goes after the interrupt and STOPPED checks, beside the LOGGING call. An instruction pre-empted by an interrupt is counted once, after the return, and a STOPPED CPU polling every 2 cycles counts nothing.
- The shim dispatches through a thread-local machine pointer, as `irqHook` does. A watched set is a sorted array searched per instruction.
## Tests
- [ ] A Kickstart address measured in the test, such as its vertical-blank handler on the insert-disk screen, counts once per frame over 300 frames; a watched address never reached counts zero. No game is named or used (the toolkit's oracle tests boot Kickstart without a disk, as `tests/boot.rs` does).
- [ ] An instruction pre-empted by an interrupt is counted once.
- [ ] Runs with no watch list, an empty one and a full one give byte-identical chip RAM, picture and CPU registers every frame for 3 000 frames: the hook only observes.
- [ ] A run restored from a snapshot gives the same per-frame counts as the run that took the snapshot, over the frames after it.
- [ ] `exec_counts.rs` is labelled full in `toolkit/smoke.toml` if it takes over 10 s (about 16 s expected), with one short test labelled smoke.
## Done when
- [ ] The patch applies at `VAMIGA_COMMIT`, `oracle.sh` builds it, the build hash changes as `build.rs` describes, and the routine runner still links and its tests pass.
- [ ] Headless speed is measured on an idle machine before and after the patch, with no watch list and with a list of 200 addresses, and all three figures are in FINDINGS and `verification.md`. The recorded figure before is 565 to 571 frames a second (`plans/smart-planner/DESIGN.md`).
- [ ] Superfrog's and Forest Dumb's checks are green after their sync caches rebuild under the new hash; the rebuild time is in FINDINGS, and smoke's first run after it is measured against its budget.
- [ ] `verification.md` documents `watch_pcs` and `pc_hits`, with their cost.
## Shared files
T04 adds `Amiga::picture_in` to `src/amiga.rs`. The two edits touch different functions; whichever merges second keeps the other's change.
