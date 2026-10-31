# 07 — Executable map

The shared map of `FOREST_DUMB_FOREVER`, so a porting task starts here instead of finding the main loop
again. Names are in [`re/symbols.toml`](../re/symbols.toml). The listing is regenerated, never committed:

```
venv/bin/python games/forest-dumb/tools/re_listing.py --xrefs    # writes build/re/forest_dumb_forever.lst
```

`fd-tools`' `tests/exe_map_claims.rs` checks what is marked (test) on the real file. What is marked
(oracle) was measured on the emulator with T00's boot script. Everything else is read from the
code and has not been run: treat a name as a reading, not a fact. Ghidra was not needed for this map.

## Addressing

`h4+$195c9` is byte `$195c9` of hunk 4. Listing addresses are different: hunks back to back from 0.

| Hunk | Type in the file | Memory | Size | Holds | Base on the oracle (oracle) |
|---|---|---|---|---|---|
| 0 | code | any | `$5be68` | the game; `load_buffer` at `$437c8` inside it, also map and tile data | `$c077d8` (slow) |
| 1 | data | chip | `$1280` | copper lists, relocated | `$10178` |
| 2 | data | chip | `$1830` | data, no relocations | `$11400` |
| 3 | BSS | chip | `$3aa28` | screens, tiles, the song at `$f000` | `$12c38` |
| 4 | data | chip | `$1c94c` | graphics and **code**: the music player and the load lists | `$4d668` |

Hunk 4's header says data, yet the program jumps into it, so `re_listing.py` decodes it as code. The
bases were the same with the crack intro's click at 1300 and at 1400. A base can be read back from
the running game: the long at `h0+$38` is hunk 4's base plus `$19f58`, because the skipped checksum code
holds that relocated reference.

## Entry and start-up

`entry` (`h0+$0`, test) saves the registers, calls `prompt_disk_2` and `read_marker`
([03-boot-chain.md](03-boot-chain.md)), waits 76 frames, and sets a copper list. At `h0+$30` a `bra.b`
skips a checksum of `h4+$199be..$19f0a` that would flash the background for ever on a mismatch. Which
of the original or the crack skips it cannot be told without an uncracked image. It then saves the OS
vectors and takes the machine: DMA `$87f0`, level 3 at `level3_handler`, level 2 at `level2_handler`.

## The main loop

Play runs inside the interrupt, not in the foreground. There are two layers:

- `level3_handler` (`h0+$1eda`): on the copper interrupt (INTENA `$c018`, it clears `$4010`), which a
  level's copper list raises, it adds one to `irq_count`, runs `game_frame` (`h0+$1fec`) when
  `game_running` is set, and sets `frame_flag`. Once a frame is the reading; not measured in play.
- `main_loop` (`h0+$252`): the foreground. Every 32 interrupts it checks for level end and game over,
  and on `frame_flag` it runs whichever of `redraw_edge_a..d` the flags at `h0+$5196..$519c` ask for.

Around them, `session_loop` (`h0+$158`) runs one session: `load_list_title`, `title_screen`, then
`level_init` and `main_loop`. `h0+$1aa` is game over (`game_over_screen`, then the high scores at
`h0+$89d0`). `next_world` advances `world_index`; after world 6 comes `end_screen`.

`load_world` picks a world's list. A list hands the machine to the OS (`system_on`), loads, and takes
it back (`system_off`).

## Custom-chip writes

Written by absolute address, 400 references by the listing's sweep: 376 in hunk 0, 24 in hunk 4 (the
player). A plain scan for a word-aligned long `$00dff0xx` or `$00dff1xx` finds 403, 379 and 24 (test);
the three extra sit where the sweep is out of step. Reads are included, and `$dff000(a6)`-relative
accesses are not.

| Registers | References | Used for |
|---|---|---|
| `$040..$066` | 236 | blitter: the bobs and the map's edges |
| `$080`, `$088` | 27 | starting a copper list: every screen change |
| `$096`, `$09a`, `$09c` | 22 | DMA and interrupts |
| `$0a0..$0d8` | 52 | Paula: the player, the sound effects, the handler's silencing |
| `$002`..`$006` | 50 | beam and blitter-busy waits |
| `$00c`, `$00e`, `$01c`, `$01e` | 4 | joystick, collision, interrupt state |
| `$000` | 7 | the base address in `lea` and friends |
| `$180` | 2 | the checksum's flash, which the skip leaves dead |

## dos.library and the file table

Four routines open files, and each opens `dos.library` itself by `OpenLibrary` and closes it after:

