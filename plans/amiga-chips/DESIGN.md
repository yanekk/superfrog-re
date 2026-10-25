---
setup:
  - python3 -m venv venv
  - venv/bin/pip install -q -r requirements.txt
test:
  - NO_COLOR=1 venv/bin/python -m unittest discover -q -s . -p 'test_*.py' || [ $? -eq 5 ]
  - cd engine && cargo test -q --workspace
---

# amiga-chips — Design

The scope is `plans/amiga-toolkit-prep/DRAFT-4-one-chip-model.md`, locked by the owner on
2026-10-25. This file does not reopen it; it records how the plan carries it out and the one
contradiction the owner settled at planning (§7). Where this file and the draft disagree, the draft
wins except for §7's decision, and the disagreement is a finding.

## 1. Purpose

One implementation of each Amiga chip piece that exists more than once in the engine today, then
those pieces plus the copper in a crate `engine/crates/amiga-chips` that names nothing of Superfrog.
It is the first of the toolkit-prep steps (`plans/amiga-toolkit-prep/README.md`, stage 1) and moves
north-star rows M2 and M4 (`NORTH-STAR.md`).

### Success criteria

- `sh plans/amiga-toolkit-prep/measure.sh` prints: blitter memory-effect implementations 1, BLTSIZE
  decodes 3, is_paula definitions 1, chip-memory containers 2, and `PASS amiga-chips` beside
  `PASS sf-paula`.
- The test block above and the sf-conform runs in §4 are green, with no tolerance touched.
- `python3 tools/test_architecture_docs.py` passes with the new crate owned.

### Stance

It is a pure refactor under the faithful bar (rust-engine DESIGN §2.1): zero behaviour change, every
check green, `Tolerance::OWNER` fixed. A red check is a bug in the refactor, never a reason to
loosen anything, because the whole value of the step is that the merged pieces are proven to be the
same pieces.

## 2. Behaviour specification

### 2.1 What is merged (draft "In scope", rows 1–4, task T01)

| # | Today | After |
|---|---|---|
| 1 | Blitter memory effect three ways: `front/blit.rs` `Blit::run<M: BlitMem>` + `logic`; `level/objects.rs` `Blit`/`blit` (l.606, 629); hand-written A→D copies `level/scroll.rs:340` `blit` and `level/panel.rs:41` `blit_panel` | `Blit::run` only. `impl BlitMem for mem::Bus` writes through `Bus::set_w`, so `WriteLog` and `span` keep working. `objects::Blit` and the three functions are deleted; their callers build a `Blit` and call `run` |
| 2 | BLTSIZE decoded by hand in `scroll.rs` (2), `panel.rs`, `objects.rs` (2), `front/blit.rs` (2) | one `bltsize_words(size) -> (h, w)` with the size-0 rule (1024 rows, 64 words) |
| 3 | `is_paula` twice, identical: `front/mod.rs:1760`, `front/story/mod.rs:88` | one |
| 4 | Chip memory three times: `AmigaMemory` in `sf-core/src/layout.rs:754` (`Ram::byte` 0 off the map), `AmigaMemory` in `sf-oracle/src/amiga.rs:119` (`Option`, `impl sf_paula::ChipMemory`), `StoryMem` in `front/story/mem.rs:11` | one `AmigaMemory` with the `Ram` trait, offering both readers (0 off the map, and `Option`) and `impl sf_paula::ChipMemory`. sf-oracle uses it; `sf-conform/src/sync.rs:228` `core_memory`'s conversion goes. `StoryMem` stays a named newtype over it, keeping its serde form and 24-bit mask |

### 2.2 What moves (draft row 5, task T02)

| # | Today | After |
|---|---|---|
| 5 | (T2) the above live in sf-core | moved to `amiga-chips`: `Blit` (with `cycles` and `word_times` unchanged), `logic`, `BlitMem`, `bltsize_words`, `Ram`, `AmigaMemory`, `SLOW_BASE` and the chip/slow constants they need, `is_paula`, and all of `render/copper.rs` |

### 2.3 The pointer-wrap question

`objects::blit` masks each pointer with `$7fffe` after every modulo add; `Blit::run` masks once at
the start. They differ only when a pointer crosses `$7fffe` mid-blit. T01's first commit is a unit
test that runs such a blit through both and records which way it comes out. If they differ,
`Blit::run` keeps the level's per-modulo behaviour for `Bus` callers, with a comment saying why,
because the level blits are what the `render_*` and level routine cases compare. If they agree, the
merge needs no special case.

