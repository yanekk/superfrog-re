# T02 — generic-clock

**Phase:** 2 · **Depends on:** T01 · **Weight:** heavy

## Goal

Move the level's beam model into `amiga-clock` as a generic `Clock` configured by a `Timing` (frame
length, fetch window, the blitter's free slots in it, the per-line steal), with the level using it
on today's parameters; move all blit timing there beside it, each calculation unchanged; and move
`Timeline`'s stepping there, leaving its Superfrog binding in sf-core. It is the draft's T2. After
it, the toolkit has a clock another game can configure for its own display. No behaviour changes,
and save states written before it still load.

## Design sections this implements

DESIGN §2.3, §2.4, §2.6, §2.7, §3.1, §3.3 (row T02).

## Files

- `engine/crates/amiga-clock/src/*`: `Timing`, `Clock<T>`, the blit timing, `Timeline<J>`.
- `engine/crates/sf-core/src/level/beam.rs`: `Level` timing, `pub type Clock`, re-exports of
  `slots_per_word` and the rest; `Cost`, `COSTS`, `routine_cost`, `Tick`, `Cut`, `Pending`,
  `Frames` stay.
- `engine/crates/sf-core/src/level/timeline.rs`: `of` and the beam trait's impl for `front::Cx`;
  `Timeline` re-exported or aliased.
- `engine/crates/sf-core/src/level/mod.rs`, `level/panel.rs`, and other `Clock { .. }` literal sites.
- `engine/crates/amiga-chips/src/blit.rs`: `word_times` and `FIXED_TO` re-exported from
  `amiga-clock`; `Blit::cycles` calls the moved formula.
- `engine/crates/sf-core/src/front/attract.rs` (`word_times` caller), `front/story/cpu.rs`
  (`refresh` through `is_refresh`).
- `engine/crates/sf-conform` files that build a `Clock` by literal, if any.

## Interface

```rust
// amiga_clock
pub trait Timing: Copy + Default + Eq + core::hash::Hash + core::fmt::Debug {
    const TIMING: Timing_;
}
/// A display's set-up as the clock needs it.
pub struct Timing_ {                      // name is the builder's
    pub frame_lines: u16,                 // level: FRAME_LINES
    pub fetch_lines: core::ops::Range<u16>, // level: 0x27..0x12c
    pub fetch_slots: core::ops::Range<u32>, // level: 0x28..0xe0
    pub blitter_free: u8,                 // free slots of each 8 in the window, bit n = slot n; level 0b10101
    pub stolen_cycles: u32,               // CPU cycles a fetching line steals; level 40
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Clock<T: Timing> {
    pub line: u16,
    pub into: u32,
    #[serde(skip)] _t: core::marker::PhantomData<T>,
}
impl<T: Timing> Clock<T> {
    pub const START: Self;
    pub const fn at(line: u16, into: u32) -> Self;   // replaces `Clock { line, into }` literals
    pub fn pos(&self) -> BeamPos;
    pub fn cpu(&mut self, cycles: u32);
    pub fn blit(&mut self, slots: u32);
    pub fn blit_cycles(&mut self, words: u32, con0: u16) -> Vec<u32>;
    pub fn nominal(&self) -> u32;
    pub fn hpos(&self) -> u16;
    pub fn next_frame(&self) -> Self;
    pub fn blitter_slot(line: u16, slot: u32) -> bool;
}

pub fn is_refresh(slot: u32) -> bool;                 // $01, $03, $05, $e2
pub fn slots_per_word(con0: u16) -> u32;
pub const FIXED_TO: u16 = 0x0d;
pub fn word_times(words: u32, line: u16, h: u16, takes: impl Fn(u16, u16) -> bool) -> Vec<(u16, u16)>;
pub fn rough_blit_cycles(con0: u16, words: u32) -> u32; // Blit::cycles' formula

/// What a Timeline moves.
pub trait TimelineBeam<J> {
    fn line(&self) -> u16;
    fn set_line(&mut self, line: u16);
    fn write(&mut self, reg: u16, value: u16);
    fn job(&mut self, job: J);
}
pub enum TimelineYield { Poll, Return }
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Timeline<J> { /* steps, tail, at, left, spare, job: Option<J>, in today's order */ }
impl<J> Timeline<J> {
    pub fn new(steps: Vec<(u32, u16, u16)>, tail: u32, job: J) -> Self;
    pub fn cycles(&self) -> u32;
    pub fn run<B: TimelineBeam<J>>(&mut self, beam: &mut B) -> TimelineYield;
}
```

```rust
// sf_core::level::beam
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Level;
impl amiga_clock::Timing for Level { const TIMING: … = /* today's FETCH_*, 40, 0b10101, FRAME_LINES */; }
pub type Clock = amiga_clock::Clock<Level>;
// sf_core::level::timeline
pub type Timeline = amiga_clock::Timeline<crate::game::Job>;
pub fn of(trace: &[Tick], costs: &[u32], writes: &[(u16, u16)], job: Job) -> Timeline;
```

The shapes are a starting point; the builder may rename and re-split them, keeping three things:
the clock's and the timeline's serde bytes, every call site's arithmetic, and no Superfrog name in
`amiga-clock`. `Timeline::of` callers move to `timeline::of` (an inherent impl cannot be added to a
foreign type).

## Tests

- [ ] `level::beam`'s unit tests (`the_cpu_gets_less_of_a_fetching_line`,
      `a_blit_takes_longer_under_the_display`) pass unchanged, through `Clock<Level>`.
- [ ] The old `Clock::cpu`, `blit`, `blit_cycles` and `blitter_slot`, kept in the test module as
      references, equal the new ones over every line and slot of a frame and a spread of cycle
      counts, word counts and BLTCON0 values (all 16 channel sets, D on and off).
- [ ] Old `word_times` (with `&Fetch`) equals the new one through `|l, h| fetch.takes(l, h)` over
      the fetch set-ups the engine uses; `Blit::cycles` unchanged over sizes and channel sets.
- [ ] `is_refresh` equals `story/cpu.rs`'s `refresh` and `blitter_slot`'s refresh test for every slot.
- [ ] A `Clock` and a `Timeline<Job>` serialise (bincode) to the same bytes as before the change:
      fixed byte strings captured from the base in the first commit.
- [ ] `Timeline::run` on a recorded restart trace makes the same writes on the same lines as before.
- [ ] `purity.rs` (no float, clock or table over 16 in `amiga-clock`).
- [ ] Every check in DESIGN §4 for every task, green, no tolerance changed.

## Done when

- The level runs on `amiga_clock::Clock<Level>`; `Clock`, the blit timing (DESIGN §2.4) and
  `Timeline`'s stepping live in `amiga-clock`, which still has no sf-core dependency and passes
  purity; `measure.sh` still prints `PASS amiga-clock`.
- The `level::beam` unit tests pass unchanged, the old-versus-new sweeps and the save-state byte
  tests pass, and `python3 tools/test_architecture_docs.py`.
- The test block and DESIGN §4's per-task runs green, no tolerance changed.

## Automated checks (the worker runs these)

```
sh plans/amiga-toolkit-prep/measure.sh
python3 tools/test_architecture_docs.py
cd engine && cargo test -p sf-conform --test beam
cd engine && cargo test -p sf-conform --test frog_cases
cd engine && cargo test -p sf-conform --test alone
cd engine && cargo test -p sf-conform --test story -- --ignored
cd engine && cargo test -p sf-conform --test channels -- --ignored
```
