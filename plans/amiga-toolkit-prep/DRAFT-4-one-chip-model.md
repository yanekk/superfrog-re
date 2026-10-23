# Draft 4 — one copy of each chip model

**Status:** draft, 2026-10-23. Not reviewed. Input for `pir plan` (2 tasks).
Source: `docs/architecture/display-hardware.md`, `memory-model.md`, `story-and-ending.md`,
`slot-and-bonus.md`; the native order's list of repeated ports.

## The change

What a blit writes is implemented three times:

- `front/blit.rs` `Blit::run<M: BlitMem>` + `logic`: general (both directions, shifts, first/last
  masks, any minterm, returns pointers); serves `State` and `StoryMem`.
- `level/objects.rs` `Blit`/`blit` (l.606, 629): ascending only, over the level `Bus` (`WriteLog`,
  `span`), own minterm loop, masks pointers `& 0x7fffe` after every modulo where `Blit::run` masks
  once.
- Hand-written A→D copies: `level/scroll.rs:340` `blit`, `level/panel.rs:41` `blit_panel` (no
  size-0 = 1024/64 rule, no masking).

Also duplicated: `AmigaMemory` twice (`sf-core/src/layout.rs:754`, `sf-oracle/src/amiga.rs:119`,
bridged by `sf-conform/src/sync.rs:228` `core_memory`) plus `StoryMem` (`front/story/mem.rs:11`) as a
third chip+slow container; `is_paula` twice (`front/mod.rs:1760`, `front/story/mod.rs:88`,
identical); the BLTSIZE decode 9 times in 5 files.

Change: (a) `BlitMem` for `mem::Bus`, writes through `Bus::set_w` so `WriteLog` and `span` keep
working, and route `objects::blit`, `scroll::blit`, `blit_panel` through `Blit::run`; (b) one
`bltsize_words`; (c) one `is_paula`; (d) one `AmigaMemory` with the `Ram` trait in a low crate,
`StoryMem` a wrapper over it. T1 does (a)–(d) inside sf-core; T2 lifts `Blit`, `logic`, `BlitMem`,
`Ram`, `AmigaMemory`, `is_paula` and `render/copper.rs` (already self-contained) into an
`amiga-chips` crate on the `sf-paula` pattern, depending on nothing of Superfrog, and adds it to
`PURE`.

## Decisions (open, for the owner)

1. **Blit timing stays out.** Five timing models (`Blit::cycles`, the slot's `cycles()*2/3` +
   `BLITTER_LOST`, `word_times`, `Clock::blit_cycles`, `sim.rs`'s SlowBlitter) differ on purpose,
   each as exact as its checks need. Recommended: leave them to draft 2, which this draft precedes
   so draft 2 has one blit to time. If nothing is said: left out.
2. **Crate or not.** Recommended: T2 does the crate move; it is what makes the chips liftable and
   measurable (north-star M5). If nothing is said: crate.

## Not changing

The cycle each blit word is written on; the level copy blits' per-plane beam charge between the
two copies (`scroll.rs:363–366`), whose order must be kept; the level `Bus` and `State` as
by-variable memory; the copper (already one implementation); `sf-paula` and its `PaulaWrite`
emitters; `sf-disk`. Game-rule duplicates (darts and swings, the slot's second lamp drawer) are not
chip models and stay for a subsystem plan.

## Done when

- One blitter memory-effect implementation; one `bltsize_words`; one `is_paula`; one
  `AmigaMemory` (north-star M2 baselines: 4 blitter implementations, 9 BLTSIZE decodes, 3 chip-memory containers, 2 `is_paula`).
- `amiga-chips` builds with no `sf-*` dependency and no Superfrog name in its source; it is in
  `PURE`.
- Checks below green. `docs/architecture/README.md` gets a line under step 4's repeated ports.

## Tests

- pir's test line runs only Python; the builder runs `cd engine && cargo test -q --workspace`
  (`minterms`, `a_blit_copies_a_shifted_word_run_with_its_masks`, `level_routines.rs`'s
  `bob_draw_matches_the_original`, `copper_irq_blits_match_the_original`,
  `tile_anim_draw_matches_the_original`, `buffer_flip_matches_the_original`, `story_blits.rs`,
  `story_draws.rs`, `front.rs` slot-attract, `render_vs_oracle.rs`, `beam.rs`, purity), then
  `cargo run -q --release -p sf-conform -- run conform/cases/render-*.toml
  conform/cases/slot-attract.toml conform/cases/front-*.toml` and
  `cargo test -p sf-conform -- --ignored story` (80 s–5 min each).
- New first, before the merge: a unit test wrapping a pointer past `$7fffe` through a modulo, to
  learn whether the per-modulo masking difference matters.

## Risks

- The masking difference: if a level blit ever wraps, the merged blitter must keep the level's
  behaviour for it. The new test decides; a divergence there is a quirk to keep, with a comment.

## Toolkit payoff

Generic after the merge: `Blit`, `logic`, `BlitMem`, `Ram`, `AmigaMemory`, `is_paula`, the copper;
later `sim.rs`'s SlowBlitter and fixed-slot machinery (depends only on copper and
`savestate::static_str`). Superfrog-only: callers and measured constants (`BLITTER_LOST`,
`LAMP_CYCLES`, 1.23/slot). Moves north-star M2, M5.
