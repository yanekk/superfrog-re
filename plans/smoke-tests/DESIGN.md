---
setup:
  - python3 -m venv venv
  - venv/bin/pip install -q -r requirements.txt
test:
  - tools/smoke.sh
---

# Smoke tests — Design

## 1. Purpose

With two games and a toolkit, running every test takes about 70 minutes, and pir's end-of-build check
runs that on every build (forest-dumb-start: two runs, 1h13m each). This plan splits the tests into a
smoke set of at most 15 minutes, which pir runs at the end of every plan, and a full set, which pir
never runs and a nightly schedule outside this plan does. It is for the owner, who wants a build's end
check to say quickly whether the system works, and for the agents who keep the smoke set honest as the
games grow.

### Success criteria

- `tools/smoke.sh` runs in 15 minutes or less on this machine with nothing else running, compile
  included, measured by T07.
- `tools/full.sh` runs every test, the ones marked `#[ignore]` included, except the screen tests of §2.1.
- A test file with no label, or a crucial area with no live smoke test, fails `tools/smoke.sh`.
- This plan's own end gate runs `tools/smoke.sh` and is green.
- No session of this plan runs the full set.

### Stance

Smoke is a selection, never a replacement: no test is deleted or weakened, and full keeps everything.
What goes in smoke is stated in files a machine checks, so a forgotten test is a failure and not a
silence.

---

## 2. Behaviour specification

### 2.1 Two commands

`tools/smoke.sh` always covers the root checks, the toolkit and every game, whatever the branch
changed. It ignores the selector because pir's end gate sets `PIR_TEST_SCOPE=all` anyway and a selection
at the end of a build would be one more way to miss a break.

`tools/full.sh` sets `PIR_TEST_SCOPE=all` and `PIR_TEST_INCLUDE_IGNORED=1` and execs `tools/test.sh`.
`toolkit/test.sh` and each game's `test.sh` pass `-- --include-ignored` to cargo when the variable is
set. The variable is new so that `tools/test.sh` on its own behaves as today.

Full leaves out the 32 Superfrog screen tests, those whose `#[ignore]` reason names `SF_E2E_OS=1`, unless that
variable is set. They panic without it, so full would be red every night whatever the code did, and with it they
post real input to the screen. They are found from that reason in the source, so a new one is left out without
being listed. With the variable set no screen test is left out; their one-at-a-time need is the later plan's (§8). Full also leaves out the `zips` tool unless `SF_T14_ZIPS` is set: it panics without it and checks nothing (owner, T04 review). Full is never run in this
plan, as the owner ordered, so it is proven with a fake `cargo` on the PATH (T04) and for real only by
the first nightly run.

### 2.2 Labels

Each project has a `smoke.toml`: `toolkit/smoke.toml` and `games/<game>/smoke.toml`. One per project
because the name check forbids game names under `toolkit/`, and because a lower-tier fact belongs at its
tier (root CLAUDE.md, rule 1). Format, with paths relative to the folder of the file:

```toml
workspace = "engine"          # the cargo workspace folder; "." for the toolkit

[tests]                       # every tests/*.rs and tests/*/main.rs, and every crate's inline tests
"crates/x/tests/quick.rs" = "smoke"
"crates/x/tests/slow.rs"  = "full"
"crates/x/tests/mixed.rs" = ["one_named_test", "another"]   # smoke runs only these
"crates/x/src"            = "smoke"                         # inline #[test]s in a crate's src/

[python]                      # every test_*.py in the project
"tools/test_x.py" = "smoke"

[[area]]
name = "picture matches the original"
covers = ["crates/x/tests/quick.rs", "crates/x/tests/mixed.rs::one_named_test"]
```

The root's `tools/test_*.py` and `tools/check_tier_names.py` carry no label and always run in smoke:
they take seconds and guard the machinery itself.

### 2.3 The check

Run first by `tools/smoke.sh`, taking seconds, and failing the run before any test starts. A worker
finds out in seconds rather than after 15 minutes. It fails when, naming the file and the rule:

