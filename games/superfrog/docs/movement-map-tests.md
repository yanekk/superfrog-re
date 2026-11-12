# The movement map's whole-level tests

`engine/crates/sf-plan/tests/map.rs` holds two `#[ignore]`d tests that check the movement map
(`sf_plan::core::map`, smart-planner T04) on whole levels:

- `w4_2_exit_needs_the_switch_at_222_4`: with every switch pressed the map reaches W4-2's exit,
  and without the switch at 222,4 it does not.
- `every_level_reaches_its_exit_with_its_coins`: every normal level's exit is reachable in the
  map through at least `coins_needed` object coins (smart-planner D13).

**Neither has ever passed.** Measured 2026-11-12 with the W4-2 test alone (debug, about 3 to 6
minutes a step): it fails at d4b4e16b, at 32a149c9 (T02, the move into `games/superfrog/`) and at
bc6121a5, the T04 commit that last wrote `map.rs`. T04 itself measured the shortfall on 2026-10-21
(`plans/smart-planner/FINDINGS.md`, the D13 row): exit unreached on W2-3, W2-4, W3-2, W3-4, W4-2,
W4-4, W5-2, W6-1, W6-2, W6-4, and W2-1 at 24 of 35 coins. The nightly full run of 2026-11-12
lists the same eleven levels, so nothing regressed. They were never seen failing in a full run
before 2026-11-12, because cargo stopped at the first failing test binary until 6dc2d1b9.

Why the map falls short is settled in the plan: the owner made the planner a whole-level explorer
(D19, 2026-10-22) and kept the map only as an optional compass that nothing waits on (D20,
DESIGN §2.2), dropping T20, which was to patch the eleven maps by hand. The tests still assert
T04's original aim. What they should hold now is the owner's call (asked 2026-11-12).
