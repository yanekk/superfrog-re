# The title, its codes and the high scores

What the title does, read from the listing (`tools/re_listing.py`, names in `re/symbols.toml`) and
measured on the oracle on 2026-11-20 (plan `forest-dumb-title`, T01). The code path is in
[07-executable-map.md](07-executable-map.md), "The title screen's code path"; this note is the
behaviour.

Every measurement starts from boot-to-level1's frame 4990, the idle title (as `world-2-start`), and
plays a committed script from `engine/conform/scripts/`, its frames counted from 4990. A step is named
by the frame it starts at, as the scripts name it: "step 5020" is the script's line `30:`.
`fd-conform/tests/title_codes.rs` replays each and holds what is written here (full, about 30 s).

## The rows

`title_cursor` (`h0+$6df2`, long) is the row: 0 START, 1 KODY, 2 KONIEC. `title_move_rows`
(`h0+$793a`) reads `title_read_joy`'s flags: Down or Up held only sets `title_down_pending` or
`title_up_pending`, and the row moves when the stick is let go, one row a release, however long it was
held. Down from KONIEC wraps to START, Up from START to KONIEC.

Measured (`title-rows.txt`): Down released in step 5020 gives row 1 at that step's end; Down held 30
frames moves nothing until its release, row 2 in step 5060; then 2 to 0 (5072), Up 0 to 2 (5100), 2 to
1 (5120), 1 to 0 (5140). Left and right are read (`title_joy_left`, `title_joy_right`) but only
unreferenced code uses them (below).

Fire (CIA-A bit 7, polled by `title_fire`, not the joystick register): on START it sets
`title_start_flag` while held, and `title_screen` returns at that pass's end. On KODY or KONIEC the
press sets `title_fire_down` and the release sets `title_selected` (`h0+$6df6`); while it is set the
rows do not move and fire is ignored, and `title_menu` (`h0+$71e6`) runs the row's action instead.

## KODY

Fire released on KODY starts the digit entry (`title_code_entry`, `h0+$7438`): the first pass clears
`code_digit_index` and sets `code_entry_on`. Digits come from the keyboard, never the joystick:
`code_read_key` takes `last_key_byte`, notes a press (bit 0 set) and decodes the next release byte, so
a digit counts when its key is let go. Keys 1 to 9 give 1 to 9 and key 0 gives 0 (raw release bytes
`$fc` to `$ec`, and `$ea`); any other key is ignored. Keypad keys are not digits.

Four digits, one byte each, go to `code_digits` (`h0+$742e`, long, first digit highest) in order,
`code_digit_index` 0 to 3. The fourth ends the entry: `title_selected` clears and the rows move again,
the cursor still on KODY. A digit cannot be changed except by typing four more. There is no clear
key and no Esc out of it (Esc quits, below).

`code_digits` holds `00 05 05 06` in the program file, and each world's load list writes its world's
code there (`h4` load lists, `+$62a`): worlds 1 and 2 `0556`, 3 and 4 `2424`, 5 and 6 `7531`. So after
a game over the KODY box shows the last code reached, and START with no typing goes back to it.

Measured (`title-code-w3.txt`): key 2 let go in step 5056 (its release byte in `last_key_byte` that
step) puts `02` in the first byte in step 5057; the fourth key let go in step 5092 ends the entry in
step 5093, `code_digits` `02040204`, the cursor on KODY.

## The codes

START runs `title_code_world` (`h0+$777e`) from `session_loop`, in the same step as `lives_reset` and
`load_world`; it compares `code_digits` as one long:

| code | world_index | cheat_flag | other effect | measured |
|---|---|---|---|---|
| 0556 | 0 (world 1) | cleared | none; the code the title starts with | `title-code-w1`, load_world in step 5128 |
| 2424 | 2 (world 3) | cleared | none | `title-code-w3`, step 5128 |
| 7531 | 4 (world 5) | cleared | none | `title-code-w5`, step 5128 |
| 2400 | 0 (world 1) | set | HELP ends a level, no life is lost (`cheat_flag`, docs/09) | `title-code-cheat`, step 5128 |

Worlds 2, 4 and 6 have no code: a code is a checkpoint every second world, as the load lists store.
No code reaches past world 6, and no code changes the lives (`lives_reset`, BCD 32, every session).

## A wrong code

Any other four digits start world 1, as 0556 does, and leave `cheat_flag` as it was: a wrong code
clears no cheat an earlier session set. Measured (`title-code-wrong.txt`, 1234): `world_index` 0 at
`load_world` in step 5128, `cheat_flag` 0 from 0 and 1 from 1 (poked before the run).

