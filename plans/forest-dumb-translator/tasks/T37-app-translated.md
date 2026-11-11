# T37 — app-translated

**Phase:** 7 · **Depends on:** T18, T32, T36 · **Weight:** heavy

## Goal

The app runs the translated program from power-on, with level 1's hand engine mounted (DESIGN §2.10): entry, title, menus, levels. `--level 1` starts at level 1 by running the program headless through the title with Start pressed. Built without a generated tree, the app says how to generate it and exits non-zero.

## Design sections this implements

DESIGN §2.10.

## Files

- `fd-app/src/main.rs` (the run loop drives `TranslatedGame` on its own thread through a channel, per T28), `fd-app/src/args.rs` (`--level 1`, `--engine hand` keeps today's engine for comparison), `fd-app/src/input.rs` or equivalent (keys and pad to `pcode_machine::Input`, keyboard codes for the title's keys), `fd-app/README.md`, `games/forest-dumb/CLAUDE.md` "Playing it".

## Tests

- [ ] Unit: argument parsing; the "not generated" message names `python3 -I games/forest-dumb/tools/translate.py --build`.
- [ ] Headless (no window): `--script` from power-on reaches level 1's first frame and its state equals the conformance run's at that frame.

## End to end (the worker drives this)

- suite: `fd-e2e` · sizes: the sizes `fd-e2e/tests/drill_sizes.rs` uses
- [ ] launch with no flags → the title's picture, equal to the headless run's at that frame
- [ ] Start by its key → level 1 begins and plays as headless frame for frame for 600 frames
- [ ] `--level 1` → level 1's first frame within the time limit

## Done when

- [ ] The three end-to-end tests pass in full; a smoke test runs the app on the stub tree and checks the message and exit code.
- [ ] `CLAUDE.md` "Playing it" describes the new start and `--engine hand`.
- [ ] The existing `fd-e2e` tests still pass: those that run without a generated tree pass `--engine hand` (owner, plan review 2026-11-11), so smoke never needs the tree.
