---
setup:
  - python3 -m venv venv
  - venv/bin/pip install -q -r requirements.txt
test:
  - tools/test.sh
---

# Monorepo and knowledge tiers — Design

## 1. Purpose

The repository becomes the monorepo the owner decided on 2026-10-28 (draft 6): one repo, three
projects — the Amiga toolkit, Superfrog, Forest Dumb Forever — and its knowledge moves out of
Claude's memory into the repo in three tiers. The toolkit cut was made against one game; a second
game is its test. Knowledge kept in the repo travels with the code, is reviewed like the code, and
is read by every session that touches the files it describes; knowledge in Claude's memory is
private to one machine and one model.

### Success criteria

- The three projects sit in their folders, the root holds only what is shared, and the repo is
  still named superfrog-re.
- Every suite that passed before the plan passes from its new home: the Python suite's 82 tests,
  the engine workspace, the toolkit workspace, under the selector with the all-override.
- Every memory fact lives in the repo at its tier; the memory directory holds only a pointer.
- A change anywhere runs the right tests, plans-only changes run nothing, the all-override runs
  everything — proven by the selector's unit tests on synthetic change lists.
- games/forest-dumb/ exists with the settled skeleton, and the cracked-copy answer is recorded
  with its date.

### Stance

- The move is atomic. On the run's branch the repo is the old layout or the new one, never
  half-moved; T02 is one task for exactly that reason.
- Tests never run less than before the plan. Every stored command survives inside the selector,
  and every doubt widens the selection.
- Knowledge lives in the repo and is machine-checked where a machine can check it; what a machine
  cannot check is the owner's to decide.
- Superfrog's faithfulness is untouched: the same suites, the same verdicts, new paths only.

## 2. Behaviour specification

### 2.1 The layout

```
superfrog-re/                 the repository, its name unchanged (decision 3)
  CLAUDE.md                   tier 1 rules and the tier map; the pir-method text below them
  knowledge/                  tier 1 notes, indexed by knowledge/KNOWLEDGE.md
  plans/                      every plan, at the root, because pir reads plans/ there
  .pir/                       one settings file; its test line is the selector tools/test.sh
  toolkit/                    tier 2: the Rust workspace, the RE tools, PLAYBOOK.md, the template
  games/superfrog/            tier 3: engine/, tools/, docs/, re/, SECRETS.md, its disks' home
  games/forest-dumb/          tier 3: the same skeleton, started from toolkit/template/
```

Games depend on the toolkit by path, never by git dependency and never pinned, so a toolkit change
and its use in a game land in one commit and one review. plans/ stays at the root: pir reads it
there, and the plans that already exist keep their old paths as history — a plan is a sealed
record, not a living index. A plan's slug names its project from here on: a Forest Dumb plan gets
a forest-dumb-… slug.

### 2.2 The three tiers and the writing rules

| Tier | Holds | Where | Read by a session |
|---|---|---|---|
| 1 generic | how the owner works and decides; the pir method; the faithful bar; the owner's controls | root CLAUDE.md (rules) and knowledge/ (notes) | always — the root CLAUDE.md loads at every start |
| 2 toolkit | what each toolkit piece does and its invariants; the oracle and its patches; the RE workbench; the playbook | toolkit/CLAUDE.md, toolkit/docs/, toolkit/PLAYBOOK.md | when it works on files under toolkit/ |
| 3 game | what's known, don't re-derive; the disk set and where it lives; formats; addressing; bug-fix procedure | games/<game>/CLAUDE.md and docs/ | when it works on that game's files |

Each tier's CLAUDE.md is short: rules and an index of its notes, one line each. Notes are the
facts, one topic a file, newest-first rows where they are logs. plans/{slug}/FINDINGS.md stays the
per-plan log; a finding that outlives its plan is promoted to a tier note when the plan finishes.

The four writing rules, in the root CLAUDE.md:

1. Write at the lowest tier where it is true. A Superfrog fact goes in games/superfrog/, even if
   it might hold elsewhere. This is what stops tier 1 from becoming the default dumping ground.
2. Promote when a second user confirms it. A lesson a game note holds that proves true for another
   game moves up to toolkit/ (or tier 1 for a way of working), in the commit that shows it. The
   toolkit is a cut against one game; the second game is the only evidence that widens it.
