# T30 — level1-cases-a

**Phase:** 5 · **Depends on:** T29 · **Weight:** heavy

## Goal

Make the translated program pass these level 1 cases from the level sync point, as the hand engine does: camera-*, character-*, stretch-*, picture-*, w1-idle, stub-level (in `engine/conform/cases/`). Every fix is general, in the translator, `pcode-rt` or the machine, never for one address (DESIGN §2.2); a difference the frame bar admits is accepted only with its measurement written into the case's notes (DESIGN §2.7).

## Design sections this implements

DESIGN §2.2, §2.7, §2.8.

## Files

- Whatever the failures lead to in `tools/pcode_fix/`, `crates/pcode-rt`, `crates/pcode-machine`; case notes for accepted differences. Each fix with a test that fails without it.
- Case files only for notes; a case's checks are never loosened.

## Tests

- [ ] Each fix's own test (made-up program or machine unit test).
- [ ] Full: every case above passes with `--engine translated`.

## Done when

- [ ] Every case above passes; any acceptance is in the case's notes with its measurement.
- [ ] If the failures need more than a session, the worker asks to add a task (CLAUDE.md) rather than stretching this one.
