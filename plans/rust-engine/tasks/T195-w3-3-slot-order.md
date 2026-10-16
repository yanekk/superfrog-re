# T195 — w3-3-slot-order

**Phase:** 2 · **Depends on:** T115; blocks T21 · **Weight:** medium

Added by T115 with the owner's yes, 2026-10-16 (DESIGN §2.1 check 4).

## Goal

The engine run alone keeps W3-3's object slots in the original's order, so `enemies-w3-3-slot-order`
plays alone green. A player can meet this: the engine on its own takes one coin fewer than the
original on the same route.

Measured by T115 (2026-10-16, on the case's route, T115's first plan of W3-3):

- The frog's path run alone equals the original's frame for frame to 6400.
- At 4053 the original has all 20 slots full and the engine alone has 19: its coin in slot 3 went a
  frame earlier, and the spiker in slot 18 differs too. From there the same objects sit in different
  slots (at 4900 the crumbling platforms are in slots 1, 2, 4 on one side and 1, 4, 5 on the other).
- At 6332–6336 the original takes three coins (20 → 17 left) and the engine alone takes two; no 17th
  ever comes. Check 4 fails at 6335: "the original's coin taken (17 left for the exit) has no match
  in the engine within 10 frames".
- The engine run with re-syncs (checks 1–3) matches the original throughout; only the run alone
  parts. Its report names `bob_records` at 3634 as the first variable that differs at a pass end.

First check whether T184's class (alone-pass-drift: the engine's pass a frame off the original's
after late frames, `vbl_count` first) explains it. T184 is ✅, so this route was run with its fix;
the report names `bob_records`, not `vbl_count`. Then find where the slot order parts (`obj_spawn`'s
free-slot scan, a despawn a pass early or late, a slot freed by a coin taken a frame early) and fix
the engine, or what check 4's run alone is given, within check 4's rule: the engine alone gets only
the inputs, the original's pacing and its clock's seeds.

## Design sections this implements

DESIGN §2.1 check 4.

## Files

- `engine/conform/cases/enemies-w3-3-slot-order.toml`, `engine/conform/scripts/enemies-w3-3-slot-order.txt`
  (T115's; no test runs the case yet).
- `engine/crates/sf-core/src/level/objects*`, `engine/crates/sf-core/src/level/{frame.rs,beam.rs}`,
  `engine/crates/sf-conform/src/{lib.rs,sync.rs}` as needed.
- `engine/crates/sf-conform/tests/enemies_w3.rs`: a test for the case.

## Interface

None new, unless the fix needs more of the original's schedule in `sf_core::Pacing` (as T184's did).

## Tests

- [ ] `enemies-w3-3-slot-order` green under `sf-conform run` (checks 1–4), with its re-syncs pinned,
      in a test in `enemies_w3.rs`.
- [ ] The case's object slots, run alone, in the original's order at 4053 and 4900 (a test that
      compares them at frames ending a pass).

## Done when

- [ ] `enemies-w3-3-slot-order` plays alone green, in the test block.
- [ ] The W3 completions (`enemies-w3-*-complete`) stay green.
