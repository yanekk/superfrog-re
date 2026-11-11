# The translator: readings

What the static translator (plan `plans/forest-dumb-translator/`) was measured to do on this game,
dated. How to run it: `tools/README.md` (`translator_trial.py`) and
`engine/crates/fd-conform/README.md` (`fd-judge --translated`, `fd-conform code-writes`).

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
