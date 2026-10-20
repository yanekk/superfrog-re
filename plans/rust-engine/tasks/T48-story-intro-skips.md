# T48 — story-intro-skips

> **Amended** by the owner, 2026-10-15. "Green", "matches" and "frame-exact" below mean DESIGN §2.1's
> faithful checks (T180's default mode); a completion case is also judged alone (check 4, T182).
> Re-sync pins, `resync = false` and `--strict` are not required. Dependencies on dropped tasks
> are gone; PLAN.md's table is the current list. Where this doc and DESIGN §2.1 disagree, §2.1 wins.

**Phase:** 2 · **Depends on:** T46, T47; blocks T21 · **Weight:** medium

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
