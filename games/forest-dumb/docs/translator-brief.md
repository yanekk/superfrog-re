# Brief for `pir plan`: the translator

Written 2026-11-06 for the planner. Slug: `forest-dumb-translator`. The sequence around this plan is in
[ROADMAP.md](ROADMAP.md). Read that, `games/forest-dumb/CLAUDE.md`, [level-1-brief.md](level-1-brief.md)
("Translation by Muse, and session size"), `plans/forest-dumb-level-1/DESIGN.md` §4.3 and §4.4,
[11-translation.md](11-translation.md), `toolkit/docs/porting-routines.md`, `toolkit/PLAYBOOK.md` steps 5
and 6, and [../spikes/muse-spark/README.md](../spikes/muse-spark/README.md) first. Several of these are on
the `pir/forest-dumb-level-1` branch until that run merges.

## The goal

A static translator: a tool that reads the main program `FOREST_DUMB_FOREVER` and writes Rust that runs
it instruction for instruction, with no model in the loop per routine. The translated program is the
starting engine for everything after level 1 (levels 2 to 6, the title and menus, sound, the ending).
Hand-written, readable Rust replaces translated code where the owner chooses, each replacement checked
against the translated code it replaces.

## Decided by the owner (2026-11-06)

- **Option B: translated first, hand-written where chosen.** Translated code is the engine each new part
  starts from; the owner picks which parts are rewritten by hand. For this game, from level 2 on, this
  amends "clean reimplementation, not static recompilation or a hybrid" (`plans/rust-engine/DESIGN.md` §7,
  owner, 2026-10-10). Superfrog's engine is unchanged.
- The shipped game still holds no emulator: no 68000 interpreter at run time, the translated code is
  compiled Rust, the custom chips are the toolkit's models as today (`plans/forest-dumb-level-1/DESIGN.md`
  §3: the app never links `fd-conform` or `amiga-oracle`).
- The faithful bar is unchanged (`knowledge/self-testing-faithful-bar.md`), judged by machine on the oracle.
- The level 1 run (`forest-dumb-level-1`) finishes as planned. This plan does not change it.

Options A (translator as a checking tool only) and C (translated code as the shipped engine, hand-written
only to change something) were weighed and not taken.

## Why: what level 1 measured

From the level 1 run's conversation logs, 2026-11-06, 51 of 70 tasks done:

- 919M tokens, about $410 at list price; 98.1% of tokens are cache reads, so a step's price is the context
  it re-sends.
- Builders 717M tokens and $305, reviewers 191M and $95. The 17 builder sessions that passed 250k context
  cost $150 of the builders' $305. No session reached the 500k warn line (largest 372k).
- With Muse translating, Opus's cost per ported routine stayed at $0.89 to $1.56; builders spent 41% of
  their cost before their first edit (DESIGN §4.4).

The remaining cost is the number of try-check-fix steps per routine. A translator's cost is per kind of
instruction (the 68000's set is fixed and documented), not per routine, and it is paid once.

## Stages, in order of work

| Stage | Delivers | Proven by |
|---|---|---|
| 0. Trial (gate) | Translate `game_frame`'s call tree (about 8 300 instructions with callees, the spike's scan) | Every routine judged on the routine oracle with the existing generators (`fd-conform/src/gen/`); a table of routines passing with no hand edit, unresolved jumps, writes into code. The owner sees it and says go before stage 1 |
| 1. Instructions | Every opcode, addressing mode and size the program uses (a census from the listing), translated | Each compared with Moira (`toolkit/crates/amiga-oracle/src/routine.rs`) on generated states: registers, condition codes, memory written, cycles |
| 2. Finding the code | Entry points (program start, the vectors the program installs, `level3_handler`), branches followed, jump tables, the code in hunk 4 | An address reached at run time and not translated stops with `untranslated: hN+$off`, never silently; a write into translated code is reported the same way |
| 3. Runtime | Memory laid out at the oracle's hunk bases, registers, custom-register reads and writes to `amiga-chips` and `amiga-paula`, interrupts at frame points, beam position from counted cycles (raster waits), the `dos.library` calls the game makes answered from `amiga-disk`'s OFS reader | Routine-oracle cases in hardware mode, as the Muse judge's round 2 |
| 4. Whole program on the bar | The translated program run from the level sync point and from power-on through `fd-conform` | Passes the level 1 cases the hand engine passes; one case per later level reaches its first pass |
| 5. The seam | A hand-written routine replaces a translated one by address (`fd-core/src/ported.rs` is the list), typed state to memory and back through `LAYOUT` and `Transplant` | The replaced and the replacing routine agree on the routine oracle and the cases stay green; level 1's hand engine mounted into the translated program as the first use |
| 6. App | The app runs the translated program with the seam | A test drives the real app (`amiga-e2e`), as in level 1 |

The trial goes first because everything after rests on it, and it is cheap: the routine oracle, the
generators and the judge already exist.

## Constraints

- Generated Rust derives from the game's code. It is written to `games/forest-dumb/build/` and never
  committed, like the listings and Muse's raw ports. What is committed is the translator, its tests and
  the hand-written code.
- Tier: per root CLAUDE.md ("Before building something"), the translator is built in the game first and
  moved to the toolkit when a second game uses it. Write its instruction and runtime parts naming no
  game so the move is a file move. The planner may propose the toolkit from the start; the owner decides.
- Decoding: `toolkit/tools/hunk_listing.py` already uses capstone; a Rust decoder of its own is the
  alternative. The planner settles it; stage 1's per-instruction test makes either safe.
- Purity: the generated crate is pure in the sense of `fd-conform/tests/purity.rs` (no clock, file,
  float, seeded hash). How the table bar applies to generated code is the planner's to settle with the
  owner, since the code itself is the game's.
- Session size: follow `plans/forest-dumb-level-1/DESIGN.md` §4.4 (packs first, the smaller start). Size
  tasks to finish well under 250k context; steps past 250k were 21% of level 1's tokens.
- Timing: cycles counted per instruction, judged on the faithful bar; nothing chased below the frame.

## Open, for the owner to decide in the planning session

Proposals only.

1. **Who can run the result.** An app built from translated code contains the game's code. Proposed: it is
   built on the owner's machine from the owner's disks and never distributed, as `build/` is today. An app
   for other owners of the disks would have to translate on their machine, which needs a Rust compiler
   there; that is a later plan if wanted.
2. **The trial's gate.** Proposed: go when at least 90% of the trial's routines pass with no hand edit and
   no code in the level's path writes into code.
3. **Order.** Proposed: this plan runs after `forest-dumb-level-1` merges and before level 2, and the level
   plans after it become "check the translated level on the oracle, then the owner's chosen rewrites".
4. **What gets rewritten by hand.** Proposed: each later level plan asks the owner which parts; nothing is
   rewritten by default.
5. **A second proving ground.** Superfrog's 212 conformance cases could test the translator later; a
   separate plan if wanted.

## Out of scope

The boot and crack programs, Superfrog, map editing, changes to the level 1 run, an app for anyone but
the owner.
