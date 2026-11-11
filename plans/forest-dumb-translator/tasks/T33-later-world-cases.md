# T33 — later-world-cases

**Phase:** 5 · **Depends on:** T32 · **Weight:** medium

## Goal

Show the translated program carries the game past level 1: for each of worlds 2 to 6, a case that starts at level 1's load with `world_index` set to that world on both sides, and compares the world's load and its first frames of play.

## Design sections this implements

DESIGN §2.1, §2.7, success criteria.

## Files

- `engine/conform/cases/world-2-start.toml` … `world-6-start.toml` (new), the poke support in `fd-conform/src/case.rs` (`poke = [{var = "world_index", value = 1}]` applied identically to both sides before `load_world`), `docs/14-translator.md`.

## Tests

- [ ] Full: each world's case passes its first 300 frames of play after the load with `--engine translated`, no stop.
- [ ] The poke is applied once, to both sides, at the same point (a stand-in test).

## Done when

- [ ] Five cases pass; the reading is in `docs/14-translator.md`.
- [ ] Any world whose start needs more than a poke (a state `next_world` sets) has that state set by the same mechanism, written in the case.