3. No game names above tier 3. The name check (§2.3) enforces it.
4. Claude's memory keeps only what is about this machine or this conversation; anything a future
   session needs goes in the repo. Memory is invisible to other sessions and other machines; the
   repo is read by every session.

### 2.3 The name check

tools/check_tier_names.py scans toolkit/ and knowledge/ and exits non-zero with one line per
offence, `path:line`. Banned tokens, matched case-insensitively with `[-_ ]+` interchangeable (so
"forest dumb forever" matches "forest-dumb"): every games/*/ directory name, plus the literal
prefix token `sf-` (Superfrog's crate prefix; the toolkit's own crates are amiga-*). Exempt: the
repo's own name token superfrog-re, and any line containing `learned on:` anywhere in it — the
marker four toolkit doc comments already use, so the check extends a convention instead of
inventing one. games/ itself, plans/ and the root CLAUDE.md are not scanned.

The check exists because a rule without a machine behind it gets overturned by the first session
it inconveniences. It runs as part of the toolkit scope and the everything scope (§2.4). If a
toolkit note genuinely needs to name a game — the playbook does, it is the story of Superfrog —
it does so in `learned on:` lines.

### 2.4 The test selector

Decision 2, settled: each project owns its test command (toolkit/test.sh, games/<game>/test.sh),
and .pir/settings.json's one test line is the selector tools/test.sh. The selector diffs the branch
against its merge base with main: files under one game run that game's command; toolkit/ runs the
toolkit's and every game's (games build on it); root files other than plans/ (.pir/, tools/,
knowledge/, CLAUDE.md, README, requirements.txt, .gitignore) run everything; plans/ alone runs
nothing; PIR_TEST_SCOPE=all runs everything. Every doubt widens the selection — including a diff
the selector cannot compute, which widens to everything rather than silently running less.

The deciding part is a pure function (§3.1); the shell part computes the changed list with
`git diff --name-only "$(git merge-base HEAD main)"..HEAD` and runs the chosen commands, one shell
each, from the repo root, printing one line per suite. The selector gets its own unit tests on
synthetic change lists.

A known consequence, accepted at the requirements check: after T02 merges, every later task's
merge-base diff includes the move (root files), so its scope widens to everything and the full
suite — the better part of an hour — runs for tasks that changed little. Correctness over speed;
the alternative (diffing against the run tip instead of main) would under-run for any session whose
branch predates a merged task. Do not "optimise" this without reopening decision 2.

### 2.5 The Superfrog move

Superfrog moves into games/superfrog/ as one task (decision 1), with git mv so history follows.
The path rules:

- Engine-to-toolkit path dependencies: engine crates already depend on the toolkit by path
  (`../../../toolkit/crates/…`); the move rewrites them to `../../../../toolkit/crates/…`. Nothing
  else changes in the workspaces; the profile overrides in engine/Cargo.toml move with it.
- Disk location: every disk_dir becomes `SUPERFROG_DISKS`, else `<repo>/games/superfrog/`, else
  the repo root. The game owns its disks, so its folder is the default; the root fallback keeps
  pre-move checkouts and the transition working, and it is what the owner's own checkout uses
  until the run merges.
- Generated output: the game's tools and sf-routeview write build/ (route-viz included) under
  games/superfrog/, and the app's bug zips default to games/superfrog/bugs/. Game artifacts live
  with the game.
- The split points the draft names: the generic halves of re_listing.py and re_ghidra_import.py,
  tools/setup/*, check_no_disk_data.py and their disk-free tests go to toolkit/tools/; Superfrog's
  chunk table stays game-side; docs/architecture splits by whether the chapter's subject moved to
  toolkit/crates.
- plans/ is untouched, including every path its files cite. History stays history.
- The owner's disk zips move from the main-checkout root into games/superfrog/ as the task's last
  action, after the suite is green (§5.2, §5.3). The way back is one mv.

### 2.6 PLAYBOOK.md and the game template

toolkit/PLAYBOOK.md records the steps Superfrog took, in order, each with what it produced and the
lesson: disks and how the game boots; the packer; the file formats; the executable map; the oracle
and the first conformance case; porting routine by routine against it; the app; the recorder; the
planner. Every step cites its artifacts by their post-move paths, and every line naming a game
carries `learned on:` — the only way the playbook can live in tier 2 under rule 3. It also carries
the north-star's game-two note: the toolkit still lacks an AmigaDOS (OFS) .adf reader, which
Superfrog never exercised and Forest Dumb needs.

toolkit/template/ holds a new game's skeleton: a CLAUDE.md outline (what the game is, its disks
and where they live, the what's-known index, addressing, bug-fix procedure), docs/ with the
numbering convention and a 01-disk-location.md holding the disk-location convention
(`<GAME>_DISKS`, else `<repo>/games/<game>`, else the repo root), a .gitignore for the game's
disks and artifacts, and a conformance case stub pointing at amiga-conform. A new game is the
template copied plus a first plan that walks the playbook's opening steps.

### 2.7 Forest Dumb Forever's skeleton and the cracked-copy decision

T04 copies the template into games/forest-dumb/, fills the outline with what the north-star
established (OFS disks labelled !EAGLE_SOFT!, six worlds of _Mapa files, no Team 17 shared items),
adds the stub test command the selector needs, and names the owner's zips in ~/Documents/Amiga as
the disk set. The game's first plan — the next pir plan run, not a task of this one — walks the
playbook's opening steps.

The cracked-copy question is put to the owner in T04: proceed with the cracked [cr WT] two-disk
set as game two's source — the faithful bar's measuring stick would be the cracked release, whose
executable may differ from the original's (the WHDLoad install is built on this same release) — or
hold for an uncracked copy, with only the toolkit's OFS reader able to proceed meanwhile. Only the
owner can weigh that; the answer is recorded in FINDINGS.md with its date and reflected in the
game's disk note.

### 2.8 The unhappy paths

- The selector cannot compute the diff (no main ref, a worktree quirk): it widens to everything.
  Running more is slower; running less is wrong.
- A games/<game>/ dir has no test.sh: the selector fails loudly. A silently skipped project is a
  suite nobody knows is gone.
- Plans-only changes: run nothing, exit 0 with a one-line reason — the end-of-run helpers and plan
  review still run everything through PIR_TEST_SCOPE=all.
- A memory file is stale when checked: it is corrected as it moves, and the correction is noted in
  the note itself. Memories are point-in-time; the check is part of the move.
- The memory move crashes midway: nothing is lost — the repo commit holds every note before
  tools/empty_memory.sh deletes anything (its seatbelt refuses to delete a file the repo does not
  hold), and the pointer is the last step.
- The zip move happens while old code still runs: the build is serial (T02 starts only after T01
  merges, and no other task of this run touches disks), the root fallback keeps post-move code
  working against zips in either place, and the owner's app reads its cache, not the zips. A
  straggler test that cannot find the zips fails with the existing clear message.
- A tool exists in two places (setup/oracle.sh is already in both root tools/setup/ and
  toolkit/tools/setup/): the move merges the two copies and the root one is removed, so the two
  never drift again.

## 3. Architecture

### 3.1 The boundary

```
tools/select.py          pure: scope(changed, env) -> {everything, projects, reasons}
tools/test.sh            world: git, the venv, exec of the chosen commands
tools/check_tier_names.py  world: reads toolkit/ and knowledge/; its deciding part, the
                           per-line offence test, is a pure function
```

scope() takes the changed list and the environment as parameters and returns the selection; it
reads no clock, no files, no network. tools/test_select.py enforces it with a source scan that
fails if select.py imports subprocess, os or socket, or opens a file. If that test fails the fix
is to move the offending code out of select.py, never to relax the test — the unit tests on
synthetic lists are only honest while the function is pure. Everything else in this plan is file
moves with no decision logic; the existing Rust purity tests (sf-conform/tests/purity.rs,
toolkit-checks/tests/purity.rs) move with their crates, unchanged.

### 3.2 Modules

- tools/select.py — the scope decision, pure, unit-tested.
- tools/test.sh — the thin shell: diff, select, run, one line per suite.
- toolkit/test.sh, games/<game>/test.sh — one project's test commands, owned by the project.
- tools/check_tier_names.py — rule 3's machine, scanning toolkit/ and knowledge/.
- knowledge/, toolkit/docs/, games/<game>/docs/ — the notes, indexed by their tier's CLAUDE.md.

### 3.3 Data flow

Changed paths and the environment in, a scope out, then the chosen suites run in order and their
exit codes sum to the selector's. Nothing else flows anywhere.

### 3.4 Storage

Knowledge is one topic per file, indexed at the tier root (knowledge/KNOWLEDGE.md, the per-tier
CLAUDE.md index sections); no new state formats anywhere. The only runtime state is the
preexisting disk-location environment variables; the selector writes nothing.

## 4. Testing

- The selector's unit tests (tools/test_select.py) prove the whole scope mapping on synthetic
  change lists, the all-override, the plans-only case and the widen-on-doubt cases.
- The name check's unit tests (tools/test_tier_names.py) prove the banned-token set, the
  normalization, the two exemptions, and that the list derives from games/*/.
