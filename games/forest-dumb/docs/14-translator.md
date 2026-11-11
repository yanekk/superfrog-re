# The translator: readings

What the static translator (plan `plans/forest-dumb-translator/`) was measured to do on this game,
dated. How to run it: `tools/README.md` (`translator_trial.py`) and
`engine/crates/fd-conform/README.md` (`fd-judge --translated`, `fd-conform code-writes`).

## Stage 1's gate: every instruction kind (T13, 2026-11-11)

**The census:** 290 instruction kinds (mnemonic, size, source and destination mode) over the 6 684
instructions of the whole-program export (`pcode_export.py --whole`), 815 distinct opcode words, one
site each (`pcode_census.py`, T04).

**The reading:** `insn_check.py --states 2000 --seed insn-gate-T13`, every kind translated alone and
stepped on Moira from 2 000 generated states a site: 1 630 000 states, **0 failing** in every column
(registers, X N Z V C, memory and custom-chip writes, cycles, stops), every kind's cycles counted.
26 780 states (1.6%) are skipped, each one the original taking an exception: an odd address from a
random `(d8,An,Xn)` index or a popped return address, or a zero divisor. The kind skipping most,
`move.l (d8,An,Xn),(An)`, still judges 949 of its 2 000. About 26 s from a cold optimised build.

The gate is guarded by `fd-conform`'s `insn_gate.rs` (full, `cargo test --release -p fd-conform
--test insn_gate -- --ignored`, about 20 s warm), which exports, runs the check with this seed and
fails on any failing kind, column, uncounted cycles or a census other than 290 kinds. With
`pcode_fix.shift_bit` switched off it fails on 9 kinds (33 137 states, ccr), as T09 found them.
Cycles follow Moira; real hardware is unmeasured (the plan's FINDINGS).

## Stage 2's gate: the whole tree (T16, 2026-11-12)

**The tree:** the whole-program export (`pcode_export.py`, 143 entries) holds 6 684 instructions;
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
