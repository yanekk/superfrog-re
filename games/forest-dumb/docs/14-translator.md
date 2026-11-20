# The translator: readings

What the static translator (plan `plans/forest-dumb-translator/`) was measured to do on this game,
dated. How to run it: `tools/README.md` (`translator_trial.py`) and
`engine/crates/fd-conform/README.md` (`fd-judge --translated`, `fd-conform code-writes`).

## The plan's result (T42, 2026-11-20)

The plan closed with every reading below. Against its success criteria (plan DESIGN §1):

| Criterion | Reading | Section |
|---|---|---|
| Every instruction kind matches Moira | 291 of 291 kinds, 1 642 000 states, 0 failing in any column, cycles included | Stage 1's gate |
| The whole program translates | 7 716 instructions, 0 unsupported, 0 unresolved jumps, no hint; 100 of 100 judgeable routines pass | Stage 2's gate |
| The routines on the machine's chips | 89 of 100; the 11 others are the oracle's hardware mode, pinned | Stage 3 |
| Level 1's cases, from the sync point and from power-on | 49 of 50 cases pass translated, the same 49 with level 1 mounted; `power-on-level-1` fails from frame 6301 | The cases |
| Worlds 2 to 6 over their first play | 5 of 5 | Worlds 2 to 6 |
| No write into code on the game's paths | 0 changed instructions over 153 600 frames | Writes into code |
| Generated code holds the purity rules | `purity.rs` scans the tree; one exemption, `CODE` | Generated code's purity |
| The app from power-on with sound, level 1 mounted | `translated_whole.rs` passes, launch to world 2's play; the drill worker-driven | The app |
| Sessions translate first | 3 of 3 fresh sessions ran `translate.py` unprompted | Stage 7 |
| Haiku for generators and rewrites | 5 of 5 generators, 19 of 21 rewrites, mark missed; owner accepted Haiku (T44) | `11-translation.md`, History, "Haiku's trial" |

What is left open for the plans after it is under "Open after the plan", at the end.

## The cases on the translated program (T29 to T31, T45 to T54, 2026-11-12 to 11-19)

**The reading, 2026-11-20 (T58, the last re-run): 49 of the 50 cases in `engine/conform/cases/` pass
on `--engine translated`**: level 1's 42 cases (both W1-1 routes to the level's end, `route-W1-1` in
21 764 passes and `route-both-W1-1` in 21 821), `first-case`, `power-on-title` and the five world
starts. `power-on-level-1` fails from frame 6301 at `h0+$1fe6` (engine $80, original $f1), the first
pass of play after SCENA 1 (T53). Its notes and `power-on-title`'s still say `Fails today` as at T47.

The frame bar these pass on (plan DESIGN §2.7; `engine/conform/README.md`): state at every pass end
paired by `irq_count`, a pass allowed to end `frames::PASS_LAG` frames apart; Paula's writes in order;
the CPU's other custom writes as a multiset a pass, a write allowed a pass early or late (T45,
`custom_carried`); the picture at each frame end; a write made by reading a write-only register
compared by register only (T47, `frames::LEFTOVER`).

How the count rose, each step a machine fix found by a case, never a case allowance:

| Task | Date | Level 1 cases passing | What made the difference |
|---|---|---|---|
| T29 | 2026-11-12 | first reading | the frame runner, Paula's writes compared |
| T30, T31, T48 | 2026-11-12 | 19 of T30's 23; 4 of T31's 18 | judging at pass ends, CLXDAT collisions, held keys |
| T47 | 2026-11-12 | (power-on) | Kickstart's DMACONR `$03f0`, INTENAR `$602c`, ADKCONR `$1100` at power-on |
| T49 | 2026-11-18 | 23 of 24 | DMA contention on `pcode-machine`, measured on the oracle |
| T52 | 2026-11-19 | `camera-bottom-right` | blit D words reach the picture at their bus slot; long blocks timed per instruction |
| T50 | 2026-11-19 | 40 of 42 | interrupts taken at instruction boundaries |
| T54 | 2026-11-19 | 42 of 42 | the bus and interrupt timing to the oracle's slot, `main_loop`'s spin (`h0+$252..$2aa`) |

The machine's power-on state, measured on the oracle (T47): Kickstart leaves DMACONR `$03f0`, INTENAR
`$602c`, ADKCONR `$1100`, and COP1LC its own list; `Machine` starts there.

## Level 1 mounted through the seam (T35, T36, T55 to T60, 2026-11-12 to 11-20)