### 2.4 Order that must survive the merge

The cycle each blit word is written on, and the level copy blits' per-plane beam charge between the
two copies (`scroll.rs:363–366`), stay in the same order. `blit_panel`'s cost and custom-register
writes (`Cost::PanelBlit`, INTENA and DMACON around the five planes, `blit_words` before each plane)
stay with its callers in the same order. The reason: any change in the order of `Bus` writes changes
`WriteLog`, which `level_as_drawn` reads, and the `render_*` cases catch it.

The merge keeps behaviour only because the callers fit `Blit::run`'s general path. Checked at
planning: every level `Blit` is ascending (BLTCON1 bit 1 clear: `con1` is 0 or a shift in bits
12–15); `blit_panel`'s two callers pass BLTSIZE `$382`/`$385` (no zero height or width), positive
modulos (`$38/$2c`, `$32/$26`) and chip addresses below `$80000` (H2 `$400`, H3 `$77fc`), so its
missing mask, zero-extended modulo and missing size-0 rule never show.

### 2.5 Out of scope (do not touch)

Taken word for word from the draft:

- **Blit timing.** `Blit::cycles`, `word_times`, the slot's `cycles()*2/3` + `BLITTER_LOST`,
  `Clock::blit_cycles`, `blitter_slot`, `sim.rs`'s SlowBlitter: not merged, not changed. `cycles`
  and `word_times` move with `Blit` byte-for-byte because they are its methods and functions; the
  one-clock plan merges them.
- The cycle each blit word is written on, and the level copy blits' per-plane beam charge between
  the two copies (`scroll.rs:363–366`): same order after the merge.
- `story/bobs.rs:699, 721`: ported 68k arithmetic that composes a BLTSIZE value; not a decode.
- `State` and the level `Bus` as by-variable memory; `LAYOUT`; transplant and probe.
- `sf-paula` (only gains a dependent), its `PaulaWrite` emitters, `sf-disk`, the CIA keyboard
  (draft 3), sprites and bobs (`front/sprite.rs`, `level/bobs.rs`).
- Game-rule duplicates (darts and swings, the slot's second lamp drawer).
- Chapter bodies in `docs/architecture/` are frozen; only front matter `owns` changes (T2).

One exception to the `level/bobs.rs` line, decided by the owner 2026-10-25 (§7): its two calls to
`objects::blit` in `masks` are switched to `Blit::run`, and nothing else in that file changes.

## 3. Architecture

### 3.1 Crate rules (word for word from the draft)

- `amiga-chips` depends on nothing in the workspace except `sf-paula` (a toolkit crate; needed for
  `ChipMemory`), plus `serde` as sf-paula does. No Superfrog name in its code.
- Added to `PURE` in `engine/crates/sf-conform/tests/purity.rs`.
- `impl BlitMem for State`, `for StoryMem` and `for mem::Bus` stay in sf-core (the trait is
  foreign, the types are local).
- Each moved file's old path stays as a one-line `pub use amiga_chips::…` re-export
  (`front/blit.rs`, `render/copper.rs`) so the frozen chapters' cited paths still exist
  (`tools/test_architecture_docs.py` checks them). A file left with real code keeps it.

What follows from them mechanically. `amiga-chips` also goes on purity's `ALLOWED` list, since
sf-core then depends on it and `the_real_pure_crates_depend_only_on_the_allowlist` checks sf-core's
tree. It gets a `[profile.dev.package.amiga-chips] opt-level = 2` in `engine/Cargo.toml` like the
other pure crates, because the copper run is not generic and would otherwise be compiled
unoptimised in the test build, slowing the render tests. The workspace picks the crate up by itself
(`members = ["crates/*"]`). `front/blit.rs` keeps its `impl BlitMem for State` and `for StoryMem`
beside the re-export, so it is a "file left with real code".

### 3.2 The boundary

The pure/shell boundary is rust-engine DESIGN §3.1, enforced by `sf-conform/tests/purity.rs`. The
new crate is pure and joins `PURE`. If purity fails, the fix is to move the code, never to relax the
test.

### 3.3 Where things live after T01

T01 keeps everything inside existing files, because a new file under `engine/crates/` must be owned
by a chapter and the `owns` lists change only in T02 (draft, out of scope). So `bltsize_words` and
`is_paula` go in `front/blit.rs` or `render/copper.rs` (both owned by display-hardware), the merged
`AmigaMemory` stays in `layout.rs`, `StoryMem` stays in `front/story/mem.rs`.

### 3.4 Storage

`StoryMem` is part of the save state (serde + bincode, rust-engine T26). A newtype
`StoryMem(AmigaMemory)` with serde derived on both serialises the same bytes as today's
`{chip, slow}` struct under bincode, so old save states still load; T01 adds a test that proves it.
Its `Default` (512 KB of each, zeroed) and its own `Debug` stay.

## 4. Testing

Taken word for word from the draft ("Tests (both tasks)"):

- pir's test line: the Python suite and `cd engine && cargo test -q --workspace` (`minterms`,
  `a_blit_copies_a_shifted_word_run_with_its_masks`, `level_routines.rs`'s
  `bob_draw_matches_the_original`, `copper_irq_blits_match_the_original`,
  `tile_anim_draw_matches_the_original`, `buffer_flip_matches_the_original`, `story_blits.rs`,
  `story_draws.rs`, `front.rs` slot-attract, `render_vs_oracle.rs`, `beam.rs`, `purity.rs`).
