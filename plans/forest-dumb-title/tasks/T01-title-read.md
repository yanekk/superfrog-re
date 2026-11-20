# T01 — title-read

**Phase:** 0 · **Depends on:** — · **Weight:** heavy

## Goal

Every later task names the title's codes, keys and screens, and nobody has read them. Read the title's
code from the listing and measure it on the oracle: the rows, KODY's digit entry, the code table and
what each code does, Esc's route, KONIEC's return, what a wrong code does, and the path from game over
through the high scores back to the title. Write it all in `games/forest-dumb/docs/16-title.md`, name
the routines and variables in `re/symbols.toml`, and prove each listed code on the oracle with a test.

## Design sections this implements

DESIGN §2.1; brief "Stages", the read.

## Files

- `games/forest-dumb/docs/16-title.md` (new), indexed in `games/forest-dumb/CLAUDE.md` Notes.
- `games/forest-dumb/re/symbols.toml`: the routines and variables read (the digit store, the code
  table, the high-score routine and table, the Esc check), each with a measured note.
- `games/forest-dumb/engine/crates/fd-core/src/layout.rs`: `LAYOUT` rows for the title variables the
  cases will compare (at least `title_cursor`, `title_start_flag`, `title_quit_flag`, the digit store,
  `mt_songpos`, the high-score table's entries), all of them here so T03 to T06 add none and do not
  collide in this file, matching `symbols.toml` (the existing test holds them together).
- `games/forest-dumb/engine/conform/scripts/`: the input scripts the measurement used, reusable by the
  cases (e.g. `title-code-<name>.txt`, keyed after `boot-to-level1`'s frame 4990).
- `games/forest-dumb/engine/crates/fd-conform/tests/title_codes.rs` (new, full) and its `smoke.toml`
  label.
- `docs/07-executable-map.md` "The title screen's code path": correct what the read contradicts (the
  Esc route, `OpenLibrary`, the OS interrupts line named in docs/14 "Docs and data").

## Interface

`docs/16-title.md` holds, each fact with its measurement and date:

```
## The rows          title_cursor values, the keys/joystick moves that change them, wrap or stop
## KODY              how a digit is entered and changed, how many digits, where they are stored
## The codes         | code | world_index | cheat_flag | other effect | measured |
## A wrong code      what the original does
## Esc               what the original does on the Amiga Esc at the title, measured
## KONIEC            quit_to_dos: the return from the entry, the stack, the frame
## Game over         game_over_screen → high scores (name entry? disk write?) → session_loop → title
## The song's loop   the frame mt_songpos first returns to 0 on the idle title
```

## Tests

- [ ] `title_codes.rs`: from the 4990 snapshot, each listed code entered by its script on the oracle;
      `world_index` and `cheat_flag` read as `load_world` is entered (`Amiga::poke_at`'s hook or
      `watch_pcs`) equal the table's row.
- [ ] A wrong code's measured effect, held the same way.
- [ ] The listing claims in `symbols.toml` pass the existing symbol/layout tests.

## Done when

- [ ] `docs/16-title.md` answers every heading above with a measurement, and `title_codes.rs` passes.
- [ ] Anything unplanned the read found (a disk write for the high scores, a hidden screen, a code
      reaching past world 6) has been put to the owner and the answer recorded in FINDINGS.

## Needs a person

Only if the read finds the high scores written to disk, or a screen or mode DESIGN §2.2 does not list:
ask the owner whether the plan checks it, with what it costs.