- The moved suites — the Python tools tests (82 in the measurement), the engine workspace, the
  toolkit workspace — are the plan's real evidence: identical commands, new homes, run through the
  selector with PIR_TEST_SCOPE=all.
- The playbook path test (toolkit/tools/test_playbook.py) keeps every cited path real.
- None of these can prove the cracked-copy judgement, which is the owner's (§5.1); no surface is
  built by this plan, so nothing needs driving end to end.

## 5. Environment — read this before running anything

| | |
|---|---|
| OS | macOS, Darwin 25.5 |
| Language / runtime | Python 3.15.1 (one shared venv at the repo root); Rust pinned to 1.99.0 by rust-toolchain.toml in the engine (after T02: games/superfrog/engine/) and toolkit/ |
| Toolchain | cargo 1.99.0, ripgrep 15.2, git, /bin/sh |
| **Deliberately absent** | Node (nothing here needs it); no new pip packages; no per-project venv or toolchain |

**The test command.** The block above is the end state: setup unchanged from the stored lines
(measured: leaves the copy clean), the test line the selector. Until T02 merges, each branch keeps
the stored three-line block in its own .pir/settings.json, so nothing breaks mid-build. The
selector redistributes exactly the stored commands, measured green in a fresh copy at plan time:
the Python suite (82 tests, 2 skipped, ~23 s), the engine workspace and the toolkit workspace, all
exit 0. The engine suite is the slow part — a warm re-run was stopped at 30 minutes — so the
everything scope is the better part of an hour; that is the status quo, not a regression.

