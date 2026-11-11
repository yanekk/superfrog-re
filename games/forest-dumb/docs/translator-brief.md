# Brief for `pir plan`: the translator

Written 2026-11-06 for the planner; rewritten 2026-11-11 after the trial (stage 0) passed and level 1
merged. Slug: `forest-dumb-translator`. The sequence around this plan is in [ROADMAP.md](ROADMAP.md).

Read first: `games/forest-dumb/CLAUDE.md`, [13-translator-trial.md](13-translator-trial.md) (what the
trial built and measured, and its notes for stages 1 to 6), `plans/forest-dumb-translator-trial/DESIGN.md`
and its `FINDINGS.md`, [11-translation.md](11-translation.md) (the Muse procedure this plan replaces as
the default), `plans/forest-dumb-level-1/DESIGN.md` §4.3 and §4.4, `plans/forest-dumb-level-1/REPORT.md`
"Risks and follow-ups", `toolkit/docs/porting-routines.md`, and `toolkit/PLAYBOOK.md` steps 5 and 6.

## The goal

Two things, the second resting on the first.

1. **A complete static translator.** A tool that reads the main program `FOREST_DUMB_FOREVER` and writes
   Rust that runs it instruction for instruction, with no model in the loop per routine, for the whole
   program and not only `game_frame`'s tree. The translated program is the starting engine for
   everything after level 1 (levels 2 to 6, the title and menus, sound, the ending). Hand-written Rust
   replaces translated code where the owner chooses, each replacement checked against the code it
   replaces.
2. **The translator is the default first pass.** A new session that has to turn the original's code into
   Rust (a porting task, a level plan, a bug fix) reaches for the translator first, without being told,
   the way level 1's sessions reached for Muse (`tools/translate.py`). Muse's literal translation stops
   being the default: the docs, the indexes, the task packs and the tools' READMEs say "translate with
   the translator", and Muse stays only where the plan decides it still earns a place (see open question
   3).

## What the trial and level 1 showed (2026-11-07 to 2026-11-11)

- **The trial cleared its gate.** Over the 87 routines Muse was given in level 1's T40, 84 of 85
  judgeable routines pass on 2 000 generated states each with no hand edit and no hint (98.8% against
  90%). That includes the seven Muse could not take (`game_frame`, 3 026 instructions, among them) and
  `camera_update`, which Muse failed. Muse passed 77 of 85 (90.6%). Owner's go, 2026-11-07.
- **Cost is per kind of instruction, not per routine.** A full run is about 85 s and no model calls. One
  general fix (Ghidra's `abcd`/`sbcd` P-code) moved nine routines at once.
- **No writes into code on level 1's path:** 0 changes over 80 964 frames of the 33 level 1 cases.
- **Ghidra 12.0 is wrong in four places found so far:** `abcd`/`sbcd` P-code, compares setting X, a
  `bra.w` into another function marked `CALL_RETURN`, and an empty relocation table for the Amiga loader.
  The translator corrects each by mnemonic or pattern, never by address.
- **Open from the trial:** `nbcd` unsupported; endless recursion overflows the Rust stack instead of
  stopping as a hang (the step counter is per call); `h0_6852` stops on `write-into-code` on generated
  states the game never reaches; the judge enters Muse ports with a7 four bytes off the oracle; the full
  translated tree takes about a minute to build.
- **Not yet compared:** condition codes and cycles. The trial compared d0-d7, a0-a6, bytes written and
  custom-chip writes in order, Muse's bar.
- **Level 1's cost, for scale:** 1 602M tokens over 86 tasks (50 planned, 36 added), 142 h of wall clock
  (`plans/forest-dumb-level-1/STATS.md`). Muse's own spend was cents (the T40 batch $0.16); the cost was
  the Opus sessions around it, each routine's try-check-fix steps, packs and reviews. The trial's whole
  build was 51M tokens in 1 h 43 m.

## Decided by the owner

- **Option B (2026-11-06): translated first, hand-written where chosen.** For this game, from level 2 on,
  this amends "clean reimplementation, not static recompilation or a hybrid" (`plans/rust-engine/DESIGN.md`
  §7, owner, 2026-10-10). Superfrog's engine is unchanged.
- The shipped game holds no emulator: no 68000 interpreter at run time, the translated code is compiled
  Rust, the custom chips are the toolkit's models (`plans/forest-dumb-level-1/DESIGN.md` §3: the app never
  links `fd-conform` or `amiga-oracle`).
- The faithful bar is unchanged (`knowledge/self-testing-faithful-bar.md`), judged by machine on the oracle.
- Reuse Ghidra before writing anything it already does: its decoding, analysis and P-code (2026-11-06).
  Its decompiled C is not a source.
- A hint (`re/translator-hints.toml`) counts as a hand edit for any gate (2026-11-07).
- Go on the trial (2026-11-07); this plan runs after level 1 merges, which it now has.

## Stages, in order of work

Stage 0 is done. The trial's pieces are the starting point: `ExportPcode.java` and `pcode_export.py`,
`pcode_rust.py`, the `pcode-rt` crate, `fd-judge --translated`, `fd-conform code-writes`,
`translator_trial.py`.

