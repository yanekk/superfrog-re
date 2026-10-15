# T155 — w2-completions-green

**Phase:** 2 · **Depends on:** T122, T127, T131, T132; blocks T18 · **Weight:** medium

Added by T122 (owner, 2026-10-15). T122 committed the W2 completion cases outside the test block,
red only from engine faults other tasks own; this task makes them green once those land.

## Goal

1. Re-run `sf-conform scan` on each W2 completion case once T127, T131 and T132 (and T166–T168,
   which add the W2-2, W2-3 and W2-4 cases) have landed. Map each red frame left to its owner, or
   fix it here if nobody owns it.
2. Move every green case into the test block: a test per case in
   `engine/crates/sf-conform/tests/frog_cases.rs`, as `complete_w4_3` does (green to the frame before
   the next level; on that frame `level_outcome` is `Completed`).
3. Check through `run_with` (as `enemies_w2_cases.rs` does) what each completion meets. For
   `w2-1-complete`, on the original (`examples/script_seen`): the mace from frame 2029, hearts lost
   at 2061 (the mace) and 2365, the spikes from 2305, the crumbling ledge from 1646, the bouncer
   from 1678, a spud stun at 795 (no reward follows it: T168 carries the stun-then-stomp). Carried
   from T43 through T122: a crumbling ledge breaking under the frog, checked through `run_with`,
   here or on whichever W2 case (T166–T168) breaks one. T166–T168 name theirs.

## The red frames (T122's `sf-conform scan`, 2026-10-15)

- `w2-1-complete` frame 1094: the frog's sprite pose differs, every variable equal (the
  spring-pad/sprite class: T127, T131).
- `w2-1-complete` frame 1508: `level_copper` at $00727c, original $e00002, engine $e00001, after a
  lag (T131/T132's completion-fault class).
- Frame 2709 is the next level's (T21's): the case ends at 2708 by design.
- A second W2-1 route (not committed) also met the exit's `BPLCON1` copper fault (T131) and
  `panel_bitmap` at 2029.

## Files

`engine/conform/cases/w2-*-complete.toml`, `engine/crates/sf-conform/tests/frog_cases.rs`; fixes, if
any, where the owner's would land.

## Done when

- [ ] Every W2 completion case (`w2-1-complete` and T166–T168's) green with `resync = false` and no
      exclusions, each with a test in the test block that also asserts `Completed` on its last frame.
- [ ] The encounter checks in goal 3 pass through `run_with`.
