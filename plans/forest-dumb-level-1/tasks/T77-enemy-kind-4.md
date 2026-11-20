# T77 — enemy-kind-4

**Phase:** 6 · **Depends on:** T46, T75; blocks T27, T48 · **Weight:** medium

**Start (owner, 2026-11-06, DESIGN §4.4):** run `python3 -I games/forest-dumb/tools/task_pack.py T77` (a reviewer adds `--review`) and read the pack it names whole, in one step, before anything else.

Added by T46 with the owner's yes (2026-11-09). T46's enemy cases meet kinds 0, 1, 2, 3 and 5, but no route here brings kind 4 (object value `$05`, four cells: W1-1:45,92, 145,52, 156,50, 187,17) into view, and kind 2 only walks left before the race (T75), so `h0_3222` (`Creatures::step_on`) has no hits (`enemy_cases.rs`'s `NOT_REACHED`).

## Goal
Show kind 4's path and kind 2 stepping right agreeing with the original, creatures live, as T46's cases do: a route or script that brings a kind 4 cell into view and keeps it there through a turn, and one where a kind 2 turns right.

If no route in W1-1 can bring kind 4's cells into view (owner, 2026-11-09), record that with the evidence (what was tried, where the character cannot reach) in docs/10 and FINDINGS, and T48 lists kind 4 as a named exception, rather than this task failing.

## Design sections this implements
§2.1, §2.2, §4.1.

## Reuses
T46's `enemy_cases.rs` and case shape, `fd-plan level W1-1 --guide` for a route, T75's race handling.

## Files
`engine/conform/cases/enemy-*.toml`, `engine/conform/scripts/enemy-*.txt`, `fd-conform/tests/enemy_cases.rs`, `fd-core/src/**` (fixes only), docs/10, `smoke.toml` (a slow case goes to full).

## Interface
Case and script files only, plus engine fixes.

## Tests
- [ ] A case meeting kind 4, every creature slot compared, its walk and a turn on the original.
- [ ] A case where kind 2 turns right, `h0_3222` with hits.

## Done when
- [ ] `enemy_cases.rs`'s `NOT_REACHED` is empty, or kind 4's unreachability is recorded with its evidence and T48 names it.
