+++
id = "copied-in-data"
title = "Copied-in data"
tier = "mechanism"
depth = "deep"
kind = "data"
mix = { amiga = 40, game = 0, data = 55, shell = 5 }
reuse = "big-changes"
owns = []                     # a view across files other chapters own (DESIGN §2.4)
+++

# Copied-in data

## 1. What it is

The engine does not carry Superfrog's tables, pictures, sounds or levels. It reads them from the
player's four disks at import (rust-engine DESIGN §2.2, §2.3), and most of what the game logic
reads is then used as the original laid it out: the game executable's hunk 0 relocated to the
address the bootstrap gives it, the tables the boot builds placed where the boot puts them, each
world's files at the addresses its load fills, and the front end's overlay programs and the story
intro as their own loaders leave them. The ported routines read all of it by original address.
This is the "static memory pushed by the system" the owner asked about. These are the bytes the
original's loaders and boot put in memory, and nothing in the engine writes them after set-up.

This chapter owns no files. It is a view across the code of other chapters:

- the disk decode, `engine/crates/sf-assets/src/gamedata.rs` (`decode`, `GameData`, the
  `FILE_NAMES` catalogue, `GameData::raw`), `engine/crates/sf-assets/src/exe.rs` (`parse_exe`,
  `ExeImage`), `engine/crates/sf-assets/src/tables.rs` (`Tables`) and
  `engine/crates/sf-assets/src/sound.rs` (`SoundData`): [disks-and-assets](disks-and-assets.md);
- the level's fixed image, `engine/crates/sf-core/src/mem.rs` (`Rom`, `loaded_hunk0`, `boot_tables`,
  `TILE_PTRS`, `tile_heights`, `Bus::b`): [memory-model](memory-model.md);
- the display's copy of it, `engine/crates/sf-core/src/render/mod.rs` (`DisplayData`, which holds
  `Rom`, `FrontData`, `StoryData`, `SoundData`, hunk 2, the frog's sprites and the level files):
  [display-hardware](display-hardware.md);
- the front end's, `engine/crates/sf-core/src/front/data.rs` (`FrontData`, `relocate`,
  `OVERLAY_AT`): [front-end](front-end.md);
- the story intro's, `engine/crates/sf-core/src/front/story/data.rs` (`StoryData`, `BOOT_BASE`,
  `INTRO_H0`..`INTRO_H3`): [story-and-ending](story-and-ending.md).

The formats are in [docs/10](../10-asset-formats.md), the executables, memory map and file table
in [docs/08](../08-executable-map.md), sound in [docs/11](../11-sound.md), the front end and the
intro in [docs/13](../13-front-end.md). Addresses are named in `re/symbols.toml`.

## 2. Kind mix

