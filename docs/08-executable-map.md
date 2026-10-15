# 08 — Executable map

What each of Superfrog's five executables does, where the game sits in memory, how its main loop
and interrupt share the work, and how it moves from scene to scene. A map for the porting tasks,
not a full understanding. Names are in [`re/symbols.toml`](../re/symbols.toml); read the code with
[`RE-WORKBENCH.md`](RE-WORKBENCH.md). Objects are in [`09-object-system.md`](09-object-system.md).

Addresses are `hN+$off` in the game executable `d1_c03` unless another exe is named. Claims
marked **(oracle)** are checked by `engine/crates/sf-oracle/tests/exe_map_claims.rs`; the rest
come from reading the code, and **(inferred)** marks what neither reading nor the oracle settled.

## The five executables

| Chunk | Role | Loaded by |
|---|---|---|
| `d1_c01` | **Bootstrap** (15 KB). Sets a copper list, loads `d1_c03` (sector `$38`, `$162` sectors) and `d1_c02` (the "ACCESSING DISK" picture, sector `$18`) with its own 11-sector trackloader, decompresses both, relocates `d1_c03` and jumps to it with `a0` = the picture. | the crack's boot chain on disk 1 (docs/02) |
| `d1_c03` | **The game** (331 KB): title, options, all worlds, slot machine, bonus level, final level, end sequence. | `d1_c01` |
| `d3_c02` | **Disk-3 boot** (182 KB, mostly an embedded hires picture). Loads `d3_c04` and `d3_c03` (audio), relocates `d3_c04`, jumps. | disk 3's own bootblock |
| `d3_c04` | **Story intro** by Eric Schwartz (343 KB): about 16 skippable scenes using `d3_c05`–`d3_c08` and `d3_c03`. At the end it asks for disk 1, checks its ID and boots `d1_c01`. Its `DISK1:S1BO` strings are a dev DOS-load path. | `d3_c02` |
| `d3_c01` | Copy of the bootstrap with a 12-sector trackloader. Nothing loads it: **unused leftover** (inferred). | — |

Disk 3 is bootable: its bootblock is disk 1's without the Copylock call. Booting disk 3 plays the
intro, then the game; booting disk 1 skips the intro. `d1_c03` never loads `d3_c02`/`d3_c04`. For the
Rust engine the intro is a separate program reached only from disk 3 (decision for T20/T21).

## Memory map (A500, 512K chip + 512K slow) **(oracle)**

| Block | RAM | What |
|---|---|---|
| `d1_c03` hunk 0 (code+data, `$4834c`) | `$C00FA0` | the game; `h0+$x` = `$C00FA0 + x` |
| hunk 1 (BSS, `$352a0`) | `$C492EC` | world tiles `LnBM` at `h1+$0`; end animation `h1+$0`/`h1+$13880` |
| hunk 2 (`$73fc`) | chip `$400` | copper lists, display (e.g. `h2+$6fd8`) |
| hunk 3 (`$77168`) | chip `$77FC` | bobs `LnBO` at `h3+$0`; map/mask file buffer `h3+$22f30`; sound `LnFX` at `h3+$64c70` |
| music `LnMU` | chip `$73EF6` (W1) | a "P41A" module (The Player 4.1A); the replayer turns its sample table into absolute pointers |
| stack | slow RAM top (`$C7FFxx`) | supervisor stack |

Hunks 2 and 3 are declared in `d1_c03`'s header but absent from the file: the game's loader still
gives them room, in chip RAM, which is why the tools add them as empty BSS (RE-WORKBENCH).

## Main loop and interrupt

Only the level-3 interrupt is the game's: `hw_takeover` (`h0+$11b6a`) installs `vbl_handler`
(`h0+$11bf8`) at vector `$6C` and enables VERTB and COPER only **(oracle: vector)**. The keyboard,
audio and CIA-B interrupts stay off; the keyboard is polled.

**In a level** `level_frame_loop` (`h0+$aa6`) sets `logic_running` (`$9d6`) and waits until
`vbl_count` (`$9d8`) reaches 2, clears it and runs one **25 Hz pass**: the spud (`$80f0`), every
object (`obj_run_all` `$1b324`), hitboxes and collisions (`$d2f6`, `$d0cc`), bob drawing
(`$19b42`, `$19120`), and so on. The pass runs every second frame.

