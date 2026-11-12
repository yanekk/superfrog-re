# The translator: readings

What the static translator (plan `plans/forest-dumb-translator/`) was measured to do on this game,
dated. How to run it: `tools/README.md` (`translator_trial.py`) and
`engine/crates/fd-conform/README.md` (`fd-judge --translated`, `fd-conform code-writes`).

## Stage 1's gate: every instruction kind (T13, 2026-11-11; re-run T43, 2026-11-12)

**The census:** 291 instruction kinds (mnemonic, size, source and destination mode) over the 7 716
instructions of the whole-program export (`pcode_export.py --whole`), 821 distinct opcode words, one
site each (`pcode_census.py`, T04). T13 read 290 kinds over 6 684 instructions; T43's call
fall-throughs (below) added `or.w Dn,Dn` (`h4+$1c946`, `$1c948`).

**The reading:** `insn_check.py --states 2000 --seed insn-gate-T13`, every kind translated alone and
stepped on Moira from 2 000 generated states a site: 1 642 000 states, **0 failing** in every column
(registers, X N Z V C, memory and custom-chip writes, cycles, stops), every kind's cycles counted.
26 780 states (1.6%) are skipped, each one the original taking an exception: an odd address from a
random `(d8,An,Xn)` index or a popped return address, or a zero divisor. The kind skipping most,
`move.l (d8,An,Xn),(An)`, still judges 949 of its 2 000. About 26 s from a cold optimised build.

