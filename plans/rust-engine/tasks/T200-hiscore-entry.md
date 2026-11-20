# T200 — hiscore-entry

**Split from:** T21 (its chain "game over … end sequence" back to the title with a score)

**Phase:** 2 · **Depends on:** T21; blocks T23, T29 · **Weight:** medium

## Goal

After a game whose score makes the high-score table, the original's title runs the name entry
(`hiscore_name_entry`, called from `hiscore_and_credits`, h0+$20e0); the engine records
`unported` for it: "hiscore_and_credits with a new high score" (`front/title.rs`) and
"hiscore_and_credits after a game with a new high score" (`front/attract.rs`). Port it, faithful
and alone, from game over and from the end sequence. High scores last the session (DESIGN §2.9).

## Files

`sf-core/src/front/{title.rs,attract.rs}` and a new `front/hiscore.rs` if wanted;
`engine/conform/{scripts,cases}/hiscore-*`; `sf-conform/tests/front.rs`.

## Interface

None new.

## Tests

- [ ] `hiscore-entry`: game over with a score above the table's lowest (e.g. `complete-w1-1`'s
      route then the last life lost, or a forced score), the title's name entry typed with the
      joystick, the table shown: faithful and alone, no `unported`.

## Done when

- [ ] Neither `unported` string above is reachable on a case; the case green in its test file.