| Stage | Delivers | Proven by |
|---|---|---|
| 1. Instructions | Every opcode, addressing mode and size the program uses (a census from the whole listing), translated, with condition codes, X and cycles; `nbcd` and the four known Ghidra defects first | Each compared with Moira (`toolkit/crates/amiga-oracle/src/routine.rs`) on generated states: registers, condition codes, memory written, cycles |
| 2. Finding the code | Entry points beyond `translate.toml`: program start, the vectors the program installs, `level3_handler`, jump tables, hunk 4; a settled rule for writes into code (`h0_6852`); a call-depth cap | An address reached at run time and not translated stops with `untranslated: hN+$off`, never silently; a write into translated code is reported the same way; endless recursion stops as a hang |
| 3. Runtime | Memory at the oracle's hunk bases, registers, custom-register reads and writes to `amiga-chips` and `amiga-paula`, interrupts at frame points, beam position from counted cycles (raster waits), the `dos.library` calls the game makes answered from `amiga-disk`'s OFS reader | Routine-oracle cases in hardware mode |
| 4. Whole program on the bar | The translated program run from the level sync point and from power-on through `fd-conform` | Passes the level 1 cases the hand engine passes; one case per later level reaches its first pass |
| 5. The seam | A hand-written routine replaces a translated one by address (`fd-core/src/ported.rs` is the list), typed state to memory and back through `LAYOUT` and `Transplant` | Replaced and replacing routine agree on the routine oracle and the cases stay green; level 1's hand engine mounted into the translated program as the first use |
| 6. App | The app runs the translated program with the seam | A test drives the real app (`amiga-e2e`), as in level 1 |
| 7. Default for new sessions | One command a session runs to get a routine's (or a tree's) translation and its judge result; the procedure written where a session meets it first; Muse's procedure demoted | A fresh session given a porting-style task and the repo's docs only reaches for the translator unprompted (a check the planner designs, on the routine oracle, not a person's reading) |

Stage 7 is new in this rewrite and is the reason the owner asked for this brief now. What it covers:

- **The command.** Today the trial is `pcode_export.py` then `translator_trial.py --skip-export`, built
  around the 87-routine trial. A session needs one documented entry, in the shape of `translate.py`'s
  (name a routine or a task, get the Rust path and a pass line), with its README in `tools/README.md`.
- **Where a session learns it.** `games/forest-dumb/CLAUDE.md` (the Notes and Tools lines that point at
  Muse today), `docs/11-translation.md` (its procedure, "Reshaping by Muse" included), `tools/README.md`,
  and `tools/task_pack.py`, which writes the Muse procedure into every porting pack. Each says
  "translator first" after this stage.
- **What a porting task becomes.** Under option B most level work is "check the translated level on the
  oracle, then the owner's chosen rewrites". A rewrite starts from the translated routine (the literal
  first pass Muse used to supply) and is checked against it through the seam.
- **The knowledge tier.** The translator stays a game tool until a second game uses it (root CLAUDE.md,
  rules 1 and 2); its instruction and runtime parts already name no game, so the move is a file move.
  `toolkit/PLAYBOOK.md` and `toolkit/docs/porting-routines.md` mention neither Muse nor the translator
  today; whether they gain a `learned on:` line is open question 4.

## Constraints

- Generated Rust derives from the game's code: written to `games/forest-dumb/build/`, never committed,
  like the listings and Muse's ports. Committed: the translator, its tests on made-up programs, the
  hand-written code, and reports holding names, addresses and verdicts only.
- No model in the translator. A stage may use one only for what the plan decides Muse keeps (question 3),
  under level 1's caps ($0.10 a routine, $5 a batch) and a `DESIGN.md` §5.3 row.
- Purity: `pcode-rt` stays in `PURE` (`fd-conform/tests/purity.rs`); the generated crate is pure in the
  same sense. How the table bar (no committed tables over 16 integers) applies to generated code the app
  links is the planner's to settle with the owner, since the code itself is the game's.
- Session size: packs first and the smaller start (`plans/forest-dumb-level-1/DESIGN.md` §4.4); tasks
  sized to finish well under 250k context. Level 1's lesson (REPORT "Risks and follow-ups"): plan more,
  smaller tasks up front, since its size limit produced 36 added tasks.
- Timing: cycles counted per instruction, judged on the faithful bar; nothing chased below the frame
  unless the plan decides sub-frame timing up front (level 1's other lesson).
- Smoke stays within 15 minutes: the whole-tree build is about a minute, so translated-program tests go
  in full unless they are crucial and fit.

## Open, for the owner to decide in the planning session

Proposals only.

1. **Who can run the result.** An app built from translated code contains the game's code. Proposed:
   built on the owner's machine from the owner's disks and never distributed, as `build/` is today.
2. **Order inside the plan.** Proposed: stage 7 does not wait for stage 6. Once stages 1 and 2 hold
   (every instruction checked, code found, the two stops), the translator is good enough to be the first
   pass for routine work, so stage 7's command and docs can land then, and stages 3 to 6 follow.
3. **What Muse keeps.** Proposed: Muse no longer translates; the translator does it for free and passed
   more. Muse may keep writing test-state generators (`tools/generate.py`, T61: 5 of 5 at $0.014) and
   drafting the readable rewrite of a routine the owner chooses to hand-write (`tools/reshape.py`), now
   from the translated routine. Alternative: retire Muse entirely and drop its key and caps.
4. **Toolkit or game.** Proposed: game tier now; `toolkit/PLAYBOOK.md` step 6 gains one `learned on:`
   line ("translate first, port by choice"), and the move to the toolkit waits for a second game.
   Alternative: build it as a toolkit piece now.
5. **What gets rewritten by hand.** Proposed: each later level plan asks the owner; nothing is rewritten
   by default.
6. **The level 1 hand engine.** Proposed: kept, and mounted through the seam (stage 5) as the first
   hand-written parts, so level 1 stays the readable engine it is and the translated program carries
   everything else.
7. **A second proving ground.** Superfrog's 212 conformance cases could test the translator; a separate
   plan if wanted.

## Out of scope

The boot and crack programs, Superfrog's engine, map editing, levels 2 to 6 themselves (each its own
plan after this one), an app for anyone but the owner.
