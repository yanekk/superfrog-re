# T38 — story-intro-skips

**Phase:** 2 · **Depends on:** T36, T37; blocks T21 · **Weight:** medium

Split from T32 (owner, 2026-10-13).

## Goal

Every skip input the intro reads, at several points, frame-exact: fire (port 2) and the left mouse
button (port 1) on the logo (if `d3_c02` reads them), on a text page (`WaitOrFire`), during a scene's set-up and play (the
handler sets `intro_skip`; `SkipCheck` after each scene goes to `END`), and on the "insert disk 1"
page, each through to the game's title.

## Design sections this implements

DESIGN §1, §2.1.

## Interface

None new: the cases script the inputs from `power-on-disk3`.

## Files

- `engine/crates/sf-core/src/front/story/` (`boot.rs`, `vbl.rs`, `intro.rs`).
- `engine/conform/{scripts,cases}/story-skip-*`.

## Tests

- [ ] One case per input and point above, scripted, green.

## Done when

- [ ] All `story-skip-*` cases green.