| Routine | Opens | Does |
|---|---|---|
| `load_file` `h0+$8ec4` | `load_name` | reads up to `$6ffff` bytes to `load_buffer`; `ESL!` goes to `unpack_esl`, else it is copied to `load_target` |
| `load_file_raw` `h0+$8f58` | `load_name` | reads straight to `load_target`; only `_BALLA` |
| `read_marker` `h0+$8fd8` | `df0:!EAGLE_SOFT!` | reads the marker, compares nothing |
| `save_scores_unreferenced` `h0+$92d8` | `DF0:ST` | writes `h0+$84b6`; see below |

Every failed open goes round the insert-disk prompt and tries again. A load
list copies a name into `load_name`, sets `load_target` and calls. There are 84 such loads.

Every file the game opens, spelled as on the disk. The program spells many in capitals; AmigaDOS
ignores case. All are on disk 2, and it opens nothing on disk 1 (test: this table against the
inventory, both ways).

| File | Disk | When |
|---|---|---|
| `!EAGLE_SOFT!` | 2 | at start, `read_marker`, before the first screen |
| `_NAPIS` | 2 | every title (`load_list_title`), each session |
| `_CZOLOWKA` | 2 | every title (`load_list_title`), each session |
| `_OPCJE` | 2 | every title (`load_list_title`), each session |
| `_BALLA` | 2 | every title and the end screen, read unpacked by `load_file_raw` |
| `_END` | 2 | the end screen (`load_list_end`), after world 6 |
| `StworkiNew.rawm` | 2 | every world |
| `StworkiNew.mask` | 2 | every world |
| `_ELEMENTY.RAWM` | 2 | every world |
| `_ELEMENTY.MASK` | 2 | every world |
| `_Gen1-1` | 2 | world 1 |
| `_Gen1-1.Mapa` | 2 | world 1 |
| `_Mapa1-1` | 2 | world 1 |
| `_Mapa1-1.Mapa` | 2 | world 1 |
| `_MapaBobow1` | 2 | world 1 |
| `_Gen2-1` | 2 | world 2 |
| `_Gen2-1.Mapa` | 2 | world 2 |
| `_Mapa2-1` | 2 | world 2 |
| `_Mapa2-1.Mapa` | 2 | world 2 |
| `_MapaBobow2` | 2 | world 2 |
| `_Gen3-1` | 2 | world 3 |
| `_Gen3-1.Mapa` | 2 | world 3 |
| `_Mapa3-1` | 2 | world 3 |
| `_Mapa3-1.Mapa` | 2 | world 3 |
| `_MapaBobow3` | 2 | world 3 |
| `_Gen4-1` | 2 | world 4 |
| `_Gen4-1.Mapa` | 2 | world 4 |
| `_Mapa4-1` | 2 | world 4 |
| `_Mapa4-1.Mapa` | 2 | world 4 |
| `_MapaBobow4` | 2 | world 4 |
| `_Gen5-1` | 2 | world 5 |
| `_Gen5-1.Mapa` | 2 | world 5 |
| `_Mapa5-1` | 2 | world 5 |
| `_Mapa5-1.Mapa` | 2 | world 5 |
| `_MapaBobow5` | 2 | world 5 |
| `_Gen6-1` | 2 | world 6 |
| `_Gen6-1.Mapa` | 2 | world 6 |
| `_Mapa6-1` | 2 | world 6 |
| `_Mapa6-1.Mapa` | 2 | world 6 |
| `_MapaBobow6` | 2 | world 6 |
| `_ZNIKAJKA1` | 2 | worlds 1 and 2 |
| `_WYBIJACZKA` | 2 | worlds 1 and 2 |
| `_Plomien1` | 2 | worlds 1 and 2 |
| `_ZNIKAJKA2` | 2 | worlds 3 and 4 |
| `_WYBIJACZKA2` | 2 | worlds 3 and 4 |
| `_Plomien2` | 2 | worlds 3 and 4 |
| `_ZNIKAJKA3` | 2 | worlds 5 and 6 |
| `_WYBIJACZKA3` | 2 | worlds 5 and 6 |
| `_Plomien3` | 2 | worlds 5 and 6 |

### What T03 left open

`DF0:ST` and its routine are unreachable as this file stands: no relocation in any hunk points at
`h0+$92d8..$93b4` (test), every branch into it comes from inside it, and zero bytes come before it, so
nothing falls through. `HD_ID` at `h0+$91f8` has no relocation either (test) and no pc-relative use in
the listing. So the game never saves high scores to disk, and the hang T03 feared cannot occur.
Whether the original called it needs an uncracked image.

## The title screen's code path

