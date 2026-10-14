# T110 — enemies-w1-activation

**Split from:** T42 (pir's context line, 2026-10-14). Carries T42's test "activation: run past an
enemy and back; activation and deactivation frames equal the original", and a kill by
`touch_kill_1hit` in a frame case.

**Phase:** 2 · **Depends on:** T42; blocks T18, T33, T106 · **Weight:** light

## Goal

1. A W1 case (`enemies-w1-activation`) that runs past an enemy until the camera leaves it (it is
   freed by `obj_despawn_if_offscreen`) and back until it spawns again from its marker; the test
   asserts the engine frees and respawns it on the original's frames (the slot's `+$40` going to
   0 and back), green against the original. Widescreen depends on this (DESIGN §2.7).
2. A frame case where `touch_kill_1hit` kills: a hover stomped or hit by the spud (pick the spud
   up first), its fall (h0+$23b34), the splat and the 200 points.

## What T42 left for this task

- World 1's twelve types and their four children are ported (`sf-core/src/enemies/`) and pass
  routine conformance (`sf-conform/tests/enemies_w1_routines.rs`). W1-1's case
  `enemies-w1-1` is green with two re-syncs pinned in `sf-conform/tests/enemies_w1.rs`; add this
  task's test there, with `run(name, &[resyncs])` and its asserts on updates run and hurts.
- Plan scripts with `cargo run --release -p sf-conform --example complete -- <level> --beam 24
  --hold 4 --steps 100 [--paced --prefix <script>] --kill|--touch|--at|--near <col,row>...`.
  Plan one or two targets at a time and pass the result back as `--prefix` with `--paced`: a plan
  made unpaced leaves the original's route (its passes lag). A step costs 650 µs; a target is
  minutes. `--kill` steers to the enemy spawned from that cell.
- Re-syncs after lags are allowed and pinned (owner 2026-10-14, T101's class): check each is
  after a frame the original ends mid-pass (`sf-conform record <case> --only --watch
  vbl=h0+9d8:W`) before pinning it; a divergence with no lag before it is a defect.
- The crawler (h0+$247b8) is stompable only while the word h0+$2d128 is 2 or more; a stomp
  search never killed one in W1-1 or W1-2. A hover (touch_kill_1hit) was never stomped either.
- Enemy positions (col,row) come from the level's BODY markers: 1 hover, 2 sitter, 3 walker,
  10 spout, 11 dropper, 12 crawler, 13 runner, 15 spiker, 16/17 patroller ends, 18 watcher.

## Files

`engine/conform/{scripts,cases}/enemies-w1-activation*` and the kill case's, `engine/crates/sf-conform/tests/enemies_w1.rs`.

## Done when

- [ ] Both green against the original, in `sf-conform/tests/enemies_w1.rs`.
