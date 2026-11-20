# Draft 1 — copied-in tables decoded at import

**Status:** draft, 2026-10-23; owner decisions recorded 2026-10-24. Not reviewed. Input for `pir plan`.
Source: `docs/architecture/copied-in-data.md` §4, §5, §8, §9; `memory-model.md` §8; native order step 1.

## The change

Ported routines read the original's tables by address out of a loaded copy of hunk 0: over the
level `Bus` falling back to `Rom` (`h0(0x…)` constants: `SHARED_FRAMES`, `ANIM_END`, `PATH` at
h0+$24454 shared by `w1/hover.rs`, `w2/flyer.rs`, `w4/swooper.rs`, `WITCH_BOB_PATH`, `PATH_113`,
`PICKUP_TABLES`, 39 frame-list literals in `frog/`), through `FrontData`'s readers (52
`.h0_b/w/l/bytes` calls: `front/chain.rs` 21, `password.rs`, `slot/*`, `level/tally.rs`, `audio/`)
and through `SoundData`'s readers (`audio/music.rs` replayer tables). `sf-assets::Tables` exists
with 13 fields, but sf-core reads none of them.

Hunk 0 is loaded four times: `Rom::h0` (`mem.rs`, `loaded_hunk0`), `FrontData::h0`
(`front/data.rs`, `relocate`, a second relocator), `FrontData::sound` and `DisplayData::sound`
(both `SoundData::from_game`, unrelocated).

Change: one relocated hunk 0, and an address-keyed typed-table layer in `sf-assets` that decodes
tables at import, turns pointers inside a table into indices, and keeps an original-address → index
map so compared pointer variables can still be resolved.

## Decisions

- 2026-10-24: minimal scope. T1 folds the four hunk-0 copies into one, merging `relocate` into
  `mem.rs::loaded`; T2 the address-keyed table framework and its `cache.rs` record; T3 the enemy
  paths (h0+$24454, boss and bonus paths) through it as proof. Reason: the full scope (frame lists,
  pickup tables, level codes, `start_resume`, slot and replayer tables) is mostly Superfrog content
  that does not carry to game two.
- 2026-10-24: the four tables that are `LAYOUT` variables (`obj_templates` h0+$1c6e2, `hiscore_table`
  h0+$2f90, `slot_lamps` h0+$6842, `demo_joy` h0+$11a2a) are excluded and keep reading through
  `Bus`. Reason: written by the engine and compared; moving them needs the address-coupling decision
  (native order step 0).
- 2026-10-24: `top_loop` is not written out as code. Reason: the toolkit does not need it, its reader is
  what draft 3 restructures, and the copyright question (copied-in-data §6) stays unopened.

## Not changing

`LAYOUT`, transplant, probe and every compared variable, including pointer variables
(`frog_anim`, object slot pointers, slot ladder rungs), which keep original addresses. Routine and
blit cost keys (`beam::routine_cost`, `Dispatch::routine`) stay keyed by routine address. Tiles at
h1, the bob sheet at h3 and hunk 2 stay `Rom` regions (draft 4 territory). Copyright boundary: every
table is still decoded from the disks at import; none becomes a `const` array.

## Done when

- One hunk-0 load in sf-core; one relocator.
- The chosen tables are read through typed fields; no `h0(0x…)`/`.h0_*` read remains for them.
- All checks below green; asset cache key bumped and re-import works from a clean cache.
- `docs/architecture/copied-in-data.md` is sealed with its plan; the change is noted in this
  plan's FINDINGS and `docs/architecture/README.md`'s step 1 gets a "done" line.

## Tests

- pir's test line runs `cd engine && cargo test -q --workspace` (owner, 2026-10-24)
  (includes `hunk0_is_relocated_to_each_target_s_base`, `variables_shadow_the_executable`,
  `the_boot_s_tables_are_the_original_s`, `spawners_match_the_original`, the
  `enemies_w*_routines.rs` and `bonus_enemy_routines.rs` suites, `purity.rs`
  `the_real_pure_crates_pass`); the builder also runs `python3 tools/check_no_disk_data.py`.
- Paths in scope: `cargo run -q --release -p sf-conform -- run` on the `enemies-w1-*`,
  `enemies-w2-*`, `enemies-w4-*`, witch and bonus cases.
- Full scope adds `cargo test -p sf-conform -- --ignored` (`attract-demo-3` ~4 min, `code-*` ~90 s
  each, `game-whole` ~25 min) and `audio_driver.rs`'s `every_tune_matches_the_original_replayer`.
- New: a decoder unit test per table, from the disks, comparing every entry with the bytes at its
  original address.

## Risks

- Table ends are not stored; animation lists end at runtime marks (`ANIM_END`). A decoder that
  guesses a length differently reads different data. Decode to the end mark, and test that.
- Whether any tune reads past a replayer table's end is unknown (copied-in-data §8): a cut-off
  decoder could change sound. Reason to leave replayer tables to the full scope.

## Toolkit payoff

Generic: the single hunk relocator (`loaded`/`parse_exe`), the image-plus-variable-shadow
`Bus`/`Rom` pattern, the address-keyed typed-table framework, the cache record macro, both
copyright checks. Possibly shared with other Team 17 / Player 4.1A games (unchecked): the replayer
table decoder. Moves north-star M3, M4.