- The builder also runs, from `engine/`: `cargo run -q --release -p sf-conform -- run
  conform/cases/render-*.toml conform/cases/slot-attract.toml conform/cases/front-*.toml` and
  `cargo test -p sf-conform -- --ignored story` (80 s–5 min each).

Added by the owner on 2026-10-25, beyond the draft: a level 1-1 smoke test in both tasks, the
case `conform/cases/complete-w1-1.toml` (W1-1 played from its start to its exit, every object live,
4658 frames, compared with the original) added to the first sf-conform run above. It is the one
whole-level play-through in this plan, so a defect that the targeted cases miss shows up in this
step rather than in the next plan's full `sf-conform all`. The full run and sf-e2e `whole_game.rs`
stay out of this plan, as the draft has them.

The conformance runs need the owner's disk images and the oracle build (`tools/setup/oracle.sh`),
which are on this machine; they are a worker's to run and record, not the owner's.

## 5. Environment

| | |
|---|---|
| OS | macOS (Darwin 25.5) |
| Language / runtime | Rust, edition 2024, workspace in `engine/`; Python 3 for `tools/` |
| **Deliberately absent** | nothing this plan needs |

**The test command.** The `test` lines of the block at the top, the repo's stored lines from
`.pir/settings.json` (owner 2026-10-24); this plan adds no tool or install, so they were not
re-measured. The person's `~/.pir/superfrog-re/settings.json` sets `testTimeoutMinutes: 120`. Each
line runs in its own shell from the repo root. The Python line is quiet (`-q`, `NO_COLOR=1`) and
`cargo test -q` prints one dot line per suite.

**Dependencies.** None added. `amiga-chips` uses `sf-paula` and `serde` (already on `ALLOWED`).

### 5.1 What the test command cannot reach

The slower sf-conform runs in §4 are outside the test block for their length, not because they need
a person. Nothing in this plan needs a person.

## 6. Recovery

A refactor on a feature branch; the way back is not merging it.

## 7. Decisions and rationale

- 2026-10-25, owner: the draft's "do not touch `level/bobs.rs`" and its deletion of `objects::blit`
  contradict, since `level/bobs.rs` `masks` calls `objects::blit` twice. Settled as: switch those two
  calls to `Blit::run` and touch nothing else in the file. The "do not touch" line is read as "the
  bob and sprite drawing are not merged", which still holds; keeping `objects::blit` alive instead
  would leave the blitter count at 2 and miss T01's target.
- Plan numbering: the draft's T1 and T2 are T01 and T02 here. There is no spike task; T01's first
  commit, the pointer-wrap test, is the probe the draft asks for, and only T01's own merge depends
  on its answer.
- The survey (planning Stage 4) is the draft itself: it names every existing copy by file and line,
  and the planner re-checked each against the code on 2026-10-25 (`measure.sh` baseline 4 / 9 / 3 /
  2 matches `NORTH-STAR.md`).

## 8. Explicitly out of scope

§2.5. Also, from `plans/amiga-toolkit-prep/README.md`: the address coupling (native order step 0),
the one-clock merge (draft 2) and the keyboard (draft 3) are other plans.