`vbl_handler` runs **every frame (50 Hz)**. With `logic_running` set it counts `vbl_count`, then
runs the frog and the level: input and animation (`frog_input_anim` `$fe34`), map-marker reactions
(`frog_tiles` `$e2e2`), the other frog routines (`$11a32 $11990 $11056 $10222 $113be`), scrolling,
the level clock (`$11d2c`, a second per 50 VBLs), the edge spawner (`map_spawn_scan_edges`
`$17810`), music and streamed samples (`$147e4`, `$13424`, `$1668c`), the keyboard every second VBL
(`kbd_poll` `$15ff2`), the progress-map animation and `display_timer`. The copper-interrupt
half (`$1693e`) runs blits queued for it. **(oracle)**: walking, the frog's x changes on
consecutive frames; objects never move on two consecutive frames while the main loop is on time.

So the frog, camera and clock are 50 Hz; enemies, items and their drawing are 25 Hz.

**Every other screen** (title, menu, options, tables, tally, slot machine, game over, end
sequence) paces itself on `wait_frame_raster` (`$73d0`: VPOS `$ff`, then `$2b`), one pass per
frame. The hall of talent and credits wait for lines `$c4` then `$c5` (`$221e`). The progress
map, world intro and end-of-world pictures are timed by `display_timer`, which the VBL handler
counts down while loading runs.

## Lag **(oracle)**

A lag event is a frame boundary where the pass is due (`vbl_count` reaches 2) while the main loop
is still busy with the previous one.

| Scene | Frames | Lag events |
|---|---|---|
| Title, menu, tables, hall, credits | every boundary | none: always parked in the raster wait |
| W1-1 standing still (probe run, not in the test) | 1 750 | none |
| W1-1, 3 000 frames walking and jumping right | 2 311 in level | 71 (busiest: 15 live slots) |
| Attract-mode W1-1 demo | 721 in level | 32 |

A late pass means the next one starts late, and the VBL handler's frog work runs in the middle of
it. The owner chose (2026-10-11) to feed the original's pass schedule to the engine in conformance
runs; normal play never lags (FINDINGS).

What conformance feeds is `Pacing::Late { writes, line, hpos, phase, step }`: the frame's Paula
writes so far (`writes`, the last at beam `line`, slot `hpos`), which of `level_frame_loop`'s 13
pass calls (`$abe`..`$b06`, from `spud_update`) the main program is in (`phase`), and how many slots
into that call's loop (`step`). The engine ends its frame there and runs the rest of the pass after
the next interrupt. The picture depends on where on the beam the work happens (a bob drawn into the
shown buffer, a tile redrawn below the beam), so the engine keeps a beam clock fed by fitted per-
routine CPU costs (`sf_core::level::beam`, T36). In conformance the clock is anchored at the
original's Paula writes and blit starts, read off the oracle (`Game::anchor_writes`,
`Game::anchor_blits`; owner, 2026-10-13); normal play uses the cost model alone.

### Per level, completed (T181)

What the shipped game does not reproduce (DESIGN §2.1: normal play never lags; accepted, owner
2026-10-15), measured on each completion case by `sf-conform run`'s "the original late at N
frame(s)": frames of play from the level's first frame through its exit fade and tally (about 185
frames, never late), the original's loads not counted. A late frame is one whose vertical blank
came with the pass still running (`Pacing::Late`).

| Level | Case | Frames | Late | Share |
|---|---|---|---|---|
| W1-1 | `complete-w1-1` | 4 658 | 683 | 15% |
| W1-2 | `complete-w1-2` | 5 367 | 617 | 11% |
| W1-3 | `complete-w1-3` | 7 913 | 571 | 7% |
| W1-4 (to 4054, short of the exit) | `complete-w1-4` | 4 054 | 457 | 11% |
| W2-1 | `w2-1-complete` | 2 709 | 712 | 26% |
| W4-1 | `complete-w4-1` | 2 330 | 429 | 18% |
| W4-2 | `complete-w4-2` | 9 421 | 3 035 | 32% |
| W4-3 | `complete-w4-3` | 8 670 | 2 712 | 31% |
| W4-4 | `complete-w4-4` | 24 384 | 5 641 | 23% |
| W5-1 | `enemies-w5-1-complete` | 2 439 | 1 113 | 46% |
| W5-2 | `enemies-w5-2-complete` | 6 279 | 1 803 | 29% |
| W5-3 | `enemies-w5-3-complete` | 7 048 | 2 130 | 30% |
| W5-4 | `enemies-w5-4-complete` | 8 378 | 2 185 | 26% |
| W6-1 | `enemies-w6-1-complete` | 6 752 | 2 066 | 31% |
| W6-3 (second map) | `enemies-w6-3-alt-complete` | 12 741 | 2 874 | 23% |

