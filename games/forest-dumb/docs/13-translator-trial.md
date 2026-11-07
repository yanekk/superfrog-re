# 13 — The translator trial

Stage 0 of [translator-brief.md](translator-brief.md), plan `forest-dumb-translator-trial` (2026-11-07).
A static translator turns Ghidra's P-code for the main program into Rust, instruction for instruction,
with no model per routine. It was run over the 87 routines of [re/translate.toml](../re/translate.toml)
(`game_frame`'s tree and `redraw_edge_a..d`'s, the routines Muse was given in level 1's T40), and each
was judged on the routine oracle. This note holds what was run, the table, the failures, the check for
writes into code, and the gate reading the owner decides on.

## What was run, and how to rerun it

From the repo root, with the disks extracted to `games/forest-dumb/build/files/` (the main checkout's
`build/` or `fd-tools extract`):

```
python3 -I games/forest-dumb/tools/pcode_export.py                    # Ghidra headless, ~10 s
python3 -I games/forest-dumb/tools/translator_trial.py --skip-export  # translate, build, judge, ~75 s
cd games/forest-dumb/engine && cargo run -p fd-conform -- code-writes # §2.6, level 1's cases
```

The pieces, each its plan task: `ExportPcode.java` and `pcode_export.py` (T01) write every reached
instruction's P-code, hunk-relative, with the program file's `HUNK_RELOC32` entries, since Ghidra's
Amiga loader records none. `pcode_rust.py` (T03) writes one Rust tree from it on the `pcode-rt` runtime
(T02). `fd-judge --translated` (T04) enters that tree at each routine and compares it with the original
on 2 000 generated states: d0-d7, a0-a6, every byte written off the stack and every custom-chip write in
order, Muse's bar. Condition codes and cycles are not compared (stage 1's). `translator_trial.py` (T06)
runs all of it twice, without and with `re/translator-hints.toml`, and writes
`build/translator/table.md` and `trial.json`. The table below is that file from the run of 2026-11-07
(export 3 962 instructions, 1 002 relocations all matched, 0 unresolved jumps).

A routine passes only with no hand edit: no change to the generated Rust, no special case for one
address, no hint. Unjudgeable routines (no generator, or the original never returns on the rig) are
listed and left out of the count.

## The table