- a game folder has no `smoke.toml`;
- a test file or a crate with inline tests has no label (found from tracked files via `git ls-files`,
  so a scratch file in a worktree does not fail it);
- a label names a file or crate that does not exist, or a test function not in that file;
- a named test is `#[ignore]`d, since smoke would then skip it while appearing to run it;
- a project with test files has no `[[area]]`;
- an area has no `covers`, or a cover is not labelled smoke, or a whole-file cover has no live test.

It is static: it reads source and never builds. It cannot tell how much of an area a test exercises;
the owner accepted that when dropping measured coverage (§7).

### 2.4 What belongs in smoke

Every fast test goes in smoke: a test file is fast when, run alone, it finishes in 10 seconds or less,
emulator or not. A slow test goes in only when a crucial area needs it, and then by named test where the
file holds more than the area needs. The owner set this so smoke carries as much as cheaply fits.

When the budget is short, fast tests move to full first and crucial tests never do. If crucial tests
alone do not fit, the worker asks the owner what to drop or swap and does not choose.

### 2.5 The crucial areas

The owner approved this list on 2026-10-31. An area may gain a test without asking; removing one needs
the owner. Candidates in T05 and T06 name the tests for each.

- Toolkit: each piece's own quick tests, and the rule checks (tier names, purity).
- Forest Dumb: reading its disks; its first comparison with the original.
- Superfrog data: reading the disks and the game data.
- Superfrog against the original: the original boots and repeats exactly; power-on to the title;
  entering a level by its code; the frog in one level; a sample of enemies; the picture; the sound.
- Superfrog level can be finished: W1-1 played to the exit against the original,
  `complete_w1_1_to_exit_fade`. Chosen over the real-app route because faithfulness to the original is
  the project's bar (root CLAUDE.md, The bar) and the app has its own smoke tests.
- Superfrog tools: widescreen; save states; the bug recorder; the route planner.
- Superfrog app: it starts, its menus work, its window opens.

The owner dropped the story intro as not crucial, and the boss area, Superfrog having only one boss.
(`complete_w1_1_to_exit_fade` is a named test in `frog_cases.rs`, whose whole file takes 9 to
15 minutes, so it can only be in smoke by name.)

### 2.6 The time limit

Smoke prints a summary with total seconds and its slowest entries. Over 15 minutes it prints a line
starting `smoke: WARNING` and still passes. At 20 minutes it kills what is running, prints which entries
did not run, and fails. Warn-then-fail rather than a hard 15 because a busy machine should not turn a good
build red, while smoke still cannot creep back towards an hour. The clock starts when `tools/smoke.sh`
starts, so pir's wait in its test queue and the setup lines do not count. Both limits are constants in one
place.

Every completed or killed run appends one line to `~/.cache/superfrog-re/smoke-times.tsv` (date, commit,
seconds, result), overridable with `SMOKE_TIMES_FILE`. pir's report records only pass or fail, and changing
pir is out of scope, so the warning alone would be read only by someone opening the end gate's output; the
history gives agents a place to look before adding slow tests (§2.7). The owner accepted this on
2026-10-31. Failure to write the history prints a line and does not fail the run, since it is not evidence
about the code.

### 2.7 Directives for agents

The rule goes in the root `CLAUDE.md` Conventions, in each project's `CLAUDE.md` and in the template,
because those are read by every session. In substance: a change that adds something crucial adds its test
to `smoke.toml` and an area for it, and a plan adding a crucial feature puts that in its task's Done when;
every new test file gets a label; fast tests are smoke; read the smoke-times history before adding a slow
test; if new tests would push smoke over 15 minutes, move fast tests to full, and if crucial tests alone
do not fit, ask the owner. This is what makes smoke grow with the games.

### 2.8 The unhappy paths

- A smoke test fails: the run carries on through the rest and exits non-zero, as `tools/test.sh` does, so
  one failure shows every break. pir's end gate then treats it as any red run.