W1 runs late on 7–15% of its frames, W2-1 on a quarter, W4–W6 on a quarter to a third, W5-1 on
nearly half; W4-3 from 3300 and W4-2 from 8499 run a pass longer than a frame for 200+ frames in a row. Levels without
a completion case yet (W2-2..4, W3, W6-2, W6-4, the bonus level) are their completion tasks' to add.

## Input

There is no input routine; each piece of code reads the hardware itself.
- Joystick (port 2): `JOY1DAT` (`$dff00c`) and fire on CIA-A PRA bit 7. In a level the frog's
  VBL routines read them every frame (`$fe42`, `$11094`, `$11210`, `$1199a`, `$11a4c`); the spud
  reads fire in the pass (`$80f0`). Up is jump.
- Mouse buttons (port 1): CIA-A PRA bit 6 and `POTGOR` bit 10, in menus (and a hidden info screen).
- Keyboard: `kbd_poll` reads CIA-A SDR every second VBL into `key_down` (`$16078`) / `key_up`.
  P (`$19`) pauses (`pause` `$7442`), ESC (`$45`) aborts.

## RNG **(oracle)**

`rng_next` (`$16084`) is a 64-bit add-and-shift generator on `rng_state` (`$160d4`, 8 bytes);
`rng_range` (`$160c0`) returns 0..d0. It is seeded **once**, by `rng_seed_from_cia` (`$7722`), from
a long read at `$bfd802`: CIA-B's time-of-day counter, which counts horizontal syncs since power-on.
The seed therefore depends on how long loading and the crack-intro click took: clicking 100 frames
later shifts the whole run by 100 frames, and the seed alone differs. The RNG is used on the title,
by the frog's idle-sound chance (VBL, `$1124c`) and by about 40 object routines (pass), so its call
order is fixed only while there is no lag. The owner chose (2026-10-11) to make the seed part of
the start: transplants copy `rng_state`, power-on conformance runs take the oracle's seed.

## Files, disks and level composition

`file_table` (`$cbfe`): 95 entries of 6 bytes, start sector, sector count, disk index 0..3. Every
entry is a carved chunk (docs/05). `load_file` (`$af84`, `a0` = the original DOS name, a dev
leftover; `a2` = the entry; `a1` = destination) loads and decompresses it.

Per world *n* the files are named `Ln…`:

| File | Content | Where (W1) |
|---|---|---|
| `LnFX` | sound effects (W1–2 share `d1_c10`, W3–6 `d2_c01`) | `h3+$64c70` |
| `LnMU` | music module | `$73EF6` |
| `LnLP` | world loading picture, shown by `world_intro_show` | screen |
| `LnBM` | tiles, 840 × 16×16 × 5 planes | `h1+$0` |
| `LnBO` | bobs (sprites) | `h3+$0` |
| `LnMS` | per-tile masks (inferred: 840 × 32 bytes) | via `h3+$22f30`, transformed |
| `LnMA1`–`4` | the four T7MP maps | via `h3+$22f30` |
| `LnET` | end-of-world picture | screen |

**(oracle)** at the first frame of W1-1, `L1FX`, `L1BM` and `L1BO` are in RAM byte for byte, and
the map is `L1MA1`'s BODY with each cell rewritten as `tile << 6 | flags`, rows `$260` bytes apart,
starting one row below `map_rows` (75% of cells equal; the rest are animated tiles and object
markers already consumed). **A level is its world's FX/MU/BM/BO/MS files plus its T7MP; object
placement lives in the T7MP BODY** (docs/09). So a `.sflevel` (DESIGN §2.10) needs only `t7mp`.

Other files: `HERO` `d1_c04` (frog graphics), `MUSC` `d1_c05`, `MPIC`/`HPIC` `d1_c06/07` (title
pictures), `AWA1/2` `d1_c08/09` (tables), `FRUT/FRUC/FRUM` (slot machine, on disks 1 and 2),
`LFFX/LFMU`, `LB*` (bonus level, `d2_c37`–`c42`, `LBLP` = `d2_c39`), `LW*` (final level, `d4_c10`–`c13`),
`INFT` `d4_c06` (hidden info screen), `END1/END2` `d3_c09/c10` (IFF ANIM end sequence).

