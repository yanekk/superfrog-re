# T02 — superfrog-move

**Phase:** 1 · **Depends on:** T01 · **Weight:** heavy

## Goal

The one atomic move into the monorepo layout (DESIGN §2.5), plus the test selector (DESIGN §2.4).
This is one heavy task on purpose: the run branch must go from old layout to new in one merge,
never sit half-moved. Everything moves with git mv so history follows; every suite that passed
before the move passes after it, through the selector.

## Design sections this implements

DESIGN §2.1, §2.4, §2.5.

## Files

- `git mv engine/` → games/superfrog/engine/ (crates, conform/, Cargo.toml, Cargo.lock,
  rust-toolchain.toml).
- `git mv re/` → games/superfrog/re/.
- `git mv docs/` → games/superfrog/docs/; then split docs/architecture/: chapters whose subject's
  code moved to toolkit/crates go to toolkit/docs/architecture/ — display-hardware,
  cpu-time-and-beam, frame-interrupts-input, memory-model, sound-chip, verification. Stays
  game-side: app, copied-in-data, disks-and-assets, enemies-and-bosses, frog-and-items,
  front-end, game-flow, glossary, level, slot-and-bonus, sound-driver, story-and-ending,
  chapter-template, and architecture/README.md (with a link to the moved chapters). These
  rulings are DESIGN §7; do not re-litigate them.
- `git mv SECRETS.md` → games/superfrog/SECRETS.md; find_secrets.py's output path updated to the
  game folder.
- tools/: the root keeps T01's tools/check_tier_names.py, tools/test_tier_names.py,
  tools/empty_memory.sh and tools/__init__.py, and gains tools/test.sh, tools/select.py,
  tools/test_select.py. The rest
  moves: → toolkit/tools/: re_listing.py's generic half (hunk parse, relocate, print symbols —
  nothing Superfrog-specific); re_ghidra_import.py's generic half (Ghidra project import, symbol
  application from a toml, decompilation export) — if it still needs Superfrog data after the
  split, keep the whole tool game-side and say why in the commit message;
  check_no_disk_data.py's scan engine (the indexed-stride run finder over given source blobs
  and repo files, game-neutral: no game names, no game formats, nothing disk-derived) with a new
  synthetic-source unit test — its Superfrog driver (default disk lookup, ATN! chunk sources,
  .sflevel decode) and its real-disks test stay game-side and import the toolkit core;
  setup/ (install-brew-deps.sh,
  install-ghidra.sh, install-rust.sh, oracle.sh — the root oracle.sh's cmake fallback folds into
  toolkit/tools/setup/oracle.sh; diff them and keep the union, then remove the root shim),
  test_install_brew_deps.py, test_oracle_sh.py. → games/superfrog/tools/: everything else,
  including re_listing.py's Superfrog driver (chunk table, RAW_REGIONS, disks_dir, the atn
  import, the CLI) importing the toolkit half, the disk-data check's driver and its test, and
  all remaining test files.
- toolkit/crates/toolkit-checks/tests/names.rs: delete — tools/check_tier_names.py is the one
  name check now (DESIGN §2.3); its team17/team-17 ban is folded into the checker's banned set.
- games/superfrog/test.sh, toolkit/test.sh (new — Interface below), and
  games/superfrog/tools/move-disks.sh (new — the §5.3 script).
- engine/crates/*/Cargo.toml: every `../../../toolkit/` becomes `../../../../toolkit/`.
- Disk lookup (sf-conform/src/disks.rs, sf-oracle/src/config.rs, the game-side re_listing.py,
  the game-side check_no_disk_data driver, sf-e2e/README.md): SUPERFROG_DISKS, else
  `<main checkout>/games/superfrog/`, else the main checkout root — the main checkout resolved
  as today via the git common dir (sf-conform/src/disks.rs main_checkout()), so the lookup works
  from any worktree (DESIGN §2.5); the error message names all three.
- Generated output: render_levels.py, find_secrets.py, export_annotated.py, unpack.py,
  extract_disks.py and sf-routeview write under games/superfrog/build/ (route-viz included);
  sf-app's bug-zip default folder becomes games/superfrog/bugs/, and handover.py,
  recording_guide.py and README's bugs/ references follow.
- .gitignore: `engine/target/` → `games/superfrog/engine/target/` (build/, renders/, bugs/,
  *.zip, *.adf, *.sflevel already match at any depth).
- .pir/settings.json: the test line becomes `["tools/test.sh"]`; setup lines unchanged.
- .claude/settings.json: the four permission rules for tools/setup/* move with the scripts —
  toolkit/tools/setup/install-rust.sh, install-brew-deps.sh, install-ghidra.sh, oracle.sh.
  Permission rules name exact paths, so stale ones silently stop auto-approving the scripts.
- README.md: every path updated; add a short "Repository layout" section naming the three
  projects and the knowledge tiers.
- Root CLAUDE.md: T01 wrote it with final paths; fix anything it got wrong, nothing else.

## Interface

```
# tools/select.py — the deciding part is pure; test_select.py enforces it
def scope(changed: list[str], env: dict) -> dict:
    # {"everything": bool, "projects": list[str], "reasons": list[str]}
    # PIR_TEST_SCOPE=all -> everything. games/<g>/... -> that game (g from the games/*/ dirs).
    # toolkit/... -> toolkit + every game. plans/... -> no projects (run nothing).
    # any other path -> everything. Unparseable input -> everything. Scopes merge by union.
