# T11 — dry-fit-and-finish

**Phase:** 4 · **Depends on:** T04, T05, T08, T09, T10 · **Weight:** heavy

## Goal

Show the toolkit is enough and Superfrog is unchanged. The dry fit (owner 2026-10-24, draft "Dry
fit") is a throwaway-shaped but kept program in `toolkit/` that depends only on toolkit crates: it
blits a pattern into chip memory, runs a copper list, draws the result to a `Picture` with the display
piece, steps the clock across a frame and plays a Paula note, with a test asserting each. Then the
whole-game proof: `sf-conform -- all` and sf-e2e `whole_game.rs`, compared with T01's baseline. Then
the READMEs and this step's north-star column.

## Design sections this implements

DESIGN §1 success criteria, §4, §3.5.

## Files

- New: `toolkit/crates/dry-fit/` (a binary with tests), `toolkit/README.md`.
- `README.md` (a toolkit section: what it is, what is in it, how to test it, the link to
  `toolkit/README.md`), `plans/amiga-toolkit-prep/NORTH-STAR.md` (a column),
  `docs/architecture/README.md` if its crate list names moved crates (front matter or tables the docs
  test checks; bodies frozen).

## Interface

```
toolkit/crates/dry-fit: depends on amiga-chips, amiga-clock, amiga-paula only.
cargo tree -p dry-fit lists no sf-* crate (asserted in a test).
```

## Tests

- [ ] Dry fit: the blit writes the expected words to chip memory.
- [ ] Dry fit: the copper's writes land on the expected lines and positions.
- [ ] Dry fit: the drawn `Picture` shows the blitted pattern with the copper's colours.
- [ ] Dry fit: the clock steps exactly one frame's cycles; a Paula note produces non-silent samples
      of the expected period.
- [ ] Its dependency tree holds toolkit crates only.

## Done when

- [ ] The dry fit passes from `toolkit/`; `toolkit/README.md` and the README section are written.
- [ ] `sf-conform -- all` gives T01's verdicts case for case (FINDINGS row listing any difference and
      its cause); `whole_game.rs --include-ignored` passes; test block green.
- [ ] `measure.sh` run and NORTH-STAR's column filled (M1–M5; J2 left for the owner).
