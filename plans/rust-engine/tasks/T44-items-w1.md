# T44 — items-w1

**Phase:** 2 · **Depends on:** T42 · **Weight:** medium

Added by T36 (owner, 2026-10-13). One of the item frame cases T36's goal 4 asked for.

## Goal

Item cases in world 1 (`sync = { level = "W1-n" }`, `enemies` excluded, compare all), picture
included, no re-sync:
- W1-2 from the start: the wings at 149,11 (flight; it did not end within 1 200 frames — find what
  ends it and play to it, or record that it lasts), the spud at 129,13. T36 planned this with
  `examples/plan` (`plan W1-2 149,11 129,13`: reached frames 384 and 464).
- W1-1: the checkpoint at 34,42 (`plan W1-1 34,42`: reached frame 220). The bottle (89,42) and spud
  (86,42) sit on a ledge reached by the mushroom's bounce; the planner fails from below — script by
  hand or skip (W3-1 has a bottle, T45).
- A bounce pad: W1-1 PAD4 16,67 or W1-2 PAD4 115,43.

Tools: `examples/items_map <level>` lists pickups by kind with cells; `examples/coverage
<level> <script> <frames>` says what a script takes; `examples/plan <level> <col,row>...
[--prefix script]` searches inputs on the engine (beam search; long routes get stuck: seed a prefix
from an existing script).

## Done when

- [ ] Cases green against the oracle; FINDINGS says which kinds each case covers.
