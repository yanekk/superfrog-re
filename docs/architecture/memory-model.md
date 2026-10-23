+++
id = "memory-model"
title = "The memory model"
tier = "mechanism"
depth = "deep"
kind = "amiga"
mix = { amiga = 80, game = 5, data = 15, shell = 0 }
reuse = "big-changes"
owns = [
  "engine/crates/sf-core/src/state.rs",
  "engine/crates/sf-core/src/layout.rs",
  "engine/crates/sf-core/src/mem.rs",
]
+++

# The memory model

## 1. What it is

The mechanism the whole engine stands on: every original variable the engine models is kept as
the original's bytes at the original's address, and the ported 68000 routines read and write it
by address, big-endian, as the 68000 does. Four pieces make it up.

- **`State`** (`engine/crates/sf-core/src/state.rs`): the bytes of every modelled variable, one
  `Vec<u8>` per layout entry, plus the few things the original keeps outside memory (`pause`, the
  program counter and `d0` while paused; `kbd_flag`, `kbd_acked`, CIA-A's serial flag) and two
  exceptions to the one-buffer-per-entry rule: the sound driver's variables, kept inside
  `AudioDriver` (`State::audio`), and `State::scratch`, bytes the front end wrote outside every
  variable, kept so a later read sees them and never compared.
- **The layout table** (`engine/crates/sf-core/src/layout.rs`): `LAYOUT`, 292 `VarDef` entries
  (`VarId`, the name from `re/symbols.toml`, address, `Width`) in address order, declared by the
  `layout!` macro so one line makes the id, the entry and the bytes `State` keeps. `index_of` finds
  an entry by address with a binary search. The hunk bases (`H0`, `H1`, `H2`, `H3`, and `h0()` …
  `h3()`) and the A500 memory picture (`Ram`, `AmigaMemory`, `SLOW_BASE`) live here too, and so
  does the `presentation` tag (`presentation`, `presentation_piece`) that says which variables the
  state check leaves to the picture and sound checks.
- **The loaded executable image** (`engine/crates/sf-core/src/mem.rs`): `Rom`, hunk 0 of `d1_c03`
  as the loader leaves it (`loaded_hunk0`, relocated to `HUNK_BASES`) with the tables the boot
  builds in it (`boot_tables`, `tile_heights`), hunk 2's contents and each world's tiles. What the
  game reads but never writes comes from here. Its contents are documented in
  [copied-in-data](copied-in-data.md); this chapter covers only how it sits under the state.
- **The bus** (`mem.rs`, `Bus`): the 68000's view for a ported routine. A read takes the state's
  variable at that address, else `Rom` (with the world loaded, the final level's while
  `final_level_flag` is set), else 0. A write goes to `State::write_byte`, optionally logging the
  beam position of writes to memory the display reads (`WriteLog`, used by
  [display-hardware](display-hardware.md)) and which entries were written (`Bus::written`).

The original's memory map these addresses come from is in
[docs/08 "Memory map"](../08-executable-map.md); the decision that the engine is one function
whose state is probed by address is [rust-engine DESIGN §3.3](../../plans/rust-engine/DESIGN.md).

## 2. Kind mix

`amiga` 80, `game` 5, `data` 15, `shell` 0.

- **`amiga`**: `State`'s byte store and its by-address readers and writers, the `LAYOUT` table
  (an address map of the original's memory, about 540 of `layout.rs`'s 916 lines), `Ram`,
  `AmigaMemory`, the transplant and probe, the `presentation` tags, and `Bus`. All of it exists
  to make the engine's memory look like the A500's.
- **`data`**: the first half of `mem.rs` (`loaded_hunk0`, `boot_tables`, `tile_heights`,
  `Rom::new`), which rebuilds the original's loaded image and boot-built tables.
- **`game`**: the named accessors in `state.rs` that give a game meaning to bytes (`State::camera`,
  `State::world`, `State::cell`, `State::rng`, `State::map_rows`).

Judged by reading each file and attributing its lines, tests with the code they test; the
`layout!` list counts as `amiga` because what it records is where the Amiga keeps each value, not
a game rule.

## 3. Where else the mechanism is used

Almost every ported routine goes through it. Counted by search on the task branch (2026-10-23),
per chapter area of `engine/crates/sf-core/src/`: calls of the by-address readers and writers
(`.b(`, `.w(`, `.l(`, `.set_b(`, `.set_w(`, `.set_l(`, `.byte(`, `.write_byte(`, on a `Bus`, a
`State` or the story's `StoryMem`) and literal original addresses (`h0(0x…)`). The counts are
rough: the pattern also matches a few unrelated calls.

| Area (chapter) | Address reads/writes | `h0(0x…)` addresses |
|---|---:|---:|
| `level/` ([level](level.md) and the mechanism chapters owning its files) | ~1260 | ~220 |
| `enemies/`, `bosses/` ([enemies-and-bosses](enemies-and-bosses.md)) | ~1150 | ~450 |
| `front/` without `story/` ([front-end](front-end.md)) | ~690 | ~30 |
| `frog/`, `items/` ([frog-and-items](frog-and-items.md)) | ~650 | ~150 |
| `slot/`, `bonus/` ([slot-and-bonus](slot-and-bonus.md)) | ~640 | ~120 |
| `audio/` ([sound-driver](sound-driver.md)) | ~430 | 0 (own offset table) |
| `front/story/` ([story-and-ending](story-and-ending.md)) | ~290 | ~230 |
| `game.rs`, `scene.rs` ([game-flow](game-flow.md)) | ~25 | — |

Named `VarId` access (`State::var`, `def(VarId::…)`) is rare inside `sf-core`: about ten uses outside `state.rs`, `layout.rs` and the story's
layout tables (`feed.rs`, `game.rs`, `mem.rs`, `level/load.rs`, `render/playfield.rs`,
`front/ending.rs`, `front/hero.rs`); the ported code names an address with a local constant
(`frog/anim.rs`'s `EXIT_SPRITES = h0(0x9f8)`) or as a base plus an offset (`FROG + f::SPUD_ON` in
`frog/mod.rs`). The renderer reads the display's memory as whole runs (`State::raw`, in
`render/mod.rs`).

Outside `sf-core` the mechanism is the interface of the verification tooling: `LAYOUT`,
`Game::probe`, `Start::Transplant`/`Start::Resume`, `VarId` or literal `h0` addresses appear in 9
files of `sf-conform/src`, 36 of its tests and 30 of its examples (about 660 references), 5 files
of `sf-e2e` and 2 of `sf-core/tests`.

The story intro has a second instance of the same mechanism. It is a program with its own memory
map, so the engine keeps the whole machine's RAM for it as the original's bytes, chip RAM at 0
and slow RAM at `$C00000` (`StoryMem` in `engine/crates/sf-core/src/front/story/mem.rs`), and its
routines read and write that by address. It has its own layout tables, `BOOT_LAYOUT` and
`INTRO_LAYOUT` in `engine/crates/sf-core/src/front/story/mod.rs`, whose ids are the `@outside`
`VarId`s of `layout.rs` (declared, not in `LAYOUT`); `Game::probe` reads the story's table out of
`StoryMem` while the story runs. Chapter: [story-and-ending](story-and-ending.md).

## 4. Why byte-exact

The 68000 code treats memory as memory, and the port keeps that rather than reinterpreting it
(the module headers of `state.rs`, `layout.rs` and `audio/mod.rs` say so). Concrete cases in the
code:

- **Overlaid widths.** The code reads a long where it wrote two words and a byte of a word it
  wrote whole. The sound driver's row's last byte is also its empty-row counter, and a volume
  slide works on a word's low byte (`engine/crates/sf-core/src/audio/mod.rs` header).
  `Bus::w(world + 1)` reading the middle of the long `world` is exercised by
  `variables_shadow_the_executable` in `mem.rs`.
- **Pointers into its own tables.** Variables hold absolute original addresses: `frog_anim`,
  `frog_anim_step` (a frame list in hunk 0), `scroll_column` (the address of the shown column in a
  display buffer), the object slots' template pointers (`obj_slots`, see the `h0(0x259ae)` put in
  `engine/crates/sf-conform/src/milestones.rs`'s tests), `text_ptr`, `module_addr`. The code
  follows them through the bus into `Rom` or the state, so those tables have to sit at their
  original addresses.
- **Null-pointer reads and writes.** A bob never drawn has no animation step yet, so
  `obj_despawn_if_offscreen` reads its frame record from address 0 and its size from address 4
  (`LowVectors` in `LAYOUT`, which is why `State::default` sets `EXEC_BASE` at 4); a swing's
  or a mace's spawner (`enemies::w3::swing`, `enemies::w2::mace`) with no slot for a link
  writes through a null slot pointer into the level 1 autovector at `$64` (`Level1Vector`, `KICKSTART_LEVEL1`).
- **Runs the code walks.** `level_map` is the original's rows of 304 cells (300 used, 4 of padding),
  and the boot-built row pointers, `$260` apart, point into it (`boot_tables`); `bob_records` is `$1730` bytes of
  records the drawer indexes by stride.

Writing all of that in native types would mean finding every such use first. Keeping the bytes
makes the port line-for-line and makes the transplant and probe a copy.

## 5. How ported code reads and writes state

- **By address, through `Bus`** in a level and the mechanics (`Bus::b`, `w`, `l`, `set_b`,
  `set_w`, `set_l`). A write goes to `State::write_byte`, which **panics** when no `LAYOUT` entry
  covers the address: a variable the port writes but the table does not map would be left out of
  every transplant and comparison.
- **By address on `State`** in the front end (`State::b`, `w`, `l`, `set_b`, `set_w`, `set_l`,
  `read`, `write`). These fall back to `State::scratch` instead of panicking, because the front end
  writes memory the engine does not model; `front/mod.rs` has a test asserting a display buffer's
  writes do not land there.
- **By whole variable**: `State::var`, `State::var_mut`, `State::bytes_of`, and runs inside one
  variable with `State::raw`, `State::raw_mut` (the renderer's bitplane fetch).
- **By named accessor** for the few values the engine's own code uses by meaning: `State::rng`,
  `camera`, `scroll_column`, `map_rows`, `world`, `map`, `cell`, `set_cell`.
- **The sound driver** keeps its own blocks (`audio::vars::ALL`, offsets in hunk 0); `State::byte`
  and `State::put` route an address inside one of them to `AudioDriver::var_block` and
  `set_var_block`, so a by-address access and the driver see the same bytes.

## 6. Transplant and probe

- **Transplant.** `State::transplant(&dyn Ram)` copies every `LAYOUT` entry's bytes out of the
  original's memory (an `AmigaMemory` of chip and slow RAM, taken from the oracle). `Game` starts
  from it with `Start::Transplant(AmigaMemory)`, or `Start::Resume { memory, cpu }`, which also
  takes the main program's registers so a front-end routine carries on from the same place
  (`engine/crates/sf-core/src/game.rs`, `Game::with_display`).
- **Probe.** `State::probe` gives every entry's value in table order, one `(VarId, u32)` per piece
  (`VarDef::pieces`: a `Bytes(n)` entry is cut into longs, then a word and a byte for a tail, so a
  divergence names the exact long). `Game::probe` returns that, or the story's table read from
  `StoryMem` while the story runs; `Game::layout` says which table the probe is in.
- **Poke.** `State::poke` writes bytes that variables cover and ignores the rest; conformance
  tests use it (through `Game::poke`) to plant a known divergence.

What `sf-conform` does with them (detail in [verification](verification.md)):

- `engine/crates/sf-conform/src/sync.rs` brings the original to a sync point on the oracle and
  starts the engine there with `Start::Transplant` (`engine/crates/sf-conform/src/lib.rs`).
- `engine/crates/sf-conform/src/compare.rs` reads each piece's value from the original's memory
  (`VarDef::read`) and zips it with the engine's probe (`vars_except`, `gameplay_vars`,
  `var_diffs`); it asserts both are in the same order and count. The faithful state check (rust-engine
  DESIGN §2.1 check 1) skips pieces `presentation_piece` tags.
- `engine/crates/sf-conform/src/routine.rs` transplants a sync point into `State`, runs one ported
  routine and the original's on Moira, and fails when the original writes a byte no `LAYOUT` or
  sound-driver variable covers ("add it to LAYOUT"), or writes one the port leaves alone.
- `engine/crates/sf-conform/src/milestones.rs` reads the plays-alone milestones (check 4) from
  the same `LAYOUT` variables on both sides: no transplant after the start, but the same
  addresses.
- `engine/crates/sf-conform/src/case.rs`: a case's `vars` list names `LAYOUT` entries.

## 7. What depends on it

- Every ported routine (the table in §3) and the renderer, which fetches bitplanes from the
  display buffers and copper list kept as state ([display-hardware](display-hardware.md)).
- [verification](verification.md): every state comparison, routine conformance, milestones, the
  `level_entry` and `recorder_setup_start` tests that compare a level entered by the engine with
  the original's memory, every `LAYOUT` variable, and the `sf-e2e` tests that read values through `Game::probe` (`persist.rs`, `view.rs`).
- **Save states** (`engine/crates/sf-core/src/savestate.rs`, chapter [game-flow](game-flow.md)):
  the body is the whole `Game` serialized with bincode field by field, and `State` serializes
  `vars` as one byte vector per `LAYOUT` entry in table order. Adding, removing or resizing an entry
  changes the encoding, which is why a state is tagged with the engine build and refused from any
  other (`StateError::OtherBuild`; rust-engine DESIGN §2.8). The same encoding is the state hash
  in `sf_core::feed::frame_hashes`, which pacing feeds and `sf-e2e`'s `whole_game.rs` compare.
- The story intro's own memory (§3) shares `Ram`, `VarDef` and the probe's form.

## 8. The coupling a native rewrite meets

DESIGN §2.5 of this plan. The comparison needs, at each compared moment:

1. the engine's value for every compared original variable, **in `LAYOUT` order, piece by
   piece**, as the original would hold it at its address (`Game::probe`, `Game::layout`);
2. a way to start the engine **from the original's memory** at a sync point (`Start::Transplant`,
   `Start::Resume`), which for check 4 happens once and for routine conformance once per generated
   state;
