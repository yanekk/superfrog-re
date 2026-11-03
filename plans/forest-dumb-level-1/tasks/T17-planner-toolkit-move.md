# T17 — planner-toolkit-move

**Phase:** 4 · **Depends on:** T02 · **Weight:** heavy

## Goal
Move the planner's explorer path into toolkit crate `amiga-plan` behind `PlanGame`, with Superfrog's routes unchanged, because every later level uses it.
## Design sections this implements
§2.11, §3.2, §3.6.
## Reuses
`sf-plan/src/core/{archive,explore,guide,shorten,repair,walk,graph}.rs`, `run.rs`, `threads.rs`, `guide_file.rs`; `map`, `moves`, `coins`, `danger` as needed by `Compass::Map`. `facts.rs` stays per game. `dry-fit` is the precedent for a made-up game.
## Files
`toolkit/crates/amiga-plan/`, `toolkit/crates/amiga-plan/tests/{made_up_game.rs,purity.rs}`, `sf-plan/src/**`, `games/superfrog/smoke.toml`, `toolkit/smoke.toml`, `toolkit/README.md`.
## Interface
```
pub trait PlanGame: Clone + Send + Sync {
    fn step(&mut self, i: FrameInput);
    fn state_bytes(&self) -> u64;
    fn cell(&self) -> Cell;
    fn lives(&self) -> i16;
    fn dying(&self) -> bool;
    fn health(&self) -> u16;
    fn playing(&self) -> bool;
    fn collectibles_left(&self) -> u16;
    fn held_flag(&self) -> bool;
    fn landed(&self) -> bool;
    fn open(&self, c: Cell) -> bool;
    fn collectible_live(&self, c: Cell) -> bool;
    fn pressed(&self, f: &Facts) -> u32;
    const ACTS: &'static [Joy];
}
pub struct Facts { pub cols: u16, pub rows: u16, pub exits: Vec<Cell>, pub collectibles: Vec<Cell>, pub gates: Vec<Vec<Cell>> }
```
Neither game's `Game` can implement a toolkit trait from a third crate, so `sf-plan` and `fd-plan` each hold a newtype around it. `sf_plan::core` stays via re-export, so its tests need few edits. `COLS`, `ROWS` become runtime values, and `InputScript`'s use gets a toolkit home or a parameter.
## Tests
- [ ] T02's pins pass, unchanged.
- [ ] `sf-plan` reach, walk, repair, facts, purity, cache and `explore.rs:88` pass; the `#[ignore]`d `explore.rs:115`, `walk.rs:180`, `cli.rs:202` run once in the session.
- [ ] A made-up grid game is solved in the toolkit's tests, with no disk.
- [ ] Toolkit purity test forbids clocks in the core.
## Done when
- [ ] `sf-plan` contains only Superfrog's facts, shell and tests.
- [ ] Tier name check passes; core comments name no game or its creatures.
- [ ] Cache key still changes when core sources change.
- [ ] Each moved crate keeps its `opt-level` override in the root manifest of every workspace that builds it (toolkit, both games), and the bench or page timing matches the pre-move run.
