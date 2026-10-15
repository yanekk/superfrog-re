# T162 — attract-demo-level-start

**Split from:** T33. Carries T33's first demo frame for frame, to the level's exit.

**Phase:** 2 · **Depends on:** T161 · **Weight:** medium

## Goal

The first demo's `level_start` and its 721 frames of play green in `front-attract` (case frames
3314 to about 5090), the frog driven by the recording (`demo_play`, ported in T14).

T33's code for this is in `plans/rust-engine/T33-wip.patch` (see T160).

## What T33 found (oracle, 2026-10-15)

- **3314, window.** `level_start`'s fade set-up starts on line 299 with interrupts off and runs
  into the next frame. The oracle's frame ends once the CPU leaves the instruction it is in (a
  divide), after the next list's first copper moves: the level list sets DIWSTRT in its second
  move, so the original's frame already has the level's window. Fix: after `run_main`, when the
  top frame waits on `Yield::Masked` and `cx.cop1lc` differs from the frame's list, rewindow from
  the first 8 moves of `render::copper_writes(cx.cop1lc)` on line 0 (the title list's DIW, its
  36th move, does not show: case frame 5151).
- **3315, fade.** The handler taken late (interrupts on at line 30) starts its fades 4 lines after
  `vbl::fade_start`'s estimate (`$12a14` reads line 36, not 35; three lines after the wait, not
  170 cycles): a `LATE_FADE_LINES` = 4 offset only when `vbl_line > 1` and fading on the level
  list. Only offsets 4..12 make 3315 green. The on-time estimate is also ~2.6 lines early but
  harmless there; compare T130.
- **3323, fade.** An `E9`/`ED` retrigger between rows makes `music_dma` wait for DMA. Add
  `TickWork::retrigger` (E9: `RETRIG` == 1; ED: low nibble == the new tick) and a
  `RETRIGGER_DMA` cost in `fade_start`: 500 and 775 fail, 600 and 700 pass; 650 used.
- **3340.** The keyboard handover (T160) must already be in.
- **4995 (the original's 10035)** needs T127: the frog bumps a block in the blank (a coin
  spawns), so `place_frog_sprites` runs a second time on line `$1e`, after the copper's sprite
  read on `$20`, and the original waits (h0+$19c92). With `sprite_wait` called there it is green.
  Until T127, exclude the picture of that frame (`task = "T127"`).

## Interface

None new, as T33: the demo runs inside `Game::step`, reached from T20's attract cycle under
`Start::PowerOn`.

## Files

`front/mod.rs`, `front/vbl.rs`, `audio/music.rs`, `conform/cases/front-attract.toml`.

## Tests

- [ ] `front-attract` green through the first demo's exit fade (about case frame 5100), with the
      4995 picture excluded for T127.
- [ ] `frog-pit-w1-1` (it has a late handler with a fade, front frame 31), the `slot-*` cases
      that `fade_start` times, `front-boot` and `ending` still green.

## Done when

- [ ] The above.