`load_list_title` (from `session_loop`) loads the pictures and the song, then `title_screen`
(`h0+$6e06`):

1. Clears the menu state and points the copper at `h1+$93c`.
2. Sets `mt_songpos` to 0 and calls `mt_init`.
3. Repeats `title_pass` (`h0+$6e6e`): `wait_line_1`, `title_read_joy`, `h0+$71e6` (the menu and the
   code digits), `title_fire`, `h0+$6fb4` and `h0+$6eea` (draw the rows and digits), then `mt_music`.
4. Leaves on `title_start_flag` (START) or on `title_quit_flag` or last key byte `$75` (KONIEC or Esc),
   with `mt_end` and `game_running` set. The OS's keyboard handler is the one in place here, so the Esc
   route looks dead; not run. `title_cursor` is the row, 0 START, 1 KODY, 2 KONIEC.

The OS's own interrupts are on through the title, because `system_on` ran for the load and nothing
took the machine back. So `irq_count` stands still (oracle) and the pace comes from the raster wait alone. One
pass takes one frame (oracle: see the counter below).

## The first case's variable: `mt_counter`, `h4+$195c9`

The player's tick counter, one byte, `$66c31` on the oracle. `mt_music` adds one and clears it on
reaching `mt_speed` (test: the instruction at `h4+$1852e` names this byte).

Evidence (oracle, 2026-10-31; T00's script: Esc 800-810, left mouse 1300-1310, `insert_disk(0,1)` at
3600, fire 3700-3710, then nothing; frame `n` means after `n` steps):

| | |
|---|---|
| Sync | frame 4922: value 0, the first frame after `mt_init` |
| Values | `(n - 4922) % 6` for every frame 4922 to 20 000, the frame cap: 15 079 frames, no exception |
| Range | 0 to 5, because `mt_speed` is 6 throughout |
| Changes | every frame |
| Before sync | 0 from frame 4700 at least |
| Repeat | two runs identical in this and in `mt_speed`, `mt_songpos`, `mt_patternpos` for all 20 000 frames |
| Click at 1400 | still sync at 4922, same values |

The song plays all its positions, 0 to 26, and loops inside the window, so a later speed command
would have shown. The whole title's RAM over 3 000 frames changes only in the player's variables
(`$66b00..$66c3d`, chip) and the stack: there is no title state besides the music.

The four criteria of DESIGN §2.8:

1. Deterministic: the two runs above.
2. Visible at the idle title with no input after the sync point: yes.
3. Computable from the state at the sync point: value 0, then plus one modulo 6.
4. No table at all.

Near misses: `irq_count` stands still at the title. `mt_patternpos` and `mt_songpos` need the song's
pattern-break commands, so a model must read `_BALLA` from the disk: the next step up, not the first.
`title_cursor` and the rest never change without input.

What it proves is small: the two sides can be compared, and one pass is one frame. It says nothing
about the picture or sound. The engine side (T08): `TitleScene::variable() -> u8`, starting at 0 at
the sync frame. T08's and T09's docs need no change, with one catch: T09's sync frame is this 4922,
not the first title frame 4921 in [03-boot-chain.md](03-boot-chain.md). The value is 0 at both, so a
counter started at 4921 runs one frame ahead.

## Memory layout

From the load lists' targets and the routines above (not run, but for the bases):

| Where | Holds |
|---|---|
| `h0+$437c8` | `load_buffer`, packed files on their way |
| `h0+$bda0`, `h0+$16318`, `h0+$28f18`, `h0+$cb38` | `_GenN-1.Mapa`, `_MapaN-1`, `_MapaN-1.Mapa`, `_MapaBobowN` |
| `h3+$0`, `h3+$4920`, `h3+$c300` | `_NAPIS` or `_END`, `_CZOLOWKA`, `_OPCJE` |
| `h3+$f000` | `song`, `_BALLA` |
| `h3+$13540`, `h3+$20c18`, `h3+$2bb18` | `_GenN-1`, `StworkiNew.rawm`, `StworkiNew.mask` |
| `h4+$0`, `h4+$1900` | `_ELEMENTY.RAWM`, `_ELEMENTY.MASK` |
| `h4+$9618`, `h4+$9898`, `h4+$9b18`, `h4+$a298` | `_WYBIJACZKA*` (two copies), `_ZNIKAJKA*`, `_Plomien*` |
| `h4+$18480..$19f6e` | the player and its variables |
| `h4+$19f6e..` | the load lists |

`load_file`'s read length of `$6ffff` is longer than `h0` from `$437c8`, which only holds because the
files are shorter.