`fd-judge --seam` (T35) holds a hand-written routine to the translated routine it replaces on the
same generated states: bytes written off the rig's stack, custom writes in order, the return, the
registers named. `camera_update` and `char_sprite_position` pass 2 000 of 2 000; level 1's frame
mount (`game_frame`, `h0+$1fec`, `--mount level-1`) passes 2 000 of 2 000 (T36, again T58).

**Mounted, 49 of 50 cases pass** (`tests/mount_level_1.rs` `PASS_MOUNTED`, T58, 2026-11-20), the
same 49 as translated alone; `DIFFER` holds `power-on-level-1`, which fails on the translated
program alone. 25 cases hear an effect (item, stomp, slide, hurt, spring) whose Paula writes are the
original's in order mounted (`HEARS`, T57, T58); W1-1 has no time bonus sound.

The mount spends the original's time (plan DESIGN §2.9): each routine is walked along its path
through the original's instructions (`fd-translated/src/seam/rules.rs`, `seam::walk`), so its chip
accesses land on the translated program's bus slots; the CLXDAT read lands on the original's line
(`CLX_LINES_OFF` 0). How it got there: T36's instant mount, 32 of 50; T55's fixed cost per callee
(`COSTS`), 39 of 50, missing by up to 7 000 cycles where a routine's time varies with state; T56
and T58's walked paths, 49 of 50.

## Generated code's purity (T34, 2026-11-12)

`fd-conform`'s `purity.rs` scans the tree fd-translated builds in (`FD_TRANSLATED`, else
`build/translator/tree/`) and every committed file whose header says `pcode_rust.py` wrote it, under
the same rules as hand-written code: no clock, random number, network or game-data table. One
exemption, the owner's (2026-11-12): `const CODE: &[(u32, u32, u32)]` of (hunk, start, end) triples,
the translator's index of what it emitted, in generated code only. A CODE of any other shape fails.

## The app (T37 to T41, 2026-11-19 to 11-20)

`fd-app` runs the translated program from power-on (T37) with its sound through `amiga-paula`
(T38), level 1 mounted, and a stop freezes the picture, names it and saves a replayable zip (T39).
KONIEC on the title quits the app as Esc does (owner, T41): the original returns to AmigaDOS
(`quit_to_dos`), `TranslatedGame::exited`.

- **End to end (T40):** `fd-e2e`'s `translated_whole.rs` (full, 503 s) plays from launch by key
  presses: the disk 2 prompt, the title, Start, SCENA 1, the one route through level 1 (steps 620 to
  23 084) with no life lost, world 2's load and SCENA screen, and 300 steps of world 2's play from
  step 23 414, the app's state equal to a headless run every step. `translated_short.rs` (22 s) is
  its first 600 route steps.
- **The drill (T41), worker-driven, 2026-11-20:** the title's rows, KODY digit entry equal to the
  original, ten spike deaths to game over and restart, the route to world 2, resizes from 400×300 to
  1200×900, focus lost, stops at 1×, 2× and 1000×700 with each zip replayed, sound. Screens in
  `build/drill/`. `translated_drill.rs` holds what it found.

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
on it with the 12 others named (2026-11-12). About 1 minute warm. With DMA contention on the machine
(T49, 2026-11-18) **89 of 100**: `h0_26ec` passes, `char_object_cells` differs by one read.

How a state is laid on the machine: chip RAM, slow RAM and the custom registers are the machine's;
every other address (the rig's stack, chip RAM's mirrors, memory an A500 lacks) is the oracle's flat
16 MB, because generators give random pointers the original follows there; the CIAs' `$BFxxxx` read
the case's bytes or 0, as on the oracle; the case's custom register reads are answered by a device on
exactly those registers; the beam starts on the case's line; DMACON is DMAEN and BLTEN only.

**The 11 that differ** (12 before T49) are the oracle's hardware mode, not the machine, each pinned with its evidence
in `tools/test_translator_whole.py` (`MACHINE_PINNED`, full):

- **Beam, 7** (`h0_26ec` passes since T49)**:** `game_frame`, `char_object_cells`, `h0_3adc`, `char_hit_check`,
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

## Worlds 2 to 6: their load and first play (T33, 2026-11-18)

**The cases:** `cases/world-2-start.toml` … `world-6-start.toml`, run by `fd-conform run
cases/world-N-start.toml --engine translated` (about 35 s each, warm) or `run-all --cases 'world-*'`.
Each starts from a snapshot of the original on the idle title at frame 4990, both sides then run
boot-to-level1's Start at 5000, the world's load, its SCENA screen and play, lined up at each load's
return as a power-on case is. `world_index` cannot be poked between frames: `title_code_world` writes
it in the step `load_world` reads it. The case's `set` row has `when = "load_world"`, and the runner
makes it on both sides as that routine is entered: the oracle's `Amiga::poke_at` (a toolkit addition,
the shim's exec hook) at its first instruction, the translated program's `TranslatedGame::write_at` at
its entry. No other row is needed: `session_loop` already does what `next_world` does before
`level_init` (`title_start_flag`, `h0+$500`, the camera, the score), so the world starts with a new
game's score and lives where a world reached by play would carry them.

