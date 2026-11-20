# tools (repo root)

The repo's test commands and its checks. Rules (when to label smoke or full, the 15-minute budget, when
to ask the owner) are under Conventions in the [root CLAUDE.md](../CLAUDE.md); this says how the
machinery works. Scripts use the repo venv `venv/bin/python` (git-ignored; a fresh worktree may lack it).

## test.sh, select_scope.py, full.sh

`tools/test.sh` is the repo's one test command. It diffs the branch against `main` (`--no-renames`) and
pipes the changed paths to `select_scope.py`, which picks the suites: a `games/<g>/` path runs that game's
`games/<g>/test.sh`; a `toolkit/` path runs `toolkit/test.sh` and every game; `plans/` alone runs nothing;
any root file, an empty diff or a failed diff runs everything (toolkit, each game, then the root
`tools/test_*.py`). `PIR_TEST_SCOPE=all tools/test.sh` forces everything. Every `games/*/` folder must
have a `test.sh`, or test.sh exits 1.

`tools/full.sh` is the nightly full set: `PIR_TEST_SCOPE=all PIR_TEST_INCLUDE_IGNORED=1` over test.sh, so
`#[ignore]`d Rust tests run too.

## smoke.sh, smoke.py, smoke_core.py

```
tools/smoke.sh               # label check, then every smoke entry under its clock
tools/smoke.sh --check-only  # label check alone (fast; run it after editing a smoke.toml)
tools/smoke.sh --list        # print each entry: project, cwd, command; run nothing
```
smoke.sh execs `smoke.py`. If the label check finds a problem, it prints `smoke: <problem>` lines and
`smoke: FAILED, the label check found N problems; no test ran`. It always runs the root
`tools/test_*.py` and `check_tier_names.py` first. It warns past 15 minutes and kills the run at 20.
`SMOKE_VERBOSE=1` shows each entry's output.

There is no project filter. To run one project's smoke set, take its lines from `--list` and run each
command in its cwd (the second column), e.g. a game's Python entry is
`../../venv/bin/python -m unittest discover -q -s tools -p test_x.py` run in `games/<g>`.

Every run appends `date<TAB>commit<TAB>seconds<TAB>ok|failed|limit` to `~/.cache/superfrog-re/smoke-times.tsv`
(`SMOKE_TIMES_FILE` overrides).

`smoke_core.py` parses `smoke.toml`, reports problems and builds the commands.

## smoke.toml: labelling a test file

Each project (`toolkit/`, each `games/<g>/`) has a `smoke.toml`:
```toml
workspace = "engine"          # the cargo workspace folder, "." if the project root

[tests]                        # Rust; keys relative to `workspace`
"crates/x-core/src" = "smoke"  # a crate's inline tests
"crates/x-core/tests/a.rs" = "full"
"crates/x-core/tests/b.rs" = ["test_name_one", "test_name_two"]   # only these in smoke

[python]                       # keys relative to the project folder; "smoke" or "full" only
"tools/test_y.py" = "smoke"

[[area]]                       # a crucial behaviour, guarded in smoke
name = "what it covers, in words"
covers = ["crates/x-core/tests/a.rs::one_test", "tools/test_y.py"]
```
What the check refuses, by its messages: `test file with no label`, `crate with inline tests with no
label`, `Python test file with no label`, `labelled, but no such test file or crate`, a listed test
`which is not in that file` or `which is #[ignore]d`, and `the project has test files and no [[area]]`.
Each `[[area]]` cover must be labelled smoke: a whole file, or `file.rs::test` named in the file's list.
A Python file is covered whole, never by test name. `test_smoke_labels.py` runs this check on the real
repo, so a missing label also fails the root tests and the full run.

## smoke_timings.py

Times each Rust test entry alone under a cap, to decide smoke (finishes in 10 s or less) or full. Prints
a TSV: project, key, seconds or `over`/`uncompiled`, status.
```
venv/bin/python tools/smoke_timings.py [--cap 11] [--stop-after SECONDS] [--done EARLIER.tsv]
```
Slow: it compiles everything. Exit 3 stopped with entries left, 4 under 2 GB disk free.

## check_tier_names.py

```
venv/bin/python tools/check_tier_names.py [ROOT]
```
Scans tracked and untracked-not-ignored files under `toolkit/` and `knowledge/` and prints `path:line` for
each line naming a game (any `games/*/` folder name, with `-`, `_` and space interchangeable, plus a few
fixed tokens), exit 1 if any. A line containing `learned on:` is allowed. The repo's own name is not an
offence.

## empty_memory.sh

One-off: empties this machine's Claude memory folder for the repo to a pointer, only after checking every
listed note is committed in the repo.

## Tests

`test_*.py` here test the script they name (`test_select.py` and `test_selector_shell.py`: select_scope
and test.sh; `test_smoke_core.py`, `test_smoke_shell.py`: smoke; `test_full_command.py`: full.sh;
`test_tier_names.py`: the name check). Run them with
`venv/bin/python -m unittest discover -q -s tools -p 'test_*.py'` from the repo root. They are unlabelled
and always run in smoke.