# The module must not import subprocess, os or socket, and must not open files — checked by a
# source scan in test_select.py; if that test fails the fix is to move the offending code out of
# select.py, never to relax the test (DESIGN §3.1).

# tools/test.sh (#!/bin/sh, run from the repo root) — thin shell:
#   changed=$(git diff --name-only "$(git merge-base HEAD main)"..HEAD || true)
# (a failed merge-base -> everything), pipes the list to venv/bin/python tools/select.py, then
# runs the chosen commands, one shell each, in this order:
#   toolkit:        sh toolkit/test.sh
#   <game>:         sh games/<game>/test.sh
#   root checks:    NO_COLOR=1 venv/bin/python -m unittest discover -q -s tools -p 'test_*.py' || [ $? -eq 5 ]
# everything = toolkit + every games/*/ + root checks. Plans-only = exit 0 with a one-line
# reason. Any command failing -> exit non-zero. One printed line per suite run.

# toolkit/test.sh (run from the repo root):
#   cd toolkit && cargo test -q --workspace
#   cd .. && NO_COLOR=1 venv/bin/python -m unittest discover -q -s toolkit/tools -p 'test_*.py' || [ $? -eq 5 ]
#   venv/bin/python tools/check_tier_names.py

# games/superfrog/test.sh — anchors itself (cd "$(dirname "$0")"), so the selector's repo-root
# cwd (DESIGN §2.4) holds whatever the caller's:
#   cd engine && cargo test -q --workspace
#   cd .. && NO_COLOR=1 ../../venv/bin/python -m unittest discover -q -s tools -p 'test_*.py' || [ $? -eq 5 ]

# games/superfrog/tools/move-disks.sh: moves the zips (any *.zip/*.adf whose name holds "Disk N")
# from the main checkout root (the git common dir's parent) into <main checkout>/games/superfrog/;
# prints what it moved; moves nothing and exits 0 if none are found. Idempotent.
```

## Tests

- [ ] tools/test_select.py: the decision-2 cases — a game path, a toolkit path, each root file
  kind (.pir/, tools/, knowledge/, CLAUDE.md, README, requirements.txt, .gitignore), plans-only,
  the all-override, an empty diff → everything (the widen-on-doubt rule: a branch identical to
  main is a doubt, never a reason to run less), mixed scopes merging by union, unparseable input
  → everything,
  and the purity source scan of select.py.
- [ ] The moved Python suites pass from their new homes (the game's discover run and
  toolkit/tools').
- [ ] A synthetic, disk-free test covers the generic half of the split re_listing.py.
- [ ] `PIR_TEST_SCOPE=all tools/test.sh` exits 0 on this branch — every suite green from its new
  home, the Python count still 82 tests (2 skipped). Budget the session: the engine suite is the
  slow part, the everything scope ~an hour (FINDINGS 2026-10-30).
- [ ] Grep checks after the move: no `engine/`, root `docs/` or moved `tools/<name>` references
  in any tracked file outside plans/ — the moved docs included (docs/BUG-FIX.md,
  docs/RE-WORKBENCH.md, the moved architecture chapters, engine/conform/README.md,
  sf-e2e/README.md, toolkit/README.md all cite `tools/setup/…` today); no `../../../toolkit`
  left under games/superfrog/engine/.
- [ ] The disk_dir tests updated for the three-step default (env, main checkout's game folder,
  main checkout root) and green.
- [ ] tools/check_tier_names.py still passes — toolkit/ gained files, none may name a game
  unmarked.
- [ ] The split check_no_disk_data.py passes on both sides: the toolkit core from toolkit/tools/
  (synthetic sources), the game-side driver from games/superfrog/tools/ (real disks).

## Done when

- [ ] The repo root holds exactly the new layout: toolkit/, games/superfrog/ (complete; no
  games/forest-dumb/ yet — that is T04), knowledge/, plans/, tools/ (selector trio, the check,
  __init__), CLAUDE.md, README.md, requirements.txt, .gitignore, .pir/ — with engine/, docs/, re/
  and SECRETS.md gone from the root.
- [ ] `PIR_TEST_SCOPE=all tools/test.sh` exits 0 on this branch, and tools/test_select.py is
  green.
- [ ] .pir/settings.json's test line is tools/test.sh; every old-path reference outside plans/ is
  gone (grep-verified); the zips are moved as the task's last action.

## Outside actions

- move the owner's disk zips — `worker` (DESIGN §5.3): run games/superfrog/tools/move-disks.sh
  as the last action, after the everything-scope suite is green on this branch. Way back: one mv
  of the zips to the repo root.