**The reading, 2026-11-19 (T53): 5 of 5 pass.** In play (the program's handler in `$6C`, a pass
ended on either side) memory is judged at each pass end, paired by `irq_count`, a pass allowed to end
`frames::PASS_LAG` frames apart, and the other custom writes are cut into passes at each side's
`PASS_END` write, a write allowed a pass early or late (`frames::custom_carried`): DESIGN §2.7's
pass-end rule, as a level case has it, with no new allowance. Before play (the title, SCENA) memory
stays judged at frame ends under T51's rules below; the picture and Paula's writes stay per frame.
Each case compares 319 pass ends in play. T51's failures below were frame ends falling mid-pass in
`game_frame`; world 3's `BLTCON0=$cfca` at 6183 is the engine's a pass before the original's.

**The reading, 2026-11-19 (T51), on T49's machine: 2 of 5 pass** (`world-5-start`,
`world-6-start`), under three rules of the owner's for cases lined up by loads (`frames.rs`):

- **The OS's stretch after a load is the load's** (`OsTail`): from a load's return until the
  program takes the machine back (`level3_handler` in `$6C`, at `system_off`), no frame is compared,
  and the taking back counts as a return, so the original runs on to its own. There the original's
  OS runs its interrupts, which the machine never does (DESIGN §2.8): world 2's byte scan
  (`h0+$7a9e`, counting `$20..$27` bytes from `h0+$cb38`) covers 3 834 bytes in 5 frames, 4 015
  with the OS's handlers at `$64/$68/$6C/$78` stubbed by `rte`, and `system_off` returns at 6026
  instead of 6029. That was the T33 reading's speed difference.
- **`beam_sample_unread` (`h0+$8dfa`) is left out of memory** (`sync::BEAM_VALUES`): its line is
  equal every play frame, its position mostly 3 colour clocks apart.
- **A byte differing at a frame's end passes when one frame on**: the engine's values over three
  frames are the original's shifted a frame, either way. `view_tiles` draws after `system_off` at
  68 to 69 tiles a frame on both sides, and at each frame's end the engine is 0 to 2 tile rows of
  about 1 100 ahead (6 or 7 frames a case).

Paula's writes are equal in all five, and so is the picture; the custom writes in all but world 3.
The three left failed in play, on a frame's end inside `game_frame`; T53 judges play at pass ends:

| World | Frames failing | First difference |
|---|---|---|
| 2 | 1 | 6154, `h0+$2c7d` (`h0_2d4c`'s slot counter), engine 1, original 2 |
| 3 | 18 | 6180, `h0+$2c7d`, engine 2, original 1; a creature blit (`h3+$e84`) and `BLTCON0=$cfca` a pass apart at 6183, T52's kind |
| 4 | 1 | 6132, `h0+$1e61` (`char_y`), engine $a4, original $a5 |

`next_world` (checked against the listing, T51) sets `world_index`, `title_start_flag`, `h0+$500`,
the camera and `score_add`, all of which `session_loop` sets too; a world reached by play differs only
in carrying the score and lives, as below.

**The T33 reading (2026-11-18):** 0 of 5 pass, every one on the same speed difference as `power-on-level-1` in world 1
(T32, T47). In each, the 13 loads line up and `world_index` is the world's on both sides; the first
difference is `h0+$7a9d` while the SCENA screen is built, then the copper list's start and the
picture two to three frames early; in play, Paula's writes from fire plus 33, as world 1 from power-on
(fire 6300, Paula 6333). The translated program alone, on each case's inputs, loads the world and
plays 300 passes with no stop (`tests/world_starts.rs`).

| World | Memory (`h0+$7a9d`) | Copper start, picture | Fire on SCENA | Paula | Frames inside loads |
|---|---|---|---|---|---|
| 2 | 5977 | 6034, 6036 | 6150 | 6183 | 713 |
| 3 | 5998 | 6054, 6056 | 6170 | 6203 | 734 |
| 4 | 5951 | 6008, 6010 | 6120 | 6153 | 701 |
| 5 | 5843 | 5899, 5901 | 6010 | 6043 | 623 |
| 6 | 5837 | 5892, 5894 | 6000 | 6033 | 600 |

The SCENA screen's fire loop (`h0+$7c7e`) first runs on the original at frame 6036, 6057, 6010, 5902
and 5894 for worlds 2 to 6, and 6056 for world 1. Its `btst` returns on fire with no wait for a
release, so fire held during the load would pass the screen at once.

**The `set`, on stand-ins** (`tests/world_starts.rs`, full): a second copy of the original making the
write as `load_world` is entered passes world 2's case cut to 40 frames of play, every channel equal
through its 13 loads, `world_index` 1 on both; making it between frames at its start, it differs at
once (`h0+$6e01`, frame 4991) and then loads world 1, `title_code_world` having written 0; a `set`
waiting for a routine neither side enters (`next_world`) fails `the set world_index at next_world: 1
writes never made on the original, 1 on the engine`.

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

## Other measurements

- **Code seen (T02):** 48 runs, 6 380 instructions, 135 entries (program entry 1, symbol 134); every
  computed target is a named symbol.
- **Build times (T04):** `fd-judge` with the whole tree, release one module 28.0 s, `--parts`
  4/8/16/32 14.8/12.8/10.8/11.2 s (default 16); dev 4 to 7 s.
- **`translate.py` (T18):** warm, `char_sprite_position` 1 s, `--task T08` 15 of 15 in 6 s; cold
  export, translation, optimised build and a warm run 86 s.
- **Interrupt checks at instruction boundaries (T50)** cost about 7% (death-spikes 46.2 s against
  43.3 s).
- **SCENA screens wait for fire:** unpressed, `irq_count` stood 1 500 steps (T40). The item sample
  opens with 106 zero bytes, so it is heard the step after it starts (T57).

## Smoke (T42, 2026-11-20)

T42 did not run smoke: in a parallel build pir runs it once, after every task merges, and appends
its time to `~/.cache/superfrog-re/smoke-times.tsv`. The last smoke run that passed before the plan
took 786 s (2026-11-11, `e320eeb7`); a cold build in a fresh worktree reached the 1 200 s limit the
same day. The plan's smoke additions, each timed alone when labelled: forest-dumb's `pcode-machine`,
`fd-translated` and `fd-conform` unit and made-up-program tests (`insn_*.rs`, `frames.rs`,
`leftover.rs`, `judge_machine.rs`, `os_answers.rs`, the no-disk tests of `seam.rs`,
`mount_level_1.rs`, `mount_timing.rs`, `os_surface.rs`), `fd-e2e`'s `not_generated.rs` and ten
Python files, about 45 s together, plus the toolkit's `contention.rs` (eighteen boots of about 5 s),
`poke_at.rs` and `routine_step.rs`. Every test needing the disks and the generated tree is full.

## Open after the plan (T42, 2026-11-20)

Moved here from the plan's FINDINGS: what a level plan meets and nobody fixed. Each names the task
that found it.

**Failing or stale:**

- `power-on-level-1` fails from frame 6301 (`h0+$1fe6`, engine $80, original $f1), on the translated
  program alone (T53, T58). The cases' `power-on-title` and `power-on-level-1` notes still say `Fails
  today` from T47 (T51 review).
- `judge_machine.rs`
  `the_hardware_reads_a_case_gives_reach_the_routine_through_the_machines_devices`: CLXDAT read
  `$8000` on the machine, `$0000` in the oracle's hardware mode (bit 15, T31), at T47.
- At T49's base: `purity.rs` flags `pcode-machine/src/collide.rs:96`'s 18-element table;
  `test_insn_check.py` finds no `// h0+$4a bra.w` in the defects tree; `test_reshape` trips on a
  stale `/var/folders/.../toolkit` symlink.
- Under load 30 to 100, `translated_app.rs` `start_by_its_key…` reaches fd-app's `--timeout 180 s`
  (T37 review); idle it passes in 65 s.

**The machine and the mount, unmeasured or latent:**

- The CPU sees an interrupt `IPL_DELAY` (8) late, at an instruction's last access (vAmiga's IPL
  pipe), not at plan DESIGN §2.8's boundary; `Machine::settle`'s requests should be 6 (T54 review).
- `game_frame_costs --passes --mounted enemy-contact`: 422 of 1 299 passes run 2 to 8 cycles longer
  mounted, all bus waits, unlocated (T58 review).
- `frame.rs`'s unwalked path (`owed`, `settle`, `pay`, `LUMP`, the `!self.walked` branches of
  `clxdat` and the sound writes) is dead code since every callee is walked (T58).
