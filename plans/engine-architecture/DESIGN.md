---
setup:
  - python3 -m venv venv
  - venv/bin/pip install -q -r requirements.txt
test:
  - NO_COLOR=1 venv/bin/python -m unittest discover -q -s . -p 'test_*.py' || [ $? -eq 5 ]
---

# Engine architecture documentation — Design

## 1. Purpose

The Rust engine (`engine/`, plan `plans/rust-engine/`) plays Superfrog faithfully, and it gets
there largely by imitating the Amiga: the game's variables are the original's bytes at the
original's addresses, ported routines charge 68000 cycles so their writes land at the original
beam position, the copper list is run cycle by cycle, and tables are read out of the original
executable as loaded. This plan writes the architecture documentation that separates that
imitation from the game's own rules and from data copied in from the original, so a later plan
can rewrite the Amiga-shaped parts as plain native code while the game still passes every
conformance check.

The same documentation rates every subsystem for reuse in another Amiga game, replacing the
rough 2026-10-15 estimate (about 20% reusable) with a per-subsystem account. That estimate is
recorded nowhere else in the repo: of about 75k lines of Rust on `pir/rust-engine`, about 13k
carry to any Amiga game (`sf-oracle` ~3.7k, `sf-paula` ~1.1k, `sf-conform/src` ~5.5k, the
`sf-app` shell ~1.7k, `sf-e2e` ~1.1k, generic parts of `sf-disk`/`sf-assets` and `render/` ~2k);
the rest (`sf-core` game logic ~38k, asset formats ~3.5k, conform cases, scripts and route
planners ~19.5k, `LAYOUT`) is per game.

The owner reads the overview; the chapters are written for the session that will plan the
native rewrite. No engine code changes in this plan.

### Success criteria

- Every tracked file under `engine/crates/` and `engine/conform/` belongs to exactly one chapter,
  and a test proves it (§2.4).
- Every Amiga-imitating mechanism has a chapter saying what it imitates, why it exists, what
  depends on it, what a native rewrite must preserve and which existing check proves it.
- Every subsystem carries a kind mix and a reuse rating with the changes reuse would need.
- The overview gives the reuse totals in lines and a recommended order for the native rewrite.

### Stance

- Faithful stays the bar (rust-engine DESIGN §2.1). The rewrite this documents is "native, still
  faithful": the docs name behaviour to preserve, never behaviour that may be dropped.
- Describe what the code does today, cite it by path and item name, and link to the RE notes in
  `docs/08`–`docs/13` rather than restating them. Restated material drifts.

---

## 2. Behaviour specification

### 2.1 The four kinds

Every subsystem is classified by the share of its code in each kind. Owner decision 2026-10-22.

| Kind | Key | Meaning |
|---|---|---|
| Pretend-Amiga | `amiga` | Imitates the Amiga's hardware or its timing: memory at original addresses, CPU cycle charging, beam position, interrupts, copper, blitter, Denise, Paula, CIA keyboard, disk loader behaviour |
| Game logic | `game` | Superfrog's own rules: physics, enemies, scoring, flow. Would exist in any reimplementation of the game |
| Copied-in data | `data` | Tables, memory images and assets read from the original's executable or disks and used as they are (hunk images, boot-built tables, jump tables, frame lists) |
| Modern shell | `shell` | Neither: the macOS app, file handling, test tooling. Any Amiga imitation inside it (PAL pacing, CRT filters) is still called out as `amiga` within its chapter |

The fourth kind exists because the app and the test tooling fit none of the three the owner
first named, and forcing them in would blur the totals (owner, 2026-10-22).

A chapter's `mix` is a rough percentage per kind, summing to 100, estimated by reading the code.
It is an estimate, not a measurement; the chapter says in a sentence how it was judged. Its value is
the totals in the overview, which multiply it by the lines each chapter owns.

### 2.2 The reuse rating

Judged against another Amiga game by any developer (owner, 2026-10-22). Where another Team 17
game might share more (the `ATN!` cruncher, `T7MP`, Player 4.1A), the chapter says so and marks
it unchecked.

| Key | Meaning |
|---|---|
| `as-is` | Carries over with no or trivial change |
| `small-changes` | Carries over; a few named, local changes (addresses, a table, a config) |
| `big-changes` | The approach carries; much of the code is rewritten |
| `superfrog-only` | Specific to this game; only the method carries |

Each chapter states the rating, what would change, and the size in lines (from
`tools/architecture.py totals`, §3.2).

A line is a line of code: owned files ending `.rs`, `.cpp`, `.h` or `.wgsl`, tests included.
Recorded input scripts (`engine/conform/scripts/*.txt`, ~95k lines), case and config `.toml`,
patches and Markdown are not counted; `totals` lists them as one separate uncounted row. Owner,
2026-10-22 at plan review: counting every file let the input scripts make verification half the
codebase, and code-only is the basis of the 2026-10-15 estimate the totals replace.

### 2.3 Chapters

