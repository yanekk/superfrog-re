# T131 — w4-completion-faults

**Phase:** 2 · **Depends on:** T103, T121, T127; blocks T18, T21 · **Weight:** heavy

Added by T103 (owner, 2026-10-14). T103 played W4-1…W4-4 from their starts to their exits and
through the tally, every chunk checked on the original, but the engine diverges from the original
on a few frames of each, from faults outside W4's objects. The exit fade's and the data bus's
faults will meet every level's completion (T104, T106, T115, T122, T126), hence `blocks T21`.

## Goal

1. **Map each fault to the task that already owns it, and re-run once those have landed:** lag
   faults to T101 (✅, re-run as is), bob-draw cuts to T121, the frog's sprites to T127
   (restore-sprite-wait). Say in `FINDINGS.md` which fault went where and which went green.
2. **Fix only what nobody owns.** Likely the exit fade and the `clr.w` data-bus word below.
3. **Make the four `complete-w4-*` cases green** with `resync = false` and no `[[exclude]]`, and
   add one test per green case to `engine/crates/sf-conform/tests/frog_cases.rs`.

## The faults (T103's `sf-conform scan`, 2026-10-14)

`complete-w4-3`'s frames 8886 and 10429 are from T103's first W4-3 route (11016 frames,
`git show c2e8a9d:engine/conform/scripts/complete-w4-3.txt`); the committed route is green and
meets neither, so re-run them from that script.

- **The exit fade, palette step a frame early.** `complete-w4-1` frame 2149, four frames after
  `level_state` 1 (2145): the engine writes that frame's fade step into the level copper's palette
  (`level_copper+$2d4`) before the copper reads it, the original after; 83 pixels one colour step
  off (`$bbb` for `$ccc`), every variable equal. The fade steps from the vertical blank
  (`level_exit_fade`, `level/exit.rs`): the handler's length at the level's end. `tally-w1-1`
  (a forced exit) does not show it. `complete-w4-2`'s copper differs from 9237, its exit, on.
- **`clr.w AUD3VOL` at a sample stream's end** (h0+$14890, `audio/sfx.rs` `stream_tick`):
  `complete-w4-3` 10429 (original `$fbfb`), `complete-w4-2` 2383 (`$0605`) and 5171 (`$e1ff`);
  the engine always writes `$33fc`, the next opcode (`audio::clr`, T14). The 68000's `clr` reads
  first, and the reference emulator's bus then holds the last word on it: here a DMA word, not the
  prefetch. Needs the beam's DMA slot at the write's cycle.
- **The frog's sprite on a spring pad.** `complete-w4-3` frame 8886, on time: the frog's sprite
  differs on rows 100–134 while it bounces on the pads at 148–157,36 (T40's open row; T127's?).
- **W4-2's busy passes.** `complete-w4-2`: 1657 (picture), 1927 and 4964 (`vbl_count` 1 against
  2 after a lag), 7731 (`playfield_buffer_0`), 8054 (`level_copper`), 9097 (`bob_draw_list`).

## Interface

Nothing new expected: fixes land in the existing level frame, beam, audio and exit code.

## Files

`engine/crates/sf-core/src/level/{exit.rs,frame.rs,beam.rs}`, `engine/crates/sf-core/src/audio/`,
`engine/conform/cases/complete-w4-*.toml`, `engine/crates/sf-conform/tests/frog_cases.rs`.

## Tests

- `sf-conform scan` of each `complete-w4-*` case: no frame differs.
- Each green case in `frog_cases.rs`; every other case in the test block stays green.

## Done when

- [ ] `complete-w4-1` … `complete-w4-4` green against the oracle with `resync = false` and no
      exclusions, in the test block.
- [ ] Each fault above recorded as fixed here or by its owner, in `FINDINGS.md`.
