# T01 — beam-pos-and-constants

**Phase:** 1 · **Depends on:** — · **Weight:** medium

## Goal

Create the toolkit crate `engine/crates/amiga-clock` holding one beam-position type and the one
definition of each frame and line length, and make every other beam position and length in the
engine a client of them: the copper, the level clock, the front end, the slot, the story, sf-paula,
and the sf-conform probes and tests. It is the draft's T1, with the crate created here rather than
in T2 (owner, 2026-10-27, DESIGN §7). No behaviour changes.

## Design sections this implements

DESIGN §2.1, §2.2, §2.8 (the base run), §3.1, §3.3 (row T01), §7 (the `measure.sh` change).

## Files

- `engine/crates/amiga-clock/Cargo.toml`, `src/lib.rs` (module names are the builder's).
- `engine/Cargo.toml`: `[profile.dev.package.amiga-clock] opt-level = 2`.
- `engine/crates/amiga-chips/Cargo.toml`, `src/copper.rs` (`Beam` → `BeamPos`, alias kept;
  `FRAME_LINES` re-exported; the copper's `LINE_CYCLES` is 227 DMA slots, not 454, so it is
  `pub use amiga_clock::LINE_SLOTS as LINE_CYCLES`, never `amiga_clock::LINE_CYCLES`), `src/blit.rs`
  (`l % 313`, `copper::LINE_CYCLES`).
- `engine/crates/sf-paula/Cargo.toml`, `src/channel.rs` (`CLOCKS_PER_LINE`, `LINES_PER_FRAME`
  derived).
- `engine/crates/sf-core/Cargo.toml`; `src/level/beam.rs` (lengths re-exported; `Clock::nominal`,
  `hpos` through `BeamPos`); `src/front/mod.rs`; `src/slot/mod.rs`, `src/slot/draw.rs`;
  `src/front/story/cpu.rs` (`LINE` is 227, from `copper::LINE_CYCLES`), `mod.rs`, `intro.rs`,
  `sim.rs`; the frame-count literals `(N * 313 + M) * BEAM_CYCLES_PER_LINE` in `level/tally.rs`,
  `front/chain.rs`, `front/menu.rs`, `front/attract.rs`; `slot/rounds.rs` `SFX_WAIT = 4 * 227`
  (four lines in slots); any other sf-core file whose length literal or import changes
  (`render/mod.rs`, `front/vbl.rs`, `level/timeline.rs`, `game.rs`, …). `level/beam.rs`'s
  `LINE_SLOTS` is `u32` where `amiga_clock::LINE_SLOTS` is `u16`.
- `engine/crates/sf-conform/examples/{vbl_probe,slot_lamps_sim,handler_end_probe}.rs`; the bare
  `227`/`313`/`454` lengths in `examples/{fit,vbl_audio,write_positions,fade_probe,fade_times,init_probe,slot_audio}.rs`;
  `tests/beam.rs`, and any sf-conform file using `copper::Beam` literals (`tests/story_draws.rs`,
  `tests/story_blits.rs`).
- `engine/crates/sf-conform/tests/purity.rs`: `amiga-clock` on `PURE` and `ALLOWED`.
- `docs/architecture/cpu-time-and-beam.md`: front matter `owns` gains `"engine/crates/amiga-clock/"`.
- `plans/amiga-toolkit-prep/measure.sh`: the beam-position row.

## Interface

```toml
# engine/crates/amiga-clock/Cargo.toml
[package]
name = "amiga-clock"
version.workspace = true
edition.workspace = true
publish.workspace = true

[dependencies]
# Save states: the level's clock and timeline are part of a game's saved state.
serde = { version = "1", features = ["derive"] }
```

```rust
// amiga_clock
pub const FRAME_LINES: u16 = 313;          // a long PAL frame
pub const SHORT_FRAME_LINES: u16 = FRAME_LINES - 1;
pub const LINE_CYCLES: u32 = 454;          // CPU cycles a line, two a DMA slot
pub const LINE_SLOTS: u16 = 227;           // DMA slots a line; h runs 0..=$E2

/// A beam position: a line (past FRAME_LINES when work runs on past the frame's end) and CPU
/// cycles into it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash,
         serde::Serialize, serde::Deserialize)]
pub struct BeamPos { pub line: u16, pub cycle: u32 }

impl BeamPos {
    pub const fn new(line: u16, cycle: u32) -> BeamPos;
    pub const fn from_slot(line: u16, h: u16) -> BeamPos;  // cycle = 2h
    pub fn slot(self) -> u16;                  // cycle / 2 (the copper's and simulator's h)
    pub fn paula_hpos(self) -> u16;            // slot clamped to LINE_SLOTS - 1, as Clock::hpos
    pub fn nominal(self) -> u32;               // line * LINE_CYCLES + cycle, as Clock::nominal
    pub fn from_nominal(t: u32) -> BeamPos;
    pub fn dma(self) -> u32;                   // line * LINE_SLOTS + slot
    pub fn from_dma(t: u32) -> BeamPos;
    pub fn plus_slots(self, n: u16) -> BeamPos; // the copper's Beam::plus
}
```

`copper::Beam` becomes `pub use amiga_clock::BeamPos as Beam` (or `pub type`), its `h` reads
`slot()`, its constructions `from_slot`; the copper's `word()` packing stays the copper's as a
function. `BeamPos` derives `Ord` on (line, cycle), which orders as `Beam`'s (line, h) did, since
cycle is 2h for every copper position. Whether the level's `Clock` stores a `BeamPos` or keeps its
two fields and converts is T02's; here it converts.

`measure.sh`'s row becomes, scanning `engine/crates` and counting the new type:

```sh
echo "beam-position types/functions: $(rg -n 'pub struct BeamPos|pub struct Clock|pub struct Beam\b|struct Cx\b|struct Cx<|^fn spend|pub fn beam\(' engine/crates --type rust | wc -l | tr -d ' ')"
```

## Tests

- [ ] Before the first change, on the task's base: the test block and DESIGN §4's sf-conform runs;
      the result recorded in `FINDINGS.md` (DESIGN §2.8).
- [ ] `BeamPos` round trips: `from_nominal(p.nominal()) == p`, `from_dma(p.dma())` for every
      slot-aligned position of a frame, `from_slot(l, h).slot() == h`; `paula_hpos` clamps at the
      line's end as `Clock::hpos` does; a position past `FRAME_LINES` converts without wrapping.
- [ ] `BeamPos` ordering equals the old copper `Beam` ordering over a sweep of positions.
- [ ] `level::beam` unit tests pass unchanged.
- [ ] The copper tests (moved with amiga-chips) and `story/sim.rs` tests pass unchanged apart from
      `Beam { line, h }` literals rewritten to `from_slot`.
- [ ] `purity.rs` passes with `amiga-clock` on `PURE`; `python3 tools/test_architecture_docs.py`.
- [ ] Every check in DESIGN §4 for every task, green, no tolerance changed.

## Done when

- `amiga-clock` exists under DESIGN §3.1; `measure.sh` prints `PASS amiga-clock`, frame/line
  constants ≤ 3 (all in `amiga-clock`), and the other M2 rows unchanged.
- One `BeamPos`; the copper's `Beam`, `story/sim.rs` `beam` and the level clock's position read
  through it; no other length definition or length literal in non-test code (DESIGN §2.2).
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