## Esc

The Amiga's Esc on the title quits to AmigaDOS, as KONIEC does. `title_pass` checks `last_key_byte`
for `$75` (Esc pressed) after every pass; `level2_handler` writes it on the title as in play (the
autovector is the program's, docs/15), so the check is live, not dead as docs/07 once read it. It acts
on the press; the check follows every pass, so (read, not run) it quits from any row and during KODY's
digit entry too.

Measured (`title-esc.txt`): Esc down in step 5010 sets `quit_flag` and enters `quit_to_dos` in that
step; the return is in step 5011, as KONIEC's below.

## KONIEC

Fire released on KONIEC: `title_menu` sets `title_quit_flag`, `title_pass` sets `quit_flag` and
returns, `session_loop` branches to `quit_to_dos` (`h0+$35c`). It puts back the saved copper lists,
autovectors 2 and 3, CIA-A's ICR, DMACON, ADKCON and INTENA, pops the 15 registers the entry saved and
returns from the entry to its caller.

Measured (`title-koniec.txt`): fire released in step 5060 sets `title_selected`; `quit_to_dos` runs in
step 5061 and its `rts` (`h0+$3ca`) in step 5062. The last line sampled in the program's code has the
stack pointer the entry left after saving the registers (`$c645a0`), the long above those 60 bytes is
the caller's return address (`$ff492e`, Kickstart's code), and Kickstart runs next: the return is the
entry's own, the stack as the entry found it. The program never runs again (50 steps watched); its
memory is reused within a frame.

## Game over

`session_game_over` (`h0+$1aa`), reached from `main_loop` when the lives run out, when `countdown` reaches
0 or on give-up (Esc in play, docs/09), three branches to the same routine (read): `game_over_screen` waits for fire pressed and released, then `high_scores`
(`h0+$89d0`), then `session_loop` again, which loads the title's files and runs `title_screen`.

`high_scores` compares `score` with the eighth entry's (`hiscore_score_8`, signed, equal enters). Below
it the table is shown. At or above it `hiscore_name_entry` asks for a name: keys are typed on release
through a key map at `h0+$8cb8` (a zero byte there ignores the key), up to 12; Backspace (`$41`) deletes
the last, Return (`$44`) ends. The name and score go into the eighth entry, `hiscore_sort` sorts the
eight best first, and the table is shown until fire is pressed and released.

The table is eight entries of a 12-byte name and a BCD score (`h0+$84b6`, `hiscore_name_N`,
`hiscore_score_N`), from the program file: eagle soft 200000, hijo 175000, axel d. 150000, bald hors
125000, dan 100000, k.m. 75000, comanche 50000, druid 25000. It lives in memory only: the routine that
would save it to `DF0:ST` is unreferenced (docs/07, docs/15), so nothing is written to disk and a
power-off forgets it.

Measured, world 2 (`world_index` 1 set at `load_world`), Esc in play to give up:

| | game-over-title.txt (score 0) | game-over-name.txt (score BCD 30000 at load_world) |
|---|---|---|
| `game_over_screen` | step 6278 | step 6278 |
| `high_scores` | 6400 (fire released 6400) | 6400, `hiscore_name_entry` the same step |
| name | none: 0 is under 25000 | A, B, Backspace, C, Return: "ac", 30000 in entry 8, druid gone |
| `session_loop` | 6500 (fire released) | 6600 |
| `title_screen` | 7447 | 7548 |

## The song's loop

`title_screen` sets `mt_songpos` to 0 and calls `mt_init`; the idle title plays positions 0 to 26 and
loops. Measured (from 4990 with nothing pressed): position 1 from step 5305, each position 384 frames
after, position 26 from 14905 for 108 frames only, and `mt_songpos` first back at 0 in step
15013, 10091 frames after `mt_init`'s frame 4922.

## Unreferenced title code

Nothing in the program reaches these; the cases do not test them.

- `h0+$7236`: moves `lives_choice` (`h0+$6dfa`) 0 to 2 with left and right and blinks a box; with
  `h0+$53e`, which would set the lives to 15, 27 or 39 by it, a difficulty choice left out. The start-up
  draws its indicator once (`h0+$7326`), always 0.
- `h0+$780e`: moves `world_index` 0 to 10 with left and right and draws it; a world select left out.
- `title_menu`'s `title_cursor == 3` branch: the rows never reach 3.
