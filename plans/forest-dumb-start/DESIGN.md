---
setup:
  - python3 -m venv venv
  - venv/bin/pip install -q -r requirements.txt
test:
  - tools/test.sh
---

# Forest Dumb Forever, the start — Design

## 1. Purpose

Forest Dumb Forever (L.K. Avalon, Eagle Soft) is game two, the first game after Superfrog to be
rebuilt in Rust on the toolkit. This plan starts the reverse-engineering: it proves the emulator
can judge the game, opens the disks, documents what they hold, and lands the first automatic
comparison of our code with the original. The measuring stick is the owner's cracked `[cr WT]`
two-disk release, decided 2026-10-30 and recorded in `games/forest-dumb/CLAUDE.md`. Everything is
judged by machine against the original running on the emulator (the faithful bar,
`knowledge/self-testing-faithful-bar.md`), never by a person playing or looking.

### Success criteria

- The two disks boot headlessly in the oracle, repeatably, with input, memory, picture and sound
  exposed per frame.
- Every file on both disks is extracted and listed in a committed inventory with its size and
  checksum; the main program matches its known 521 640 bytes.
- The disk layout, boot chain, file formats and the first executable map are documented; every
  level map and every sprite sheet renders from the real data.
- The toolkit's frame length is a value a game supplies; Forest Dumb's measured value is recorded
  and used.
- The first conformance case compares one modelled variable of our stub with the original at the
  title screen and passes in the game's test suite.

### Stance

- The cracked release is the measuring stick, whatever its differences from the original; an
  uncracked copy, if one turns up, is the thing to compare against.
- Disk images and everything derived from them stay out of git; only descriptions of what they
  hold are committed.
- Where a good third-party reader exists, take it in behind our interface rather than writing our
  own (owner, 2026-10-30); permissive licences only, because the toolkit crates are linked into
  the engine.
- The first case models one variable, the smallest real port; it proves the two sides can be
  compared before any routine is ported.

## 2. Behaviour specification

### 2.1 The gate: the oracle spike (T00)

Before anything is designed on top, one throwaway session proves the oracle serves this game: the
two disks boot headlessly from power-on, two runs with the same input produce byte-identical
memory and frames, input arrives per frame, and memory, picture and sound are readable. It also
measures the frame length, the boot time to the title, what input the crack intro needs, and
whether the game wants disk 2 in a second drive or a swap. If the disks do not boot, or a run is
not repeatable and nothing in the oracle's configuration fixes it, the plan stops and reports.
Why: the playbook's step 5 lesson. The first game's spike ran after foundation work; the lesson
is to run it first, because "faithful" is a claim nobody can check without it.

### 2.2 The OFS reader (T01)

The toolkit gains an OFS reader for AmigaDOS (DOS\0) floppies in `amiga-disk`, behind the crate's
own interface: parse the image, list the files, read a file's bytes, errors with byte offsets.
Behind it sits a third-party crate, evaluated first and not assumed good: `amiga-ffs` (MIT or
Apache-2.0, reads OFS), then `affs-read` (MIT), then a hand-written reader cross-checked against
amitools if neither passes. The evaluation runs the candidate on the real disks and compares the
file list and contents against amitools' `xdftool`. Why our interface: callers stay on one API
and one error style whatever the underlying library, and the purity allowlist admits exactly the
chosen crate. Why not the GPL `format198x` family: the toolkit crates are linked into the engine,
and a copyleft dependency changes what the whole build may be distributed as. amitools itself
stays a dev-time cross-check, never linked.

### 2.3 The disks and the inventory (T02)

The import tool finds the two zips in the order `FOREST_DUMB_DISKS`, then `games/forest-dumb/`,
then the repo root (`games/forest-dumb/docs/01-disk-location.md`), unpacks the two 901 120-byte
ADFs, extracts every file through the OFS reader into `games/forest-dumb/build/` (git-ignored),
and writes a committed inventory of names, sizes and checksums. Known facts are asserted, not
re-derived: the volume name `FOREVER`, the main program's 521 640 bytes, the 48 data files, the
`!EAGLE_SOFT!` marker on disk 2. Why the inventory is committed: names, sizes and checksums are a
description of the disks, like Superfrog's chunk inventory; the files themselves never are. The
owner's zips are opened read-only, and the tool writes nowhere but `build/`.

### 2.4 The boot chain (T03)

Documented from the disks themselves: the boot block, `startup-sequence`, the crack's chain
(`Show -o Kwach4`, `BillClintonToDupek`, `forest_dumb_forever`), and when and why the game asks
for disk 2 (the `!EAGLE_SOFT!` marker). Every file name found as a string in the main program
must appear in the inventory: the map task deepens that table, this task proves the reader and
the inventory agree with what the game actually names. Why: the boot chain decides which tools
apply (playbook step 1), and the name agreement is the OFS reader's first real-world check.