`mix = { amiga = 40, game = 0, data = 55, shell = 5 }`, judged by reading the roughly 1.4k lines
that implement the mechanism (the `Rom` half of `mem.rs`, `front/data.rs`, `story/data.rs`,
`tables.rs`, `exe.rs`, the raw-blob part of `gamedata.rs`, `sound.rs`'s loader). Code that imitates
the AmigaDOS loader and the boot counts as `amiga`: relocating to fixed hunk bases, placing images
at original addresses, letting a variable shadow the executable. Code that decodes and keeps the
original's bytes counts as `data`. The asset errors count as `shell`. The chapter owns no lines, so
the mix weighs nothing in the overview's totals; it describes the mechanism, not owned code.

## 3. Mechanisms it uses, and where else they are used

**The loaded hunk image (`Rom`)** is the centre: `Rom::new` relocates hunk 0 of `d1_c03` to `H0`
(`loaded_hunk0`), runs `boot_tables` over it, adds hunk 2 (`accessing-picture`, the panel and HUD
graphics) and, per `world` value, the regions that world's files fill. `Bus::b` reads a
[layout table](memory-model.md) variable first and falls back to `Rom::byte`. It picks the final
level's files while `final_level_flag` (h0+$1e) is set, whatever `world` says. Every routine on the
level's bus therefore reads copied-in data whenever it touches an address no variable covers. That
covers the frame lists in `engine/crates/sf-core/src/level/bobs.rs`, the object templates in
`engine/crates/sf-core/src/level/objects.rs`, the frog's animations in
`engine/crates/sf-core/src/frog/anim.rs`, the enemies' paths in `engine/crates/sf-core/src/enemies/`
and the bonus level's in `engine/crates/sf-core/src/bonus/enemies.rs`.

**Boot-built tables**: `boot_tables` rebuilds what `rng_seed_from_cia` and `load_hero` build, at
their addresses: the map's row pointers (h0+$1730c), the display ring's row pointers (h0+$174a0) and
row offsets (h0+$176a2), the frog's frame records (h0+$19d8a) and the 896 tile pointers
(`TILE_PTRS`, h1+$30fe0). `tile_heights` is h0+$d950 over a world's `LnMS`, built once per world in
`Rom::heights` and poked into the `tile_heights` variable by `engine/crates/sf-core/src/level/load.rs`
and `engine/crates/sf-core/src/front/chain.rs`. `hero_sprites` in `render/mod.rs` is `load_hero`'s
conversion of `HERO` to `HERO_SPRITES` (h3+$4c0b0).

**Byte-exact state at original addresses** depends on this image for its power-on values: `boot_image`
in `level/load.rs` fills every `LAYOUT` variable in hunk 0 from `Rom::byte`, hunk 2's from
`DisplayData::h2_bytes`, and the frog's sprites from `DisplayData::hero`. A variable's starting value
is the executable's byte at its address.

**A second copy for the front end**: `FrontData::new` relocates hunk 0 again (without the boot's
tables) and the Accessing-disk overlay program to `OVERLAY_AT` (h3+$3a560), with the unpacked
file's tail kept behind it as `hunk_relocate` leaves it. `title::overlay_place` writes the overlay
into memory and patches its copper list. `FrontData::file` hands the front end, the slot machine
and the game flow files as `load_file` leaves them (`MPIC`, `HPIC`, `HERO`, `FRUT`/`FRUC`/`FRUM`,
`LnLP`, `LnET`) and the pictures packed in hunk 0 (`h0@3464c`, `h0@34e92`, `h0@36a04`,
`h0@37a0a`).

**The story's bus simulator** runs over its own copy: `StoryData` relocates `d3_c02` to `BOOT_BASE`
and `d3_c04` to `INTRO_H0`..`INTRO_H3`, relocates the `d3_c03` data hunk (`data_hunk`), and keeps the
four picture banks unpacked and the four sector spans raw. `front/story/intro.rs` writes them into
`StoryMem` where the original's loaders put them.

**Paula writes** take their samples from `SoundData`: the modules and effect banks as files, loaded
at `BANK_ADDR` (h3+$64c70), and the replayer's tables (`ARP_STEPS`, `PERIODS`, `FINETUNES`,
`VIBRATO_TABLE` in `engine/crates/sf-core/src/audio/music.rs`) read from an unrelocated hunk 0.

## 4. The catalogue

Offsets are into the game executable `d1_c03` unless the row says otherwise. Chapters are this
documentation's ids. The native option is one of three: **keep** (keep reading the original's
bytes at import, as now), **decode** (decode at import to a typed structure, the `Tables` pattern),
or **code** (regenerate in code from facts about the original, which DESIGN §2.3 lets us commit).

