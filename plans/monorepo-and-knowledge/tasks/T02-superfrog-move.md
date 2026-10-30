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
  nothing Superfrog-specific), re_ghidra_import.py and its test if it is disk-free (read it;
  if it needs Superfrog data it stays game-side and tests the game's use),
  check_no_disk_data.py + test_check_no_disk_data.py, setup/ (install-brew-deps.sh,
  install-ghidra.sh, install-rust.sh, oracle.sh — the root oracle.sh merges over
  toolkit/tools/setup/oracle.sh; diff them and keep the union, then remove the root copy),
  test_install_brew_deps.py, test_oracle_sh.py. → games/superfrog/tools/: everything else,
  including re_listing.py's Superfrog driver (chunk table, RAW_REGIONS, disks_dir, the atn
  import, the CLI) importing the toolkit half, and all remaining test files.
- games/superfrog/test.sh, toolkit/test.sh (new — Interface below), and
  games/superfrog/tools/move-disks.sh (new — the §5.3 script).
- engine/crates/*/Cargo.toml: every `../../../toolkit/` becomes `../../../../toolkit/`.
- Disk lookup (sf-conform/src/disks.rs, sf-oracle/src/config.rs, the game-side re_listing.py,
  toolkit/tools/check_no_disk_data.py, sf-e2e/README.md): SUPERFROG_DISKS, else
  `<repo>/games/superfrog/`, else the repo root (DESIGN §2.5); the error message names all three.
- Generated output: render_levels.py, find_secrets.py, export_annotated.py, unpack.py,
  extract_disks.py and sf-routeview write under games/superfrog/build/ (route-viz included);
  sf-app's bug-zip default folder becomes games/superfrog/bugs/, and handover.py,
  recording_guide.py and README's bugs/ references follow.
- .gitignore: `engine/target/` → `games/superfrog/engine/target/` (build/, renders/, bugs/,
  *.zip, *.adf, *.sflevel already match at any depth).
- .pir/settings.json: the test line becomes `["tools/test.sh"]`; setup lines unchanged.
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

# games/superfrog/test.sh (run from games/superfrog/):
#   cd engine && cargo test -q --workspace
#   cd .. && NO_COLOR=1 ../../venv/bin/python -m unittest discover -q -s tools -p 'test_*.py' || [ $? -eq 5 ]

# games/superfrog/tools/move-disks.sh: moves the zips (any *.zip/*.adf whose name holds "Disk N")
# from the main checkout root (the git common dir's parent) into <main checkout>/games/superfrog/;
# prints what it moved; moves nothing and exits 0 if none are found. Idempotent.
```

## Tests

- [ ] tools/test_select.py: the decision-2 cases — a game path, a toolkit path, each root file
  kind (.pir/, tools/, knowledge/, CLAUDE.md, README, requirements.txt, .gitignore), plans-only,
  the all-override, an empty diff, mixed scopes merging by union, unparseable input → everything,
  and the purity source scan of select.py.
- [ ] The moved Python suites pass from their new homes (the game's discover run and
  toolkit/tools').
- [ ] A synthetic, disk-free test covers the generic half of the split re_listing.py.
- [ ] `PIR_TEST_SCOPE=all tools/test.sh` exits 0 on this branch — every suite green from its new
  home, the Python count still 82 tests (2 skipped). Budget the session: the engine suite is the
  slow part, the everything scope ~an hour (FINDINGS 2026-10-30).
- [ ] Grep checks after the move: no `engine/`, root `docs/` or moved `tools/<name>` references
  outside plans/ (README, both CLAUDE.md files, .pir, scripts); no `../../../toolkit` left under
  games/superfrog/engine/.
- [ ] The disk_dir tests updated for the three-step default (env, game folder, root) and green.
- [ ] tools/check_tier_names.py still passes — toolkit/ gained files, none may name a game
  unmarked.
- [ ] check_no_disk_data.py passes from toolkit/tools/.

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
