# The Amiga toolkit (tier 2)

What carries from one Amiga game to the next: the Rust crates under crates/, the RE tools, and how
to start a game. A game depends on it by path, so a toolkit change and its use land in one commit.
[README.md](README.md) lists the crates; `test.sh`, run from the repo root, is the toolkit's test command.

## Rules

- No game's name, code or data enters this tree. `tools/check_tier_names.py` (at the repo root) fails
  any line naming a game here, except a line carrying `learned on:`, which says where a value or a
  lesson came from.
- A lesson enters here when a second game confirms it (root CLAUDE.md, rule 2). A backlog note is
  headed "Backlog": it is a plan, not a fact.
- Every new test file gets a label in [smoke.toml](smoke.toml); something crucial also gets a smoke test and an `[[area]]`. The full rule is under Conventions in the root CLAUDE.md.

## Notes

- `crates/<crate>/README.md`: how to call a crate (`amiga-oracle`, `amiga-conform`, `amiga-plan`, `amiga-app`, `amiga-input`, `amiga-routeview`). Read it, in one step, before opening the crate's source.
- [PLAYBOOK.md](PLAYBOOK.md): the steps the first game took, in order, each with its artifacts and lesson.
- [docs/porting-routines.md](docs/porting-routines.md): the rules a porting task names in its Done when: readings are not evidence, the 68k traps, proving a port, the port register.
- [template/](template/): a new game's starting kit, copied to `games/<game>/`.
- [docs/reuse-for-another-game.md](docs/reuse-for-another-game.md): what carried to the toolkit, what stayed per game, and the toolkit's history.
- [docs/explorer-go-explore-idea.md](docs/explorer-go-explore-idea.md): backlog, an archive-return-explore search for routing and bug finding.
- [docs/architecture/](docs/architecture/display-hardware.md): the hardware and the toolkit crates, six chapters: display, CPU time and the beam, frame and input, memory, sound, verification.
