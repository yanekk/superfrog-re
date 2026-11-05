# T65 — level-start-draw

**Phase:** 2 · **Depends on:** T55 · **Weight:** small

Added by T55 with the owner's yes (2026-11-05): `level_init` draws the whole first view before play (`h0+$62e` its tiles, `h0+$6c4` its `collision_map`), and no task ported it. T55's edge redraws only add the row or column the view scrolls to, so `Game::new(Start::Level1)` starts with an empty `collision_map`, and an engine run without the original (the app, the planner) has no solid ground on screen. Cases are unaffected: they transplant `collision_map` from the original. T13 and T20 wait for it (`blocks T13, T20`).

## Goal
Port `level_init`'s first draw of the view so a game started on its own holds the original's `collision_map` at the level sync point, and the first view's tiles as blits for the picture. The routines, read from the listing (a reading until the rig shows it):
- `h0+$62e` with its per-cell helper `h0+$672`: 23 columns by 16 rows of tiles from the tile map's top-left, copied by the CPU (not the blitter) into the play screen at `h3+$398`, `$e60` a band, 2 bytes a column.
- `h0+$6c4` with its per-cell helper `h0+$708`: the same 23 by 16 cells' shapes from `_GenN-1.Mapa`, by the shape map, into `collision_map` at `h3+$fa50`, `$2e0` a band, 2 bytes a column, 16 words `$2e` apart.

## Design sections this implements
§2.1, §2.3, §4.1, §4.3.

## Reuses
T55's `level/redraw.rs` (`LevelImage`, `Io`, `Blit`, the band offsets) and `level/collision.rs`; T32's rig; `toolkit/docs/porting-routines.md`.

## Files
`fd-core/src/level/redraw.rs` or a new `fd-core/src/level/start.rs`, `fd-core/src/game.rs` (`Game::new` runs it), `fd-core/src/ported.rs` (appends), `re/symbols.toml` (the four as `func`), `fd-conform/tests/` (a rig test file and the sync-point check), `docs/` note, `smoke.toml`.

## Interface
`Game::new(data, Start::Level1)` holds the first view's `collision_map`. The tiles copied by the CPU are memory writes, not blits: how the engine hands them to the picture (T10) is this task's to decide and record.

## Tests
- [ ] Each of the four routines through the rig against the original, 10 000 states from a generator written for them (Muse through T61's service, or by hand), memory and the registers callers read compared.
- [ ] `Game::new(Start::Level1)`'s `collision_map` equals the original's at the level sync point, byte for byte over what `CollisionMap` keeps.

## Done when
- [ ] The four routines have `PORTED` entries and `func` symbols, and pass the rig.
- [ ] Follows `toolkit/docs/porting-routines.md`: interruptible points listed in the `docs/` note before porting; each fix carries a test that fails without it.
- [ ] Whether the creature slots and animated cells are also non-empty at the sync point is measured and written in the `docs/` note; if they are, the open question goes to the owner, not into this task's scope.
- [ ] Smoke labels for the new test files; the sync-point check is smoke.
