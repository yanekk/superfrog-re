# T69 — level-start-state

**Phase:** 2 · **Depends on:** T07, T65 · **Weight:** small

Added by T15 with the owner's yes (2026-11-06). `Game::new(Start::Level1)` is not the original at the end of pass 1, the first pass end after the level sync point (frame 6302): measured by T15, it differs in `char_anim_ptr` (`$c491a0`, original `$c48f20`), `char_falling` (0, original 1), `char_jump_t` (0, original 4) and `char_update_t` (3, original 0); a pass later `char_y` is `$70` against `$71`. Cases hide it because they transplant the original at the first pass end; the app, the planner and a bug recording start from `Game::new`, and DESIGN §2.3 requires that start to equal the original's. T15's `bug compare` therefore starts its engine from the original's state and prints the difference (`(T69)` in its report). T20 and T28 wait for it (`blocks T20, T28`): a route or a recording must start where the original starts.

## Goal
`Game::new(Start::Level1)` equals the original at pass 1's end in every `LAYOUT` variable and in what T65 already lays (collision plane, play screen), so an engine run from it alone stays on the original's play.

## Design sections this implements
§2.3, §2.8.

## Reuses
T65's `level_start.rs` (the sync-point checks), T15's `bug::compare` (its `start_differs`), `State::level_1`.

## Files
`fd-core/src/state.rs` (`State::level_1`), `fd-core/src/game.rs` if pass 1's character update must run in `Game::new`, `fd-conform/tests/level_start.rs`, `fd-conform/src/bug.rs` (the transplant in `Owners`, once nothing differs), `games/forest-dumb/docs/BUG-FIX.md` (the start-difference caveat).

## Interface
None new. First measure why the four differ: whether the sync point (frame 6301) is before pass 1's character update (`char_update` and its pickers run in pass 1, after `irq_count` reaches 1) or `State::level_1` holds wrong values. Say which in docs/12.

## Tests
- [ ] A test reads every `LAYOUT` variable but `mt_counter` from the original at the first pass end after the sync point and asserts `Game::new` holds the same.
- [ ] A recording of `character-run-jumps`' first 200 steps compared with the original without the transplant at the start (`bug compare`) is `same`.

## Done when
- [ ] `bug compare` reports `the engine's own start equals the original's`, and BUG-FIX.md loses its start-difference caveat.
- [ ] `level_start.rs`, `level_entry.rs`, `recording_compare.rs` and T41's `character_run.rs` stay green.
- [ ] Smoke label for any new test file.
