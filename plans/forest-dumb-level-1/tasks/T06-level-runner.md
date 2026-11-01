# T06 — level-runner

**Phase:** 1 · **Depends on:** T03, T04 · **Weight:** heavy

## Goal
Grow `fd-conform` from one byte at the title to a level case runner: many variables of several widths and hunks, picture comparison, mid-run input, and a routine rig, so every porting task has a judge.
## Design sections this implements
§2.1, §2.2, §4.
## Reuses
`amiga-conform` (`compare`, `PictureJudge`, `PassMatch`, `report`, `script`), `sf-conform/src/{case,freeze}.rs` as pattern, `amiga-oracle/src/routine.rs`, `sf-conform/src/routine.rs` as pattern. `Gen` is extracted to the toolkit if it ends up generic, since this is its second user.
## Files
`games/forest-dumb/engine/crates/fd-conform/src/{lib.rs,case.rs,routine.rs,freeze.rs}`, `fd-conform/conform/README` updates, `fd-core/src/layout.rs`, `fd-conform/tests/{runner.rs,input_timing.rs}`, `smoke.toml`.
## Interface
```
pub struct Var { name: &str, hunk: u8, offset: u32, width: Width }  pub const LAYOUT: &[Var];
pub trait Transplant { fn transplant(&mut self, mem: &dyn Mem); fn probe(&self, v: &Var) -> u32; }
// case: sync, frames, compare = ["vars","picture"], vars = [...], exclude = [{what, task}], freeze = bool
pub fn run(case: &Case) -> Result<Verdict>
pub struct Rig; impl Rig { pub fn check(&mut self, name: &str, cases: usize, entry: u32, setup: &Setup) -> Result<()> }
```
## Tests
- [ ] A mid-run input reaching the engine one frame early or late fails, closing `forest-dumb-start`'s surviving mutant.
- [ ] A one-off value at frame N is named by pass, frame and values.
- [ ] A one-pixel difference outside tolerance fails; an excluded area passes and is reported.
- [ ] Unknown case keys are errors.
- [ ] A routine hang is a bounded count, not a silent pass.
- [ ] The first case still passes.
## Done when
- [ ] A stub level case runs state and picture on a short stretch and fails where it should.
- [ ] Case format documented in `engine/conform/README.md`.
- [ ] Purity test covers everything added.