| Item | Comes from | What it is | Read by | Native option |
|---|---|---|---|---|
| Hunk 0 as loaded (`Rom::h0`) | `d1_c03` hunk 0, relocated to `H0` | Code and data of the game; the fallback for every address no variable covers, and every variable's power-on value | every level routine via `Bus`; [memory-model](memory-model.md) (`boot_image`) | decode the tables the routines actually read; the image itself goes once nothing reads by address |
| Hunk 0 as loaded (`FrontData::h0`) | the same, relocated again, no boot tables | The front end's view of hunk 0 | [front-end](front-end.md), [game-flow](game-flow.md), [slot-and-bonus](slot-and-bonus.md), [sound-driver](sound-driver.md) | as above; one image, not two |
| Hunk 0 unrelocated (`SoundData`) | `d1_c03` hunk 0 via `sound.rs`'s own `load_file` | The replayer's arpeggio, period, finetune and vibrato tables; the driver's power-on variables; streamed samples | [sound-driver](sound-driver.md) | decode, keeping the over-read past a table's end (§8) |
| Boot-built tables | built by `boot_tables` from facts plus end marks in hunk 0 | Map row pointers, display ring row pointers and offsets, frog frame records, `TILE_PTRS` | [level](level.md), [frog-and-items](frog-and-items.md), [display-hardware](display-hardware.md) | code; they are arithmetic over addresses, except the frog's records, which read `hero_rects` (h0+$1a6f4) |
| `tile_heights` | computed from each world's `LnMS` (h0+$d950) | Per tile, per pixel column, how high the mask stands | [level](level.md) (collision), [frog-and-items](frog-and-items.md) | code at import (already is) |
| Tiles at h1 (`Rom::worlds`) | `LnBM` per world, decoded to `Tileset` then re-encoded | 840 tiles the routines and `TILE_PTRS` address | [level](level.md), [display-hardware](display-hardware.md) | decode; a native renderer reads the `Tileset` |
| Bob sheet at h3 | `LnBO` per world | Five planes of 320 × 320 the bob blits cut from | [display-hardware](display-hardware.md), [enemies-and-bosses](enemies-and-bosses.md) | decode |
| Frog sprites at `HERO_SPRITES` | `HERO`, converted as `load_hero` does | Hardware-sprite frames | [display-hardware](display-hardware.md), [frog-and-items](frog-and-items.md) | decode |
| Hunk 2 (`DisplayData::h2`) | `d1_c02`, a bare `HUNK_DATA` block | The panel's parts, the HUD's digits, the level copper list's home | [display-hardware](display-hardware.md), [level](level.md) (panel) | decode |
| Object templates | h0+$1c6e2, 160 × $ae (`obj_templates`) | Each object kind's starting slot, with routine pointers | [level](level.md) (`obj_spawn_cell`), [enemies-and-bosses](enemies-and-bosses.md), [frog-and-items](frog-and-items.md) | decode to typed kinds; pointers become kind ids |
| Marker jump table | h0+$178ec, 7 × 64 `bra.w` (`obj_flag_jumptab`) | What a map-cell marker spawns, per world | replaced by `Dispatch::register_marker` (code); `Tables::obj_marker_stubs` decodes it but no engine code reads it | code (already is) |
| Frame lists and animation tables | e.g. h0+$2afb0, h0+$2c9b4, h0+$2d128 (`level/bobs.rs`), h0+$10492..$10f52 (frog), h0+$2af0a (`ANIM_END`) | Sequences of frame records the animators step through | [display-hardware](display-hardware.md), [frog-and-items](frog-and-items.md), [enemies-and-bosses](enemies-and-bosses.md) | decode |
| Movement paths | e.g. h0+$24454 (`enemies/w1/hover.rs`), h0+$25ade (`bosses/witch.rs`), h0+$28b8a (`bonus/enemies.rs`) | Per-step offsets enemies and bosses follow | [enemies-and-bosses](enemies-and-bosses.md), [slot-and-bonus](slot-and-bonus.md) | decode |
| Pickup tables | h0+$f382 (`items/pickups.rs`) | Per world, (tile, handler) longs: what a pickup drawn in the tiles gives | [frog-and-items](frog-and-items.md) | decode |
| Attract demo joystick | h0+$11a2a (`DEMO_JOY` in `frog/anim.rs`) | The recorded input the attract demo replays | [frog-and-items](frog-and-items.md), [front-end](front-end.md) | decode |
| `top_loop`'s code | hunk 0 from h0+$a8, h0+$e6 | The game's sequence of calls per world and level, read instruction by instruction by `Chain::run` | [game-flow](game-flow.md) | code: write the sequence out, as `front/story/scenes.rs` already does for the intro |
| Level codes | h0+$24dc, 28 × 10 (`password_table`), and each handler's `move.l` immediates | Six digits and the world and level they start | [front-end](front-end.md) (`password::table`), [slot-and-bonus](slot-and-bonus.md) (the code shown) | decode |
| Start and resume points | h0+$bde, 7 × $14 (`start_resume_table`) | Each world's entry and level resume addresses in `top_loop` | [game-flow](game-flow.md) (`game_start_at`) | decode, as entry points of the native flow |
| File and disk tables | h0+$cbfe, 95 × 6 (`file_table`); h0+$b11e (`disk_id_table`) | Where each file is, and on which disk | [disks-and-assets](disks-and-assets.md) (`catalogue`), [front-end](front-end.md) (the disk a load asks for), [sound-driver](sound-driver.md) | decode (already is, in `Tables`) |
| Slot machine tables | h0+$57b6, $606a, $668a, $670a, $3b2e8, $3b1ae; h0+$6842 (`slot_lamps`) | Prize ladder, pointer stops, reel symbols, lamp rectangles, attract patterns | [slot-and-bonus](slot-and-bonus.md), read from memory by address | decode; `Tables` already holds them, unread |
| Starting high scores | h0+$2f90, 10 × 8 and ten names | The table a game starts with | [front-end](front-end.md) (`front/hiscore.rs`), as the `hiscore_table` variable whose power-on value is the image's | decode |
| Packed pictures in hunk 0 | `ATN!` chunks at h0+$3464c, $34e92, $36a04, $37a0a | Options screen, the title's eyes, the overlay's picture, the progress map | [front-end](front-end.md), [game-flow](game-flow.md) | keep, unpacked at import |
| Overlay programs | `ATN!` at h0+$35fee (insert disk), h0+$36a04 (Accessing disk) | Small programs with their own copper list, picture and fade | [front-end](front-end.md) (`overlay_place`), [game-flow](game-flow.md) | decode the picture, palette and fade; replace the program with code |
| Front-end files | `MPIC`, `HPIC`, `AWA1`, `AWA2`, `FRUT`, `FRUC`, `FRUM`, `LnLP`, `LnET`, `END1`, `END2`, `INFT` | Pictures and pages loaded over the display buffers as files | [front-end](front-end.md), [slot-and-bonus](slot-and-bonus.md), [game-flow](game-flow.md), [story-and-ending](story-and-ending.md) | decode (`Picture`, `Anim` already exist in `sf-assets`) |
| Level maps | `LnMAm` (`T7MP`), re-encoded from `LevelData` into `DisplayData::levels` | The map file, loaded raw at h3+$22f30 and unpacked by the ported `map_unpack` | [level](level.md); replaced by a `.sflevel` ([docs/14](../14-sflevel-format.md)) | decode (the editor's format already is one) |
| Sound bank | `MUSC`, `LnMU`, `LBMU`, `LFMU`, `L1FX` (two copies), `LBFX`, `LFFX`; streamed samples in hunk 0 (h0+$1491e) | Player 4.1A modules, effect banks, channel 3 samples | [sound-driver](sound-driver.md), [sound-chip](sound-chip.md) | keep the sample bytes; decode the modules' headers |
| Story intro image | `d3_c02` at `BOOT_BASE`; `d3_c04` at `INTRO_H0`..`INTRO_H3`; `d3_c03` data hunk; banks `d3_c05`..`d3_c08`; raw spans | A whole second program and its memory | [story-and-ending](story-and-ending.md) | decode the pictures and tables; the program is already replaced by `scenes.rs` |

## 5. Data addressed by original pointers inside other data

This is what ties copied-in data to the [memory model](memory-model.md). The loader adds each
target hunk's base to every long a `HUNK_RELOC32` group names (`mem.rs::loaded`, and the same again
in `front/data.rs::relocate` and `story/data.rs::data_hunk`). After that, every pointer inside a table
is an original address, and the engine follows it as one:

