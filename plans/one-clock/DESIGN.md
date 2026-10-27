---
setup:
  - python3 -m venv venv
  - venv/bin/pip install -q -r requirements.txt
test:
  - NO_COLOR=1 venv/bin/python -m unittest discover -q -s . -p 'test_*.py' || [ $? -eq 5 ]
  - cd engine && cargo test -q --workspace
---

# one-clock — Design

The scope is `plans/amiga-toolkit-prep/DRAFT-2-one-clock.md`, settled by the owner on 2026-10-24
and locked for this plan on 2026-10-27 ("plan exactly what it says, three tasks as it splits them,
do not reopen its decisions"). This file does not reopen it. It records how the plan carries it out
and the two places where the code had moved on from the draft, which the owner settled at planning
(§7). Where this file and the draft disagree, the draft wins except for §7's decisions, and the
disagreement is a finding.

## 1. Purpose

The engine keeps time in five beam models in three units (draft "The change"). This plan gives it
one beam-position type, one set of frame and line lengths, and one generic `Clock`, in a new
toolkit crate `engine/crates/amiga-clock` that names nothing of Superfrog. The level runs on that
clock with today's parameters; the front end and the slot draw keep their own rounding as named
policies on it; the story simulator stays its own cycle-exact engine and adopts only the type and
the lengths. It is stage 2 of `plans/amiga-toolkit-prep/README.md` and moves north-star rows M2
(frame/line constants, beam position), M4 and M5.

### Success criteria

- `sh plans/amiga-toolkit-prep/measure.sh` prints `frame/line length constants` ≤ 3, all of them in
  `amiga-clock`; the beam-position row, now scanning `engine/crates`, lists only `BeamPos` and its
  clients; `PASS amiga-clock` beside `PASS sf-paula` and `PASS amiga-chips`.
- The test block above and every run in §4 are green, with no tolerance touched.
- `python3 tools/test_architecture_docs.py` passes with the new crate owned.

### Stance

A pure refactor under the faithful bar (rust-engine DESIGN §2.1): zero behaviour change, every
check green, `Tolerance::OWNER` fixed. A red check is a bug in the refactor, never a reason to
loosen anything, because the step's value is that the merged clocks are proven to be the same
clocks. Minimal scope (draft, 2026-10-24): the shared clock only; costs stay inline.

## 2. Behaviour specification

### 2.1 The beam-position type (T01)

One `amiga_clock::BeamPos`: a line and a position in it, held in CPU cycles (`LINE_CYCLES`, 454 a
line, two per DMA slot), because the level clock's position is not always on a DMA-slot boundary
(`Clock::cpu` adds `cycles * 454 / per`) and a coarser type would round it. Its conversions are
the three units the engine uses: CPU cycles from the frame's start (`nominal`, today's
`Clock::nominal`), DMA slots (227 a line: the copper, the story simulator, `word_times`), and the
Paula write's `hpos` (a DMA slot, clamped to the line's last as `Clock::hpos` does). Every
conversion is exact where today's code is exact and rounds where today's code rounds, at the same
place; a conversion that would round differently from the code it replaces is a bug.

The copper's `Beam` (`amiga-chips/src/copper.rs`) is replaced by `BeamPos`; `copper::Beam` stays as
a re-export alias so `sf-conform`'s tests and the frozen chapters keep resolving. `story/sim.rs`'s
`beam(t)` becomes a client (`BeamPos::from_dma` plus the frame count).

### 2.2 One definition per length (T01)

