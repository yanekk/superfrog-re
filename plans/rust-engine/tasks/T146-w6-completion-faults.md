# T146 — w6-completion-faults

**Phase:** 2 · **Depends on:** T104; blocks T18, T21 · **Weight:** medium

Added by T104 (owner, 2026-10-14): the engine faults T104's W6 completion cases meet. Their
scripts were planned and checked chunk by chunk on the original, so they are real completions;
the engine leaves the original's run on these frames.

## Goal

Every `enemies-w6-<n>[-alt]-complete` case T104 left outside the test block green with
`resync = false` and nothing left out, and in the test block (`tests/enemies_w6_cases.rs`).

First check each fault against the tasks that already own its kind, and re-run after they land:
T121 bob-draw-cut for the playfield buffer, T131 (W4's lag and copper faults, from T103) for the
copper list; the AUD3VOL value looks like a data-bus or uninitialised read, so check it against
T131's `clr.w` data-bus item. Fix only what is left.

## What is known (T104)

- `enemies-w6-1-complete` diverges at frame 530, after a lag: `playfield_buffer_0` (h0+$17e9c)
  differs from +$a0 on, the bob drawer's picture memory.
- `enemies-w6-4-complete` diverges at frame 154, after a lag: `level_copper` (+$3b4, +$3c4). The
  T40 case `enemies-w6-4` is red the same way at 1120 (`level_copper` +$2ac onward).
- `enemies-w6-3-alt-complete` diverges at 5939: Paula write #4 of the frame, `AUD3VOL`, original
  `$f4fd`, engine `$33fc` (beam line 7), every other write equal.
- `enemies-w6-2-complete` matched the original through 7 488 frames.
- T104's PROGRESS note names which cases are outside the test block and the frame each stops at.

## Interface

None new.

## Files

- `engine/conform/cases/enemies-w6-*-complete.toml` (cases, unchanged scripts)
- `engine/crates/sf-conform/tests/enemies_w6_cases.rs`
- wherever each fault lives in `engine/crates/sf-core/src/`

## Tests

- [ ] each red case above green in `sf-conform -- all`, and in `enemies_w6_cases.rs`.
- [ ] `enemies-w6-4` green to its 1 400 frames (or its own blocker named, if it is T102's picture race).

## Done when

- [ ] Every W6 completion case is in the test block.
