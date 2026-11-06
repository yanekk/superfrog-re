# superfrog-re — start here

One repository, three projects: the Amiga toolkit, and the games built on it, each reverse-engineered
and rebuilt in Rust. Its name stays superfrog-re. **Working on a game (a level, a bug zip, a route, its
tools)? Read `games/<game>/CLAUDE.md` first**; today that is `games/superfrog/`. For the toolkit, read
`toolkit/CLAUDE.md`.

## Where knowledge lives

| Tier | Holds | Where |
|---|---|---|
| 1 generic | how the owner works and decides, the pir method, the faithful bar, the owner's controls | this file; notes in `knowledge/`, indexed by `knowledge/KNOWLEDGE.md` |
| 2 toolkit | what each toolkit piece does and its invariants, the oracle, the RE workbench, the playbook | `toolkit/CLAUDE.md`, `toolkit/docs/`, `toolkit/PLAYBOOK.md` |
| 3 game | what's known, the disk set, formats, addressing, bug-fix procedure | `games/<game>/CLAUDE.md`, `games/<game>/docs/` |

Each tier's `CLAUDE.md` is rules and an index, one line a note. `plans/{slug}/FINDINGS.md` stays the
per-plan log; a finding that outlives its plan is promoted to a tier note when the plan finishes.

## The four knowledge rules

1. Write at the lowest tier where it is true. A Superfrog fact goes in `games/superfrog/`, even if it
   might hold elsewhere.
2. Promote when a second user confirms it. A lesson a game note holds that proves true for another
   game moves up to `toolkit/` (or tier 1 for a way of working), in the commit that shows it.
3. No game names above tier 3. The name check (`tools/check_tier_names.py`) enforces it: `toolkit/`
   and `knowledge/` must not name a game except in a marked `learned on:` line.
4. Claude's memory keeps only what is about this machine or this conversation; anything a future
   session needs goes in the repo.

## The bar

A game's engine is faithful to the original, judged by machine against the original on the oracle,
never by the owner playing or looking (`knowledge/self-testing-faithful-bar.md`).

## Before building something

Check the toolkit first, then how the other games did it (`toolkit/PLAYBOOK.md` lists what each step
produced). Reuse what fits, and move it into the toolkit when a second game needs it. Do not carry over
a game's details: an engine matches its own original, whatever another game did.

## Conventions

- Disk images and ROMs are copyrighted and are **never committed** (git-ignored). Nor is anything
  derived from them. Bring your own copies; each game's `CLAUDE.md` says where they go.
- Do RE work on a feature branch and open a PR to `main`. Commit tools and docs (small); leave
  regenerable `build/` artifacts out of git.
- Tests: smoke and full. pir runs `tools/smoke.sh` (15 minutes at most) at the end of every plan and for
  every single run; a nightly schedule runs `tools/full.sh`; a session runs neither unless its task says so.
  - Every new test file gets a label in its project's `smoke.toml`. A file that finishes alone in 10 s or
    less is smoke.
  - A change adding something crucial adds its test to smoke and an `[[area]]` for it. A plan adding a
    crucial feature puts this in that task's Done when.
  - Before adding a slow test, read `~/.cache/superfrog-re/smoke-times.tsv`. If smoke would pass 15 minutes,
    move fast tests to full first. If crucial tests alone do not fit, ask the owner.

<!-- ─────────────────────────────────────────────────────────────────────────
     Appended by plan-implement-review. Everything above this line is the
     project's own CLAUDE.md; everything below is the shared working method.
     ───────────────────────────────────────────────────────────────────────── -->


# How we work together

Work goes through `pir`. A plan is written once, read back once by a session that did not write it,
then built task by task, many at once, each in its own session. Every task is reviewed by a fresh
session that did not build it. A small change can skip the plan.

| Command | What it does |
|---|---|
| `pir plan` | Brainstorm, settle requirements, check what the code already does, split into tasks, write `plans/{slug}/`; a fresh reviewer reads the plan back before anything is built |
| `pir start {slug}` | Start (or open) the parallel build of a reviewed plan |
| `@repo/single {prompt}` in `pir` | A small change: a short brief, read back, built, tested and reviewed by fresh sessions, handed to the finisher for the owner's `Go` |

A plan is read back before it is built, because a mistake in a plan is copied into every task and a
task review cannot see it. Read `plans/{slug}/DESIGN.md` before changing behaviour; if you disagree
with a rule there, say so, do not implement something else. How parallel mode behaves today is in
pir's `/docs`, not in a plan's `DESIGN.md`, which records why.

## Who you are talking to

The owner is the product manager: owns what is built and why, and does not read the code.

**Plain English in conversation.** No jargon; keep the reasoning, drop the vocabulary. If a term is
needed for a decision, teach it in one line. Code, comments, commits and `plans/` stay technical.

**You decide how; the owner decides what.** Solve technical problems without asking and say so
afterwards in one line. Ask before acting when the plan would change (a task split, reordered, dropped
or added), a `DESIGN.md` rule is wrong or about to be contradicted, something a user would see, hear
or do is unspecified, or two behaviours are both defensible. Never invent a rule to get unblocked.

