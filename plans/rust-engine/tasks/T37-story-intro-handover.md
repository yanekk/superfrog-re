# T37 — story-intro-handover

**Phase:** 2 · **Depends on:** T36; blocks T21 · **Weight:** medium

Split from T32 (owner, 2026-10-13).

## Goal

The intro's end, frame-exact: the "insert disk 1" page (`END` in `story/scenes.rs`: c38e waits
for fire, the disk read, the fire release, `text_out`), then the hand-over to the game, which starts
as T20's `Start::PowerOn` does (`Game::step_paced` switches `Story` to `Front::power_on` on
`Stage::Over`). The oracle inserts disk 1 when the original's chain reaches h0+$310
(`sync::step_inserting`).

## Design sections this implements

DESIGN §1, §2.1.

## Files

- `engine/crates/sf-core/src/front/story/` (`scenes.rs` `END`, `intro.rs`, `mod.rs`),
  `engine/crates/sf-core/src/game.rs` (the switch to the game, its load re-sync).
- `engine/conform/{scripts,cases}/story-*`.

## Tests

- [ ] From the intro's start, untouched, to the title screen of the game started from disk 1,
      loads re-synced as T20 does.

## Done when

- [ ] The case is green; docs/13 has the hand-over.