| Routine | Subsystem | Instructions | Without hints | First difference or stop | Untranslated | Write into code | Unsupported | With hints | Muse |
|---|---|---|---|---|---|---|---|---|---|
| `h0+$774` `h0_774` | camera | 20 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$7cc` `h0_7cc` | camera | 22 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$844` `redraw_edge_a` | camera | 325 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$a06` `h0_a06` | camera | 43 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$b20` `redraw_edge_b` | camera | 325 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$ce4` `h0_ce4` | camera | 42 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$df6` `redraw_edge_c` | camera | 365 | pass |  | 0 | 0 | 0 | pass | too-big |
| `h0+$104c` `h0_104c` | camera | 42 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$114a` `redraw_edge_d` | camera | 362 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$1390` `h0_1390` | camera | 43 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$1fec` `game_frame` | rest | 3026 | pass |  | 0 | 0 | 0 | pass | blocked |
| `h0+$20b0` `countdown_step` | rest | 17 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$2104` `play_keys` | rest | 36 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$2146` `help_key` | rest | 10 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$2176` `esc_key` | rest | 12 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$21c0` `h0_21c0` | drawing | 156 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$24ce` `h0_24ce` | objects | 69 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$2648` `h0_2648` | sound | 12 | unjudgeable: no generator (sound) |  |  |  |  | unjudgeable: no generator (sound) | no-gen |
| `h0+$2698` `char_object_cells` | objects | 255 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$26ec` `h0_26ec` | objects | 241 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$29ca` `h0_29ca` | drawing | 35 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$2a6e` `score_add_routine` | objects | 10 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$2c7e` `h0_2c7e` | drawing | 220 | pass |  | 0 | 0 | 0 | pass | blocked |
| `h0+$2d4c` `h0_2d4c` | drawing | 201 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$2dd6` `h0_2dd6` | objects | 122 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$2e0e` `h0_2e0e` | objects | 111 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$2fae` `h0_2fae` | objects | 237 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$2fea` `h0_2fea` | objects | 226 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$320a` `h0_320a` | objects | 6 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$3222` `h0_3222` | objects | 6 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$32f8` `h0_32f8` | objects | 11 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$3322` `h0_3322` | objects | 11 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$3530` `h0_3530` | objects | 15 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$3566` `h0_3566` | objects | 16 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$35a0` `h0_35a0` | objects | 16 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$35dc` `h0_35dc` | objects | 16 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$3698` `h0_3698` | objects | 86 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$3ab4` `h0_3ab4` | drawing | 3 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$3ac2` `h0_3ac2` | camera | 5 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$3adc` `h0_3adc` | sound | 12 | unjudgeable: no generator (sound) |  |  |  |  | unjudgeable: no generator (sound) | no-gen |
| `h0+$3b2a` `char_hit_check` | objects | 183 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$3dd8` `h0_3dd8` | objects | 18 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$3e0c` `char_update` | character | 729 | pass |  | 0 | 0 | 0 | pass | blocked |
| `h0+$3e30` `char_step` | character | 462 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$3ea8` `h0_3ea8` | character | 185 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$410c` `h0_410c` | character | 58 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$416c` `h0_416c` | character | 13 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$4274` `h0_4274` | character | 25 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$4696` `h0_4696` | character | 21 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$4718` `h0_4718` | character | 104 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$47e2` `h0_47e2` | character | 15 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$48be` `h0_48be` | character | 104 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$497c` `h0_497c` | character | 15 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$4b0e` `h0_4b0e` | character | 51 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$4be6` `char_step_4be6` | character | 13 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$4c28` `h0_4c28` | character | 13 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$4c6a` `h0_4c6a` | character | 34 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$4d32` `h0_4d32` | character | 6 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$4d54` `h0_4d54` | character | 6 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$4d7e` `camera_update` | camera | 178 | pass |  | 0 | 0 | 0 | pass | fail |
| `h0+$4e9a` `h0_4e9a` | camera | 32 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$4ea6` `h0_4ea6` | camera | 33 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$4eb4` `h0_4eb4` | camera | 33 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$4ec2` `h0_4ec2` | camera | 32 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$4f84` `char_probe` | character | 199 | pass |  | 0 | 0 | 0 | pass | too-big |
| `h0+$4f90` `char_probe_flags` | character | 60 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$513c` `h0_513c` | camera | 13 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$51fc` `h0_51fc` | camera | 23 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$526a` `h0_526a` | camera | 23 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$52d8` `h0_52d8` | camera | 23 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$5346` `h0_5346` | camera | 23 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$5462` `char_sprite_position` | drawing | 34 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$5820` `h0_5820` | drawing | 177 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$5e44` `h0_5e44` | drawing | 200 | pass |  | 0 | 0 | 0 | pass | too-big |
| `h0+$62b6` `h0_62b6` | drawing | 227 | pass |  | 0 | 0 | 0 | pass | too-big |
| `h0+$681c` `h0_681c` | camera | 27 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$6852` `h0_6852` | camera | 13 | fail | write-into-code: h0+$68f4 | 0 | 34 | 0 | fail | pass |
| `h0+$6882` `h0_6882` | camera | 78 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$69d6` `lives_minus_one` | rest | 12 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$6a04` `countdown_plus_100` | objects | 17 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$6a4c` `items_left_minus_one` | objects | 14 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$8dfc` `read_joystick` | character | 40 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$94ba` `h0_94ba` | drawing | 66 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$94fe` `h0_94fe` | drawing | 53 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$9550` `h0_9550` | drawing | 41 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$9754` `h0_9754` | drawing | 61 | pass |  | 0 | 0 | 0 | pass | pass |
| `h0+$9b06` `char_sprite_copy` | drawing | 73 | pass |  | 0 | 0 | 0 | pass | pass |

Gate: 84 of 85 judgeable routines pass with no hand edit (98.8%), against 90%: met. With hints 84 pass; 2 unjudgeable, left out of the count. 2000 states a routine.