**It is the only evidence a session may produce on its own.** The commands carry the quiet
settings (unittest `-q` with `NO_COLOR=1`, cargo `-q`); failures print in full; to debug, drop the
`-q`s or run the project's test.sh directly. No colour is forced on this machine (no FORCE_COLOR,
CI or CLICOLOR_FORCE in the environment), and NO_COLOR=1 is set inside the commands, not trusted
to the caller.

**Dependencies.** No task in this plan adds a dependency. The selector and the check are Python
stdlib plus /bin/sh and git; the template is text. requirements.txt stays the one shared list at
the root, installed into the one root venv; a project adds a Python dependency by appending there
until two projects genuinely diverge.

**End to end.** Nothing in this plan has a surface; there is nothing to drive.

### 5.1 What the test command cannot reach

| Cannot be tested automatically | Why it needs a person |
|---|---|
| The cracked-copy decision for game two | Only the owner can weigh proceeding on a cracked release — whose executable may differ from the original's — against holding for an uncracked copy. |

### 5.2 Seatbelts

| Flag / mechanism | Default | Effect |
|---|---|---|
| tools/empty_memory.sh's repo check | refuses | Deletes a memory file only if the repo already holds its content; the pointer is written last. |
| games/superfrog/tools/move-disks.sh | run last, after the green suite | Moves the zips only after the code and tests agree on the new home; idempotent; moves nothing if none are found. |
| The selector's widen-on-doubt | everything | Any unparseable diff runs everything, never less. |

**Never ask the owner to run an unbounded version of anything here to find something out, and
never run it yourself.** The two moves above are bounded by their scripts' own checks and by their
one-command ways back.

