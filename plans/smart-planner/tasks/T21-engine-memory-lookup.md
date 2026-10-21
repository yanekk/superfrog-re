# T21 — engine-memory-lookup

**Phase:** 1 · **Depends on:** T04; blocks T20 · **Weight:** light

Added by T04 (owner, 2026-10-21): on a quiet machine 11 of the 24 maps take over 5 min to build
on 10 threads (W5-4 566 s, W1-4 440 s), and the profile puts the time in the engine, not the map.

## Goal

The engine's memory access finds a byte's variable without a binary search, so a map builds
faster. Performance only, as T19 was: the game state after every frame stays exactly what it is
today, for `step`, `step_paced` and `step_unseen` alike. No game mechanic or faithful-bar change
(DESIGN §1 Stance).

## What T04 found

A `sample` of a one-thread W2-4 map build (`map_stats --threads 1 W2-4`, 6.2 k frames/s; 10
threads 25 k): `sf_core::layout::index_of` (`layout.rs`, `LAYOUT.partition_point` on every
access) ~35% of busy samples, then `State::byte`, `Bus::set_b`, `beam::Clock::blit_cycles`,
`Bus::b`, `State::put`. The planner's own code does not show.

## Design sections this implements

DESIGN §5 (the measured speeds); the 5-min build line of T04's "Done when".

## Files

- `engine/crates/sf-core/src/layout.rs`, `engine/crates/sf-core/src/state.rs` (and `mem.rs` if the
  bus path needs it)
- a test beside the code (below)

## Interface

None changes: `State`'s and the bus's accessors keep their signatures; only how a byte's layout
entry is found changes.

## Tests

- [ ] Beside the code: the new lookup agrees with `index_of`'s answer for every address the layout
      covers and for addresses just outside each entry.

## Done when

- [ ] Every conformance case in `engine/conform/cases/` gives the same result as before the change
      (the full `sf-conform` run, faithful and strict, compared case by case), and `sf-conform`'s
      `tests/step_unseen.rs` passes.
- [ ] Per-level map build time and frames for all 24 levels, 10 threads, quiet machine, in one
      FINDINGS row (`cargo run --release -p sf-plan --example map_stats`).
- [ ] Builds still over 5 min after the change are recorded in FINDINGS and accepted (owner,
      2026-10-21); the move set is not shrunk.
