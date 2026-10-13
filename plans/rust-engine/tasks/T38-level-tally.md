# T38 — level-tally

**Phase:** 2 · **Depends on:** T37; blocks T21 · **Weight:** medium

Added by T15 (owner, 2026-10-13). The exit reached (marker 54 with no coins left, `level_state`
1), the loop ends into `level_exit_fade` and `level_tally` (h0+$4466): the tally picture unpacked
(h0+$33234 by h0+$ce38), the pages for time, tile coins and coins (up to five each), its text,
the fade in, the wait for fire with up/down choosing points or the slot machine (h0+$4798), the
points added (5 000 a page) and the extra life check, the fade out. The slot machine is T19's.

## Goal

W1-1 finished to the tally and past it, every frame matching the original; `LevelOutcome::
Completed { tally }` carries the pages and the choice.

## Done when

- [ ] A W1-1 completion case, through the tally, green in `sf-conform -- all`.
