---
setup:
  - python3 -m venv venv
  - venv/bin/pip install -q -r requirements.txt
test:
  - tools/smoke.sh
---

# Forest Dumb Forever: the title and menus — Design

Brief: `games/forest-dumb/docs/title-brief.md` (2026-11-20). Read with it `games/forest-dumb/CLAUDE.md`,
`docs/07-executable-map.md` "The title screen's code path", `docs/14-translator.md` and the translator
plan's `DESIGN.md` §2.2, §2.7, §2.8 and §2.10, which this plan works under and does not restate.

## 1. Purpose

Everything a player can do before and between the levels behaves as the original, judged by machine on
the oracle: the disk 2 prompt, the title and its music, the three rows (START, KODY, KONIEC), entering a
level code, Esc, and game over through the high scores back to the title. The translator already runs
all of this in the app (translator T37 to T41), so this is mostly a checking plan: write the cases the
title never had, run them on the translated program, and fix what fails in the translator or the
machine. It also adds one app feature the owner asked for at planning: Game ▸ Start Level… offering
worlds 1 to 6 and a cheat box.

### Success criteria

- `docs/16-title.md` lists every code the title accepts and what it does, each entered on the oracle by
  a test that checks its effect.
- One case each (§2.2) passes on `--engine translated`, and the 50 existing cases read as before
  (49 pass, `power-on-level-1` fails from 6301).
- The same paths driven by key presses in the real app are equal to a headless run every step.
- Start Level… starts any world 1 to 6, with or without the cheat, and a recording made there replays
  to the same state on the translated program.

### Stance

- The translated program is the title. No hand-written title Rust (owner, 2026-11-20), because nothing
  about the title needs one and a second copy would have to be held equal to the first.
- A difference is fixed by pattern in the translator or the machine (translator DESIGN §2.2), never by
  a hint, an address special case or an edit to generated Rust, because a special case hides the next
  routine's identical defect.
- The frame bar is translator DESIGN §2.7 as it stands, with its four owner allowances. A new
  allowance is the owner's to grant, because an allowance is a place where "faithful" stops meaning
  equal.

---

## 2. Behaviour specification

### 2.1 What the read settles first

`title_pass` (`h0+$6e6e`), the menu and digit routine `h0+$71e6`, `title_fire`, `title_code_world`
(`h0+$777e`), the Esc route (last key byte `$75`), KONIEC's return to AmigaDOS (`quit_to_dos`) and the
game over path (`game_over_screen`, the high scores at `h0+$89d0`, `session_loop` again) are read from
the listing and measured on the oracle (T01) before any case is written. The cases, the app tests and
the docs all name the codes and keys the read finds, so guessing them would put a wrong value in every
later task. The read's result goes in `games/forest-dumb/docs/16-title.md` (tier 3, root CLAUDE.md
rule 1). If the read finds a screen, a mode or a disk write nobody planned for (a hidden menu, the high
scores saved to disk), the worker brings it to the owner before it is planned around, because it is a
change to what this plan checks.

### 2.2 The cases

One case each, on the oracle and on `--engine translated`, in `engine/conform/cases/`:

| Case | Path | Judged to |
|---|---|---|
| `title-idle-loop` | power-on, the idle title past the song's first loop back to position 0 | the loop measured by T01, `mt_songpos` back at 0 |
| `title-rows` | Down and Up through START, KODY, KONIEC and back, from the 4990 snapshot | `title_cursor` at each row |
| `title-code-wN` | KODY with world N's code, then START | world 1: play's first frame; worlds 2 to 6: 300 passes, as `world-N-start` |
| `title-code-cheat` | KODY with the cheat code, then START | the world's start with `cheat_flag` set |
| `title-code-wrong` | KODY with a code that is no code | what T01 measures (the title unchanged, or a world 1 start) |
| `title-koniec` | KONIEC, fire | both sides quit (§2.3) |
| `title-esc` | Esc on the title (the Amiga key) | what T01 measures: quit, or nothing over N frames |
| `game-over-title` | world 2, lives set low at `load_world`, deaths to game over, the high scores, the title | both sides back in `title_screen` |
| `game-over-name` | as above with a name typed, only if T01 finds name entry | the name in the table, the title |

World 1 cases stop at play's first frame, because `power-on-level-1` fails from frame 6301 on the
translated program and its fix belongs to the level 2 plan (owner, 2026-11-20; level 2 brief,
"Carry-over"). The game over cases die in world 2, whose start passes, for the same reason. A later
world reached by a code is judged no further than its start (300 passes), because its play is its
level plan's.

