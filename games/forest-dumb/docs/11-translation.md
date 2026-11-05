# 11 — Translation

Literal translation of the original's routines by Muse, judged on the routine oracle (plan
`forest-dumb-level-1`, DESIGN §4.3). This note holds the inventory (T34); the service (T33), the
generators (T35 to T39, T53, T54, T56) and the batch's table (T40) add their sections.

## Inventory: `game_frame`'s call tree and the edge redraws

Measured 2026-11-05 (T34) by `tools/call_tree.py` from the program `fd-tools extract` writes; the result,
one entry a routine, is [`re/translate.toml`](../re/translate.toml). Rerun with
`venv/bin/python games/forest-dumb/tools/call_tree.py`, which keeps each routine's classification by
address; `--check` compares the toml with the program, `--summary` prints the tables below.

- **The roots are `game_frame` and `redraw_edge_a..d`.** The edge redraws run from `main_loop`, not
  `game_frame`, but write `collision_map`, which is play state (T00), and T09's goal named them; the owner
  added them as roots on 2026-11-05, with T55 and T56 to take them.
- **87 routines, 4 041 instructions of their own.** `game_frame`'s tree is 73 of them and 3 105 (3 026
  with callees counted once; some routines share tails, so own flows overlap by 79); the edge redraws add
  14 and 936, sharing nothing with it. The spike's "about 8 300" counted address ranges, data included,
  and did not follow calls written as names.
- **No call or jump the scan could not follow.** No `jsr (an)`, no computed `jmp`, no Kickstart call
  is reached from the roots. The `[[unfollowed]]` table is empty.
- **The listing's linear sweep is not usable for this.** After data inside the code it decodes out of
  step: 7 of the tree's call targets (`h0+$2c7e`, `h0+$3698`, `h0+$5820`, `h0+$94ba` among them) fall
  inside an instruction there. `call_tree.py` decodes from each address the flow reaches instead.
- Sizes agree with T00's for every routine T00 sized (`char_probe` 199, `char_step` 107, `h0+$3ea8` 93,
  `char_hit_check` 131, `h0+$26ec` 153, `camera_update` 48, `char_sprite_copy` 73).

Each entry has `own`, `instructions` (with callees) and `unit`: what one translation call sees, its
callees less `split_at` and what only they reach. `split_at` is chosen greedily, the callee whose removal
drops the unit most first, until the unit is 180 or less. Sixteen routines are split.

**Four routines are `by_hand`:** their own flow is over 180 with no callee left to split at, so T33
answers `too-big` and the porting task's builder translates them.

| Routine | Own | Port task | What it reads as |
|---|---|---|---|
| `char_probe` | 199 | T08 | the character's collision probe on `collision_map` |
| `h0_5e44` | 200 | T52 | tile copies restoring the screen behind creatures |
| `h0_62b6` | 224 | T25 | animated map cells, codes `$20`-`$28`; also writes `collision_map` |
| `redraw_edge_c` | 181 | T55 | one of the four edge redraws |

`h0_62b6` calls `h0_3ab4` (a blitter wait), which is split off; that changes nothing, since 224 is its
own flow.

### Subsystems and tasks

Classified from what each routine reads and writes, and from T00's note and the spike: a **reading**,
noted per entry where it is not measured. The subsystem picks the generator batch, by the kind of state
a generator has to build (blitter and screen for `drawing`). The port task is the task that owns the
behaviour, so drawing of collectibles, creatures and animated cells goes to T22, T52 and T25, which list
`render/objects.rs` in their files, not to T10.

- `sound`: `h0_2648` and `h0_3adc`, which only start a sample on audio channel 2. Out of scope, no
  tasks. Four routines also start a sound inline: `h0_21c0` and `char_hit_check` on channel 1, `h0_3ea8` on
  2, `h0_26ec` on 3. How a port treats those writes, with sound out of scope (§8), is its porting task's call.
- `rest`: `game_frame` itself goes to T08, the first task to edit `Game::step`; each later porting task
  adds its calls. `countdown_step`, `play_keys`, `esc_key`, `lives_minus_one` go to T23, `help_key` to T26.
- `char_hit_check` and its box test `h0_3dd8` are `objects` and T24: they read the five creature slots.
- `h0_24ce` (map codes `$10`-`$13` under the character) and `h0_21c0` (blits a tile at the cell it
  marked) read as a mechanic: T25. T21 confirms what those codes are.

Object routines reached only through dispatch tables are T21's and T49's, not this scan's (§4.3).
The edge redraws and their 10 helpers are `camera`, generators T56, port T55; T09 keeps the edge flags
that trigger them (`h0_4e9a` to `h0_5346`). Three helpers read as other subsystems' (`h0_681c`, `h0_6852`
touch the object slots; `h0_6882` adds to `h0_62b6`'s animated-cell list), which T55 checks. The rest of
`main_loop`, about 110 routines, is outside this plan's translation.

### Each task's share

Own instructions summed per task, `sound` left out; DESIGN §4.3's limit is 1 500 instructions or 15
routines. The first count put T08 at 21 routines, T24 at 17, T35 at 20 and T38 at 21. The owner added
T51 to T54 on 2026-11-05 to take the excess, and T55, T56 for the edge redraws, so every task now fits.

| Porting task | Routines | Own instructions | By hand |
|---|---|---|---|
| T08 | 15 | 741 | `char_probe` |
| T09 | 10 | 191 | |
| T10 | 7 | 237 | |
| T22 | 8 | 365 | |
| T23 | 4 | 55 | |
| T24 | 13 | 503 | |
| T25 | 3 | 446 | `h0_62b6` |
| T26 | 1 | 10 | |
| T51 | 6 | 112 | |
| T52 | 4 | 421 | `h0_5e44` |
| T55 | 14 | 936 | `redraw_edge_c` |

| Generator task | Routines | Own instructions | By hand |
|---|---|---|---|
| T35 | 14 | 705 | `char_probe` |
| T36 | 10 | 191 | |
| T37 | 14 | 1 070 | `h0_5e44`, `h0_62b6` |
| T38 | 10 | 548 | |
| T39 | 6 | 101 | |
| T53 | 6 | 112 | |
| T54 | 11 | 354 | |
| T56 | 14 | 936 | `redraw_edge_c` |

A `by_hand` routine still needs its generator: its port is proven on the rig like any other.
