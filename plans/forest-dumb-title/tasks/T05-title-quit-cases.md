# T05 — title-quit-cases

**Phase:** 1 · **Depends on:** T01, T02 · **Weight:** light

## Goal

KONIEC quits the app (translator T41) but no case holds it against the original, and Esc at the title
was read, never run. Add `title-koniec` and `title-esc` (DESIGN §2.2) from the 4990 snapshot.

## Design sections this implements

DESIGN §2.2, §2.3.

## Files

- `games/forest-dumb/engine/conform/cases/title-koniec.toml`, `title-esc.toml` and their scripts.
- `games/forest-dumb/engine/crates/fd-conform/tests/title_quit_cases.rs` (new, full): one test per case of this task
  running it translated, with a `PASSING` / `DIFFER` list (as T03's), labelled in `smoke.toml`.

## Interface

`title-koniec`: Down, Down, fire; `goal = { exited = true, by_frame = … }`. `title-esc`: the Amiga Esc
key (a `K45` script line) on the title; goal as T01 measured: `exited` if it quits, or, if it does nothing, `all`
with `title_cursor` and `title_quit_flag` at their starting values by the case's last frame, the case
running a stated stretch past the key (a case lined up by loads refuses `idle`, and its memory is
compared every frame, which holds the stretch). Both cases `align = "loads"` from the snapshot.

## Tests

- [ ] Each reaches its goal on the oracle.
- [ ] Each runs translated; `title_quit_cases.rs` names its reading.

## Done when

- [ ] Both cases exist, reach their goals on the oracle, and their translated readings are recorded.