- **Routine pointers.** An object template's `+$00` (init), `+$04` (update) and `+$10` (touch) are
  hunk 0 addresses. A spawn copies the template into a slot, and `level::call` looks the slot's
  long up in `Dispatch::routine`, keyed by original address. `items/pickups.rs` matches a pickup
  handler's address the same way, and `beam::routine_cost` charges cycles by it.
- **Code addresses.** `chain::target` maps the targets of `top_loop`'s `jsr abs.l` instructions to
  the engine's routines. `start_resume_table`'s rows and the level codes' handlers are hunk 0
  addresses, and `password::table` reads world and level out of the handler's instructions.
  `front/mod.rs` finds a load's `file_table` entry by scanning for its `lea abs.l, a2`.
- **Frame and path tables.** Animation lists and enemy paths hold longs pointing at frame records,
  sprite data or other tables in hunk 0 or hunk 3. `slot_ladder`'s rungs point at `slot_lamps`
  records.
- **Boot-built pointers.** `boot_tables` writes original addresses: map rows into h1, display ring
  rows into `playfield_buffer_0` (h3+$10600), frog frames into h3+$4c0b0, tiles into h1.
- **Other programs' bases.** The overlay is relocated to `OVERLAY_AT`; the intro to the bases its
  boot program uses, measured on the oracle (`story/data.rs`).