- Disks or Kickstart missing: tests fail naming where they looked, as today. Smoke needs them too.
- A test hangs: the 20-minute limit ends it and fails the run.
- Smoke killed by anything else, pir's `SIGKILL` of the test line's process group included: no entry survives,
  so no emulator is left running. pir's kill reaches only that group and cannot be caught, so an entry in a
  group of its own needs something that outlives the runner to end it (T02).
- A break only full can see: it passes the end check and shows in the next nightly run, possibly after the
  merge. The owner accepted this by asking for the split.

---

## 3. Architecture

### 3.1 The boundary

```
tools/smoke_core.py   pure: parses manifest text, takes file lists and source text as arguments,
                      returns problems and a run list. No files, clock, processes or environment.
tools/smoke.py        shell: git ls-files, reads files, runs commands, the clock, the history.
tools/smoke.sh        finds the repo root and execs venv/bin/python tools/smoke.py.
```

`tools/test_smoke_core.py` holds a test scanning `smoke_core.py`'s imports for anything beyond `re`,
`tomllib` and `dataclasses`. If it fails, the fix is to move the code, never to relax the test. Everything
on the pure side is testable in milliseconds, and each rule that leaks across needs a slow rig.

### 3.2 Run list

Entries run in order: root checks, toolkit, each game alphabetically. Each Rust entry is one cargo call
from the workspace folder:

- whole file: `cargo test -q -p <crate> --test <file>`
- named tests: the same with `-- --exact <names>`; a test inside a module is labelled by its path
- inline tests: `cargo test -q -p <crate>` with `--lib` when it has a library and `--bins` when it has binaries
  (`sf-app` has no library, and `--lib` is an error there)

Per-entry calls are chosen over one big call because test file names repeat between crates (`purity`,
`front`) and `--test` matches all of them. `cargo tree` shows `-p` resolves different features of shared crates
than `--workspace`, so a workspace pre-build does not cover per-crate calls. If that costs real time, T07 may
group calls into `-p` sets; that is a how, recorded in FINDINGS.

Python entries keep today's working directories, because tests may rely on them: the toolkit's from the
repo root, a game's from its folder, using `unittest discover -q -s <dir> -p <file>`.

### 3.3 Storage

The manifests are in git. The history is a local, append-only file with one short write per run, and is
outside the repo so that it never dirties a worktree. `plans/smoke-tests/timings.tsv` (T03) holds the
measured file times; it contains no disk-derived data.

---

## 4. Testing

`tools/test_smoke_core.py`: every rule of §2.3 on small synthetic inputs. `tools/test_smoke_shell.py`: the
limits, the kill and the history on fake commands with limits of a fraction of a second, never real
sleeps of minutes. `tools/test_full_command.py`: the flag reaches cargo through a fake `cargo` on the PATH.
`tools/test_smoke_labels.py` (T07): the check passes on this repo, so full catches a label gap too. T07's
real run and this plan's end gate are the only evidence that smoke fits. Nothing here can show that full
passes, because full is not run.

**Two sanctioned runs beyond a worker's regression set.** T03's capped timing pass and T07's real smoke
runs are what those tasks are for. Each carries its own time bound and goes through `pir-test`. No worker
runs `tools/full.sh` or `PIR_TEST_SCOPE=all tools/test.sh`.

---

## 5. Environment

| | |
|---|---|
| OS | macOS 26 (Darwin 25.5.0), arm64, 10 cores, 16 GB |
| Rust | rustc 1.99.0, pinned by `rust-toolchain.toml` |
| Python | python3 3.15.1 in `venv/`; `tomllib` is in its standard library |
| **Deliberately absent** | cargo-nextest, cargo-llvm-cov: not added, per the dependency rule below |

**The test command.** `tools/smoke.sh`, the block's line. The stored line in `.pir/settings.json` is
still `tools/test.sh` until T08 changes it, and this block already names the new one so this plan's end
gate runs smoke. The gate reads the block only at the end, by which time the script is merged. Plans
already written keep their own blocks.

**Setup.** The stored lines, unchanged; this plan installs nothing. They leave the copy clean.

