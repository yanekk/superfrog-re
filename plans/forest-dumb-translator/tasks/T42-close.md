# T42 — close

**Phase:** 7 · **Depends on:** T21, T33, T34, T41 · **Weight:** light

## Goal

Close the plan: the readings in one place, the game's guide and roadmap brought up to date, and the lessons that outlive the plan moved to their tier's notes (root CLAUDE.md, knowledge rules).

## Files

- `games/forest-dumb/docs/14-translator.md` (final reading: census, gate, whole tree, cases, worlds, default check, Haiku trial).
- `games/forest-dumb/CLAUDE.md` (Notes, Playing it, Backlog), `docs/ROADMAP.md` (the translator done; a level plan now means checking translated code and the owner's rewrites), `docs/translator-brief.md` (a line pointing at the result).
- Promotions: FINDINGS rows that bind later level plans go to `games/forest-dumb/docs/` notes; a lesson true for any game goes to `toolkit/docs/porting-routines.md` with a `learned on:` line.

## Tests

- [ ] `tools/check_tier_names.py` passes.
- [ ] Smoke is run once and its time compared with `smoke-times.tsv`; any growth this plan caused is named in FINDINGS.

## Done when

- [ ] `docs/14-translator.md` holds every reading; ROADMAP and CLAUDE.md match what was built.
- [ ] FINDINGS is under its ceiling, with promoted rows replaced by pointers.
