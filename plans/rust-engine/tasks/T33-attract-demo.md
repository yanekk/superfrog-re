# T33 — attract-demo

**Phase:** 2 · **Depends on:** T16, T20 · **Weight:** medium

Added by T20 (owner, 2026-10-12): attract mode ends with a W1-1 demo played from recorded input
(`top_loop` h0+$a8: `demo_world_load`, `demo_level_load`, `demo_init`, then `level_frame_loop`;
`demo_index` cycles three entries). It needs the frog, items and W1 enemies, which T20 does not
depend on, so T20 stops its attract cycle at the demo's start.

## Goal

The attract demo in the engine: from T20's attract cycle into the demo level, the recorded input
driving the frog (where `demo_init` points it), every frame matching the original including its
lag (DESIGN §2.1), the abort on fire or key, and the return to the title so the cycle repeats.

## Design sections this implements

DESIGN §2.1, docs/08 "Scene flow".

## Files

- `engine/crates/sf-core/src/front/attract.rs` (or T20's title module) — the demo's place in the cycle.
- `engine/crates/sf-core/src/level/` — the demo's recorded input feeding the frog (T14's input path).
- `engine/conform/cases/attract-*`; `docs/13-front-end.md` — the demo's input format and timing.

## Tests

- [ ] title attract mode cycling for 5 000 frames, demo included (the test T20's doc asked for).
- [ ] each of the three `demo_index` entries, and an abort by fire in the middle of the demo.
- [ ] the credits before the demo past the original's frame 6416, where T20 left the scroll's blit
      and the beam crossing inside one row (4 pixels; FINDINGS 2026-10-12, `front::attract::scroll`).

## Done when

- [ ] All cases green; the full cycle in `sf-conform -- all`.
