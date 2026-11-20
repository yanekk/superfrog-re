# T05 — architecture-chapters-carve

**Phase:** 1 · **Depends on:** T02 · **Weight:** medium

## Goal

Carve the generic parts of the six hardware-side architecture chapters into toolkit/docs/architecture/,
under the name check. T02 was ruled to move them whole (DESIGN §7), but they name the game on 322 lines
and own game files, so the owner decided on 2026-10-30 (T02) to leave them in
games/superfrog/docs/architecture/ and split them here.

## Design sections this implements

DESIGN §2.2, §2.3, §2.5 (the docs/architecture split).

## Files

- New toolkit/docs/architecture/: display-hardware, cpu-time-and-beam, frame-interrupts-input,
  memory-model, sound-chip, verification; each holds what is true of the hardware or toolkit crate.
- games/superfrog/docs/architecture/: the same six reduced to what the game does on top, keeping
  every game file in some chapter's `owns`; README.md links the toolkit chapters.
- games/superfrog/tools/architecture.py and its test: chapters load from both trees, as DESIGN §7
  says the validator does; toolkit files are owned by toolkit chapters.
- toolkit/CLAUDE.md: one index line.

## Interface

No code interface. The chapter front-matter format is unchanged.

## Tests

- [ ] games/superfrog/tools/test_architecture_docs.py green over both trees: every file owned once,
  every cited path real.
- [ ] tools/check_tier_names.py passes; no line gets `learned on:` just to pass it (owner, 2026-10-30).

## Done when

- [ ] The six toolkit chapters exist, name no game unmarked, and the game chapters keep what is
  specific to the game with nothing lost; the checks above are green.

## Outside actions

None.
