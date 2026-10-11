# 10 — Asset formats

Every format `engine/crates/sf-assets` decodes that [`04`](04-t7mp-level-format.md) (levels) and
[`06`](06-graphics-and-rendering.md) (tiles) do not cover, and where each file is. `decode` builds
`GameData` from the decompressed chunks; nothing here is copied into the repository (DESIGN §2.3).
Addresses are `hN+$off` in the game executable `d1_c03` (docs/08).

**How each claim is checked**, by the tests named in brackets:
[oracle] `sf-conform/tests/graphics_vs_oracle.rs` against the reference emulator or the game's own
routine; [python] `sf-assets/tests/graphics_vs_python.rs` against `tools/render_levels.py`;
[disks] `sf-assets/tests/levels.rs` on the real disks. **(inferred)** marks what neither settled.

## Which chunk is which

The game finds its files through `file_table` (h0+$cbfe, 95 × `sector, count, disk index`) and
gives `load_file` a name with each (`SDDISK1:L1BM`, a development leftover). `catalogue` pairs
those names, in entry order, with the chunk at each entry's disk and start sector; the disk index
means the disk whose ID `disk_id_table` gives (`2222`..`5555` = disks 1–4). The eleven chunks no
entry names are the executables and what only they load. [disks: every chunk named once, every
name is a string in the game]

- **Copies.** `L1FX`, `FRUT`, `FRUC`, `FRUM`, `LFFX`, `LFMU` are on disk 1 and again, byte for byte,
  on disk 2 (`d2_c01`–`c06`) so a one-drive game need not swap back. `decode` reads disk 1's. The W1–2
  and W3–6 sound effects are the same file. [disks]
- **W6 has two sets of maps**, both called `L6MA1`–`4`: `d4_c02`–`c05` (entries 42–45) and
  `d4_c14`–`c17` (entries 6–9). `level_load` (h0+$a144) takes the second set only when DF1–DF3 are
  connected and the disk in DF0 is disk 1; an A500 with one drive plays the first. Map 1 is the same
  file; maps 2–4 differ in their start position (`REMX`/`REMY`) and up to ~830 bytes of cells.
  `GameData` keeps both (`LevelId::alt`). docs/LEVEL-INDEX.md's "W6-5..W6-7" are these copies.
- **Stragglers.** `d1_c16` (`L1MU`), `d2_c39` (`LBLP`) and `d4_c16` (W6-3, second set) keep their
  trailer in the sector after the carve, which `file_table` reads too. `unpack` retries a chunk that
  does not decompress with the next sector appended; all 106 chunks then decompress. [disks]

## Bitmaps

Every bitmap the game keeps is **plane-major**: all of plane 0's rows, then plane 1's, a row being
`width / 8` bytes, leftmost pixel in the top bit; plane `p` is bit `p` of the colour index. The
32-colour palettes are 32 big-endian `0x0RGB` words.

## Pictures

| Picture | File | Bytes | Layout | Palette | Check |
|---|---|---|---|---|---|
| Title and menu | `MPIC` d1_c06 | 61 440 | 320 × 256 × **6** | none: the copper sets it per line | [oracle] each line's (index → colour) is one colour on the title screen |
| Hall of talent | `HPIC` d1_c07 | 57 664 | 320 × **288** × 5 | after the bitmap | [oracle] screen = rows 0–255 |
| Features table | `AWA1` d1_c08 | 51 264 | 320 × 256 × 5 | after | [oracle] screen equal |
| Bonus table | `AWA2` d1_c09 | 51 264 | 320 × 256 × 5 | after | [oracle] screen equal |
| End of world | `L1ET`…`L6ET` | 51 264 | 320 × 256 × 5 | after | same format as `AWA1` (not reached) |
| World loading | `L1LP`…`L6LP`, `LBLP` | 71 136 / 72 144 / 70 464 | 320 × **352** × 5 | after (`+$11300`) | [oracle] `L1LP`: screen = rows 0–255 |
| Progress map | packed at h0+$37a0a (`pic_progress_map`) | 51 264 | 320 × 256 × 5 | after | [oracle] screen equal |
| Slot machine | `FRUT` d1_c11 | 61 440 | 320 × 256 × 6 | none (copper) | not reached |
| Slot machine lit areas | `FRUM` d1_c13 | 10 240 | 320 × 256 × 1 | — | not reached |

- **Loading pictures** are shown in two parts (two bitplane-pointer sets in the copper list,
  `world_intro_show`): from `h3+$10600` and from `h3+$129a0`, planes `$3700` apart. After the palette
  come 672 (W1, W3, W6) or 1 680 (W2, W4, W5) bytes that look like small loading-screen graphics;
  `Picture::extra` keeps them as they are (inferred, unexplained). `LBLP` has none.
- **The progress map** is an `ATN!` chunk inside hunk 0. `progress_map_show` copies `$fa0` longs of
  it to `h3+$10600` and decompresses it in place, so its palette lands at `h3+$1ce00`, from which the
  game fades it in.
- `HPIC`'s 288 lines are shown 256 at a time.

## The level palette

