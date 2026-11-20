# T04 — title-code-cases

**Phase:** 1 · **Depends on:** T01, T02 · **Weight:** medium

## Goal

KODY has no case: the codes reach worlds and the cheat, and the drill saw the digits equal but nothing
holds it. Add one case per code T01 lists (`title-code-wN`, `title-code-cheat`) and one for a wrong
code (`title-code-wrong`), each entering the code by input from the 4990 snapshot, with no `set` row,
so the original's own `title_code_world` writes `world_index` and `cheat_flag`.

## Design sections this implements

DESIGN §2.2.

## Files

- `games/forest-dumb/engine/conform/cases/title-code-*.toml` and their scripts (T01's measured scripts
  reused).
- `games/forest-dumb/engine/crates/fd-conform/tests/title_code_cases.rs` (new, full): one test per case of this task
  running it translated, with a `PASSING` / `DIFFER` list (as T03's), labelled in `smoke.toml`.

## Interface

A world 2 to 6 code's case: as `world-N-start.toml` (`align = "loads"`, fire on SCENA, 300 passes,
goal `world_index` and `irq_count` 300) but the world chosen by the typed code. A world 1 code's case
(if T01 finds one) and START with no code: judged to play's first frame (`enters = "game_frame"` or
`level3_handler` taking the machine, whichever T01 shows is first), never past 6301 (DESIGN §2.2).
`title-code-cheat`: goal includes `cheat_flag` as T01 measured. `title-code-wrong`: goal as T01
measured.

## Tests

- [ ] Each case reaches its goal on the oracle.
- [ ] Each runs translated; `title_code_cases.rs` names its reading.
- [ ] No case carries a `set` row: the code alone chooses the world.

## Done when

- [ ] One case per listed code and one wrong code exist and reach their goals on the oracle.
- [ ] Their translated readings are in their notes and `title_code_cases.rs`.
