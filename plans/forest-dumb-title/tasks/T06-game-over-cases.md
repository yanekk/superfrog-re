# T06 — game-over-cases

**Phase:** 1 · **Depends on:** T01, T02 · **Weight:** medium

## Goal

Game over's way back to the title, through the high scores, has no case. Add `game-over-title` and,
if T01 found name entry, `game-over-name` (DESIGN §2.2): world 2 started as `world-2-start` does,
`lives` set low as `load_world` is entered, deaths by input to game over, the high-score screen, and
`session_loop` back into `title_screen`.

## Design sections this implements

DESIGN §2.2, §2.3.

## Files

- `games/forest-dumb/engine/conform/cases/game-over-title.toml`, `game-over-name.toml` (if any) and
  their scripts.
- `games/forest-dumb/engine/crates/fd-conform/tests/game_over_cases.rs` (new, full): one test per case of this task
  running it translated, with a `PASSING` / `DIFFER` list (as T03's), labelled in `smoke.toml`.

## Interface

`align = "loads"`, sync at the 4990 snapshot, `set = [{ var = "world_index", value = 1, when =
"load_world" }, { var = "lives", value = <T01/docs-09's BCD for one life>, when = "load_world" }]`,
`goal = { enters = "title_screen", by_frame = … }`. The worker checks nothing after `load_world`'s
entry sets `lives` again before play; if something does, the row's `when` is the routine after it. `game-over-name` adds the typed name and a `vars`
check of the table's entry before the title.

## Tests

- [ ] Each reaches its goal on the oracle; the high-score routine is entered on the way (coverage).
- [ ] Each runs translated; `game_over_cases.rs` names its reading.

## Done when

- [ ] The case(s) exist and reach their goals on the oracle, their translated readings recorded.
