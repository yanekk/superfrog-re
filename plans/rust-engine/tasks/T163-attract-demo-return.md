# T163 — attract-demo-return

> **Amended** by the owner, 2026-10-15. "Green", "matches" and "frame-exact" below mean DESIGN §2.1's
> faithful checks (T180's default mode); a completion case is also judged alone (check 4, T182).
> Re-sync pins, `resync = false` and `--strict` are not required. Dependencies on dropped tasks
> are gone; PLAN.md's table is the current list. Where this doc and DESIGN §2.1 disagree, §2.1 wins.

**Split from:** T33. Carries "the return to the title so the cycle repeats" and T33's 5 000-frame
test.

**Phase:** 2 · **Depends on:** T162 · **Weight:** small

## Goal

After the demo, `title_screen` again: its load frame and the overlay's fade-out green, and the
cycle runs on (T20's doc asked for 5 000 frames of attract mode, demo included).

T33's code for this is in `plans/rust-engine/T33-wip.patch` (see T160).

## What T33 found (oracle, 2026-10-15)

- **Case frame 5121 (the original's 10469), the title's load after the demo.** The engine drew
  the lines above the load from memory as it was before the part that loads, so the overlay put
  up in that same part showed as garbage; the original put it up 300 frames earlier. Fix in
  `Front::run_main_split`: when a part makes the frame's load (`cx.load_point` set during it),
  drop that part's `before` entry.
- **Case frame 5151, window.** See T162: only the next list's first 8 moves count.
- With these and the T127 wait, `front-attract` was green to 7 000 frames.

## Interface

None new, as T33: the demo runs inside `Game::step`, reached from T20's attract cycle under
`Start::PowerOn`.

## Files

`front/mod.rs`; `conform/cases/front-attract.toml`; `sf-conform/tests/front.rs`;
`docs/13-front-end.md` (the demo's place, its input format: three words a step, start frame,
joystick and fire `$f000`, frames; `$ffff` ends; fire or the end stops it).

## Tests

- [ ] `front-attract` raised to 5 200 frames (the title back), in the test block as
      `the_attract_cycle_matches_the_original` in `front.rs`.

## Done when

- [ ] The above, and docs/13's "attract demo (T33)" mentions point at what is built.