### 5.3 Outside the code — who acts

| Action | Command (exact, wrapped) | Bin | Why this bin | Way back | Expected cost | Login check |
|---|---|---|---|---|---|---|
| Move the owner's four disk zips from the main-checkout root into games/superfrog/ | games/superfrog/tools/move-disks.sh (ships in T02; idempotent, reports what it moved) | worker | Local, reversible, free; the owner pre-approved the move in draft 6 decision 1 | mv games/superfrog/*.zip back to the root | none | none (no remote) |
| Empty this machine's Claude memory directory for the repo to a pointer | tools/empty_memory.sh (ships in T01; refuses any file the repo does not hold) | worker | Reversible from the repo — the commit holds every note; the owner pre-approved it in draft 6 | Copy the twelve notes back from knowledge/, toolkit/docs/, games/superfrog/docs/ | none | none (no remote) |

**Credentials present on this machine:** none are needed — the plan touches no cloud account, paid
service or remote; its only outside objects are local files (the zips, the memory folder). Nothing
was measured missing because nothing is assumed.

## 6. Recovery

If the merged run has to be undone: reverting the merge commit restores the old layout, the zips
come back with one mv, and the memory files are all in the repo — copy them back from their notes.
The root fallback in disk_dir means old and new checkouts both keep finding the disks through the
transition, whatever order the steps land in.

## 7. Decisions and rationale

- Owner, 2026-10-28 (draft 6, settled and binding): one repo with three projects; knowledge in the
  repo in three tiers, not in Claude's memory; Superfrog moves into games/superfrog/ (decision 1);
  each game gets its own test command behind a selector with the all-override (decision 2); the
  repo keeps its name superfrog-re (decision 3).
- Owner, 2026-10-27 (NORTH-STAR J1): game two is Forest Dumb Forever; the disk set at hand is the
  cracked [cr WT] two-disk release in ~/Documents/Amiga.
- Plan time, owner-confirmed at the requirements check: the planner-stall rule goes to
  games/superfrog/ (tier 3), not knowledge/, because it names Superfrog's tools — the draft's "it
  stays tier 3 until the planner is generalised".
- Plan time: disk lookup is SUPERFROG_DISKS, else games/superfrog/, else the repo root. The root
  fallback exists for the transition and for pre-move checkouts; remove it only when nothing old
  remains.
- Plan time: the zips move at the end of T02. Safe because the build is serial — T02 starts only
  after T01 merges, and no other task of this run touches disks.
- Plan time, owner told at the task-list check: after T02 every task's scope widens to everything
  by the settle rule; accepted as the cost of never under-running.
- Plan time: PLAYBOOK.md lives in tier 2 under "learned on:" markers — the only reading of rule 3
  under which the draft's tier table and the playbook can both hold.
- Plan time: docs/architecture splits by where the subject's code moved; the borderline chapters
  are ruled in T02's doc so the worker does not have to argue with itself. The chapter validator
  stays game-side and checks both trees.
- Plan time: requirements.txt stays one shared list and README.md stays at the root with updated
  paths; neither gets split, because nothing forces the cost of two places to look yet.
- Plan time: the memory emptying and the zip move are worker actions with scripts as seatbelts;
  both were pre-approved by the owner in draft 6 and both are one command back.

## 8. Explicitly out of scope

- Forest Dumb Forever's reverse-engineering: that is its own first plan, the next pir plan run,
  walking PLAYBOOK.md's opening steps.
- Writing back to .adf files (the ATN! compressor): unrelated, still open.
- The map editor: still next for Superfrog, in its own plan.
- Renaming the repository (decision 3 keeps superfrog-re).
- Rewriting old plans' paths: plans/ is sealed history.
- Splitting the repo into submodules or separate git repositories: the monorepo is the settled
  shape, and path dependencies are its point.
- Per-project venvs or toolchains: one venv, one requirements.txt, one pinned Rust.
- Touching the pir-method text in the root CLAUDE.md: it is installed by pir-install and stays
  byte-for-byte.
- Promoting the planner-stall rule to tier 1: that happens when the planner is generalised, which
  a later plan may do.