### Disk swaps

Each disk's last 4 bytes (offset `$dbffc`) are its ID: `2222`, `3333`, `4444`, `5555` for disks
1–4. `disk_id_table` (`$b11e`) gives the ID each file-table disk index needs. `load_file` checks
the cached ID per drive (`drive_disk_cache`). On a miss `disk_request` (`$ade4`) reads the ID of
every connected drive (so DF1–DF3 work with no prompt), else runs the embedded insert-disk prompt
(`$35fee`, an ATN! hunk exe, "INSERT DISK n AND PRESS FIRE", inferred) and rescans. The disk-change
line is only used to detect "no disk". Disks 2 and 4 share sector numbers; only the disk index tells
them apart.

Swaps in a one-drive game (code-derived): disk 1 for the title, W1 and W2; disk 4 for L2ET; disk 2
for W3–W5, the bonus level and W6's FX/MU; disk 4 for W6's graphics and maps and the final level;
disk 3 for the end sequence; disk 1 again for the title.

## Scene flow

```
boot (init_and_script $26: take over, Accessing-disk overlay, probe drives, load HERO, seed RNG)
top_loop $5c:
  title_screen $109e ── fire ──> menu: START GAME / OPTIONS (lives, game mode, hi-score save, level code)
     │ 400 frames idle: attract_mode = -1
     └─> award_pages $1e98 (features, bonus tables) ─> hiscore_and_credits $20e0 (hall, credits)
         ─> attract demo (W1-1, recorded input) ─> top_loop
  game_start_at $b66 (password -> start world/level via start_resume_table)
  per world 1..6:  world_load_wN (world_intro_show: LnLP "LOADING ... PLEASE WAIT")
     per level 1..4:  level_load (progress_map_show) ─> level_init_wN, level_start
                      level_frame_loop until level_state >= 0 ─> level_exit_fade, game_over_check
                      ─> level_tally $4466 (points, or slot_machine $49dc)
     end_world_pic_wN (LnET)
     after W5: bonus level (world 7: LB files) ─> L6ET
  after W6 L4: final level (LW files, 10:00 clock, the witch)
  end_sequence $a6e4 (END1/END2 from disk 3) ─> top_loop
  game over (game_over_seq $74d0) ─> top_loop
```

### Entering a level (T14, `sf_core::level::load`)

From a level's loader to its first frame (`level_frame_loop`), every step proven against the
original for all 29 levels:

- `map_unpack` (`$1647a`): the map file lands in `playfield_buffer_1`; its `BODY` goes to
  `level_map + $4c0` (row 0), 300 cells a row plus 4 of padding, each cell rotated right 10
  (`tile | marker << 10` becomes `tile << 6 | marker`). `map_palette` (`$1622a`) copies 32 colours
  from `$44` past the `PALA` tag to `level_palette` (h1+$31de0), the fade's target.
- `map_find_start` (`$162f4`): the first start marker (`$35`) gives the camera's start: its cell's
  pixel less (`$9b`, `$ae`), at least 0, `$40` higher in the final level; (`$10`, `$160`) without
  one. The bonus level starts at (0, 0). `level_start_point` (`$164fa`) puts the frog `$90`, `$70`
  from it and keeps the camera within (`$10`, `$20`)..(`width − $168`, `height − $f0`).
- `map_count_coins` (`$16374`): `coins_left_for_exit` = the coins (marker 8), at most 99.
- `level_start` (`$6952`) runs the frog once, fades the palette in (over by the first frame), slides
  the panel in over `$18` frames (`$8510`), draws its digits, starts the drop-in (`$6ad2`) and the
  music if it is off.
- The final level is world 6 with `final_level_flag` set; its files (`LW*`) replace W6's tiles.

`level_state` (`$a06`): negative while playing; the exit sets 1, the bonus level's end 2, the
witch's defeat `$64`, a death `$190` (then the level restarts). Passwords: `password_table`
(`$24dc`), 28 entries of 6 digits and a handler; entries 0–23 start W1L1..W6L4, 24–27 the bonus
level. `password_check` (`$2432`) runs on leaving the entry and in `game_start_at`.