A level is drawn in the **32 colours after its world's loading picture** (`LnLP + $11300`), not in a
colour table of the level file (docs/04's `PALA`–`PALD` are empty). `world_intro_show` fades them in
from `h3+$21900`, and they stay for the level. [oracle: W1-1's copper list moves exactly these to
COLOR01–31, except COLOR06, which the game changes while it plays (colour cycling, inferred)]

- W1–W6's palettes start with black and carry the grey ramp `111 222 444 aaa` at 26–29.
- The bonus level's (`LBLP`) has neither: colour 0 is `b66` (inferred correct: same code path).
- The final level loads no picture (`world_load_lw`) and keeps W6's (inferred).

## Sheets

The bob and sprite sheets are plane-major bitmaps 320 pixels wide. Which rectangle is which frame
is in the executable's object and animation tables, for the mechanic tasks.

| Sheet | File | Size | Check |
|---|---|---|---|
| The frog | `HERO` d1_c04 | 320 × 384 × 4 | [oracle] equals RAM at h1 on the title |
| A world's bobs | `L1BO`…`L6BO`, `LBBO`, `LWBO` | 320 × 320 × 5 | [oracle] `L1BO` equals RAM at h3 at W1-1's start |
| Slot machine reels | `FRUC` d1_c12 | 320 × 384 × 4 | not reached |

The small digits in the top-left of each `LnBO` look like the status panel's (inferred).

## Tiles and masks

- `LnBM` is docs/06's format: 840 tiles (W1–W6, `LWBM`), **640** for the bonus level (`LBBM`,
  102 400 bytes). [python: all eight tilesets decode as `decode_tiles` does; oracle: `L1BM` equals RAM
  at h1 at W1-1's start]
- `LnMS` (no `LBMS`): 26 880 bytes = 840 × 32, read as one 16 × 16 one-plane mask per tile, a word a
  row (inferred; the game rewrites it as it loads it into `h3+$22f30`).

## Font

The hall-of-talent and credits text uses one 8 × 8 one-plane font in hunk 0: 40 glyphs side by
side, a 320-pixel strip of 8 rows (40 bytes a row) at h0+$378ca, in the order of the string at
h0+$322e. The printer (h0+$30fc) clears a cell for the string's first character (a space) and
copies glyph `k - 1` for the character at position `k`. [oracle: equals RAM] Other text (menus,
tables, the panel) is part of the pictures and bob sheets.

## ANIM (the end sequence)

`END1` (d3_c09, 112 frames) and `END2` (d3_c10, 25 frames) are standard IFF `FORM ANIM`: a first
`FORM ILBM` (`BMHD` 320 × 256 × 5, ByteRun1 `BODY` with each row's planes interleaved, `CMAP`,
`DPPS`, `CRNG`, `DPAN`, `CAMG`), then one `FORM ILBM` per frame with an `ANHD` (operation 5,
interleave 0, reltime 1) and a `DLTA` of operation 5 (byte vertical delta: per plane a pointer, per
byte column an op count; op 0 = run of a value, bit 7 = copy that many bytes, else skip rows).
Interleave 0 means each delta applies to the frame two back: the player draws into two buffers.
The game's player is `h0+$ab02` (open: a0, a1 = the two buffers, a2 = the file, a3 = its state,
d0 = bytes a row, d1 = rows) and `h0+$ab82` (draw the next frame; d0 ≠ 0 at the end), with the
buffers at `h3+$3d090` and `h3+$49890`. [oracle: on the routine oracle every frame of both equals
ours]

## Executables

Hunk executables (`0x3F3`) are kept as hunks: kind, memory flags, declared size, contents as the
file has them (unrelocated, so a long the loader relocates reads as an offset into its target
hunk), and their `HUNK_RELOC32` groups. Hunks declared but absent (`d1_c03`, `d3_c04`: hunks 2 and 3)
are kept as absent, sized from the header. Beside the five on disk, two more are `ATN!`-packed in
`d1_c03` hunk 0: the insert-disk prompt (h0+$35fee) and the Accessing-disk overlay (h0+$36a04).

## Tables

`Tables` holds what the code reads by offset, each field documented with its offset: `file_table`,
`disk_id_table`, `password_table` (28 × six ASCII digits and a handler), `start_resume_table`,
the starting high-score table (h0+$2f90, 10 × score and a long, inferred), `obj_flag_jumptab` (7 × 64
`bra.w`, kept as targets) and `obj_templates` (160 × $ae bytes). Each mechanic task adds its own the
same way.

## Kept as bytes

`GameData::raw` keeps, decompressed and by catalogue name: the sound (`MUSC`, `L1FX`, `LFFX`,
`LFMU`, `LBFX`, `L1MU`–`L6MU`, `LBMU`; T11's), the story intro's pictures and sound (`intro-gfx-1`–`4`
= d3_c05–c08, 204 800 bytes each, and `intro-sound` = d3_c03; the intro is a separate program and
whether the engine plays it is T20's call), the Accessing-disk picture (`d1_c02`, a bare `HUNK_DATA`
block the bootstrap hands the game in a0) and `INFT` (d4_c06), the hidden info screen's plain-text
`PAGE`.
