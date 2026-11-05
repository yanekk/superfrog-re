# T06 — level-runner

**Phase:** 1 · **Depends on:** T03, T04, T31, T32 · **Weight:** medium

## Goal
Grow `fd-conform` from one byte at the title to a level case runner: many variables of several widths and hunks, picture comparison, mid-run input, goals, and the coverage report, so every frame-matching task has a judge (`toolkit/docs/porting-routines.md` §4). The routine rig and the port register are T32's (split 2026-11-05, DESIGN §4.3).
## Design sections this implements
§2.1, §2.2, §4, §4.1.
## Reuses
`toolkit/docs/porting-routines.md` §4, which this runner enforces; `amiga-conform` (`compare`, `PictureJudge`, `PassMatch`, `report`, `script`), `sf-conform/src/{case,freeze}.rs` as pattern; `sf-conform/src/lib.rs`' nothing-compared failure (T120 there) as pattern; T31's `watch_pcs` and `pc_hits`; T32's `PORTED`.
## Files
`games/forest-dumb/engine/crates/fd-conform/src/{lib.rs,case.rs,freeze.rs,coverage.rs}`, `engine/conform/README.md` updates, `fd-core/src/layout.rs`, `fd-conform/tests/{runner.rs,input_timing.rs}`, `games/forest-dumb/docs/09-play-measured.md` (re-measured rates), `smoke.toml`.
## Interface
```
pub struct Var { name: &str, hunk: u8, offset: u32, width: Width }  pub const LAYOUT: &[Var];
pub trait Transplant { fn transplant(&mut self, mem: &dyn Mem); fn probe(&self, v: &Var) -> u32; }
// case: sync, frames, compare = ["vars","picture"], vars = [...], goal = {all = [{var, value}], by_frame} | {idle = true}, exclude = [{what, task}], freeze = bool
pub struct Coverage { pub hits: Vec<(&'static str, u32)> }   // per case, registered routines' hit counts on the original
pub fn run(case: &Case) -> Result<Verdict>
```
## Tests
- [ ] A mid-run input reaching the engine one frame early or late fails, closing `forest-dumb-start`'s surviving mutant.
- [ ] A one-off value at frame N is named by pass, frame and values.
- [ ] A one-pixel difference outside tolerance fails; an excluded area passes and is reported.
- [ ] Unknown case keys are errors.
- [ ] The first case still passes.
- [ ] A case that compares nothing (no pass finished, no frame or variable compared) fails.
- [ ] A level case without a `goal` is an error; a case whose two sides agree but miss the goal fails. `idle = true` is for state-only stretches: it passes only when the run reaches its last frame with no event and every listed variable compared on every pass.
- [ ] A run reports each `PORTED` routine's hits on the original; a registered routine the case never reaches shows zero.
## Done when
- [ ] A stub level case runs state and picture on a short stretch and fails where it should.
- [ ] Case format and coverage report documented in `engine/conform/README.md`.
- [ ] Purity test covers everything added.
- [ ] T00's per-frame rate answers are re-measured with T31's counter at `game_frame` and `level3_handler`, and the note marks them measured; a result that differs from T00's is raised, not patched in.
