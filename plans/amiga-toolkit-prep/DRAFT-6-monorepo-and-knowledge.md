# Draft 6 — one repository for the toolkit and every game; knowledge kept in it

**Status:** draft, 2026-10-28. Not reviewed. Owner decision 2026-10-28: a monorepo; the toolkit is
one project in it, Superfrog the second, Forest Dumb Forever the third. Owner ask 2026-10-28:
knowledge lives in the repo in three tiers, generic, toolkit and game, not in Claude's memory.
This supersedes draft 5's phase B (the split into its own repository) and its decisions 1–3.

## Layout

```
superfrog-re/                 the repository (rename optional, decision 3)
  CLAUDE.md                   tier 1: the owner, the pir method, the faithful bar, the knowledge rules
  knowledge/                  tier 1 notes, indexed by KNOWLEDGE.md
  plans/                      every plan (pir reads plans/ at the root); slugs name their project
  .pir/                       one settings file; its test line is the selector tools/test.sh
  toolkit/                    tier 2: Rust workspace (draft 5), RE tools, PLAYBOOK.md, CLAUDE.md, docs/
  games/superfrog/            tier 3: engine/, tools/, docs/, conform cases, CLAUDE.md
  games/forest-dumb/          tier 3: same skeleton, started from toolkit/template/
```

Games depend on the toolkit by path (`../../toolkit/crates/…`); no git dependency, no pinning, a
toolkit change and its use in a game land in one commit and one review.

## The three tiers

| Tier | Holds | Where | Read by a session |
|---|---|---|---|
| 1 generic | how the owner works and decides; the pir method; the faithful bar ("judged by machine, no hand checks"); asking the owner to play when a planner stalls; the owner's controls; pir mid-run amendments | root `CLAUDE.md` (rules) and `knowledge/` (notes) | always: the root CLAUDE.md is loaded at every start |
| 2 toolkit | what each toolkit crate does and its invariants; the oracle and its vAmiga patches; timing and chip lessons; the RE workbench; `PLAYBOOK.md`, the order of steps that worked | `toolkit/CLAUDE.md`, `toolkit/docs/`, `toolkit/PLAYBOOK.md` | when it works on files under `toolkit/` (Claude Code loads a folder's CLAUDE.md on use) |
| 3 game | "what's known, don't re-derive", the disk set and where it lives, formats (ATN!, T7MP), the addressing scheme, level index, bug-fix procedure | `games/<game>/CLAUDE.md`, `games/<game>/docs/` | when it works on that game's files |

Each tier's CLAUDE.md is short: rules and an index of its notes, one line each (the shape
`MEMORY.md` has today). Notes are the facts, one topic a file, newest-first rows where they are
logs. `plans/{slug}/FINDINGS.md` stays the per-plan log; a finding that outlives its plan is
promoted to a tier note when the plan finishes.

**Rules for writing knowledge** (go in the root CLAUDE.md):

1. Write at the lowest tier where it is true. A Superfrog fact goes in `games/superfrog/`, even if
   it might hold elsewhere.
2. Promote when a second user confirms it. A lesson a game note holds that proves true for another
   game moves up to `toolkit/` (or tier 1 for a way of working), in the commit that shows it.
3. No game names above tier 3. A test, extending `measure.sh`'s name check, fails when `toolkit/`
   or `knowledge/` code or notes name a game, except in a marked "learned on" source line.
4. Claude's memory keeps only what is about this machine or this conversation; anything a future
   session needs goes in the repo.

## Moving what exists

| Today | Goes to |
|---|---|
| Root CLAUDE.md "How we work together" | stays, tier 1 |
| Root CLAUDE.md project part (Orientation, What's known, Tile addressing, Next up, Conventions) | `games/superfrog/CLAUDE.md`; disk-safety and branch conventions to tier 1 |
| `docs/01`–`14`, `LEVEL-INDEX.md`, `BUG-FIX.md` | `games/superfrog/docs/` |
| `docs/architecture/` | split: chapters of toolkit parts to `toolkit/docs/`, the rest to `games/superfrog/docs/` |
| `docs/RE-WORKBENCH.md`, `tools/re_listing.py`, `tools/re_ghidra_import.py`, `tools/setup/*`, `tools/check_no_disk_data.py` | `toolkit/` (generic halves; Superfrog's chunk table stays) |
| Memory `self-testing-faithful-bar`, `owner-controls`, `pir-midrun-amendment`, `planner-stall-ask-owner` | `knowledge/` (the planner rule names Superfrog's tools until the planner is generalised; it stays tier 3 until then) |
| Memory `reuse-for-another-game`, `explorer-go-explore-idea` | `toolkit/` notes (toolkit history, backlog) |
| Memory `atn-decompression`, `t7mp-level-format`, `collision-and-secrets`, `milestone-2-bug-recorder`, `next-plan-difficulty-modes`, `video-ghost-trial` | `games/superfrog/` notes (backlog items marked so) |

Each memory file is checked against the code before it moves (memories are point-in-time); the
memory directory is then emptied to a pointer at the repo.

## PLAYBOOK.md and the game template

`toolkit/PLAYBOOK.md`: the steps Superfrog took, in order, each with what it produced and the
lesson: disks and how the game boots; the packer; file formats; the executable map; the oracle and
first conformance case; porting routine by routine against it; the app; the recorder; the planner.
`toolkit/template/` holds a new game's skeleton: CLAUDE.md outline, `docs/` numbering, `.gitignore`
for disks, a disk-location variable, a conformance case stub. A new game is the template copied
plus a first plan that walks the playbook's opening steps. Forest Dumb Forever is its first user.

## Decisions (owner, 2026-10-28)

1. **Superfrog moves into `games/superfrog/`**, as one task after draft 5's phase A. Mechanical,
   but it touches every path: `SUPERFROG_DISKS`'s default, the disk zips in the root, `build/`,
   scripts, `docs/architecture` citations, `.pir/settings.json`.
2. **Each game has its own test command.** pir runs one `test` line, so the line is one selector,
   `tools/test.sh`, the pattern pir's own repo uses (a test command that picks by change, widened by
   `PIR_TEST_SCOPE=all`, which pir sets for plan-review verification and end-of-run helpers). Each
   project owns its command: `toolkit/test.sh`, `games/<game>/test.sh`. The selector diffs the branch
   against its merge base with `main`: files under one game run that game's command; `toolkit/`
   runs the toolkit's and every game's (they build on it); root files other than `plans/` (`.pir/`,
   `tools/`, `knowledge/`, CLAUDE.md) run everything; `plans/` alone runs nothing;
   `PIR_TEST_SCOPE=all` runs everything. Every doubt widens the selection. The selector gets its own
   unit tests on synthetic change lists.
3. **The repository keeps its name, `superfrog-re`.**

## Order

Draft 3 (A+B) and draft 2 finish; draft 5 phase A builds `toolkit/`; this draft's tasks then: T1
knowledge tiers and rules, root CLAUDE.md split, memory moved; T2 Superfrog into `games/superfrog/`
(decision 1); T3 `PLAYBOOK.md` and `toolkit/template/`; T4 `games/forest-dumb/` from the template,
with the cracked-copy decision (NORTH-STAR J1) taken there.
