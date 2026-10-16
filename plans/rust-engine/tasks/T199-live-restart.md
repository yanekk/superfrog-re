# T199 — live-restart

**Split from:** T21 (its done-when "No case file anywhere excludes a variable or object")

**Phase:** 2 · **Depends on:** T21; blocks T23, T29 · **Weight:** small

## Goal

The last two case files with an `[[exclude]]` of objects or enemies run with everything live,
green faithful and alone (T21 removed the other 33; `story-intro-full`'s picture exclusion is the
owner's and stays):

1. `frog-pit-w1-1`: live, frame 307 `frog_x` $96 vs $95: the restart after the first death
   (`level_init_w1`, frames 293–306) ends a frame late on the original. Measured (oracle,
   `main_chain` + DMACON writes): frozen, `bob_setup`'s mask blits start (DMACON $8400) on line 69
   of frame 297; live, on line 160, and `level_start` follows a frame later. The engine's restart
   costs were fitted to the frozen run (T37). Find what costs more with the objects live
   (`bob_setup` h0+$19a0a..$19a52, h0+$18f6c ×3, $18ef8, $15fa0; VBL lengths were equal) and
   model it; the second death's restart (612) too.
2. `items-shield-w3-1`: live, only frame 295 differs (`bob_records`, after a lag) with
   `resync = false`; decide with the owner between a pinned re-sync and the fix.

## Files

`engine/conform/cases/{frog-pit-w1-1,items-shield-w3-1}.toml`; `sf-core/src/level/{bobs.rs,
exit.rs,beam.rs}` as the cause needs; `sf-conform/tests/frog_cases.rs`.

## Interface

None new.

## Tests

- [ ] `frog_cases::frog_pit_w1_1` and `items_shield_w3_1` green with no exclusion.

## Done when

- [ ] `grep -l 'what = "objects"\|what = "enemies"' engine/conform/cases/*.toml` prints nothing.