Three meanings, each defined once, in `amiga-clock`: `FRAME_LINES` (313, a long PAL frame),
`LINE_CYCLES` (454 CPU cycles a line) and `LINE_SLOTS` (227 DMA slots a line), plus
`SHORT_FRAME_LINES = FRAME_LINES - 1` derived from it. Every other constant or bare literal that
means one of these lengths in non-test code becomes a use of them or a typed constant derived from
them: `level/beam.rs`, `front/mod.rs` (`FRAME_LINES`, `BEAM_CYCLES_PER_LINE`), `slot/mod.rs`
(its 312), `front/story/cpu.rs` (`LINE`), `story/mod.rs` (`LONG_FRAME`), `story/intro.rs`
(`LINES`), `copper.rs` (its `LINE_CYCLES` is 227 DMA slots: it maps to `LINE_SLOTS`, not to
`LINE_CYCLES`), `blit.rs` (`l % 313`), `sf-paula/src/channel.rs` (`CLOCKS_PER_LINE`,
`LINES_PER_FRAME`), and the probes and tests in `sf-conform` (`examples/*.rs` `LINE`,
`tests/beam.rs`'s `LINE: f64`). A derived constant outside `amiga-clock` does not reuse the names
`measure.sh` counts (`FRAME_LINES`, `LINE_CYCLES`, `BEAM_CYCLES_PER_LINE`, `LINE`), so the count
stays honest; a `pub use` re-export keeps an old path alive without counting. Reason for the
derived form: the value is written once, so a second game with another frame length changes one
line. A number that only happens to equal 227, 313 or 454 but means a cost (`vbl.rs` `FETCH_READ`,
`KEYBOARD`) is not a length and is left alone; test expectations may keep their literals.

The slot machine's 312-line frame stays a setting of the slot (`SHORT_FRAME_LINES`), never "fixed"
to 313 (draft "Not changing"). `slot/draw.rs`'s `spend` and its line sum wrap at the front end's
313 today while the slot's `CUT_LINE` uses 312; each keeps the length it uses today.

### 2.3 The generic clock (T02)

`amiga_clock::Clock<T: Timing>` is today's `level::beam::Clock` with its Superfrog parameters
lifted into a `Timing` (frame length, the fetch window's lines and slots, the blitter's free slots
inside the window, the CPU cycles a fetching line steals). The draft says "per-word steal"; the
level's steal is per fetching line (`FETCH_STOLEN_CYCLES` 40), so the parameter is the level's own
unit and the slot's per-word steal is the slot's policy (§2.5). The level supplies its timing as a
type (`level::beam::Level`) and keeps `pub type Clock = amiga_clock::Clock<Level>`, so every call
site and the `level::beam::Clock` path stay.

The clock keeps its fields (`line`, `into`) and its serde form: `Clock` is inside `Pending`, which
is part of the save state, and a save state written before the change must load after it. The
timing marker is `#[serde(skip)]` and zero-sized.

### 2.4 Blitter timing in one home (T02; owner, 2026-10-27)

All blit timing moves into `amiga-clock`, each calculation unchanged: `blitter_slot`,
`blit_cycles` (vAmiga SlowBlitter), `slots_per_word`, `word_times` and `FIXED_TO` (from
`amiga-chips`), and `Blit::cycles`'s formula (as a function of BLTCON0 and the word count;
`Blit::cycles` stays a method that calls it). `word_times` takes the fetch test as a function of
(line, slot) instead of `&copper::Fetch`, because `amiga-chips` depends on `amiga-clock` and not
the reverse; its callers pass `|l, h| fetch.takes(l, h)`. The four memory-refresh slots (`$01`,
`$03`, `$05`, `$e2`), written out in `blitter_slot` and `story/cpu.rs` `refresh`, become one
`is_refresh(slot)`. `story/sim.rs`'s HRM word table is not `slots_per_word`: they differ for B and
C without D (0x6, 0xE), so both stay. Fitted constants stay where they are (`BLITTER_LOST`, `Cx`'s
1.23 a slot, the slot's `cycles()*2/3`).

### 2.5 Named policies (T03)

`Cx` (`front/mod.rs`) and the slot draw (`slot/draw.rs`) say their positions with `BeamPos` and
spend through functions in `amiga_clock::policy`, each named for its rounding, each reproducing
today's arithmetic exactly:

- Whole lines: `Cx::busy` with `into: None` rounds up to whole lines (`div_ceil`). Kept because
  `Cx`'s `AudioDma` positions are the coarse ones its constants were fitted on, and a finer clock
  could move note restarts (draft decision 2026-10-24; rust-engine FINDINGS T213 2026-10-19).
- Cycle-exact on a line: `Cx::busy` with `into: Some` (T203).
- Per-slot steal: `Cx::blit_wait` (1.23 CPU cycles per fetched slot, whole lines).
- Per-word steal: `slot/draw.rs` `spend` and its line sum (tenths of a cycle per fetched word,
  rounded up at the end).

Any refit is a separate decision (draft). The story's `Intro::t` and story `Cx` adopt `BeamPos`
and the constants only (T01); their arbitration is not touched.

### 2.6 The timeline (T02)

`Timeline`'s stepping (cycles spent on whole lines, each Paula write made where it falls, a yield
at the frame's last line) moves to `amiga-clock` as `Timeline<J>`, generic over the job it carries
and run against a small trait for the beam it moves (its line, a write, a job handed over). What
binds it to Superfrog stays in `level/timeline.rs`: building it from a trace of `Tick`s with
`Cost`s (today's `Timeline::of`) and the trait's impl for the front end's `Cx`, whose `Yield` it
maps to. Reason: `Timeline` reads `front::Cx`, `Yield` and `game::Job` today, so it cannot move as
it is. Its serde form stays (field order unchanged) for the same save-state reason as §2.3.

### 2.7 What does not change

Word for word from the draft: every `Cost` kind, its tables (`COSTS`, `BLANK_COSTS`,
`RESTART_COSTS`), every `spend` call site, every fitted constant (`VBL_END_LINE`, `FADE_*`,
`BLITTER_LOST`, the 1.23/slot), the slot's 312-line frame, the story's cycle-exact arbitration, the
cut-point types (`Cut`, `Pending`, `Frames`: draft 3 part C). Paula writes keep their beam positions
exactly. Also `routine_cost` and `Tick` stay in sf-core, being Superfrog's.

### 2.8 Unhappy paths

- A red check before any change: amiga-chips merged with pir's final run red (app tests timing out
  in the full suite, `plans/amiga-chips/REPORT.md`). T01 runs the §4 checks on its base before its
  first change and records the result, so a red that was already there is not taken for this
  plan's.
- A red check after a change is a refactor bug: find it, never touch a tolerance or a fitted
  constant.
- An old save state that no longer loads is a bug in §2.3/§2.6.

## 3. Architecture

### 3.1 Crate rules (following amiga-chips DESIGN §3.1)

- `amiga-clock` depends on nothing in the workspace, plus `serde` (save states). No Superfrog
  name in its code.
- `amiga-chips` and `sf-paula` depend on it (toolkit crates may depend on each other), as do
  `sf-core` and whichever shell crates import it directly.
- It joins `PURE` and `ALLOWED` in `engine/crates/sf-conform/tests/purity.rs`, and gets
  `[profile.dev.package.amiga-clock] opt-level = 2` in `engine/Cargo.toml`, since the copper run
  and the clock's loops run in every render and level test.
- Each moved item's old path stays as a `pub use amiga_clock::…` re-export (`level/beam.rs`,
  `level/timeline.rs`, `copper.rs`, `blit.rs`) so callers outside the crate and the frozen
  chapters' cited paths keep resolving (`tools/test_architecture_docs.py`). A file left with real
  code keeps it.
