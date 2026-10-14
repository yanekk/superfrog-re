# T115 — w3-completions

**Split from:** T44 (owner, 2026-10-14: "five tasks T111–T115"). Carries T44's done-when item "a scripted completion of each level in world 3 matches the oracle
(in `sf-conform -- all`)".

**Phase:** 2 · **Depends on:** T44, T101, T102, T111, T112, T113; blocks T18 · **Weight:** heavy

## Goal

One conformance case per W3 level (`enemies-w3-N-complete`, N = 1..4) whose script plays the
level from its first frame to its exit: every coin the exit needs taken (`frog+$5c` to 0), the
exit (marker 54) reached, `level_state` 1, the loop ended into the tally (T38's). Green against
the oracle with enemies and items live, each re-sync pinned in its test (none once T101 has
landed). It waits on T101 and T102 because W3-1 and W3-4 meet both faults early (W3-1's pads at
244, the frog's bob at W3-4 194), and on the per-level cases for their routes.

## What T44 left for this task

- World 3's eighteen types and the swing's links are ported (`sf-core/src/enemies/w3/`) and pass
  routine conformance (`sf-conform/tests/enemies_w3_routines.rs`). `enemies-w3-1` (1 378 frames)
  and `enemies-w3-4` (193 frames) are green, their re-syncs pinned in
  `sf-conform/tests/enemies_w3.rs`; add this task's test there, with `run(name, &[resyncs])` and
  its asserts on updates run and hurts.
- Plan with `cargo run --release -p sf-conform --example complete -- <level> [--paced --prefix
  <script>] [--steps 250 --beam 96] --kill|--touch|--at|--near <col,row>`, one target a leg, each
  leg from the last with `--paced`: an unpaced leg of 150 frames already left the original's route
  in W3-1, which lags nearly every frame. Check each leg with a conformance run before the next.
- `--kill` takes the marker's map cell (col,row of BODY); `--at`, `--near` and `--touch` compare
  with the frog's centre in pixels, whose row is the map row + 3 (the frog standing on row 40's
  floor is `--at <col>,43`).
- Re-syncs after lags are allowed and pinned (owner 2026-10-14, T101's class): check each is after
  a frame the original ends mid-pass (`sf-conform record <case> --only --every --watch
  vbl=h0+9d8:W`) before pinning it; a divergence with no lag before it is a defect.
- World 3's markers (`obj_flag_jumptab` row `$200`): 5 pad picture (87), 13 spiker staying (97),
  14/15 scenery in the cell below (89/90), 16 bouncer (88), 17 scenery (34), 18/19 dart left/right
  (55/56), 20 shuffler (92), 21 hopper (94), 23 drip (99), 24 flow (98), 25 spiker (96),
  26 stompable (91), 27 swing (65, links 66), 28 jumper (95), 36 crumbling platform (150).
  Stompable: 91, 92, 95 walking; killed by one hit: 95 in the air (the spud too).

## Interface

None new: the conformance case `enemies-w3-N-complete` (N = 1..4), its script, and a `#[test]` in
`sf-conform/tests/enemies_w3.rs` using `run(name, &[resyncs])`.

## Tests

- [ ] One test per level in `enemies_w3.rs`: the case green to the tally, `level_state` 1 at the
      exit, re-syncs pinned (or none).

## Files

`engine/conform/{scripts,cases}/enemies-w3-*-complete*`, `engine/crates/sf-conform/tests/enemies_w3.rs`,
`engine/crates/sf-conform/examples/complete.rs`.

## Done when

- [ ] A scripted completion of each level in world 3 matches the oracle (in `sf-conform -- all`).
- [ ] Each is in the test block, re-syncs pinned (or none).
