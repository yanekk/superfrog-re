# T100 — front-vbl-timing

**Phase:** 2 · **Depends on:** T37; blocks T21 · **Weight:** medium

Added by T37 (owner, 2026-10-13). The front end's vertical blank timed by its work, so a fade's
colours reach the copper on the original's line between levels.

## Goal

`frog-pit-w1-1`'s picture compared on every frame (its `picture` exclusion, frames 308–323,
gone), and the case run on through the second death and restart (900 frames).

## What is known (T37, measured on `frog-pit-w1-1`)

- On the level's copper list the front's VBL steps a fade a channel at a time
  (`front::fade::due_channels`, the history snapshots in `Front::step`): fade in 95 cycles a
  channel looked at, 96 more if changed; fade out 71 and 119; from 16 lines after the handler
  starts. The copper reads the play field's 32 colours on line `$34` (`$e0`).
- What is left is where the handler starts the fade: the oracle's first audio write is on line 4
  or 5 for the same work, constant over a music row (rows every 6 frames), line 6–7 on the tick
  before a row (`fetch_rows`), +1 line on keyboard-poll frames (INTENA `$0008`), more on new-note
  frames (each DMACON stop ~1.3 lines, each other write ~0.13: the level's `AudioStop`,
  `AudioWrite` costs). Colour 14 is read half stepped (R only) when the handler ends on line 56,
  fully stepped at 54–55.
- Frame 308: `level_start`'s fade set-up runs to line 30 with interrupts off (INTENA `$4000`
  on 307:299, `$c000` on 308:30); the engine's ends on 36.
- Running to 900 frames: the second exit fade's first frame (613) and the second init's end
  (641–642, one blit across the frame's end) slip by the same handler lengths.
- T94 builds the slot screen's interrupt length from the same parts (`slot::interrupt_lines`);
  share the model if it fits both.

## Interface

None new: the front's vertical blank (`Front::step`) takes as long as its work, fades included.

## Files

- `engine/crates/sf-core/src/front/{mod.rs,vbl.rs,fade.rs}`, `audio/` (what the music tick did)
- `engine/conform/cases/frog-pit-w1-1.toml`

## Tests

- [ ] `frog-pit-w1-1` with no `picture` exclusion, 900 frames; the front cases still green.

## Done when

- [ ] The above, in the test block (`frog_cases::frog_pit_w1_1`).
