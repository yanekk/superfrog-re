# Draft 4 — one copy of each chip model

**Status:** scope locked 2026-10-25 (rewritten after planner `plan-92b9` asked what the brief
meant; its brief arrived cut off at "Last task's Done when:"). Owner decisions 2026-10-24. Input for
`pir plan`, slug `amiga-chips`, exactly 2 tasks. This file supersedes the prompt text.
Source: `docs/architecture/display-hardware.md`, `memory-model.md`, `story-and-ending.md`,
`slot-and-bonus.md`; the native order's list of repeated ports.

## Goal

One implementation of each Amiga chip piece that exists more than once today, then those pieces,
plus the copper, in a crate `engine/crates/amiga-chips` that names nothing of Superfrog. Pure
refactor under the faithful bar (rust-engine DESIGN §2.1): no behaviour change, no tolerance
touched, every check green.

## In scope (the complete list)

| # | Today | After |
|---|---|---|
| 1 | Blitter memory effect three ways: `front/blit.rs` `Blit::run<M: BlitMem>` + `logic`; `level/objects.rs` `Blit`/`blit` (l.606, 629); hand-written A→D copies `level/scroll.rs:340` `blit` and `level/panel.rs:41` `blit_panel` | `Blit::run` only. `impl BlitMem for mem::Bus` writes through `Bus::set_w`, so `WriteLog` and `span` keep working. `objects::Blit` and the three functions are deleted; their callers build a `Blit` and call `run` |
| 2 | BLTSIZE decoded by hand in `scroll.rs` (2), `panel.rs`, `objects.rs` (2), `front/blit.rs` (2) | one `bltsize_words(size) -> (h, w)` with the size-0 rule (1024 rows, 64 words) |
| 3 | `is_paula` twice, identical: `front/mod.rs:1760`, `front/story/mod.rs:88` | one |
| 4 | Chip memory three times: `AmigaMemory` in `sf-core/src/layout.rs:754` (`Ram::byte` 0 off the map), `AmigaMemory` in `sf-oracle/src/amiga.rs:119` (`Option`, `impl sf_paula::ChipMemory`), `StoryMem` in `front/story/mem.rs:11` | one `AmigaMemory` with the `Ram` trait, offering both readers (0 off the map, and `Option`) and `impl sf_paula::ChipMemory`. sf-oracle uses it; `sf-conform/src/sync.rs:228` `core_memory`'s conversion goes. `StoryMem` stays a named newtype over it, keeping its serde form and 24-bit mask |
| 5 | (T2) the above live in sf-core | moved to `amiga-chips`: `Blit` (with `cycles` and `word_times` unchanged), `logic`, `BlitMem`, `bltsize_words`, `Ram`, `AmigaMemory`, `SLOW_BASE` and the chip/slow constants they need, `is_paula`, and all of `render/copper.rs` |

## Out of scope (do not touch)

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

## Crate rules

- `amiga-chips` depends on nothing in the workspace except `sf-paula` (a toolkit crate; needed for
  `ChipMemory`), plus `serde` as sf-paula does. No Superfrog name in its code.
- Added to `PURE` in `engine/crates/sf-conform/tests/purity.rs`.
- `impl BlitMem for State`, `for StoryMem` and `for mem::Bus` stay in sf-core (the trait is
  foreign, the types are local).
- Each moved file's old path stays as a one-line `pub use amiga_chips::…` re-export
  (`front/blit.rs`, `render/copper.rs`) so the frozen chapters' cited paths still exist
  (`tools/test_architecture_docs.py` checks them). A file left with real code keeps it.

## Tasks

**T1 — one copy of each, inside sf-core** (rows 1–4). First commit: a unit test that runs a level
blit whose pointer passes `$7fffe` through a modulo, through both the old `objects::blit` and
`Blit::run`, to learn whether masking per modulo (`objects.rs`) versus once at the start
(`Blit::run`) differs. If it differs, `Blit::run` keeps the level's per-modulo behaviour for `Bus`
callers, with a comment saying why. Then the merges.

Done when:
- `sh plans/amiga-toolkit-prep/measure.sh` prints: blitter memory-effect implementations 1, BLTSIZE
  decodes 3 (the helper and the two story ports), is_paula definitions 1, chip-memory containers 2
  (`AmigaMemory`, `StoryMem` newtype).
- The test line and the sf-conform runs below are green.

**T2 — the `amiga-chips` crate** (row 5), depends on T1.

Done when:
- The crate exists under the rules above; `measure.sh` lists `PASS amiga-chips` beside
  `PASS sf-paula`, and the T1 rows are unchanged.
- `docs/architecture/display-hardware.md`'s `owns` gains `"engine/crates/amiga-chips/"`;
  `memory-model.md`'s gains `"engine/crates/amiga-chips/src/<memory file>"` (longest prefix wins);
  `python3 tools/test_architecture_docs.py` passes.
- `docs/architecture/README.md` gets one line under "Recommended order", step 4's repeated ports:
  the blitter and chip memory are merged and live in `amiga-chips`.
- `plans/amiga-toolkit-prep/NORTH-STAR.md`'s log gets a column "amiga-chips" with this branch's
  `measure.sh` values and M1 as this task's runs; J1 and J2 left "not asked" for the owner.

## Tests (both tasks)

- pir's test line: the Python suite and `cd engine && cargo test -q --workspace` (`minterms`,
  `a_blit_copies_a_shifted_word_run_with_its_masks`, `level_routines.rs`'s
  `bob_draw_matches_the_original`, `copper_irq_blits_match_the_original`,
  `tile_anim_draw_matches_the_original`, `buffer_flip_matches_the_original`, `story_blits.rs`,
  `story_draws.rs`, `front.rs` slot-attract, `render_vs_oracle.rs`, `beam.rs`, `purity.rs`).
- The builder also runs, from `engine/`: `cargo run -q --release -p sf-conform -- run
  conform/cases/render-*.toml conform/cases/slot-attract.toml conform/cases/front-*.toml` and
  `cargo test -p sf-conform -- --ignored story` (80 s–5 min each).

## Risks

- The masking difference (T1's first test decides it).
- Anything that changes the order of `Bus` writes changes `WriteLog`, which `level_as_drawn` reads:
  `render_*` cases catch it.

## Toolkit payoff

After T2, `amiga-chips` holds the blitter, chip memory, the Paula register filter and the copper.
Superfrog-only, staying in sf-core: the callers, the `BlitMem` impls and the measured constants
(`BLITTER_LOST`, `LAMP_CYCLES`, 1.23/slot). Moves north-star M2 and M4.