`Tables` in `sf-assets` keeps such pointers unrelocated, as hunk 0 offsets. That makes them
readable as `re/symbols.toml` names, but the engine reads the relocated copies in memory, not
`Tables`. A native rewrite that drops original addresses must turn each pointer into something
typed (a kind id, an index into a decoded list) at import, in one pass that knows every table's
layout. A table whose layout nobody decoded keeps the rewrite reading by address.

## 6. The copyright boundary

Rust-engine DESIGN §2.3: graphics, sound, levels, strings and every numeric table are read from the
player's disks at import, located by chunk and offset, and never copied into source, tests, docs or
fixtures. Facts about them (addresses, sizes, formats, meanings) are ours and are committed.
`tools/check_no_disk_data.py` fails on any tracked file holding a run of 128 or more bytes found in a
disk chunk. The purity test (`engine/crates/sf-conform/tests/purity.rs`,
`the_real_pure_crates_pass`) flags integer array literals of more than 16 elements in the pure
crates.

What this means for a native rewrite:

- **Tables cannot be baked into source.** Every table in §4 marked "decode" stays read from the
  disks at import. A native rewrite moves it from "bytes at an address" to a typed field read at a
  named offset, the pattern `engine/crates/sf-assets/src/tables.rs` documents. It cannot become a
  `const` array.
- **Arithmetic tables may become code.** The row pointers, row offsets and tile pointers in
  `boot_tables`, and `tile_heights` computed from the masks, are computed from facts and may be
  written as code. They already are.
- **Sequences may be written as code where they are structure, not content.** `scenes.rs` already
  writes the intro's scene sequence step by step, and `Dispatch` already replaces
  `obj_flag_jumptab`. `top_loop` could follow. Whether a long sequence copied as code still counts
  as a fact is a judgement for the owner, not settled here.
- **The import step stays.** A native build still needs the disks, `sf-disk`'s `ATN!` decoder and the
  asset cache, so the "read at run time" approach survives any rewrite.
- **Exported files carry the data.** A `.sflevel` holds the level's original files. It is the
  player's own file and is never committed, so the editor works within the same boundary.

## 7. What depends on it

- Every routine that reads through `Bus` or `FrontData::h0_*`, so nearly all of
  [level](level.md), [frog-and-items](frog-and-items.md), [enemies-and-bosses](enemies-and-bosses.md),
  [slot-and-bonus](slot-and-bonus.md), [front-end](front-end.md) and [game-flow](game-flow.md).
- [memory-model](memory-model.md): power-on values come from the image (`boot_image`), and
  `variables_shadow_the_executable` fixes the read order (a variable first, then the image).