3. for routine conformance, the engine's state **before and after** one routine, by address, and
   proof that every byte the original wrote is one the engine keeps;
4. for milestones, a handful of variables by `VarId` on both sides.

A native rewrite that keeps game values in native types (an `i16` field for `frog_x`, a struct per
object, a `Vec` of enemies) breaks all four. Its routes:

| Route | What it is | Cost |
|---|---|---|
| Keep a mapping layer | Native code keeps the byte store as now, behind typed accessors per variable (`frog.x()` reading `h0+$7d6`) | Least change to `sf-conform` and the checks; the engine is still byte-shaped underneath, so overlays and pointers stay to be handled by the accessors; only the call sites get nicer. The 292-entry table stays |
| A probe adapter | Native state, plus a projection from it to original addresses for `probe`, and the inverse for transplant | The checks stay as they are and keep their power. Each native field needs a writer and a reader to the original's bytes, including pointers (a native enemy reference has to become the original's slot address) and overlaid fields; transplant needs the inverse, which for pointers means mapping the original's addresses back to native objects. A mistake in the adapter shows up as a divergence, so the checks also test the adapter |
| Change the checks | Compare in native terms: milestones and picture and sound only, or a hand-written per-subsystem equivalence | No adapter. Check 1 (state pass by pass) and routine conformance lose their address-level precision, so a divergence is found later and named less exactly; starting mid-game from the original's memory needs a different route (from the oracle by input only, or a native-state importer, which is the adapter's inverse again) |