### 2.5 The packer (T04)

Classify every file on both disks: packed or not, by which packer, with evidence. The main
program is stored unpacked on disk (the north star's record), so the question is the 48 data
files. A known packer gets a taken-in decoder; a custom one gets the game's own unpacking routine
run under the emulator's routine runner, located with the map task's help if needed. Why check
first: the playbook's step 2 lesson. Superfrog's packer was unknown only because no known packer
applied.

### 2.6 The file formats (T05)

The level files (`_Mapa1-1.Mapa` and their `_Gen*`, `_MapaBobow*` companions) and the graphics
they point at are documented and proven by rendering: every level map and every sprite sheet
becomes a picture in `build/`, and a picture is how a person and a session point at the same spot
(`<level-id>:<col>,<row>`, the playbook's step 3 lesson). A render is proven right by the
worker-judged pass in T05's end-to-end section, not by its non-blank test alone. The formats stay
in Python tools this plan; the Rust readers arrive with the porting plan that consumes them.

### 2.7 The executable map (T06)

One shared map of the main program, committed as names and notes (`re/symbols.toml`,
`docs/07-executable-map.md`): the hunks, the entry point, the main loop, the custom-chip write
sites, the dos.library file table, the title screen's code path. Regenerated listings and
decompilation live in `build/` only, never in the repo. The map also chooses the variable the
first case models (§2.8), with evidence. Why one map: every porting task starts from it instead
of finding the main loop again (playbook step 4).

### 2.8 The first conformance case (T08, T09)

Shaped like the stub in `games/forest-dumb/conform/case-stub.txt`: boot the disk from power-on on
the oracle and on our side, run N frames of idle title screen fed the same input on both, compare
the one modelled variable (state, not picture; the first game's first case compared only its
title's random generator the same way). The variable must be deterministic across the spike's
runs, computable purely from the game's state at the sync point, and small enough to model
without embedding a table; a table over the purity limit comes from the disks at run time, not
from the crate. The map task picks the variable. If the title offers no candidate, the owner is
asked before the last phase is reshaped.

### 2.9 The frame length (T00, T07)

The toolkit's clock assumes PAL's 313 lines everywhere; the toolkit's README names parameterising
it as the next game's first step. The oracle already reports the frame length of every step
(`amiga_frame_lines`: 313 for a long frame, 312 for a short one), so the spike measures what
Forest Dumb actually runs. The parameterising task threads the frame length as a value through
the remaining consumers (the copper run, the blitter timing, the timeline, Paula's per-frame
resampling; the beam clock and the policy functions already take it), keeps `FRAME_LINES` as the
PAL constant for tests, and leaves Superfrog's engine passing the value it always did, unchanged.
Why now: the brief makes it part of this start, and a toolkit change and its use land in one
commit; the use is `fd-core`'s timing carrying the measured value.

### 2.10 The unhappy paths

- A zip is missing or corrupt: the import reports which file and what is wrong by name and
  checksum, as Superfrog's importer does, and never produces a partial inventory silently.
- The disks do not boot, or a run is not repeatable: the gate stops the plan and reports what
  broke repeatability and what was tried.
- The game wants disk 2 in a particular way at boot: the spike settles the arrangement (second
  drive or swap) before any later task depends on it.
- The disks are absent from a test run: tests that need them fail loudly naming where they
  looked, matching Superfrog's real-disk tests; the owner places the zips once (§5.1).
- A data file is packed with something unknown: the game's own routine under the emulator is the
  reference (playbook step 2), located with the map task's help.
- The title offers no modelable variable: the map task reports the candidates and the owner
  decides.
- A crash mid-extraction: `build/` is regenerable, and re-running the import is the whole
  recovery.

## 3. Architecture

### 3.1 The boundary

Pure side: `amiga-disk` (the OFS reader with the rest), `fd-core` (the stub: the game's timing,
the placeholder scene, the modelled variable). Shell side: `fd-tools` (import and extraction),
`fd-conform` (the oracle and comparison runner for this game), the Python tools (they read and
write files freely; they are tools, not engine crates). Everything on the pure side takes its
inputs as parameters, reads no clock and opens no file; the game's current frame arrives as an
argument.

Enforced by two tests, both existing patterns: the toolkit's purity test (which also admits the
OFS dependency on its allowlist) and the game workspace's own purity test added in T09, patterned
on Superfrog's. If either fails, the fix is to move the code to the shell side, never to relax
the test.

### 3.2 Modules

- `toolkit/crates/amiga-disk/src/ofs.rs`: OFS parse, list, read, behind the crate's `ReadError`.
- `toolkit/crates/amiga-clock`, `amiga-chips`, `amiga-paula`: the frame length as a parameter
  where it is still a constant.
- `toolkit/crates/toolkit-checks`: the purity allowlist admits the chosen OFS crate, one purity
  line per entry, and its game-prefix list gains `fd-` so no toolkit crate can lean on either game.
- `games/forest-dumb/engine/crates/fd-tools`: locate, extract, inventory; the amitools
  cross-checks.
- `games/forest-dumb/engine/crates/fd-core`: the game's timing, the placeholder scene, the
  modelled variable and its layout entry.
- `games/forest-dumb/engine/crates/fd-conform`: the oracle and comparison runner, the scripts,
  the cases, the workspace purity test.
- `games/forest-dumb/tools/`: Python RE tools (renderers, the name scan), tested by unittest like
  Superfrog's.
- `games/forest-dumb/docs/`, `games/forest-dumb/re/`: the numbered chapters and the committed
  symbol map.

### 3.3 The decision function

The whole first comparison rests on one pure function: `fd-core`'s placeholder scene advances per
frame, taking the frame's input and producing the modelled variable. The oracle side reads the
same variable from the real game's memory at the address the map recorded. Nothing else in the
first case decides anything; everything else feeds it.

### 3.4 Data flow

The zips (the owner's, read-only) go through `fd-tools` to `build/` and the inventory. The oracle
reads the same ADF bytes directly. The case: the sync script feeds both sides the same input
frame by frame; `fd-conform` compares the engine's variable against the oracle's memory at the
layout address and prints the verdict.

### 3.5 Storage

`build/` holds every derived artifact (extracted files, renders, listings) and is git-ignored and
regenerable; the inventory, the docs and the symbol map are committed text; the oracle's trace
cache lives under `~/.cache/amiga-toolkit/`. A crash mid-write loses only what a re-run of the
tool regenerates.

## 4. Testing

- The toolkit's pure tests prove the OFS reader on synthetic images built in-test, and the frame
  parameterisation on non-313 lengths; the purity test guards the boundary.
- `fd-tools` tests prove locate and extraction on the real disks (failing loudly when they are
  absent, matching Superfrog's) and assert the known facts (volume, sizes, counts).
- `fd-core` tests prove the modelled variable behaves as the map's evidence describes.
- The first case, in the game's test suite, compares the variable against the original in the
  emulator, the one place our code and the original meet automatically.
- What none of this reaches is in §5.1.

## 5. Environment — read this before running anything

| | |
|---|---|
| OS | macOS (darwin) |
| Language / runtime | Rust 1.99.0, pinned in the engine workspace as Superfrog's is; Python 3.15.1 in fresh worktree venvs (the machine's `python3`; the main checkout's venv is 3.14.8) |
| Toolchain | rustup 1.29.1, cmake at /opt/homebrew/bin |
| Installed here | Ghidra (ghidraRun, plus the ghidra MCP server), Kickstart ROMs in `~/amiga-roms/` (1.3, 2.04, 3.1), vAmiga already built at the pinned commit in `~/.cache/amiga-toolkit/vamiga/`, amitools in the venv (from requirements.txt) |
| **Deliberately absent** | No Rust OFS reader is a dependency yet; T01 adds one. Nothing else this plan needs is missing. |

**The test command.** The block's `test` line, `tools/test.sh`, runs the suites the branch's diff
against main needs, from the repo root; a change under `toolkit/` runs the toolkit's and every
game's suites, a change under `games/forest-dumb/` runs that game's suite alone. The lines come
from `.pir/settings.json` and are not re-derived here. The plan adds a new package folder (the
game's engine workspace) but no new install step: the pinned Rust toolchain is already on this
machine and the venv line is unchanged. The stored lines were re-measured in a fresh worktree
during planning: the setup leaves the copy clean, and the everything-run (toolkit, both games,
root checks) is green in about 70 minutes (plan review re-ran it 2026-10-31, same result).
Re-measured because the plan adds a new package folder.

**Quiet, no colour, loud on failure.** The suites already run quiet (`cargo test -q`, `-q`
unittest discovery) and set `NO_COLOR=1` inside the commands themselves; failures print in full
and the exit codes carry the result. Verbose output for debugging: drop the `-q` flags. Nothing
forces colour on this machine: no `FORCE_COLOR`, `CI` or `CLICOLOR_FORCE` in the environment or
the shell profile (checked 2026-10-30).

**Setup.** The block's venv lines leave the copy clean: `venv/` is git-ignored. Verified in the
re-measurement.

**Dependencies.** The policy the owner set on 2026-10-30: take in a good third-party reader
behind our interface rather than writing our own. What may be added: exactly the crates the tasks
name for this job (`amiga-ffs` for OFS; a packer decoder only if T04 finds a known packer), each
behind our interface, permissive-licensed only, admitted on the purity allowlist with a reason
line. What may not: anything else, and nothing copyleft, because the toolkit crates are linked
into the engine and the licence follows the link.

**End to end.** Nothing in this plan has a surface a person moves through: the tools are
command-line, the first case is judged headless by the harness, and the renders — the plan's
pictures — are judged by the worker who builds them, in T05's end-to-end pass, never handed to
the owner for a feel. No rig task and no prototype; the direction was confirmed in the
requirements list, not in a mock.

### 5.1 What the test command cannot reach

| Cannot be tested automatically | Why it needs a person |
|---|---|
| Placing the two disk zips where the tools look | The owner's files, in a folder sessions cannot list; one copy action, then everything is machine-checkable |

### 5.2 Seatbelts

| Flag / mechanism | Default | Effect |
|---|---|---|
| The oracle's frame cap | 20 000 frames, every run names its own cap | No emulator run is unbounded |
| `fd-tools` write target | `build/` only, asserted in its tests | A bad path cannot overwrite the owner's zips or the repo |
| The owner's zips | opened read-only | The only irreplaceable input is never written |

### 5.3 Outside the code — who acts

| Action | Command (exact, wrapped) | Bin | Why this bin | Way back | Expected cost | Login check |
|---|---|---|---|---|---|---|
| Fetch and build vAmiga at the pinned commit | `toolkit/tools/setup/oracle.sh` | `worker` | Public read-only fetch, already built on this machine; a rebuild changes only the local cache | `rm -rf ~/.cache/amiga-toolkit/vamiga` | none | none |
| Fetch a crates.io dependency | `cargo add amiga-ffs` (or the permissive packer decoder T04 names) | `worker` | Public read-only fetch, recorded in Cargo.lock | revert the lockfile and remove the dependency | none | none |

Credentials measured at plan time: none exist for this plan. No cloud account, paid service,
domain or device is touched; the two rows are both public read-only fetches of source code.

## 6. Recovery

`build/` is regenerable: delete it and re-run the import and the renderers. The oracle's cache
(`~/.cache/amiga-toolkit/`) can be deleted and rebuilt by the script above. The one thing that
cannot be regenerated is the owner's zips, which nothing in this plan writes to.

## 7. Decisions and rationale

- The cracked `[cr WT]` release is the measuring stick (owner, 2026-10-30, recorded in the game's
  CLAUDE.md). It may differ from an original; an uncracked copy is the comparison if one turns
  up.
- Take in third-party readers behind our interface, permissive licences only (owner, 2026-10-30,
  this planning session). The planner recommended hand-written readers; the owner chose take-in,
  and the plan follows it. `amiga-ffs` is the candidate because it is MIT/Apache-2.0 and reads
  OFS; the GPL `format198x` family is out because the toolkit crates are linked into the engine.
- The first case includes the smallest real port, one modelled variable at the title (owner,
  2026-10-30). The alternative, a harness proof with the oracle compared against itself, would
  leave "the two sides can be compared" unproven; the stub the repo ships presupposes two sides.
- The oracle spike is the gate and phase 0, not a later task: the playbook's step 5 lesson,
  learned on Superfrog, where the spike ran after foundation work.
- No prototype: nothing in this plan has a feel to confirm; the direction was confirmed in the
  requirements list, not in a mock.
- The frame parameterisation lands in this plan even though Forest Dumb may measure the same 313
  lines: the toolkit's README names it as the next game's first step, and a toolkit change and its
  use land in one commit (the use is fd-core's timing).
- The inventory commits sizes and checksums: a description of the disks, like Superfrog's chunk
  inventory; the files themselves stay out.
- No Rust readers for the file formats yet: the formats are proven in Python tools this plan, and
  a reader nobody consumes is exactly what a later plan would rebuild differently.

## 8. Explicitly out of scope

- Porting the game itself, the app, the window, sound, the bug recorder, the route planner: the
  next plans (playbook steps 6 to 9).
- Writing to disk images or repacking them.
- FFS volumes: these disks are OFS, and only OFS is exercised.
- The WHDLoad install: it patches the game, so it is never the measuring stick; its recorded
  agreement with the disk set stays a cross-check only.
- An uncracked copy: none exists to work from; if one turns up, compare (decided 2026-10-30).
