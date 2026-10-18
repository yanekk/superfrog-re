# T210 — window-fits-the-game

**Phase:** 3 · **Depends on:** T27; blocks T29 · **Weight:** light

Added by T27 with the owner's yes (2026-10-18).

## Goal

The window is sized for 320×256 (`sf-app/src/window.rs` `GAME`), but a level's and the progress
map's display window is 334×261. At `Scale::N(n)` the level does not fit `n ×` the window, so it
is drawn at `n − 1` (a 2× window shows W3-1 at 1×, centred in black), in 4:3 and widescreen alike
(T27 drill, FINDINGS). T23's window tests shot only the 320×256 boot screens, so they never saw it.

1. Find the largest display window the game opens on any screen (levels, map, title, story intro,
   slot machine, bonus level, end sequence), from the engine's pictures headless, not a guess.
2. Size the window and its 1× minimum for it, so `Scale::N(n)` shows every screen at `n`; a
   smaller screen is centred with black around it (DESIGN §2.7).
3. Widescreen (T27, `window::extra_columns`) takes its extra columns from that height, so 16:9 is
   still filled; recheck T27's 67 and say in PROGRESS what it became.

## Design sections this implements

DESIGN §2.7.

## Files

- `engine/crates/sf-app/src/window.rs`, `main.rs` as needed.
- `engine/crates/sf-e2e/tests/window.rs`, `view.rs`: the sizes they assert.

## Interface

None new: `window::initial_size`, `min_size`, `frame` keep their signatures.

## End to end (the worker drives this)

- suite: `sf-e2e` · sizes: 1×, 2×, 3×, 2× widescreen
- [ ] a shot on a level (W3-1 by Continue, as `view.rs` reaches it) at 1×, 2×, 3× and 2×
  widescreen is drawn at exactly that scale (`assert_integer_scaled`), not one less.
- [ ] a shot on the title at the same sizes: the scale asked for, centred.

## Done when

- [ ] e2e green, `window.rs` and `view.rs` included.
- [ ] Every screen shows at the scale chosen; the largest display window and where it was found are
  recorded in PROGRESS.
