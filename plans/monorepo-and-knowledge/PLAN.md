# Implementation plan

4 tasks in 2 phases. Each has a file in [tasks/](tasks/) with its goal, the files it touches, the
interfaces it defines, and what "done" means.

Track state in [PROGRESS.md](PROGRESS.md). Read [DESIGN.md](DESIGN.md) first.

---

## Shape of the build

- **The risky unknowns were measured at plan time, so there is no spike.** The stored test lines
  pass in a fresh copy (measured), the "learned on:" marker convention already exists in the
  toolkit, and PIR_TEST_SCOPE's contract was read from pir's own engine. Phase 0 is empty.
- **The move is atomic.** T02 is one heavy task rather than several lighter ones because the run
  branch must never sit half-moved: a repo where the root CLAUDE.md points one way and the files
  another misleads every session until fixed.
- **The plan is serial by nature.** Each task rearranges what the next sits on: T02 needs T01's
  CLAUDE.md split to have landed first, T03 cites the moved paths, T04 copies the template. No
  ordering is cut to make the plan look wider.

```
Phase 1  ▸  T01, T02        reorganize: knowledge tiers, then the Superfrog move
Phase 2  ▸  T03, T04        the second game: playbook and template, then Forest Dumb's skeleton
```

---

## Phase 1 — Reorganize

At the end of this phase the repo has its final shape: knowledge lives in its three tiers, the
memory folder holds only a pointer, Superfrog lives in games/superfrog/, the selector runs every
suite from its new home, and the zips have moved.

| # | Task | Depends on |
|---|---|---|
| [T01](tasks/T01-knowledge-tiers.md) | knowledge-tiers | — |
| [T02](tasks/T02-superfrog-move.md) | superfrog-move | T01 |

**T02 depends on T01 for merge order, not for files.** T01 rewrites the root CLAUDE.md to tier 1
and writes games/superfrog/CLAUDE.md with final paths; T02 makes those paths true. In the other
order, every session between the two merges would read a CLAUDE.md pointing at moved files. The
edge guarantees T01 lands first.

## Phase 2 — The second game

At the end of this phase the toolkit holds the playbook and the template, games/forest-dumb/
exists with the settled skeleton, and the cracked-copy answer is recorded with its date.

| # | Task | Depends on |
|---|---|---|
| [T03](tasks/T03-playbook-and-template.md) | playbook-and-template | T02 |
| [T04](tasks/T04-games-forest-dumb.md) | games-forest-dumb | T03 |

The Task cell is the task's kebab slug, matching its `tasks/T{nn}-{slug}.md` filename. Every task
is built by an autonomous worker; T04's cracked-copy question is raised through the normal
question path, not a separate kind of task. No phase builds a surface, so no phase ends in a
drill.

---

## Critical path

```
T01 → T02 → T03 → T04
```

There are no off-path tasks. Leaves: T04 only — it is the final deliverable, and nothing else is
terminal.

## Parallel width

4 tasks · longest dependency chain 4 · up to 1 could run at once. The plan is a pipeline, not a
fan-out; the width is honest, and no dependency was cut to inflate it.

## Rough sizing

Not hours — a relative sense of where the weight is.

| Weight | Tasks |
|---|---|
| **Heavy** | T02 — the move's long tail of path references, the selector, and the everything-scope run (~1 h) at the end |
| **Medium** | T01 — the CLAUDE.md split, ten checked note moves, the check and its tests |
| **Light** | T03 (two documents, one small test), T04 (a copy, a stub, one question) |

The place this will overrun is T02: every path reference the move breaks is found by grep and the
suite, and a miss shows up as a red suite rather than a wrong plan. Budget the session for the
final everything-scope run.

## Decisions still open

The cracked-copy question, put to the owner in T04 and recorded there: proceed on the cracked
[cr WT] set, or hold for an uncracked copy. Nothing else is open; say so if that changes.