The routes can be mixed per subsystem: an adapter for a rewritten subsystem while the rest keeps
the byte store. This chapter presents them; choosing is the rewrite plan's.

## 9. What a native rewrite must preserve

| Amiga-shaped part | Behaviour to keep | Check that proves it |
|---|---|---|
| Values at original addresses | Every compared variable equals the original's at the end of each pass | Every `sf-conform` case's state check (`compare::gameplay_vars`), e.g. `engine/conform/cases/complete-w1-1.toml`; `runner.rs`'s `transplant_then_probe_returns_the_originals_values` |
| Overlaid widths | A read of part of a variable sees the bytes the last write of any width left | `state.rs`'s `every_variable_round_trips_through_memory`; `mem.rs`'s `variables_shadow_the_executable`; routine conformance per ported routine (`sf-conform/src/routine.rs`, `Rig::check`) |
| Pointers into the image | Pointers hold the original's absolute addresses and reach the same tables | `mem.rs`'s `hunk0_is_relocated_to_each_target_s_base`; every case compares the pointer variables (`frog_anim`, `obj_slots`) |
| Null-pointer reads at 0 and 4, write to `$64` | Same values read and written as on Kickstart 1.3 | No check is known to cover these by name. `enemies-w3-3` plays the swing (`enemies_w3.rs`) and the `enemies-w2-*` cases the mace (`enemies_w2_cases.rs`); whether either reaches the no-slot write is unknown: find out by logging writes to `level1_vector` in those runs |
| Every written byte is mapped | The engine keeps every byte the original writes in what it models | `routine.rs`'s "the original writes …, which the engine does not keep"; `state.rs`'s `a_write_outside_the_table_is_refused` |
| Variables over the image | A variable shadows the executable's bytes at its address; the final level reads its own files | `mem.rs`'s `variables_shadow_the_executable`; no test covers the `final_level_flag` choice (`FINAL_LEVEL_FLAG` in `Bus::b`) by name; the final level's cases (`engine/conform/cases/final-win.toml`) play on its files |
| Presentation split | No variable a gameplay rule reads is left out of the state check | `layout.rs`'s `no_gameplay_variable_is_presentation`, `only_a_bobs_drawing_records_are_presentation`; `faithful.rs`'s `a_presentation_variable_differing_is_not_a_state_difference` |
| Planted divergence named | A one-variable difference is reported with its frame and name | `runner.rs`'s `a_planted_one_variable_divergence_is_named_with_its_frame` |
| Save state round trip | A loaded state runs on as the saved one; another build's is refused | `engine/crates/sf-core/tests/savestate.rs` (`a_loaded_level_runs_on_as_the_saved_one`, `a_state_from_another_build_or_cache_is_refused`) |

