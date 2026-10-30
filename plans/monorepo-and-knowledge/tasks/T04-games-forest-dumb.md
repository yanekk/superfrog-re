# T04 — games-forest-dumb

**Phase:** 2 · **Depends on:** T03 · **Weight:** light

## Goal

Create games/forest-dumb/ from the template: the second project exists, the selector discovers
it, and the run ends with the cracked-copy question answered and recorded. The game's own first
plan — walking PLAYBOOK.md's opening steps — is the next pir plan run, not this task.

## Design sections this implements

DESIGN §2.7.

## Files

- games/forest-dumb/CLAUDE.md — the template's outline filled with what the north-star
  established: Forest Dumb Forever, © 1998 L.K. Avalon (developer Eagle Soft); AmigaDOS (OFS)
  disks labelled !EAGLE_SOFT!; six worlds of _Mapa level files; no Team 17 shared items (ATN!,
  T7MP, Player 4.1A do not apply). Headed backlog: "first plan: walk PLAYBOOK.md's opening
  steps" (a future pir plan run, not this task). The disk section reflects the cracked-copy
  answer once it is in.
- games/forest-dumb/docs/01-disk-location.md — the convention with FOREST_DUMB_DISKS; the named
  default: the owner's zips in ~/Documents/Amiga (Forest Dumb Forever!!! (1996)(L.K.
  Avalon)(PL)(Disk N of 2)[cr WT].zip), outside the repo.
- games/forest-dumb/docs/README.md and .gitignore — from the template.
- games/forest-dumb/conform/case-stub.txt — from the template.
- games/forest-dumb/test.sh — the stub: prints "forest-dumb: no tests yet" and exits 0, until
  the first plan adds real tests.
- FINDINGS.md — the cracked-copy answer, dated, ✅.

## Tests

- [ ] tools/test.sh on this branch selects the game scope (the whole diff is under
  games/forest-dumb/) and exits 0 through the stub.
- [ ] The name check still passes: the banned list grew by "forest-dumb" (derived from games/*/);
  toolkit/ and knowledge/ stay clean.
- [ ] The template copy check from T03, run against the new game directory, passes.

## Done when

- [ ] games/forest-dumb/ holds the skeleton above, and the selector runs its stub green.
- [ ] The cracked-copy answer is recorded with the date in FINDINGS.md and reflected in the
  game's disk note.
- [ ] The game's CLAUDE.md names the first plan as the next step.

## Needs a person

The cracked-copy question, asked once the skeleton is complete, through the normal question path:

- Proceed with the cracked [cr WT] two-disk set as game two's source. The faithful bar's
  measuring stick would be the cracked release, whose executable may differ from the original's
  (the WHDLoad install is built on this same release); the toolkit's OFS reader is needed either
  way.
- Or hold for an uncracked original. Game-two RE waits; only the toolkit's OFS reader can
  proceed meanwhile.

Expect: the answer decides what the disk note records and what the first plan targets.
Tell me: proceed with the cracked set, or hold for an uncracked copy.