Three tiers. A mechanism chapter documents one imitation technique wherever it is used, because
the imitation is woven through the game rather than layered under it (survey, 2026-10-22). A
subsystem chapter documents a part of the game and names the mechanisms it leans on. A component
chapter is the lighter treatment of a crate around the game (owner chose tiered depth,
2026-10-22).

| id | Tier | Depth | Task |
|---|---|---|---|
| `memory-model` | mechanism | deep | T02 |
| `copied-in-data` | mechanism | deep | T03 |
| `cpu-time-and-beam` | mechanism | deep | T04 |
| `frame-interrupts-input` | mechanism | deep | T05 |
| `display-hardware` | mechanism | deep | T06 |
| `sound-chip` | mechanism | deep | T07 |
| `sound-driver` | subsystem | deep | T07 |
| `game-flow` | subsystem | deep | T08 |
| `front-end` | subsystem | deep | T09 |
| `story-and-ending` | subsystem | deep | T10 |
| `level` | subsystem | deep | T11 |
| `frog-and-items` | subsystem | deep | T12 |
| `enemies-and-bosses` | subsystem | deep | T13 |
| `slot-and-bonus` | subsystem | deep | T14 |
| `disks-and-assets` | component | light | T15 |
| `app` | component | light | T16 |
| `verification` | component | light | T17 |

Every deep chapter uses the section layout in `docs/architecture/chapter-template.md` (T01):
what it is; the kind mix and which parts are which; the mechanisms it uses; what depends on it;
for each Amiga-shaped part what a native rewrite must preserve and the existing check that
proves it (an `sf-conform` case, a crate test, an `sf-e2e` test); a non-binding sketch of the
native shape; reuse rating and changes; sources. A light chapter keeps the kind mix, the reuse
section and the places it touches the Amiga-shaped parts, and may drop the rest.

The native sketch is non-binding because designing the rewrite is the later plan's job; the
sketch exists so that plan starts from an informed shape, and it is labelled as such so nobody
treats it as decided.

### 2.4 The front matter and the drift check

Each chapter opens with TOML front matter between `+++` lines:

```
+++
id = "level"
title = "The level"
tier = "subsystem"            # mechanism | subsystem | component
depth = "deep"                # deep | light
kind = "game"                 # the dominant key of §2.1
mix = { amiga = 30, game = 60, data = 10, shell = 0 }
reuse = "superfrog-only"      # a key of §2.2
owns = ["engine/crates/sf-core/src/level/"]
+++
```

`owns` lists path prefixes (a directory ending in `/`, or a file). Each tracked file under
`engine/crates/` and `engine/conform/` belongs to the chapter with the longest matching prefix.
A mechanism chapter may own nothing (`copied-in-data` is a view across files others own).

`tools/test_architecture_docs.py` fails when (owner chose the light check, 2026-10-22):

1. a tracked file in that domain matches no chapter's prefix;
2. two chapters list the identical prefix, or a prefix matches no tracked file;
3. front matter is missing a key, has a value outside the enumerations, or a `mix` that does not
   sum to 100;
4. a repo path written in a chapter or the overview (in backticks or as a link target, starting
   `engine/`, `docs/`, `plans/`, `tools/` or `re/`) does not exist, after stripping a `:line` or
   `::item` suffix; a path containing `*` or `{` is skipped as a pattern.

While the chapters are being written, `kind`, `reuse` and `mix` may be `"tbd"`. T18 sets
`ALLOW_TBD = False` in the test, after which a `tbd` fails it. This is the one switch, so it is
obvious when the docs became complete.

It deliberately does not check item names, addresses or prose. The owner declined the stronger
check because it adds friction to every future change (2026-10-22). A future plan that adds a
file under a directory no prefix covers (a new `sf-core/src/editor.rs`) fails the test; the fix is
to add the path to the right chapter's `owns` and a sentence to that chapter, never to widen a
prefix to a whole crate to silence it.

### 2.5 The memory-address coupling

`sf-conform` transplants the original's memory into the engine and probes the engine's values
back out by original address (`layout::LAYOUT`, rust-engine DESIGN §3.3). A rewrite that drops
the byte-exact state breaks that comparison. Both `memory-model` and `verification` must state
what the comparison needs and what a native rewrite would have to provide instead (a mapping
layer, a probe adapter, or a change to the checks), because that is the decision the rewrite
plan will hit first. The docs present the options; they do not choose.

### 2.6 The overview

`docs/architecture/README.md`: what the documentation is and how to read it, the four kinds and
the reuse scale, a diagram of how the crates and mechanisms connect (Mermaid, which GitHub
renders), the table of every chapter with kind, mix, reuse and lines, the reuse totals in lines
per rating and per kind (computed by `tools/architecture.py totals`), and a recommended order for
the native rewrite with the reason for each step. The repository `README.md` links to it, per
CLAUDE.md "The README follows every major feature".

### 2.7 The unhappy paths

- A chapter task finds code that belongs to another chapter better than the T01 assignment says.
  It notes it in `FINDINGS.md` and leaves `owns` alone, because another chapter task may be
  writing that file's chapter at the same time. T18 settles moves.