`lives` set low at `load_world` is a case's way of reaching game over in a few deaths rather than a
full count; the path from the last death on is the original's own and is what the case judges. Setting
it is the existing `set … when` row (translator T33), not a new mechanism.

HELP ending a level under the cheat is already held by `level-end.toml` (translator T26, a level case,
passing translated), so it gets no new case. `title-code-cheat` proves only that the code sets
`cheat_flag`.

Long cases (the idle loop, the world starts, game over) go in the full set; a case is run in smoke only
through a short test, because smoke is at 842 s of its 900 (`~/.cache/superfrog-re/smoke-times.tsv`,
2026-11-20).

### 2.3 Two new goals

A case can end two ways no goal names today (T02):

- `goal = { enters = "ROUTINE", by_frame = F }`: both sides enter `ROUTINE` (a `func` of
  `re/symbols.toml`) in the same aligned frame, by oracle frame `F`. Used with `title_screen` for "back
  at the title". General, because any later plan's "it got to screen X" is the same check.
- `goal = { exited = true, by_frame = F }`: the original returns from the program's entry to its caller
  with the stack as the entry found it, and the translated program reports `TranslatedGame::exited`,
  in the same aligned frame. The same narrow rule as fd-app's KONIEC (translator T41): any other jump to
  0 is a stop, not an exit.

### 2.4 Fixing

Every title case passes on the translated program (T07). The fix for a failure lands where the defect
is (`tools/pcode_*.py`, `pcode-machine`, `fd-translated`'s host), with a test on a made-up program or a
machine unit test that fails without it, as the translator plan did. A fix may not change the reading
of the 50 existing cases; `run-all --engine translated` is run before and after. A known suspect: the
OS's interrupts run through the title on the original and never on the machine (translator DESIGN
§2.8), so a long title run may drift. If closing such a gap needs an OS model the translator plan
chose not to build, that is the owner's choice, not a fix.

### 2.5 Start Level… and `--level N`

Owner, 2026-11-20. Game ▸ Start Level… offers worlds 1 to 6 (`W1-1` to `W6-1`), a "Cheat on" checkbox
(off when the window opens) and "Enable recorder?" as today. It starts the chosen world by running the
translated program headless from power-on through the title with fire (`to_level_1_input`'s way) and
writing `world_index` and, with the box ticked, `cheat_flag` as `load_world` is entered
(`TranslatedGame::write_at`), the way the world-start cases do. The owner chose this over typing the
code on KODY because it is simpler and does not depend on the code table. World 1 runs on level 1's
hand engine through the seam as today; worlds 2 to 6 run translated. The title's codes stay on the
title as the original has them.

`--level N` takes 1 to 6 and starts the same way with the cheat off, so there is one start path and a
test can reach any world from the command line. `--engine hand` keeps today's window, level 1 only and
no cheat box, because the hand engine has only level 1.

A bug recording names where play started. Version 3 of the format adds `world-N` starts (N 1 to 6) and
a `cheat` field; `level-1` and `power-on` keep reading as before, and version 1 and 2 zips still
replay. `fd-conform bug replay` starts a `world-N` recording the same way (`write_at` as `load_world`
is entered), so the replay reaches the state the app recorded. Comparing a translated recording with
the original (`bug compare`) is not built today for any start, since `compare` retimes a level 1 hand
engine recording to the level sync point; it stays out of scope here (FINDINGS), because the owner
asked for the replay and a translated compare needs its inputs lined up by loads, a design of its own.

### 2.6 The unhappy paths

- A code the original rejects: the case records what the original does, and the app does the same,
  since it runs the same program.
- A stop of the translated program on the title or in a Start Level… start: today's stop band and zip
  (translator T39). `--level N`'s headless run exits 1 with no zip on a stop (T39's open gap); this
  plan does not change it, and the finding stays open.
- Start Level… chosen mid-play or during a recording: as today (the recorder asks to save first).
- The Amiga Esc: fd-app's Esc always quits the app and never reaches the game, so the app test reaches
  the Amiga Esc through a key binding in settings, as `game_over.rs` does.

---

## 3. Architecture

### 3.1 The boundary

Unchanged from the translator plan: `fd-translated`'s generated tree and `pcode-machine` are pure and
held by `fd-conform/tests/purity.rs`; `fd-app`, the oracle and the runner are the shell. New pure code
here: the recording header (`fd-core/src/recording.rs`), the Start Level spec and its values
(`fd-app/src/recorder.rs`), `--level`'s parsing (`fd-app/src/args.rs`) and the case parser
(`fd-conform/src/case.rs`), each unit-tested. If `purity.rs` fails, the fix is to move the code, never
to relax the test.

### 3.2 Modules touched