## 10. Native sketch (non-binding)

Non-binding: designing the rewrite is a later plan's job. One shape: native per-subsystem state
(`Frog`, `Objects`, `Level`), each with a generated or hand-written `project(&self, &mut
[u8])`/`import(&[u8])` pair against its slice of `LAYOUT`, so `probe` and `transplant` stay the
interface `sf-conform` sees. Subsystems not yet rewritten stay in the byte store, and the `Bus`
falls through to it. Pointer-valued variables map to native references through a table of the
original's base addresses (object slots `$…` apart, frame lists in `Rom`). `Rom` stays as a
read-only data source either way.

## 11. Reuse

**`big-changes`**, 1815 lines (`python tools/architecture.py totals`, 2026-10-23).

The approach carries to any Amiga game whose port is checked against the original by memory:
byte store, table-driven transplant and probe, a bus over the loaded image. Roughly 700 lines are
generic and carry nearly as they are: `State`'s byte store and readers, `Width`, `VarDef` and
`pieces`, `index_of`, `Ram`, `AmigaMemory`, `loaded_hunk0`, `Rom::byte`, `Bus`. The rest is
rewritten per game: the 292-entry `layout!` list and its tests, the hunk bases (another game's
loader places hunks elsewhere), `presentation`'s tags, `boot_tables` and `tile_heights`,
`Rom::new`'s choice of files, the sound driver's special case, the named accessors, and the
Kickstart values. That is more than "a few named, local changes", so not `small-changes`.

## 12. Sources

- `engine/crates/sf-core/src/state.rs`, `engine/crates/sf-core/src/layout.rs`,
  `engine/crates/sf-core/src/mem.rs` and their tests.
- `engine/crates/sf-core/src/game.rs` (`Start`, `Game::with_display`, `Game::probe`),
  `engine/crates/sf-core/src/savestate.rs`, `engine/crates/sf-core/src/feed.rs`,
  `engine/crates/sf-core/src/audio/mod.rs`, `engine/crates/sf-core/src/front/story/mem.rs`.
- `engine/crates/sf-conform/src/compare.rs`, `engine/crates/sf-conform/src/routine.rs`,
  `engine/crates/sf-conform/src/sync.rs`, `engine/crates/sf-conform/src/milestones.rs`,
  `engine/crates/sf-conform/tests/runner.rs`, `engine/crates/sf-conform/tests/faithful.rs`.
- [docs/08 "Memory map"](../08-executable-map.md);
  [rust-engine DESIGN](../../plans/rust-engine/DESIGN.md) §2.1, §2.8, §3.3;
  this plan's DESIGN §2.5.
