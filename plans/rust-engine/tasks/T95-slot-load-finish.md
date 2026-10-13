# T95 — slot-load-finish

**Split from:** T92

**Phase:** 2 · **Depends on:** T92; blocks T93 · **Weight:** medium

Carries all of T92's "Done when". T92's first session stopped at pir's context line with a draft
committed but never compiled (commit "WIP T92, untested, does not compile yet"; its message has
the full oracle timeline). Read T92's and T72's docs first: they are the spec.

## Goal

Finish T92: the engine resumes at the slot sync point and matches the original frame for frame
through the loads, the overlay fade-out, the music start, the count, `slot_init`, `slot_digits`
and the fade-in set-up, up to the attract loop's first raster wait (h0+$4d24).

## Already on the branch (draft, untested)

- `engine/crates/sf-core/src/slot/mod.rs`: `slot::Routine`, `SlotOver`, `resume`, the loads,
  lamps (26 blits), `print_text` (h0+$6f3a), `slot_init`, `slot_digits`.
- `front/common.rs`: `MusicStart::keeping_module(ret, 18)`. `front/mod.rs`: `Routine::Slot`,
  `Routine::SlotOver`. `engine/conform/cases/slot-load.toml` (17 frames).
- `sf-conform/examples/slot_dbg.rs`: scratch probe, the original per frame (chain, CPU writes,
  blits; `PAULA=1`). Delete it before the last commit unless it earns its keep.

## Left

- `pub(crate) mod slot;` in `sf-core/src/lib.rs`; make the draft compile; fix it where wrong.
- `Scene::Slot { coins_in }` (`scene.rs`, coins from h0+$49c6), run by `Front` in `game.rs` like
  `Scene::Front`.
- `Front::resume`: try `slot::resume`; transplant the sound driver with
  `audio::files_for(long h0+$40da, long h0+$9ec)` there (not the title's); the copper list at
  the sync point is the overlay's/BLANK.
- Run `slot-load`, calibrate the cycle constants so Paula write order matches.

## What the oracle shows (T92, 2026-10-13)

Engine frames 0–16 are the original's 758–774. The screen is blank throughout: the case turns
on memory and Paula write order. Lamps' blits (773, lines 45–99, INTENA $8010 at 99) must end
before the copper's DMACON $0020 at line 107. The score's print starts at line 299 of 773 and
ends in 774. At frame 0 the engine first differed in `playfield_clean` from +$4d00
(the overlay, `OVERLAY_AT`).

## Files

`engine/crates/sf-core/src/{lib.rs,scene.rs,game.rs,slot/mod.rs,front/mod.rs,front/common.rs}`,
`engine/conform/cases/slot-load.toml`, `engine/crates/sf-conform/tests/front.rs`.

## Interface

`Scene::Slot { coins_in }`; `Routine::Slot(slot::Routine)`, `Routine::SlotOver`.

## Tests

- [ ] `slot-load` green, in sf-conform's tests (`tests/front.rs`).
- [ ] Existing `front-*` and `ending` cases still green.

## Done when

- [ ] `slot-load` green: vars, picture and Paula through the frame the attract loop starts.
