# T159 — attract-demo-cycles

**Split from:** T33. Carries T33's "each of the three `demo_index` entries, and an abort by fire"
tests and "the full cycle in `sf-conform -- all`".

**Phase:** 2 · **Depends on:** T158 · **Weight:** medium

## Goal

Three whole attract cycles (`demo_index` 0, 1, 2) green, and fire in the middle of a demo
ending it back to the title.

## What T33 found (oracle, 2026-10-15)

- **Case frame 8023, the second demo's load.** The engine reached it on line 246, still busy
  with `blit_restore` (0x100 lines, six planes) begun on line 20 of the frame before: `Blit::cycles`
  over-counts an A-to-D copy (the original's finishes inside its frame). The original's load ends
  on line 87, before the copper's DMACON on 107. Fix used: `Cx::loaded` sets `cx.line` to the
  runner's load line outright instead of the larger of the two (the work before a load happened
  long before in the original). The ordering then matched; check the first demo and T34's loads
  still fit.
- **Case frame 8045, the second `demo_init`.** The original's `DMACON $8400` is 51 lines later
  than the engine's: its sub-records part (h0+$18ef8) takes 147 sampled lines, against about 7 in
  the first demo, because the first demo's bob records are still in the draw list. The engine's
  `SubRecords` (1000 a record) is fitted to a run with one record. Refit it per record and per
  sub-record copied (`0x23` and `0x46` words), checking the restart cases.
- Cases to add: `attract-demo-3.toml` (power-on, about 15 500 frames, three demos; slow: outside
  the test block, in `all`), `attract-abort.toml` (fire held a few frames mid-demo: `demo_play`
  ends it, `level_frame_loop` stops on fire, `LevelOver` back to the title).

## Interface

None new, as T33: the demo runs inside `Game::step`, reached from T20's attract cycle under
`Start::PowerOn`.

## Files

`front/mod.rs`, `level/beam.rs`, `level/bobs.rs`; `conform/cases/attract-*.toml`.

## Tests

- [ ] `attract-demo-3` and `attract-abort` green; the picture frames needing T127 excluded with
      `task = "T127"`.

## Done when

- [ ] The above; `sf-conform -- all` runs them.
