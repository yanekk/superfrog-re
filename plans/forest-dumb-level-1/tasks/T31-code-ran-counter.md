# T31 — code-ran-counter

**Phase:** 0 · **Depends on:** — · **Weight:** medium

## Goal
The oracle counts, per frame, how often the original executed each of a list of watched instruction addresses, so a case can show that a ported routine really ran in play and T00's per-frame rate questions can be answered directly. Today the oracle has no execution hook: `set_profile` samples one PC per raster line, which misses short routines.
## Design sections this implements
§4 (coverage), §7 (owner, 2026-11-03: a sixth vAmiga patch, accepting one rebuild of every sync cache).
## Reuses
The hook pattern of `toolkit/crates/amiga-oracle/patches/0005-irq-entry-hook.patch` (a null function pointer a client installs); `amiga_irqs`' per-frame collection in `shim/shim.cpp` and `src/amiga.rs` as the model for a per-frame list; `toolkit/tools/setup/oracle.sh` to build.
## Files
`toolkit/crates/amiga-oracle/patches/0006-exec-hook.patch`, `toolkit/crates/amiga-oracle/shim/{shim.h,shim.cpp}`, `toolkit/crates/amiga-oracle/src/{ffi.rs,amiga.rs}`, `toolkit/crates/amiga-oracle/tests/exec_counts.rs`, `toolkit/docs/architecture/verification.md` (owns the new API), `toolkit/smoke.toml`, `games/forest-dumb/docs/` note if T00 has landed and its rate questions can be re-measured.
## Interface
```
impl Amiga {
    pub fn watch_pcs(&mut self, pcs: &[u32]);            // replaces the watched set; empty turns it off
    pub fn pc_hits(&self) -> &[(u32, u32)];              // (pc, count) for the last stepped frame, watched pcs only
}
```
The hook is called before each instruction with its PC and does nothing unless a set is installed, so a run with no watch list is unchanged. A watched set is a sorted array searched per instruction; the speed cost with a set installed is measured and written in the chapter.
## Tests
- [ ] A watched PC inside the title's frame loop counts once per pass over 300 frames; a PC never reached counts zero.
- [ ] Runs with no watch list, an empty one and a full one give byte-identical chip RAM, picture and Paula writes every frame for 3 000 frames: the hook only observes.
- [ ] Snapshot and restore keep the watch list and reset the counts.
- [ ] Headless speed with no watch list stays within 5% of today's (about 550 to 610 frames a second).
## Done when
- [ ] The patch applies at `VAMIGA_COMMIT`, `oracle.sh` builds it, and the build hash changes as `build.rs` describes.
- [ ] Superfrog's and Forest Dumb's checks are green after their sync caches rebuild under the new hash; the rebuild time is in FINDINGS, and smoke's first run after it is measured against its budget.
- [ ] `verification.md` documents `watch_pcs` and `pc_hits`, with their cost.
## Shared files
T04 adds `Amiga::picture_in` to `src/amiga.rs`. The two edits touch different functions; whichever merges second takes the other's change.