**Adding a task mid-run** is the one plan change a parallel worker may make, only with the owner's
yes (or the coordinator agent's): it writes the new task on its branch, may name existing tasks it
`blocks`, and never edits, splits, reorders or re-depends an existing task.

**The coordinator agent stands in for the owner** in a parallel build: it may answer what the plan
settles, settle what the design leaves open, approve going against a design rule or adding a task. It
never answers an `ask`-bin action or a destructive command. A worker asks as it would ask the owner,
in its own session. Beyond-routine decisions are listed in `plans/{slug}/REPORT.md`.

**How to ask:** one decision at a time: the goal in a sentence, the options with their cost, your
recommendation, and what you will do if there is no answer. Do everything that does not depend on it
meanwhile.

**The owner is your hands on the real machine** (a screen, a login, a device, a reboot): give the exact
command and what to look for, then wait for the answer before building on it. A live action (a deploy,
a paid call) is not on that list: you run it within the bins the owner approved (`DESIGN.md §5.3`).

## The rules every session follows

**Scope is strict.** Touch only your task. Anything else you notice goes in `plans/{slug}/FINDINGS.md`
and is left alone. The one exception is a worker adding a task with approval, as above.

**Only the test command is evidence.** Its lines are the setup/test block `DESIGN.md` opens with. A
claim that needs a screen, a login, a reboot, a camera or a paid call without a bin cannot be
established by a session: say what you built, what it has not been shown to do, and hand over one
block, with the plan's seatbelt on the command:

```
Needs you — I cannot see this from here:

  <the exact command, with its seatbelt>

Expect: <what should happen>
Tell me: <the one or two things only a person can answer>
```

Never run the unbounded version yourself. A screen the project draws is driven by the session itself
and recorded as worker-driven. Mark a half-checked task unverified in `PROGRESS.md` and the report; an
answer goes in `FINDINGS.md` with the date, the only record anything was seen working.

**Say the way back before changing the live world** (a credential, a published page, a paid service,
a file on a real device): one line on whether and how it can be undone. `DESIGN.md §5.3` puts every
live action in a bin: `worker` (run it, say so after), `ask` (explain, run, the permission prompt is the
owner's yes), `person`. Anything irreversible, costly, public or infrastructural is `ask` unless the
owner moved it at plan review. An action with no row is `ask`, and the missing row goes in
`FINDINGS.md`; a way back that mattered goes there too, dated.

**Commit messages:**

```
plan(screen-time): daily screen budget with a warning     ← the plan
plan-review(screen-time): 6 fixes, 3 decisions            ← the read-back: every fix and decision with its reason
T05: policy decision function                             ← implementation
T05 review: fix warning threshold                         ← a fix found while reviewing
T05 review: clean                                         ← body: risk list + deviation verdicts
fix: stale branch on restart                              ← a fix of reviewed work, with a trailer:
Escape: coordinator-restart-resume T03
```

A review commit's body carries its risk list, written before reading the builder's account, and each
recorded deviation marked `held by <check>` or `falsified`, about ten to fifteen lines. Every fix of a
✅ task or a merged single run ends with `Escape: {plan} T{nn}`, `Escape: single {name}` or
`Escape: unknown` (`git log --grep '^Escape:'` counts them).

**Where sessions run.** `pir` runs each session in its own worktree on a `pir/…` branch. Nothing
reaches the base branch except through the owner or the finisher on the owner's go: no session merges
into it, pushes, rebases or resets history its procedure does not cover. The base branch is
`baseBranch` in `.pir/settings.json` (overridable in `~/.pir/{repo}/settings.json`); `pir` never guesses
it.

## The files

`ls plans/` lists the plans. Each has:

1. `PROGRESS.md`: task states and the queue, always current; sixty words to a Notes cell; the `Task`
   cell is the task's slug (its `tasks/T{nn}-{slug}.md` and its worker's name).
2. `FINDINGS.md`: what the build taught, newest first, forty words a row including every column; where
   "verified by hand" is written down.
3. `PLAN.md`: the task list, phases and dependencies; changed only by the owner.
4. `tasks/`: one file per task: goal, files, interface, acceptance criteria.
5. `DESIGN.md`: why things are as they are, the environment and the verification contract.

A plan with a feel to it may have `prototype/`, the throwaway mock, a non-binding reference.

Both `PROGRESS.md` and `FINDINGS.md` are read at the start of every session, so they stay short: count
the words. A reviewer reads `FINDINGS.md` only after its cold read (task doc, diff, its own risk list).
Whoever appends, compacts first when the file is over its ceiling (60 rows or 15 KB for findings, 12 KB
for progress): merge duplicate lessons, drop a row now enforced by code, a test or `DESIGN.md` (name
where in the commit), cut a ✅ task's Notes to one line once the next task is reviewed. Never drop a ✅
hand-verification row and its date, or a term someone would grep for. Fix the over-budget row you were
reading anyway; that needs no finding.

Write them in flat prose: at most one bold phrase a row, no em-dash asides, no aphorisms, no
restating the row above or what the code says. The argument goes in the commit message.

## Rules on coding

- **Comments explain why**: the non-obvious constraint, dense where surprising, absent where not.
- **Small commits, one per task**, referencing it: `T04: budget ledger and gap classification`.
- **No clock, random number or network in the core**: inputs arrive as parameters, the time as an
  argument; `DESIGN.md § Architecture` names the boundary.
