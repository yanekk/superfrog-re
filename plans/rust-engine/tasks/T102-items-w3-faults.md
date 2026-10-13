# T102 — items-w3-faults

**Phase:** 2 · **Depends on:** T101; blocks T21 · **Weight:** medium

Added by T84 (owner, 2026-10-14). T84's W3-1 item case `items-shield-w3-1` (committed, not in the
test block) matches the original on all 1 130 frames but two engine faults. Neither is in the
pickups.

## Goal

1. **Start by re-running the case once T101 has landed.** The frame-244 fault is probably one of
   T101's lag faults; if T101 fixes it, only goal 3 is left.
2. **Frame 244, after a lag (the first pad's bounce).** Frames 236–243 are all late (cut in phases
   5, 11, 5, 5, 5 at line 89, 5, 13, 5); 244 is on time. There `playfield_buffer_0` differs in two
   regions, in every plane: x 288–360, ring lines 0–64 (the scroller's new tiles as the camera
   rises) and x 32–80, ring lines 147–193 (the frog's bob). The only re-sync in the case.
3. **Frame 366, on time.** Variables and both buffers match at the frame's end; the picture
   alone differs on rows 122–157: the frog on the pad at 33,40, in the same place but a different
   animation frame. A beam race on the frog's bob erase/draw.
4. Make `items-shield-w3-1` green with `resync = false` and add it to
   `sf-conform/tests/frog_cases.rs`.

## Tools

`items-shield-w3-1.toml` with `resync = true` is red only at 366. `examples/profile <case> 236
246 top` prints each frame's pacing. `Divergence::var_detail` is cut to 40 entries in
`sf_conform::run_inner`; raise it locally for the whole buffer diff (T84 did, not committed).

## Files

`engine/crates/sf-core/src/level/{frame.rs,beam.rs}`, the bob drawing in
`level/objects`; `engine/conform/cases/items-shield-w3-1.toml`;
`engine/crates/sf-conform/tests/frog_cases.rs`.

## Done when

- [ ] `items-shield-w3-1` green against the oracle with no re-sync, in the test block.
- [ ] The other item cases in the test block still green.