- `docs/architecture/cpu-time-and-beam.md`'s front matter `owns` gains `"engine/crates/amiga-clock/"`
  (T01). Chapter bodies are frozen; only front matter changes.

### 3.2 The boundary

The pure/shell boundary is rust-engine DESIGN §3.1, enforced by `purity.rs`. The new crate is pure.
If purity fails, the fix is to move the code, never to relax the test. `Cx::blit_wait`'s 1.23 is
integer arithmetic (`* 123 / 100`) and stays so.

### 3.3 Where things live after each task

| After | `amiga-clock` holds |
|---|---|
| T01 | `BeamPos`, `FRAME_LINES`, `SHORT_FRAME_LINES`, `LINE_CYCLES`, `LINE_SLOTS` |
| T02 | + `Timing`, `Clock<T>`, `is_refresh`, `blitter_slot`, `blit_cycles`, `slots_per_word`, `word_times`, `FIXED_TO`, the rough blit-cycle formula, `Timeline<J>` |
| T03 | + `policy` (§2.5) |

## 4. Testing

From the draft ("Tests"), for every task:

- The test block: the Python suite and `cd engine && cargo test -q --workspace` (owner 2026-10-24).
- The builder also runs, from `engine/`: `cargo test -p sf-conform --test beam`
  (`the_cost_model_alone_puts_paula_writes_on_the_originals_lines`, 70 %/98 % bands),
  `cargo test -p sf-conform --test frog_cases`, `cargo test -p sf-conform --test alone`,
  `cargo test -p sf-conform --test story -- --ignored` (slot and story cases, 80 s–5 min each),
  `cargo test -p sf-conform --test channels -- --ignored`
  (`the_whole_games_channels_sound_as_the_original_does`, ~7 min cached).