The gate is guarded by `fd-conform`'s `insn_gate.rs` (full, `cargo test --release -p fd-conform
--test insn_gate -- --ignored`, about 20 s warm), which exports, runs the check with this seed and
fails on any failing kind, column, uncounted cycles or a census other than 291 kinds. With
`pcode_fix.shift_bit` switched off it fails on 9 kinds (33 137 states, ccr), as T09 found them.
Cycles follow Moira; real hardware is unmeasured (the plan's FINDINGS).

## Stage 2's gate: the whole tree (T16, 2026-11-12)

**Every call falls through (T43, 2026-11-12).** Ghidra 12.0's no-return analysis judges `load_file`
(`h0+$8ec4`) non-returning: in hunk 4's load lists the instruction after each `jsr load_file` holds a
relocated long Ghidra had marked data. It then sets `CALL_RETURN` on all 82 calls to it, which drops
their fall-through. The export follows each instruction's own fall-through, never Ghidra's, and lists
such calls under `check.no_return_calls` (82). The export grew from 6 684 to 7 716 instructions,
hunk 4 from 910 to 1 898, and reaches two routines in data hunk 1 (`h1+$484`, `$49e`) that copy colour
tables. Power-on now reaches the title with no stop (`os_run.rs`).

**The tree:** the whole-program export (`pcode_export.py`, 143 entries) holds 7 716 instructions
(6 684 at T16's reading, before T43; re-checked 2026-11-12: 100 of 100 still pass);
`pcode_rust.py` translates every one, **0 unsupported**. **0 unresolved computed jumps**: Ghidra finds
no jump table it cannot follow. The 24 computed sites it gives no target are all calls through a
library base, `jsr -n(a5)` or `jsr -n(a6)` (exec, graphics and dos.library), whose target is the OS's
jump table outside every hunk: the machine answers them (DESIGN §2.8), they are not translated. 19 are
in `re/os-calls.toml`; the five at `h0+$943a`..`$94ae` (dos `Write`, -48, among them) are on a path
T01's run did not take. `re/translator-hints.toml` holds **no hint**.

**The routines:** `translator_trial.py --whole --cases 2000` judges every routine of
`re/translate.toml` and every registered generator (`fd-judge --generators`, 105): 101 routines,
**100 of 100 judgeable pass with no hand edit**, 2 000 states each, none stopped untranslated, unsupported
or writing into code. Beyond the trial's 85: `view_tiles`, `view_tile`, `view_shapes`, `view_shape`,
`h0_6a96`, `h0_6ae8`, `h0_6b70`, `h0_6b78`, `h0_6bc0`, `h0_6c20`, `h0_6c80`, `h0_988a`, `h0_9a34`,
`h4+$18870` (the music player's channel tick) and `h0_3adc` (a sound routine, by the spike's
generator). Unjudgeable: `h0_2648` (sound, no generator). Six routines have self-modifying states, 218
in all, the six of the table below, each passing on the rest. Five spike generators named by address
(`h0+$24ce`, `$2dd6`, `$2fae`, `$774`, `$3b2a`) duplicate a routine's own generator and are not run.

The judge is built optimised: unoptimised, `view_tiles` (368 tile blits a state) took 340 s for 2 000
translated states, past the 120 s a run, where the original alone took 5 s; optimised the whole judging
takes about 25 s and the run about 1 minute warm. Guarded by `tools/test_translator_whole.py` (full).

## Stage 3: the routines on the machine (T27, 2026-11-12)

**The reading:** `translator_trial.py --whole --machine --cases 2000` judges the same 100 routines with
the translated tree running on `pcode-machine` (`fd-judge --translated --machine`,
`judge::translated::machine`) instead of the judge's copy of the oracle's hardware mode: blits,
DMACONR's busy bit, the beam and every custom register read come from the machine's models.
**88 of 100 pass**; nothing stops untranslated, unsupported or writing into code. The owner closed T27
on it with the 12 others named (2026-11-12). About 1 minute warm.

How a state is laid on the machine: chip RAM, slow RAM and the custom registers are the machine's;
every other address (the rig's stack, chip RAM's mirrors, memory an A500 lacks) is the oracle's flat
16 MB, because generators give random pointers the original follows there; the CIAs' `$BFxxxx` read
the case's bytes or 0, as on the oracle; the case's custom register reads are answered by a device on
exactly those registers; the beam starts on the case's line; DMACON is DMAEN and BLTEN only.

**The 12 that differ** are the oracle's hardware mode, not the machine, each pinned with its evidence
in `tools/test_translator_whole.py` (`MACHINE_PINNED`, full):

- **Beam, 8:** `game_frame`, `char_object_cells`, `h0_26ec`, `h0_3adc`, `char_hit_check`,
  `char_update`, `char_step`, `h0_3ea8`. The oracle steps VHPOSR's byte by one a read; the machine
  reads it from counted cycles. Each but `game_frame` waits for four line changes
  (`move.b $dff006,d1; cmp.b $dff006,d1; beq`): from line b the oracle reads b to b+7 and leaves
  d1 = b+6, the machine waits a real line each time and leaves b+3 (`h0_3adc` from line $01: $07
  against $04). `game_frame` stores VHPOSR in `beam_sample_unread` (`h0+$8dfa`), $0304 against $3e86.
  With an oracle-style beam in the judge all 8 pass (measured, not kept). The case gives only the
  first read (`Setup::beam`), which the machine gets.
- **Blit, 4:** `h0_a06`, `h0_ce4`, `h0_21c0`, `h0_29ca`. A generated blit runs past the top of chip
  RAM; the oracle's writes on at `$08xxxx`, the machine's wraps to `$00xxxx` as a 512 KB A500's does
  (`h0_a06` case 71: the same 23 bytes at `$080004..$0805c4` and `$000004..$0005c4`). The game's own
  blits never cross the top.

## Stage 7: sessions translate first, unprompted (T21, 2026-11-12)

`tools/default_check.py` gave three fresh `claude -p` sessions (default model, 15 turns, Read, Grep,
Glob and `python3 -I games/forest-dumb/tools/*` allowed, Write and Edit offered but denied) the prompt
"Port char_sprite_position of Forest Dumb Forever to Rust for a level plan. Start the way this repo's
docs say." in a scratch worktree of HEAD. **3 of 3 pass**: each read the root and game `CLAUDE.md`,
`docs/11-translation.md` "The procedure" and `tools/README.md`, then ran `tools/translate.py
char_sprite_position` at turn 6, 7 and 6, before any other Rust. None wrote Rust by hand. 11 to 12
turns, 32 to 158 s, $0.36 to $0.38 list price each.

Limit: a scratch worktree has no `build/`, so `translate.py` stopped at once (`no program at
.../FOREST_DUMB_FOREVER: run cargo run -p fd-tools -- extract`), and each session then asked for the
extract, which the check does not allow. The check shows what a session reaches for, not that the
translation it got was used.

## Writes into code (T15, 2026-11-11)

Translated code cannot follow a write into its own instructions, so a store onto a translated
instruction's byte stops the run `write-into-code: hN+$off` (DESIGN §2.4). The judge sets aside a
generated state on which the original itself changes such a byte: it is `self-modifying`, outside
the game's reach, and is not compared (DESIGN §2.5). A translated store into code where the original
does not write there still fails.

**On the game's own paths the original never writes into its code.** `fd-conform code-writes --all`
over level 1's 42 cases, the power-on run through the title into level 1 (7 000 frames) and the world
2 to 6 starts: 48 runs, 153 600 frames, **0 changed instructions**. The power-on run's watch starts
at the program's entry `h0+$0`; before it, the crack's programs run in the same memory. About 3
minutes in release.

**The trial's routines** (`translator_trial.py`, whole-program export, 2 000 states a routine):
85 of 85 judgeable pass with no hand edit; 2 unjudgeable (sound, no generator). Six routines have
self-modifying states, each passing on the rest:

| Routine | Self-modifying states | How the original writes into code |
|---|---|---|
| `h0_6852` | 35 | the slot in `d7`, up to 33 here and 0 to 4 in the game (`h0_681c`), indexes past its table into `h0+$68f4` |
| `h0_a06` | 47 | a blit whose generated destination lands in code |
| `h0_ce4` | 67 | a blit |
| `h0_29ca` | 65 | a blit |
| `h0_21c0` | 3 | a blit |
| `h0_26ec` | 1 | a blit, through its callee `h0_29ca` |

The blits are the judge's shim writing memory: it runs a blit at the BLTSIZE write, which the code
check does not see, so these states passed before T15 too and now are left out.

Limit: the oracle's routine runner reports only bytes whose value changed, so an original that stores
the value a code byte already held is judged, and fails on the translated `write-into-code`.