- `seam::walk` times every `dbcc` as `dbf` (`Decision::Count` has no condition); all 178 here are
  `dbf`. `mount_timing.rs` runs contention out, so a dropped `bus.sync` passes it (T56 review).
- The shift loop (`lsl.l #8` four times, `move.w`) with refresh alone on the bus: 11 873 slots on
  `pcode-machine`, 11 866 on the oracle, not held in `tests_dma.rs` (T49 review).
- T60 fixed `seam::walk` making a `block_start` after a return and after `skip_call` (one extra
  poll); `mount_timing.rs`'s full test was not re-run after it (no disks in that session).
- With `PASS_LAG` 0, 7 of 24 translated level cases failed on a pass ending a frame apart, both
  ways (T52); `custom_carried` allowing no pass, all 24 passed.
- The whole-tree gate stays 100 of 100 with `pcode_fix` `arith` and `shift_bit` both off (dead
  flags, write-only registers); only `insn_gate.rs` guards those corrections (T16 review).
- `DEPTH_CAP` 32 rests on static call depth (8 from `h0+$0`, no recursion); running depth is
  unrecorded (T14, T28). `STEP_CAP_GLOBAL` 10 000 000; the longest run was 3 968 blocks.

**Tools:**

- `task_pack.py` hard-codes `PLAN = 'forest-dumb-level-1'`, so a level 2 plan gets no pack; wording
  naming the old model stays in `tools/reshape.py`'s docstring, `re/reshape.toml`'s header,
  `call_tree.py`'s `LIMIT` and a smoke.toml `[[area]]` (T20 review).
- `whole::heard` counts any non-zero sample; a switched-off channel holds its last value (level 1:
  -368 left), so T40's "heard over level 1" passes with no effect playing. Judge variation (T41).
- A stop during `--level 1`'s headless run through the title (`Translated::to_level_1`) exits 1 with
  no zip (T39).
- `fd-judge --seam`'s JSON carries no count of the hand engine's runs, so a mount that fell through
  would print `pass` (`seam.rs` asserts `Level1::passes()`); `watch_returns` counts a mounted
  routine's entry and return, against its doc (T36, T36 review).
- A load-aligned case's `Verdict` headline reads `same on all 0 passes compared` (T53 review).
- `insn_check.py` `aggregate`: one encoding's `not-counted` cycles hides the kind's other cycle
  failures in that column (T11 review).
- `cargo fmt` rewrites the pcode-rt fixtures (no `#[rustfmt::skip]`), failing
  `test_pcode_rust.py`'s currency check (T06 review).
- `--out` naming a file or an unwritable path gives a traceback in `pcode_export.py`,
  `re_listing.py`, `translate_batch.py`, `translator_trial.py`; `load_symbols` and `load_entries`
  give a bare `KeyError` on a row missing a key (T03, T04 reviews).
- A fresh worktree needs `fd-tools extract` before `translate.py --build`, then `touch
  games/forest-dumb/engine/buildscripts/fd_translated.rs`, or the build keeps the cached stub (T38).
- The toolkit's `toolkit-checks/tests/purity.rs:162` and Superfrog's `sf-conform/tests/purity.rs:183`
  lex `0x18f32` as a float literal (fixed in fd-conform's copy, T34 review).
- The toolkit's `LineSample::owners` doc (`amiga-oracle/src/amiga.rs`) omits that owners are the line
  before `vpos`; Superfrog's `sf-conform/examples/vbl_audio.rs` counts audio owners at `s.vpos`
  unshifted, a line late (T52 sweep).
- `Cpu68k::call` with an odd entry runs the instruction at the even address below (fixed in `step`,
  T05 review).

**The translator, beyond this program's census:** the sinc's `subx` V ignores X and `negx`'s C and V
come from the wrapped `src + X` (T08); register-count rotates past the size and `asr` by a register's
X are wrong (T09); a `mulu` or `divu` with a memory source stops `Stop::Unsupported` (T11). A program
using them needs a correction first.

**Docs and data:** docs/07 says `OpenLibrary` where the file routines call `OldOpenLibrary` (-408),
and that the OS's interrupts are taken through the title, where autovectors 2 and 3 are the
program's (T01). `boot-to-level1.txt` says SCENA 1 waits for fire from 6206; its fire loop
`h0+$7c7e` first runs at 6056 (T33). `os-surface --worlds 1,3,6` lists `lea $0.l`..`$3.l` at
`h0+$691e`..`$6930` as OS reads; they are constants (T01 review). The round trip over
`camera-right-left`'s first 100 steps leaves `edges.pos` (`h0+$1e66`), `p840`, `p83c` and the
`input_log` bytes untransplanted; `game_frame` reads none.
