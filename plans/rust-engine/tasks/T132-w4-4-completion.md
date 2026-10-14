# T132 — w4-4-completion

**Split from:** T103

**Phase:** 2 · **Depends on:** T103; blocks T18, T131 · **Weight:** heavy

Carries T103's "Done when" item for W4-4 (its completion against the oracle) and, of T103's
types, the pacers (markers 2/3, templates 136/137) and marker 20's dart (template 61), each
hurting the frog. Marker 21's dart (template 62) is T133's.

## Goal

`complete-w4-4`: W4-4 from its start to its exit and through the tally, every chunk checked on the
original, as T103's `complete-w4-1`…`-3` are: coins until `coins_left_for_exit` (h0+$832) is 0,
the switches whose gates bar the way, the exit, the tally with fire 70 frames after the exit.

## What T103 learned (its routes were in `/tmp`, discarded at its split)

- `cargo run --release --example plan_exit -- W4-4 --ground 27,22,24 --coins 0 --frames 30000
  --out <script>`; continue a stopped plan with `--prefix <script>` (its `-` line ends it).
- W4-4 needs 97 coins; its gates: switch 48 (159,60) → 115,68; 49 (63,18) → 33,31; 50 (119,37)
  → 171,12; 51 (174,26) → 249,61. The exit (286,46) needs 51's gate open.
- A route that worked: the pacer's hurt at 105,64 and marker 20's dart at 102,53 come early
  (`--hurt t136 --via 102,51 --hurt t61` from about frame 1990: hurts at 2038, 3867); then the
  last 18 coins only by `--via 119,37 --via 181,14` (switch 50 first; without it the planner
  wandered 7 000 frames); then `--via 174,26` and the exit (T103's reached it at 16 294).
- Traps: the field cannot see the way into the pacer room (it routes through gate 115,68);
  stomping a pacer drops the frog into the spikes at 90-94,75; a hurt's knock-back near 247,18
  or 195,33 leaves the frog where every path dies. Bottles (two tiles) refill the hearts.
- The clock matters: W4-4 starts with about 3 minutes; bottles add 2.

## Interface

None new: a case and a script in `sf-conform`'s formats; `plan_exit` as T103 left it.

## Files

`engine/conform/scripts/complete-w4-4.txt`, `engine/conform/cases/complete-w4-4.toml`;
`engine/crates/sf-conform/tests/frog_cases.rs` if it is green.

## Tests

- `examples/slots_log` on the case shows the pacer's and marker 20's dart's hurts (the slot within
  48 pixels), listed in the commit message with each type's spawns and frees.
- `sf-conform scan` of the case: the frames that differ, each named for T131 in `FINDINGS.md`
  (T103's earlier W4-4 route differed at 827, 1146, 1279, 1446, 3946, 5979, 8129, 9510, 10457,
  12210 and the exit fade); green, it goes in the test block as `complete_w4_3` does.

## Done when

- [ ] `complete-w4-4` plays W4-4 to its exit and through the tally on the original.
- [ ] The pacers' and marker 20's dart's hurts in it; its faults in `FINDINGS.md` for T131.