- T03 ends with `cargo run -q --release -p sf-conform -- all` (~2 h, T217's figure) and
  `cargo test -p sf-e2e --test whole_game -- --include-ignored` (the file whole, its two ignored app
  runs included, about 90 min more).
- New: `BeamPos` conversion round-trip tests; `Clock` with Superfrog's parameters reproduces
  today's `level::beam` unit tests unchanged.

Beyond the draft, how the zero-change claim is shown for each moved calculation: the task keeps
the old function in its test module as the reference and compares it with the new one over a
sweep of inputs (every line and slot of a frame, a spread of cycle counts and BLTCON0 values), and
a save-state test proves `Clock` and `Timeline` serialise to the bytes they did before.

The conformance runs need the owner's disk images and the oracle build (`tools/setup/oracle.sh`),
which are on this machine; they are a worker's to run and record, not the owner's.

## 5. Environment

| | |
|---|---|
| OS | macOS (Darwin 25.5) |
| Language / runtime | Rust 1.99.0 (cargo 1.99.0), edition 2024, workspace in `engine/`; Python 3 for `tools/` |
| **Deliberately absent** | nothing this plan needs |

**The test command.** The `test` lines of the block at the top, the repo's stored lines from
`.pir/settings.json`. The person's `~/.pir/superfrog-re/settings.json` sets
`testTimeoutMinutes: 120`. Each line runs in its own shell from the repo root. The Python line is
quiet (`-q`, `NO_COLOR=1`); `cargo test -q` prints one dot line per suite. Re-measured at planning
in a fresh worktree because amiga-chips' final run was red (§2.8): green on `874c78f`, the cargo line
taking 1 h 45 min of the 120-minute limit (`FINDINGS.md`). A task that adds slow tests to the block
may push it over; the slow runs in §4 stay outside it for that reason.

**Setup.** The stored `setup` lines (a venv and `pip install -q`); they leave the copy clean.

**Dependencies.** None added. `amiga-clock` uses `serde` only (already on `ALLOWED`).

### 5.1 What the test command cannot reach

The slower sf-conform and sf-e2e runs in §4 are outside the test block for their length, not
because they need a person. Nothing in this plan needs a person.

### 5.3 Outside actions

None. Every step is local: edits on the task branch, builds, the test block, and the runs in §4
against the local oracle and the owner's disks. An action that turns up anyway is `ask`
(CLAUDE.md).

## 6. Recovery

A refactor on a feature branch; the way back is not merging it.

## 7. Decisions and rationale

- 2026-10-27, owner: the draft's "this draft owns the blitter timing merge" is read as one home,
  same maths (§2.4): every blit timing calculation moves to `amiga-clock` unchanged, identical
  pieces shared. The alternatives were moving only the level's three (leaving blit timing split
  across two crates) and one real model (a refit, against the draft's no-change rule). At planning
  the story's HRM word table was first described to the owner as identical to `slots_per_word`; it
  differs for B and C without D, so it stays separate and the owner was told.
- 2026-10-27, owner: the draft puts the crate in T2, but the copper's `Beam` and its lengths moved
  to `amiga-chips` after the draft was written, and sf-core cannot give `amiga-chips` a type. T01
  creates `amiga-clock` with `BeamPos` and the lengths; T02 adds the clock. The three tasks and
  their order are the draft's. The alternatives were parking them in `amiga-chips` (the same code
  moved twice) and leaving the copper for T02 (T01 misses its count).
- 2026-10-27, brief: `measure.sh`'s beam-position row scans `engine/crates`, not sf-core, and also
  counts `pub struct BeamPos`, so a beam type in a toolkit crate is not hidden from it (T01).
- The draft's "per-word steal" Clock parameter is the level's per-line steal (§2.3); the slot's
  per-word steal is its policy (§2.5). Both keep today's numbers, so there is nothing to choose.
- `Clock` stays generic over a timing type rather than carrying its parameters as a field, so its
  save-state bytes and every call site stay as they are (§2.3).
- The survey (planning Stage 4) is the draft and `docs/architecture/cpu-time-and-beam.md` §3,
  re-checked against the code on 2026-10-27. Found beyond the draft: `Timeline`'s ties to `Cx`
  (§2.6), `word_times`'s tie to `Fetch` (§2.4), sf-paula's and the story's own length constants
  (§2.2), the slot's 313 in `spend` (§2.2).
- Plan numbering: the draft's T1–T3 are T01–T03. No spike: nothing here is unknown enough to
  change the architecture, and each task proves its own moves by comparing old and new code.

## 8. Explicitly out of scope

§2.7. Also from the draft: turning the 89 level and 26 front-end charge points into timed events
with a refit (a native-rewrite step); measuring how many picture frames depend on display writes'
beam positions (cpu-time-and-beam §5, ~2 h with zeroed costs). From
`plans/amiga-toolkit-prep/README.md`: the address coupling, draft 3 and the extraction are other
plans. The project CLAUDE.md's stale "T213 open" line is not touched here.
