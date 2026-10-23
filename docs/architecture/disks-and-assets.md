+++
id = "disks-and-assets"
title = "Disks and assets"
tier = "component"
depth = "light"
kind = "shell"
mix = { amiga = 15, game = 0, data = 40, shell = 45 }
reuse = "big-changes"
owns = [
  "engine/crates/sf-disk/",
  "engine/crates/sf-assets/",
]
+++

# Disks and assets

## What it is

Import, the engine's first stage (rust-engine [DESIGN §2.2–§2.4](../../plans/rust-engine/DESIGN.md)):
the player's four disk images in, a typed `GameData` out, kept between launches in a cache file.
Both crates are pure: bytes in, values out, no files, clock or threads (the shell in `sf-app` reads
and writes the files).

- **`sf-disk`** (`engine/crates/sf-disk/`): `identify` finds the four disks among the offered files
  (zipped or raw ADFs, by SHA-1) and names every problem by slot or file; `carve` cuts a disk into
  its `ATN!` chunks; `decompress` unpacks one chunk. It also holds the zip reader and the stored-only
  zip writer the bug recorder uses (`engine/crates/sf-disk/src/zip.rs`, `write_stored`).
- **`sf-assets`** (`engine/crates/sf-assets/`): `unpack` turns the disk set into decompressed chunks
  plus the story intro's raw sector spans; `catalogue` names each chunk by the game's own
  `file_table`; `decode` builds `GameData` (levels, world tiles and masks, sheets, pictures, the
  end ANIMs, the font, seven executables, the `Tables`, and files kept as bytes in `GameData::raw`);
  `cache_encode`/`cache_decode` and `decide` keep it between launches; `sflevel_encode` and
  `sflevel_decode` are the `.sflevel` file ([docs/14](../14-sflevel-format.md)); `sound::SoundData`
  is the sound read from the disks the way the game's `load_file` reads it.

The formats themselves are in the RE notes: the disk and container in
[docs/01](../01-disk-format.md), the boot in [docs/02](../02-boot-chain.md), the codec in
[docs/03](../03-atn-codec.md), the level file in [docs/04](../04-t7mp-level-format.md), the rest in
[docs/10](../10-asset-formats.md). What the engine later does with the decoded bytes is the
[copied-in data](copied-in-data.md) chapter.

## Kind mix

`mix = { amiga = 15, game = 0, data = 40, shell = 45 }`, judged by sorting each file (and the
parts of `gamedata.rs` and `sound.rs`) into one kind and weighing by its lines, tests included:
the codec, the carver and the code that reads sectors the way the original's loaders do count as
`amiga`; the format decoders as `data`; disk identification, zip, hashes, the cache, the launch
decision, `.sflevel` and the error types as `shell`. Nothing in either crate is Superfrog's rules.

By crate (lines are code lines, `.rs`, tests included):

| Crate | Lines | amiga | data | shell |
|---|---:|---:|---:|---:|
| `sf-disk` | 1 791 | 42 | 0 | 58 |
| `sf-assets` | 4 863 | 5 | 55 | 40 |

By module, with who the format belongs to. **Amiga-generic** is any Amiga program's format;
**Team 17** is the publisher's own, possibly shared with its other games (unchecked, DESIGN §8);
**Superfrog** is this game's layout or catalogue; **none** is not Amiga at all.