- `fd-conform`: case goals (`case.rs`, `frames.rs`), new cases, the read's oracle test,
  `bug.rs`'s `replay_translated` for world starts.
- `fd-core/src/recording.rs`: `LevelId` W1-1 to W6-1, `StartPoint::World`, version 3.
- `fd-app`: `recorder.rs` (spec, `chosen`), `args.rs` (`--level N`), `translated.rs` (`to_world`),
  `main.rs`/`menu.rs` where the start is wired.
- `fd-e2e`: new tests; `smoke.toml` labels.
- The translator and machine, wherever T07's fixes land.

---

## 4. Testing

Cases on the oracle prove the translated program against the original. `fd-e2e` proves the app equal
to a headless translated run, by key presses, and a world-start recording replaying to the app's state. Unit tests hold the parsers and the recording format. Nothing here needs a person.

---

## 5. Environment — read this before running anything

| | |
|---|---|
| OS | macOS (Darwin 25.5, arm64) |
| Language / runtime | Rust 1.99.0, cargo 1.99.0; Python 3.15.1 |
| Toolchain | the repo's venv for Python tools; Ghidra for `translate.py`'s export |
| Deliberately absent | no Amiga hardware; no uncracked disk set |

**The test command.** `tools/smoke.sh`, the stored line pir already runs, unchanged by this plan. A
session runs neither smoke nor full unless its task says so (root CLAUDE.md). The cases run with
`cargo run --release -p fd-conform -- run cases/NAME.toml --engine translated` and
`run-all --engine translated --cases 'title-*'` in `games/forest-dumb/engine/`.

**A fresh worktree** needs, once: `cargo run -p fd-tools -- extract` in `engine/`, then
`python3 -I games/forest-dumb/tools/translate.py --build`, then `touch
games/forest-dumb/engine/buildscripts/fd_translated.rs` (translator T38). The disks and the Kickstart
are found as `docs/01-disk-location.md` says.

**Smoke budget.** 842 s of 900 at the last pass (2026-11-20). Every new test file gets a label in
`games/forest-dumb/smoke.toml`; only tests of a few seconds go in smoke, and the drill (T11) checks the
total.

**Dependencies.** No new crate or Python package. The Start Level window is the toolkit's
`amiga_app::start_level_window`, which already takes dropdowns and checkboxes.

**End to end.** `fd-e2e`, reused (README): the script layer through `--script` for the title paths and
Start Level… (`N: start …` lines, `fd-app/README.md`), the OS layer (`FD_E2E_OS=1`) for the window's
real controls, at the app's default scale and 400×300 to 1200×900 as T41 drove. The free backend is the
translated program on the owner's disks, run locally.

### 5.1 Seatbelts

| Flag / mechanism | Default | Effect |
|---|---|---|
| fd-app `--max-frames`, `--timeout` | set by every fd-e2e launch | a run cannot outlive its test |
| a case's `frames` | per case | the oracle's frame cap |

---

## 7. Decisions and rationale

- 2026-11-20, owner: translated only, no hand-written title (brief, Open 1).
- 2026-11-20, owner: Start Level… offers worlds 1 to 6 and a cheat box, against the brief's proposal of
  leaving it unchanged; it sets the world and cheat directly at `load_world` rather than typing the
  code on KODY, which the owner judged simpler.
- 2026-11-20, owner: the cheat code is in scope, case and app test (brief, Open 3).
- 2026-11-20, owner: the high-score table is covered, with a name entry case if it has one.
- 2026-11-20, owner: `power-on-level-1`'s 6301 difference stays with the level 2 plan; world 1 cases
  stop at play's start and game over is checked in world 2.
- Planner: a plan, not single runs (brief, Open 4), since cases, fixes, a recording format change and
  app tests span several sessions.
- Planner: `--level N` takes 1 to 6 with no cheat, so the app has one start path for Start Level… and
  the command line.
- Planner: two general goals (`enters`, `exited`) rather than per-case code, so later plans can reuse
  them.
- Planner: no mock, since Start Level… reuses its window and the title already exists.
- Planner: the two power-on cases' stale `Fails today` notes are left to the level 2 plan, whose brief
  carries them, so two plans do not edit the same lines.

---

## 8. Explicitly out of scope

- The levels' own play, each level's plan; a code's world is judged to its start only.
- `power-on-level-1`'s 6301 difference: the level 2 plan's.
- The ending after world 6, the crack's intro and picture, sound beyond the title's music, widescreen:
  the brief's out of scope.
- Controls… and the rest of the app's menus, unchanged (owner).
- `bug compare` of a translated recording against the original, for any start: not built today and not
  asked for; a recording from a world start replays only.
- A hand-written title (owner).
