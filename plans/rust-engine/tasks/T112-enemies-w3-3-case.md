# T112 — enemies-w3-3-case

**Split from:** T44 (owner, 2026-10-14: "five tasks T111–T115"). Carries T44's test "frame conformance per level of world 3" for W3-3.

**Phase:** 2 · **Depends on:** T44; blocks T18 · **Weight:** light

## Goal

`engine/conform/cases/enemies-w3-3.toml` and its script: W3-3 from its first frame with every
object live, meeting its enemy types (bouncer 88, darts both ways, shuffler, hoppers, flows,
spiker, swing, jumper, crumbling platforms, scenery 89, pads), hurt by each it reaches and stomping
the stompable ones where it can; green against the original, in the test block. Nothing was
planned on W3-3 yet.

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

None new: the conformance case `enemies-w3-3`, its script, and a `#[test]` in
`sf-conform/tests/enemies_w3.rs` using `run(name, &[resyncs])`.

## Tests

- [ ] `enemies_w3::w3_3`: the case green but for its pinned re-syncs; every type met's update ran,
      the frog hurt, the stomped ones' updates ran.

## Files

`engine/conform/{scripts,cases}/enemies-w3-3*`, `engine/crates/sf-conform/tests/enemies_w3.rs`.

## Done when

- [ ] `enemies-w3-3` green (re-syncs pinned, or none), in `sf-conform/tests/enemies_w3.rs`.
- [ ] Every W3-3 type met, or a FINDINGS row naming which T115's completion meets instead.
