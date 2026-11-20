# T15 — items-and-scoring

**Phase:** 2 · **Depends on:** T14 · **Weight:** medium

## Goal

Everything in a level that is not an enemy: coins, fruit and other pickups, power-ups (wings, speed
and the rest T07 lists), score, lives, the level timer, the status panel's contents, death and
respawn, the exit and the level-complete tally. Registers its object types into T14's dispatch.

## Design sections this implements

DESIGN §2.1, §2.11 (crash-only fixes, if found).

## Files

- `engine/crates/sf-core/src/items/{mod.rs,pickups.rs,powerups.rs}`
- `engine/crates/sf-core/src/level/{rules.rs,exit.rs,death.rs}`
- `engine/crates/sf-core/src/render/panel.rs` (contents)
- `engine/crates/sf-core/tests/items_routines.rs`, `engine/conform/{scripts,cases}/items-*`

## Interface

New object kinds via `register`; `LevelOutcome { Completed { tally }, Died, TimeUp, GameOver }`
returned by the level scene to the flow (T21).

## Tests

- [ ] routine conformance for each pickup and power-up handler.
- [ ] frame conformance: collect every pickup type at least once; each power-up active until it ends;
      lose a life to an enemy-free hazard and to time-up; game over; finish W1-1 to the tally.
- [ ] score, lives and timer variables equal every frame on all those runs.

## Done when

- [ ] Cases green; W1-1 completion run in `sf-conform -- all`.
- [ ] Exclusions from T14's case files that this task covers are removed.
- [ ] `LevelOutcome` is what the level scene returns.