- [verification](verification.md): the routine oracle (`engine/crates/sf-conform/src/routine.rs`)
  hands ported routines `DisplayData::rom`, and `sf-conform`'s scripts read the level codes through
  `FrontData`. A transplant overwrites variables but never the image, so the image must equal the
  original's memory wherever no variable is.
- [story-and-ending](story-and-ending.md): `StoryMem` starts from `StoryData`.

## 8. What a native rewrite must preserve

| Amiga-shaped part | Behaviour to keep | Check that proves it |
|---|---|---|
| Hunk 0 relocated to `HUNK_BASES` | Every relocated long equals the original's loaded value | `hunk0_is_relocated_to_each_target_s_base` (`engine/crates/sf-core/src/mem.rs`); every `*_routines.rs` test in `engine/crates/sf-conform/tests/` runs ported routines over it |
| Variables shadow the image; image gives power-on values | A variable's start value is the executable's byte; reads prefer the variable | `variables_shadow_the_executable` (`mem.rs`); `engine/conform/cases/front-boot.toml` and every power-on case compare all variables |
| Boot-built tables at their addresses | Row pointers, ring rows and offsets, frog frame records, tile pointers equal the original's memory | `the_boot_s_tables_are_the_original_s` (`engine/crates/sf-conform/tests/frog_routines.rs`) |
| `tile_heights` per world, the bonus level leaving the last world's | The heights equal h0+$d950's; the bonus level keeps W5's (T203) | the `tile_heights` variable compared by every level case, e.g. `engine/conform/cases/code-w1-1.toml`; the bonus-level carry-over by `engine/conform/cases/game-whole.toml` only |
| World files at h1 and h3, frog sprites at h3+$4c0b0 | Tiles, bobs and `HERO` sit where the loads put them | tiles and bobs: `sheets_tiles_and_the_font_are_the_copies_in_ram` (`engine/crates/sf-conform/tests/graphics_vs_oracle.rs`), which compares the decoded `L1BM` and `L1BO` with the oracle's RAM, and W1's tiles in `the_boot_s_tables_are_the_original_s`; the frog's sprites: the `hero_frames` variable (h3+$4c0b0) compared by every level case, e.g. `engine/conform/cases/code-w1-1.toml` |
| Final level read through `final_level_flag` | Its `LW*` files replace W6's whenever the flag is set | `engine/conform/cases/whole-final-load.toml`, `engine/conform/cases/final-win.toml` |
| Hunk 2 (panel, HUD) | The panel draws from the original's bytes | the picture comparison of the `render-*` cases, e.g. `engine/conform/cases/render-w1-1.toml` |
| Routine pointers in templates dispatched by address | Each template's init, update and touch run the matching port | `the_kinds_and_markers_are_registered` and `spawners_match_the_original` (`engine/crates/sf-conform/tests/enemies_w2_routines.rs`, and the per-world files beside it) |
| `top_loop` read as code | The order of calls, branches and resume points is the original's | `engine/conform/cases/chain-start-w1-1.toml`, the other `chain-*` cases, `engine/conform/cases/game-whole.toml` |
| Level codes read from their handlers | Every six-digit code starts where the original starts it | `every_six_digit_code_starts_where_the_original_starts_it` (`engine/crates/sf-conform/tests/front.rs`) |
| Overlay relocated to `OVERLAY_AT` | Its picture, copper list and fade as the original runs them | `engine/conform/cases/chain-start-w1-1.toml`, `engine/conform/cases/whole-final-load.toml` |
| Slot machine files and tables | Loads and rounds as the original | `the_slot_machine_loads_as_the_original_does` (`front.rs`), `the_slot_machine_tables_and_pictures` (`engine/crates/sf-assets/tests/levels.rs`) |
| Attract demo joystick | The demo replays the recorded input | `engine/conform/cases/attract-demo-3.toml`, `the_attract_cycle_matches_the_original` (`front.rs`) |
| Sound bank, replayer tables, streamed samples, including reads past a table's end | Paula sees the original's samples and periods | `every_tune_matches_the_original_replayer`, `streamed_samples_play_the_originals_bytes` (`engine/crates/sf-conform/tests/audio_driver.rs`). Whether any tune actually reads past a table's end is unknown; reading the `music.rs` index ranges against the table sizes would settle it |
| Story intro image at its measured bases | The intro plays as the original | `the_intro_from_disk_3_matches_the_original` (`engine/crates/sf-conform/tests/story.rs`), `engine/conform/cases/story-intro-full.toml` |
| Level map loaded raw and unpacked | A disk level and its `.sflevel` export play identically | `an_unchanged_export_plays_as_the_disk_level_for_1500_frames` (`engine/crates/sf-assets/tests/sflevel.rs`) |
| No disk data in the repo | Nothing copied in from the disks is committed | `tools/check_no_disk_data.py` with `tools/test_check_no_disk_data.py`; `the_real_pure_crates_pass` |

