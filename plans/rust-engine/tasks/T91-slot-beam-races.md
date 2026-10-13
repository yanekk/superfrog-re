# T91 — slot-beam-races

**Split from:** T72

**Phase:** 2 · **Depends on:** T72 · **Weight:** light

Carries T72's "Done when" item for the beam-race excuse (T72 doc, "Beam races"), with no slot
code: the runner's half, so T93/T94 only have to fill `FrameOut::beam_races`.

## Goal

A pixel that differs only on lines a step of the engine's wrote within 4 lines of the beam passing
them is excused (owner, 2026-10-13), counted and printed, never silently.

## What to build

- `FrameOut::beam_races: Vec<(u16, u16)>` (sf-core `game.rs`): raster-line ranges a drawing step
  wrote while the beam was within 4 lines of them. Empty everywhere today; the front end's
  `FrontOut` carries it through.
- `sf-conform`: `compare::picture_except(ours, theirs, rows)` (rows = picture rows to skip, from
  the ranges and the picture's `top`), used by `compare::frame` when the frame has races; a frame
  that matches only with the excuse counts in `Outcome::raced`. `report`/`main.rs` print
  `beam races excused at N frame(s)` after a green run, as they print re-syncs.
- `engine/conform/README.md`: a paragraph on the excuse, its 4-line window, and that it is
  reported.

## Files

`engine/crates/sf-core/src/{game.rs,front/mod.rs}`, `engine/crates/sf-conform/src/{compare.rs,
report.rs,lib.rs,main.rs}`, `engine/conform/README.md`.

## Interface

`FrameOut::beam_races: Vec<(u16, u16)>`; `compare::picture_except(ours, theirs, rows)`;
`Outcome::raced`.

## Tests

- [ ] sf-core unit test: the race-row computation (a step written on lines 120..130 with the beam
  at 118 races rows 120..130; with the beam at 110, none).
- [ ] sf-conform unit test: `picture_except` ignores exactly the given rows; a differing pixel
  outside them still diverges.

## Done when

- [ ] Both tests green; every existing case unchanged (nothing races yet).
