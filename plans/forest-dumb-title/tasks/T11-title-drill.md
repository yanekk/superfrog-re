# T11 — title-drill

**Phase:** 3 · **Depends on:** T07, T09, T10 · **Weight:** light

## Goal

A worker uses the title and Start Level… as a person would, on the fixed program, and catches what no
case thought to assert; then closes the plan's docs. Every defect with one right answer gets a fix and
an end to end test; anything with two defensible answers goes to the owner.

## Design sections this implements

DESIGN §2.2, §2.5, §2.6; pir-e2e § 3.

## Files

- Fixes and their tests wherever the drill finds them; `fd-e2e` tests labelled.
- `games/forest-dumb/docs/16-title.md` (the drill's reading), `docs/ROADMAP.md` (the title plan done),
  `games/forest-dumb/CLAUDE.md` (index, backlog), `docs/14-translator.md` "Open after the plan" (items
  this plan closed).

## End to end (the worker drives this)

- suite: fd-e2e script and OS layers · sizes: 400×300, the default, 1200×900
- [ ] The whole title: prompt, idle with music, every row, every code, a wrong code, Esc, KONIEC; and
      unasked: keys held across a row change, digits typed fast, focus lost on KODY, a resize mid-entry.
- [ ] Start Level… for each world, cheat on and off, recorder on and off, Cancel; Start Level… opened
      on the title and in play.
- [ ] Game over to the title and START again.
- [ ] Screens saved to `build/drill/title/`, never the repo.

## Done when

- [ ] Every interaction above driven, a dated worker-driven FINDINGS row written.
- [ ] Every one-answer defect fixed with a test that fails without it; two-answer ones asked.
- [ ] Smoke's total with this plan's labels read from `smoke-times.tsv` or a run, under 900 s.
