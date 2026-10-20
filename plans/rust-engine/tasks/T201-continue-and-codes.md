# T201 — continue-and-codes

**Split from:** T21 (its tests "game over and continue; starting from each password" and
"`start_at(progress)` equals entering that password on the title")

**Phase:** 2 · **Depends on:** T21, T200; blocks T23, T29 · **Weight:** medium

## Goal

1. Game over and continue: a case from a level with points to game over, the title (with T200's
   name entry), then that world's level code typed and the game going on there, faithful and
   alone. Superfrog has no continue screen: `game_over_check` (h0+$749c) returns to `top_loop`;
   continuing is the level code.
2. Every level code from power-on: W1-1..W6-4 and the bonus level (`password_table` entries
   0–24), each to its level's first frames, faithful and alone. The typing is
   `Game::type_code`'s / `sync.rs`'s `drive_to`; `chain-level-code` (W3-1) is the model. W6-2..4's
   first map set needs a second drive (docs/10): run those with df1 connected, or the `alt` set.
   Slow ones `#[ignore]`d and named in `conform/README.md`.
3. `start_at` against the original: T21's `tests/flow.rs` proves `start_at` equals the code typed
   in the engine (but for `TITLE_TIME`); add one oracle case typing a code with `CODE_TIMES`
   (`chain-level-code` already does, for W3-1) for a world whose files need a disk swap.

## Files

`engine/conform/{scripts,cases}/{continue-*,code-*}`; `sf-conform/tests/{chain.rs,flow.rs}`.

## Interface

None new.

## Tests

- [ ] `continue-w2`: game over in W2 with points, the title, W2-1's code, W2-1 played.
- [ ] One case per code, green faithful and alone.

## Done when

- [ ] All green, no `unported`; slow cases listed in the README.
