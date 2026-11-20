# T03 — playbook-and-template

**Phase:** 2 · **Depends on:** T02 · **Weight:** light

## Goal

Write toolkit/PLAYBOOK.md — the steps Superfrog took, in order, each with what it produced and
the lesson — and build toolkit/template/, a new game's starting kit. Game two's first plan walks
the playbook's opening steps; the template is the folder that plan starts from. Both live in tier
2, so both pass the name check.

## Design sections this implements

DESIGN §2.6.

## Files

- toolkit/PLAYBOOK.md (new). The steps, in the draft's order: disks and how the game boots; the
  packer; the file formats; the executable map; the oracle and the first conformance case;
  porting routine by routine against it; the app; the recorder; the planner. Each step names the
  artifacts it produced by their post-move paths (games/superfrog/docs/NN-…,
  games/superfrog/tools/…, the engine crates, plans/…) and states the lesson for the next game.
  Every line naming a game carries `learned on:` — the checker fails the task otherwise. Include
  the north-star's game-two note: the toolkit still lacks an AmigaDOS (OFS) .adf reader, which
  Superfrog never exercised and Forest Dumb needs.
- toolkit/template/ (new):
  - CLAUDE.md — the outline a game's tier-3 CLAUDE.md fills in: what the game is, its disks and
    where they live, the what's-known index, the addressing scheme, the bug-fix procedure — each
    as a headed placeholder, no game names.
  - docs/01-disk-location.md — the disk-location convention, with the placeholder filled by a
    new game: `<GAME>_DISKS`, else `<repo>/games/<game>`, else the repo root.
  - docs/README.md — the numbering convention (01, 02, … in discovery order, as Superfrog's).
  - .gitignore — the game's own patterns: its disk zips and its build artifacts (the root's
    *.zip/*.adf already cover disks; this one documents and covers the game-specific extras).
  - conform/case-stub.txt — the shape of a first conformance case: boot the disk, run N frames,
    a state or picture verdict; a pointer to the toolkit's amiga-conform crate as the harness.
- toolkit/tools/test_playbook.py (new): extracts every path PLAYBOOK.md and the template cite
  (games/superfrog/…, toolkit/…, plans/…) and fails if one does not exist on this branch.

## Interface

The template's five files above are the interface; a new game (T04) copies them and fills the
placeholders. The playbook's step headings are the draft's nine steps, in that order.

## Tests

- [ ] toolkit/tools/test_playbook.py green: every cited path exists.
- [ ] The name check passes over toolkit/ with the playbook and template present (the
  "learned on:" marking works on real prose, not only fixtures).
- [ ] Copying toolkit/template/ into a scratch folder yields exactly the five pieces, none naming
  a game.
- [ ] toolkit/test.sh green on this branch (the workspace plus the new python test).

## Done when

- [ ] toolkit/PLAYBOOK.md holds the draft's steps in order, each with its artifacts at real
  paths (the path test proves it) and a lesson; game names appear only in "learned on:" lines.
- [ ] toolkit/template/ holds the five pieces, and a scratch copy of it is complete and
  game-name-free.
- [ ] test_playbook.py is green and the name check passes over toolkit/.