**Quiet.** Smoke prints one line per project plus the final summary, passes `-q` to cargo and
`NO_COLOR=1` to Python, and sets `CARGO_TERM_COLOR=never`. Nothing forces colour here (`env` shows only
`COLORTERM` and `TERM`), and these keep that true regardless of the caller. Failures print in full. For
detail, run one entry's cargo command without `-q`, or set `SMOKE_VERBOSE=1`.

**Dependencies.** None added: Python standard library and the tools already here.

**Timings from past runs, 2026-10-31.** Mined from 388 session transcripts of the last 14 days, by reading
`test result ... finished in` per test binary. Rough: many are from a busy machine or a partial filter.
Everything-run about 70 min; frog_cases 513 s; recorder_compare 262 s; story 197 s; recorder_drill 192 s;
cases 156 s; view 144 s; widescreen_complete 130 s; front 109 s; sf-e2e smoke 48 s; first_case 33 s;
exe_map_claims 33 s; window 30 s; menus 18 s. By the worst sample about 35 of 139 files were 10 s or less,
64 over, and 40 had no sample. T03 replaces these with measurements.

### 5.1 What the test command cannot reach

| Cannot be tested automatically | Why it needs a person |
|---|---|
| The nightly schedule and how `tools/full.sh` fares there | Out of this plan; the owner sets it up and sees the first night's result |

### 5.2 Seatbelts

| Flag / mechanism | Default | Effect |
|---|---|---|
| Smoke's hard limit | 20 min | Kills the process group and fails, so smoke cannot run away |
| T03's per-file cap | 11 s | Stops each file once it cannot be fast, so the pass never amounts to the full set; at most about 30 min |

### 5.3 Outside the code

None. No task deploys, pays, publishes or changes anything outside this machine's checkouts; the one write
outside the repo is the local history file (§2.6).

---

## 7. Decisions and rationale

All by the owner, 2026-10-31.

- pir runs smoke at the end of a plan and never runs full; a nightly schedule outside this plan runs full.
- Proof of coverage is the machine-checked checklist and labels. Measured code coverage was first chosen as
  a separate nightly command, then dropped from the requirements the same day.
- `@repo/single` runs take smoke too, since they read the same stored line T08 changes and a second one needs a pir
  change. A plans-only change now waits for smoke, and a break only full sees waits for the nightly.
- Over budget: warn at 15, fail at 20, rather than fail at 15 or warn only.
- Fast tests all go in smoke, fast being 10 s alone. Fast tests leave for full before crucial ones.
- W1-1's finish is proven against the original, not in the real app and not both.
- Full includes the 72 `#[ignore]`d tests that need no screen. Their total time is unknown, one alone can take
  40 minutes, and some may fail, so the first nightly run may be long or red. Chosen knowingly over leaving them out.
- The 32 screen tests leave full and go to a separate later plan running them in a `tart` virtual machine, so the
  nightly can pass from the start and never takes the screen. They run at night only once that plan is built.
- The warning does not reach the build report, because that needs a pir change; it is in the gate's output
  and the history file instead.

Survey, by the planner. `tools/test.sh` and the projects' `test.sh` are extended for full rather than
replaced. Smoke is a new runner rather than a mode of `tools/test.sh` because it runs per entry with a clock,
which does not fit a selector of whole projects. Existing tests carry nearly every area, so few or no new
tests are planned. The older `faster-tests` plan (never built) speeds up the full run; it is independent and
neither blocks nor is blocked.

---

## 8. Explicitly out of scope

- The nightly schedule and any notice of its results: the owner's, separately.
- Running the 32 screen tests in a `tart` macOS virtual machine: a separate later plan. Its image is 25 GB against
  24 GB free here on 2026-10-31, and the tests are unproven in a guest.
- Measured code coverage: dropped by the owner.
- Making the full set faster: `faster-tests` is that plan.
- Any change to pir, whose installed copy is not in this repo.
- Moving, deleting or weakening a test: a selection must not change what is tested.
- Re-timing or un-ignoring the `#[ignore]`d tests: the first nightly run will show what they need.