- A claim cannot be settled by reading (whether a piece of timing matters to conformance). The
  chapter says what is unknown and how to find out, and does not guess.
- Code changes on `main` during the build (T213 is still open). Chapters describe the code as at
  their task branch; the drift check catches new files at merge.

---

## 3. Architecture

### 3.1 Files

```
docs/architecture/README.md              overview (T01 stub, T18 complete)
docs/architecture/glossary.md            the mechanisms and terms every chapter uses (T01)
docs/architecture/chapter-template.md    the section layout (T01; no front matter)
docs/architecture/{id}.md                one per chapter, §2.3
tools/architecture.py                    parse front matter, assign files, totals
tools/test_architecture_docs.py          the drift check, §2.4
```

The files sit under `docs/` because that is where the project's documentation lives and where the
README already points. They go in their own folder because `docs/01`–`14` are RE notes about the
original; these chapters describe our engine.

Each chapter task edits only its own chapter file, so seventeen tasks can run at once without
conflicts. That is why the classification lives in each chapter's front matter and not in one
shared inventory file.

### 3.2 `tools/architecture.py`

Pure functions over strings and path lists plus a thin `main`:

```
parse_front_matter(text: str) -> dict | None      # None when the file has no +++ block
load_chapters(root: str) -> dict[str, dict]       # id -> front matter, + "path"
tracked_files(root: str) -> list[str]             # git ls-files under the domain
assign(files, chapters) -> (dict[file, id], list[problem])
line_counts(root, assignment) -> dict[id, int]  # code lines only, §2.2; plus the uncounted rest
totals(chapters, lines) -> table rows             # per chapter, per kind, per rating
main: `python tools/architecture.py totals`       # prints the overview's table
```

Python 3.11+ for `tomllib` (the venv here is 3.14). No new dependencies.

---

## 4. Testing

The only automated test is §2.4, with self-tests feeding it synthetic chapters and file lists:
an unowned file, a duplicate prefix, a stale prefix, a bad enum, a mix of 99, a missing path, a
`::item` suffix, a `*` pattern, and `tbd` under both settings. It proves structure, not truth.
Truth is the reviewer's job: each chapter's review samples its claims against the code.

---

## 5. Environment

| | |
|---|---|
| OS | macOS (Darwin 25.5) |
| Python | 3.14.8 in `venv/` |
| Rust | cargo 1.99.0 (only to read code; no task builds the engine) |
| Deliberately absent | No doc generator (mdBook, rustdoc publishing): plain Markdown in the repo is what the project already uses |

**The test command** is the stored `.pir/settings.json` lines, measured on a fresh worktree on
2026-10-22: setup clean, 34 tests pass in about 36 s, quiet by `unittest -q`, `NO_COLOR=1`. For
detail, drop `-q` and add `-v`. The Rust suites are not in this block because no task changes
Rust code; a chapter that cites a test by name checks it exists by reading, not running.

**Dependencies.** None added. Standard library only.

### 5.1 What the test command cannot reach

| Cannot be tested automatically | Why it needs a person |
|---|---|
| Whether the prose is right | Reviewed by a fresh session sampling claims against the code, not by the owner |

### 5.3 Outside the code — who acts

None. No task deploys, calls a paid service, uses a login or reaches another person; the only
install is the setup block's `pip install` into `venv/`. An action with no row is `ask`.

---

## 7. Decisions and rationale

- Rewrite aim is native and still faithful, not free to change (owner, 2026-10-22), so every
  Amiga-shaped part lists behaviour to preserve and the check that proves it.
- Coverage of everything, tiered: deep for the game and Paula, light for app, disks/assets and
  verification (owner, 2026-10-22). The verification tooling is most of the reusable code and
  carries the address coupling, so leaving it out would hide both.
- Granularity per subsystem, not per file or function (owner, 2026-10-22). Mixed subsystems say
  inside the chapter which parts are which.
- Light drift check (owner, 2026-10-22), §2.4.
- Fourth kind `shell` (owner, 2026-10-22), §2.1.
- Reuse judged against another Amiga game (owner, 2026-10-22), with a Team 17 note marked
  unchecked.
- Mechanism chapters alongside subsystem chapters, because the survey found the imitation woven
  through every subsystem (memory, cycle charging, copper) rather than isolated in one layer.
- New folder `docs/architecture/`, linking to `docs/08`–`13` and `plans/rust-engine/DESIGN.md`
  rather than restating them: those describe the original, these describe our engine, and
  restated material drifts. Nothing in the repo already classifies the code this way or rates it
  for reuse (survey, 2026-10-22).
- Classification in per-chapter front matter, not a shared inventory, so chapter tasks never
  edit the same file.

## 8. Explicitly out of scope

- Any change to engine code, including the native rewrite itself: this plan is its input.
- Checking whether other Team 17 games share `ATN!`, `T7MP` or Player 4.1A: needs their disks and
  its own RE work; the chapters only flag the possibility.
- Checking item names, addresses and prose automatically: declined as friction (§2.4).
- Extracting a reusable Amiga toolkit: proposed separately (2026-10-15); these docs inform it.
