# T03 — title-idle-rows-cases

**Phase:** 1 · **Depends on:** T01, T02 · **Weight:** light

## Goal

The idle title is judged today only for 280 frames, short of the song's loop, and moving between the
rows has no case. Add `title-idle-loop` and `title-rows` (DESIGN §2.2), run each on the oracle and on
`--engine translated`, and record the reading in each case's notes.

## Design sections this implements

DESIGN §2.2, §2.4 (no fixing here: a translated failure is recorded for T07).

## Files

- `games/forest-dumb/engine/conform/cases/title-idle-loop.toml`, `title-rows.toml` and their scripts.
- `games/forest-dumb/engine/crates/fd-conform/tests/title_idle_cases.rs` (new, full): one test per case of this task,
  each running it translated, with a `PASSING` / `DIFFER` list so a known failure is named, not hidden.
  Labelled in `smoke.toml`. One file per case task, so T03 to T06 run side by side without touching the
  same file.

## Interface

`title-idle-loop`: power-on (`start = "entry"`, `align = "loads"`, `swap` as `power-on-title`), frames
to past T01's measured loop frame, goal `mt_songpos` back to 0 after its last position.
`title-rows`: from the 4990 snapshot, Down to KODY, Down to KONIEC, Up, Up, each move held as T01
measured; goal `title_cursor` 0 at the end, `vars` including `title_cursor`.

## Tests

- [ ] Each case records its goal on the oracle (the original reaches it).
- [ ] Each runs translated; `title_idle_cases.rs` lists it as passing or with its first difference.

## Done when

- [ ] Both cases exist, reach their goals on the oracle, and their translated reading is in their
      notes and in `title_idle_cases.rs`.
