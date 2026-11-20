# <Game> (tier 3)

Rules and an index of what is known about the game. Paths are from the repo root unless they are
links. A fact true of this game goes here even if it might hold elsewhere (root CLAUDE.md, rule 1).
Start from `toolkit/PLAYBOOK.md`, whose opening steps fill most of this file.

## What the game is

<Title, publisher and year, what kind of game it is, and what this project is building.>

## Its disks and where they live

<The disk set: how many, their file names, and which release they are. Never committed.>
Lookup: [docs/01-disk-location.md](docs/01-disk-location.md).

## Tests

- Every new test file gets a label in [smoke.toml](smoke.toml); something crucial also gets a smoke test and an `[[area]]`. The full rule is under Conventions in the root CLAUDE.md.

## What's known (don't re-derive)

<One line a fact, each pointing at its note under docs/: the disk layout, the packer, the file
formats, the executables, the engine and how it is judged.>

## Addressing

<How a person points at a spot in the game, and how a session resolves that to data.>

## Bug-fix procedure

<What a session does when handed a bug report: replay it, compare it with the original on the
oracle, then a fix with a new conformance case or a report that the original does the same.>

## Notes

<Besides the numbered docs/ chapters, one line a note: [docs/topic.md](docs/topic.md), what it holds.>