| Module | Lines | Kind | Format | What is specific |
|---|---:|---|---|---|
| `sf-disk/src/adf.rs` | 59 | shell | Amiga-generic (DD ADF, `ADF_SIZE`) | `DiskSlot` has four slots |
| `sf-disk/src/identify.rs` | 263 | shell | none | `KNOWN` (four SHA-1s), the messages name Superfrog |
| `sf-disk/src/zip.rs`, `sha1.rs` | 436 | shell | none | nothing |
| `sf-disk/src/carve.rs` | 144 | amiga | Team 17 (`ATN!` chunks back to back on sector starts) | the sanity bounds match `tools/atn.py` |
| `sf-disk/src/atn.rs` | 248 | amiga | Team 17 (`ATN!`, a port of the game's own 68000 routine) | nothing beyond the format |
| `sf-assets/src/exe.rs` | 189 | data | Amiga-generic (AmigaDOS hunk executable) | `ExeId`, the seven executables |
| `sf-assets/src/picture.rs` (`Bitmap`), `palette.rs` | 185 | data | Amiga-generic (plane-major bitplanes, OCS `0x0RGB`) | `PictureId` and each picture's size and depth |
| `sf-assets/src/anim.rs` | 253 | data | Amiga-generic (IFF `FORM ANIM`, ByteRun1, delta op 5) | `AnimId` |
| `sf-assets/src/t7mp.rs` | 161 | data | Team 17 (`T7MP`, "Team 17 MaP") | nothing beyond the format |
| `sf-assets/src/tiles.rs`, `sprites.rs`, `font.rs` | 220 | data | Superfrog (per-tile plane-major `LnBM`, `LnMS` masks, 320-wide sheets, the font strip at h0+$378ca) | all of it |
| `sf-assets/src/tables.rs` | 233 | data | Superfrog (tables at hunk 0 offsets) | all of it |
| `sf-assets/src/gamedata.rs` | 705 | data, amiga | Superfrog (`FILE_NAMES`, `OTHER_FILES`, `disk_with_id`, which chunk is which) | all of it; `unpack` and `unpack_chunk` are `amiga` (below) |
| `sf-assets/src/sound.rs` | 414 | data, amiga | Superfrog (`file_table`, the stream table at h0+$1491e, original addresses); Player 4.1A modules (`P41A`) | all of it |
| `sf-assets/src/sflevel.rs` | 566 | shell | ours (JSON over the `T7MP` file) | the level ids |
| `sf-assets/src/cache.rs`, `blake3.rs`, `launch.rs`, `error.rs`, `lib.rs` | 961 | shell | none | the body is `GameData` field by field |
| tests (`sf-disk/tests/`, `sf-assets/tests/`) | 1 598 | per what they test | | the real-disk tests need the owner's disks |

## Where it touches the Amiga-shaped parts

These crates imitate no hardware and charge no time. They touch the Amiga-shaped parts in two
ways: some of their code reproduces what the original's loaders do with the disk, and the values
they hand over are shaped for the engine's [memory model](memory-model.md), which places them at
the original's addresses.

**Hunk images into `mem::Rom`.** `parse_exe` keeps every hunk as the file has it: contents
unrelocated, the `HUNK_RELOC32` groups as `Reloc { target, offsets }`, and a hunk the header
declares but the file does not carry kept as absent with its declared size (`d1_c03`'s hunks 2 and
3). It does not relocate, because addresses in `re/symbols.toml` are `hN+$off`, which an
unrelocated long reads as. Relocation is the consumer's: `loaded_hunk0` in
`engine/crates/sf-core/src/mem.rs` adds `HUNK_BASES` (the bootstrap's bases, `H0`..`H3`) to every
relocated long, and `front/data.rs` (`relocate`) and `story/data.rs` do it again for their own
copies ([copied-in data §3](copied-in-data.md)). `Rom::new` also takes hunk 2 from
`GameData::raw["accessing-picture"]` and re-encodes each world's `Tileset` to `LnBM` bytes
(`Tileset::encode`) to place at `H1`, so the ported routines read tiles as the game laid them out.

**Tables by offset.** `Tables::read` decodes the game's tables at their hunk 0 offsets
(`file_table`, `disk_id_table`, `password_table`, `obj_templates`, the slot machine's), with the
pointers inside them kept as unrelocated `h0` offsets. `sf-core` reads few of them; the ported
routines read the same bytes by address through `Rom` (FINDINGS 2026-10-23, T03). `catalogue` is the
main reader: it maps a `file_table` entry's disk index through `disk_id_table` to the disk whose
last four bytes carry that ID (`disk_with_id`, `2222`..`5555`), the check the game makes on a disk
swap ([docs/08](../08-executable-map.md)).

**Levels as files.** `t7mp_decode`/`t7mp_encode` are lossless, so `DisplayData` re-encodes each
`LevelData` to the original map file and the ported `map_unpack` reads it where the game loads it.
A `.sflevel` goes through the same encoder (`Game::with_override` in
`engine/crates/sf-core/src/game.rs`), which is why an unchanged export plays as the disk level.

**Sound at original addresses.** `engine/crates/sf-assets/src/sound.rs` carries original
addresses itself: `H0`, `H3`, `BANK_ADDR` (h3+$64c70) and `STREAM_BUFFER` (h3+$64b70), and each
`StreamSample::addr` is `H0` plus the table's offset. `SoundData` keeps hunk 0 unrelocated for the
replayer's tables (`h0_byte`, `h0_word`). `sf-core` re-exports these (`audio/mod.rs`). This is the
one place an asset crate depends on the memory map.

**The original's loaders, imitated.** Three reads copy the loader's rules rather than the format:

- `unpack_chunk` retries a chunk with the next sector appended when the carve alone does not
  decompress: `d1_c16`, `d2_c39` and `d4_c16` keep their trailer in the sector after the carve, and
  the game's `file_table` reads that sector too ([docs/03](../03-atn-codec.md)).
- `sound::load_file` reads a file as the game's `load_file` does: the entry's start sector and
  sector count from `file_table`, whole sectors, then `decompress`.
- `unpack` reads the story intro's four sector spans by its own `intro_file_table` (`d3_c04`
  h0+$cf26) and keeps them raw, as its trackloader leaves them; two hold several packed banks back
  to back that carving does not find ([docs/13](../13-front-end.md)).

**Pictures and palettes** are decoded from plane-major bitplanes to colour indices (`Bitmap`), and
the OCS palette kept as 12-bit `0x0RGB` (`Palette`); turning them into the screen is the
[display hardware](display-hardware.md) chapter's.

### What a native rewrite must preserve

| Amiga-shaped part | Behaviour to keep | Check that proves it |
|---|---|---|
| `ATN!` decompression | Byte-identical output to the game's routine on every chunk | `every_chunk_the_emulator_decodes_decodes_identically` and `malformed_input_returns_its_error` (`engine/crates/sf-disk/tests/atn_reference.rs`); `ten_thousand_mutations_never_panic_or_overallocate` (`engine/crates/sf-disk/tests/atn_fuzz.rs`) |
| The three stragglers | A chunk whose trailer lies in the next sector decodes with that sector, as the game's load does | `the_stragglers_need_the_next_sector` (`atn_reference.rs`); `game_data_is_complete` (`engine/crates/sf-assets/tests/levels.rs`, `L1MU` is 40 514 bytes); `the_stragglers_are_what_the_game_loads` (`engine/crates/sf-oracle/tests/exe_map_claims.rs`) |
| Carving | The same chunks, offsets and order as `tools/atn.py` and the inventory | `carve_matches_the_python_carver`, `carve_matches_the_chunk_inventory` (`engine/crates/sf-disk/tests/real_disks.rs`) |
| `file_table` and disk IDs | Each entry resolves to the chunk at its disk and start sector | `the_catalogue_names_every_chunk_by_the_game_file_table` (`levels.rs`) |
| Intro sector spans | Kept raw, whole, with every packed bank in them | `the_story_intro_s_sector_spans_are_kept_raw` (`levels.rs`) |
| Hunk images | Unrelocated contents, relocation groups and declared sizes, absent hunks kept with their size | `game_data_is_complete` (`levels.rs`, hunk sizes and presence of `d1_c03`); `hunk0_is_relocated_to_each_target_s_base` (`engine/crates/sf-core/src/mem.rs` tests) |
| Tables by offset | The values at the named offsets | `game_data_is_complete`, `the_slot_machine_tables_and_pictures` (`levels.rs`); the routines that read the bytes by address are covered by the `sf-conform` cases, not here |
| Tiles placed at `H1` | `Tileset::encode` gives back the original `LnBM` bytes | `tilesets_encode_back`, `every_tileset_decodes_as_the_python_tool_decodes_it` (`engine/crates/sf-assets/tests/graphics_vs_python.rs`) |
| Level maps | `T7MP` round-trips byte for byte; an exported `.sflevel` plays as the disk level | `all_thirty_maps_decode_and_encode_back_byte_for_byte` (`levels.rs`); `an_unchanged_export_plays_as_the_disk_level_for_1500_frames` (`engine/crates/sf-assets/tests/sflevel.rs`) |
| Pictures, sheets, font, ANIMs | Decoded pixels and palettes equal what the original shows or keeps in RAM | `pictures_are_the_screens_the_original_shows`, `sheets_tiles_and_the_font_are_the_copies_in_ram`, `w1_1_shows_the_level_palette`, `anims_are_what_the_game_s_own_player_draws` (`engine/crates/sf-conform/tests/graphics_vs_oracle.rs`); pictures no run reaches share their decoder with one that is (that file's module comment) |
| Sound files and stream samples | Modules, banks and streamed samples as the game loads them, at their original addresses | `every_tune_and_effect_decodes`, `streamed_samples_play_the_originals_bytes`, `every_tune_matches_the_original_replayer` (`engine/crates/sf-conform/tests/audio_driver.rs`) |

No check compares `SoundData::from_game` (the path the app takes, from the cache) with
`SoundData::from_disks` (the path `audio_driver.rs` takes); both read the same unrelocated hunk 0
and `load_file` bytes, but nothing asserts they agree.

## Reuse

**`sf-disk`: `big-changes`** for another Amiga game by any developer. Carries with a table change:
`adf.rs`, `zip.rs`, `sha1.rs` and `identify.rs` (`KNOWN`, the count of `DiskSlot`, the game's name
in the messages): about 760 lines. Replaced: `carve.rs` and `atn.rs` and their tests (about 700
lines), because another developer's disks use another trackloader and cruncher. For another Team
17 game that uses `ATN!` and the same sector-aligned layout it would be `small-changes`; whether
any does is unchecked (DESIGN §8). 1 791 lines.

**`sf-assets`: `big-changes`.** The approach carries: catalogue the chunks by the game's own file
table, decode to one typed value, keep it in a versioned, hashed cache, decide on launch. Carries
as is: `exe.rs`, `Bitmap` and `Palette`, `anim.rs`, `blake3.rs`, `launch.rs`, `error.rs` and the
cache framing (about 1 100 lines). Rewritten per game: `gamedata.rs`'s catalogue and `decode`,
`tables.rs`, `tiles.rs`, `sprites.rs`, `font.rs`, the `PictureId` sizes and `sound.rs` (its
addresses and `file_table` reading), and the cache body follows `GameData`. `t7mp.rs` and
`sflevel.rs` carry only to a game using `T7MP`, which another Team 17 game might (unchecked);
`sound.rs`'s module reading carries to any game using Player 4.1A, also unchecked here. 4 863
lines.

The front matter carries the combined rating, `big-changes`, over 6 654 lines.

## Sources

- RE notes: [docs/01](../01-disk-format.md), [docs/02](../02-boot-chain.md),
  [docs/03](../03-atn-codec.md), [docs/04](../04-t7mp-level-format.md),
  [docs/08](../08-executable-map.md), [docs/10](../10-asset-formats.md),
  [docs/11](../11-sound.md), [docs/13](../13-front-end.md), [docs/14](../14-sflevel-format.md).
- Design: rust-engine [DESIGN](../../plans/rust-engine/DESIGN.md) §2.2 (import), §2.3 (copyright
  boundary), §2.4 (unpacking), §2.10 (level override), §3.5 (storage).
- Code: `engine/crates/sf-disk/src/`, `engine/crates/sf-assets/src/`, and their consumers
  `engine/crates/sf-core/src/mem.rs` (`Rom::new`, `loaded_hunk0`),
  `engine/crates/sf-core/src/render/mod.rs` (`DisplayData::new`),
  `engine/crates/sf-app/src/import.rs`.