Muse's column is level 1's T40 batch (`build/translate/batch.json`) for the same routines: 77 pass,
1 fail (`camera_update`), 4 too-big, 3 blocked, 2 no generator. Of the 85 judgeable routines Muse
passed 77 (90.6%); the translator passes 84 (98.8%), including all seven Muse could not take
(`game_frame` at 3 026 instructions among them) and `camera_update`. Muse's column is information,
not part of the gate.

## What failed, and why

The first full run (T06) passed 75 of 85 (88.2%). T07 sorted the ten failures by cause and fixed every
cause that a general fix reaches:

| Category | Routines | What |
|---|---|---|
| Ghidra's P-code, corrected in the translator | 9 | Ghidra 12.0's `abcd`/`sbcd` call an undefined `bcdAdjust`, `abcd` drops its result, Z is set on non-zero, `-(a7)` steps 1; its compares set X, which the 68000 leaves. Corrected by mnemonic and P-code pattern, never by address (`pcode_rt::bcd`, Moira's arithmetic, all 262 144 inputs agree) |
| Needs stage 2 | 1 | `h0_6852` |
| Needs a hint | 0 | `translator-hints.toml` holds none; the with-hints column equals the plain one |
| Fixed otherwise, unexplained | 0 | |

`h0_6852` fails 34 of 2 000 states, each stopping with `write-into-code: h0+$68f4`. Its generator gives
the slot in `d7` values up to 33; the game gives it 0 to 4 (`h0_681c`), so the generated states write
the slot's byte into the routine's own code. The original does the same on those states; the
translator stops instead of guessing, as DESIGN §2.5 requires. Muse's port passes because the judge
compares only bytes written, and both wrote the same byte. On the game's own path the write never
happens (next section). Whether translated code should follow a write into code or stop is stage 2's.

The translator found four defects of Ghidra 12.0 in all, for stage 1's check against Moira: the
`abcd`/`sbcd` P-code, `subflags` setting X on `cmp`, `cmpa`, `cmpi`, `cmpm`, a `bra.w` into another
function overridden as `CALL_RETURN`, and the empty relocation table. `nbcd` is left unsupported (not
in the trial's tree).

## Writes into code on level 1's path

`fd-conform code-writes` (T05) runs level 1's 33 conformance cases on the original without their
freeze, watches every even address of the program's five hunks, and compares each executed
instruction's bytes with the loaded program on first run and with the previous frame after. Total, 2026-11-07:
`{"cases":33,"frames":80964,"changed":0}`. No instruction the game executed on level 1's path changed.

What it cannot see: a write restored within one frame, code run outside the program's hunks, and
paths the cases do not take. `first-case` (the title) is not a level case and is skipped.

## The gate

Go when at least 90% of judgeable routines pass with no hand edit, and level 1's path writes into no
executed code (DESIGN §2.7). The reading: **85 judgeable routines (2 unjudgeable: the sound routines
`h0_2648`, `h0_3adc`, no generator), 84 pass with no hand edit, 98.8% against 90%, met; level 1's
path writes into no executed code (0 changes in 80 964 frames), met.**

**Owner's answer, 2026-11-07: go.** The second plan, `forest-dumb-translator` (stages 1 to 6), is next
once level 1 merges.

## What the trial says for stages 1 to 6

- **Stage 1, instructions.** Ghidra's P-code is not to be trusted for rare instructions and flags:
  two of the instruction kinds the tree used were wrong. The census against Moira should cover
  condition codes and X, which the trial did not compare, and start from the four defects above.
- **Stage 2, finding the code.** The 87 routines' tree needed no hint and has no unresolved jump;
  the export follows every direct branch once entries are seeded from `re/symbols.toml` (Ghidra alone
  finds 3 837 instructions). The two stops work as designed. `h0_6852` is the case to settle: a write
  into a translated instruction on a state the game does not reach.
- **Stage 3, runtime.** Addresses are hunk-relative and resolved at run time, so the oracle's bases
  were never baked in; this carries to the app unchanged. The runtime's step counter is per call, so
  endless recursion overflows the Rust stack rather than stopping as a hang (open, no trial routine
  hit it).
- **Cost.** A full trial run takes about 85 s and no model calls. One general fix moved nine routines.
