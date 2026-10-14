# T113 — enemies-w3-4-case

**Split from:** T44 (owner, 2026-10-14: "five tasks T111–T115"). Carries T44's test "frame conformance per level of world 3" for W3-4 past frame 193.

**Phase:** 2 · **Depends on:** T44, T102; blocks T18 · **Weight:** light

## Goal

Take `enemies-w3-4` on from its first crumbling platform through W3-4, meeting the types no
other case meets: the scenery of marker 17 (34, at 270,69), the scenery below marker 15 (90, at
95,40) and the drips (99, markers 23 at 75–86,48), and the rest present (shufflers, stompables,
darts, hoppers, flows, spikers, swing, jumper); hurt by each it reaches, stomping where it can;
green against the original, in the test block.

The frog starts at the level's right end, under three crumbling platforms (markers at 289,49;
284,52; 287,55) it climbs. At frame 194 the original draws only part of the frog's bob as
it jumps from the first platform, every variable equal: the beam race T102 owns (as
`items-shield-w3-1` at 366), which is why this task waits on T102.

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

None new: the conformance case `enemies-w3-4` (extended past frame 193), its script, and a `#[test]`
in `sf-conform/tests/enemies_w3.rs` using `run(name, &[resyncs])`.

## Tests

- [ ] `enemies_w3::w3_4` extended: the case green past the platforms; the updates of 34
      (h0+$249e8), the drip (h0+$287e0) and every other type met ran, the frog hurt.

## Files

`engine/conform/{scripts,cases}/enemies-w3-4*`, `engine/crates/sf-conform/tests/enemies_w3.rs`.

## Done when

- [ ] `enemies-w3-4` green past the platforms (re-syncs pinned, or none), its test extended.
- [ ] Types 34, 90 and 99 met; every other W3-4 type met, or a FINDINGS row naming which T115's
      completion meets instead.