## 9. Native sketch (non-binding)

Non-binding: designing the rewrite is a later plan's job. One shape it could take:

- One import pass in `sf-assets` decodes every table the game reads into typed structures, the way
  `Tables` does now, with pointers turned into kind ids and indices. `Rom`, `FrontData::h0` and
  `SoundData`'s copy collapse into it.
- Code that reads by address moves table by table to the typed fields. The image stays as long as
  any reader is left, and the conformance checks show each move kept the behaviour.
- The arithmetic boot tables and `tile_heights` stay as code. The overlay and `top_loop` become
  code too, with their pictures and palettes decoded.
- The asset cache keeps holding the decoded result, so import still runs once.

## 10. Reuse

**Rating: `big-changes`.** The approach carries to another Amiga game: read the original's data
from the player's disks at run time, keep the executable as its loader leaves it in memory, read
tables by original address while porting, and decode them as the port matures. Much of the code
must be rewritten.

- **Carries as-is:** `parse_exe` and the hunk relocation (`loaded`, `relocate`, `data_hunk`); the
  `Rom` and `Bus` pattern of a fixed image under modelled variables; the copyright stance and its two
  checks.
- **Superfrog-only:** the hunk bases (`H0`..`H3`, `OVERLAY_AT`, the intro's bases), every offset in
  §4, `boot_tables`, h0+$d950's height rule, the file names and the custom trackloader's
  `file_table`, the packed pictures and overlays. A game loaded through AmigaDOS would replace the
  file table with a filesystem reader.
- Another Team 17 game may share the `ATN!` packing, a similar `file_table` and Player 4.1A's tables
  (unchecked).

Size: the chapter owns no lines (`python tools/architecture.py totals` prints 0). The code it
describes belongs to [disks-and-assets](disks-and-assets.md), [memory-model](memory-model.md),
[front-end](front-end.md) and [story-and-ending](story-and-ending.md), and is counted there.

## 11. Sources

- [docs/08](../08-executable-map.md) (the executables, memory map, files and disks),
  [docs/10](../10-asset-formats.md) (formats, `Tables`, kept bytes), [docs/11](../11-sound.md),
  [docs/13](../13-front-end.md), [docs/14](../14-sflevel-format.md).
- [rust-engine DESIGN](../../plans/rust-engine/DESIGN.md) §2.2 (disks and import), §2.3 (copyright
  boundary), §3.5 (storage); `plans/rust-engine/FINDINGS.md` (T14 on the final level's files, T203
  on the bonus level's heights).
- Code: `engine/crates/sf-core/src/mem.rs`, `engine/crates/sf-core/src/render/mod.rs`,
  `engine/crates/sf-core/src/front/data.rs`, `engine/crates/sf-core/src/front/story/data.rs`,
  `engine/crates/sf-core/src/front/chain.rs`, `engine/crates/sf-core/src/front/password.rs`,
  `engine/crates/sf-core/src/level/load.rs`, `engine/crates/sf-core/src/level/mod.rs`,
  `engine/crates/sf-assets/src/gamedata.rs`, `engine/crates/sf-assets/src/exe.rs`,
  `engine/crates/sf-assets/src/tables.rs`, `engine/crates/sf-assets/src/sound.rs`, `re/symbols.toml`.
