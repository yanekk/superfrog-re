# North star — are we ready to extract the Amiga toolkit?

Answered once now (baseline) and again after each draft lands. The session that finishes a step
runs the measured questions and appends a column to the log; the owner answers the two judgement
questions in plain words, and the session writes the answers down with the date.

**Ready** when all of these hold at once:

1. M1 is green.
2. Every M2 row for a piece that goes into the toolkit is at its target: keyboard, blitter memory
   effect, BLTSIZE decode, chip memory, `is_paula`, frame/line constants, beam position.
   The hunk-0 row is wanted but not required, since the relocator can be lifted with one caller.
3. M4 lists at least three passing crates: `sf-paula`, a chips crate (draft 4) and a clock crate
   (draft 2).
4. J1 is yes: game two chosen (yes, 2026-10-27) and its disk images at hand (yes, a cracked
   release, 2026-10-27).
5. J2 is "extract now".
6. (Proposed 2026-10-24, owner to confirm.) The dry fit in `README.md`'s readiness plan passes.

M3 and M5 are tracked but gate nothing.

## Measured questions

Run `sh plans/amiga-toolkit-prep/measure.sh` from the repo root for M2–M5. M1 is the checks.

| # | Question | How | Target |
|---|---|---|---|
| M1 | Does the engine still play exactly as the original after the step? | `cd engine && cargo test -q --workspace`; `cargo run -q --release -p sf-conform -- all` (~2 h); sf-e2e `whole_game.rs` | all green, no tolerance changed |
| M2 | How many copies of each Amiga piece are left? | `measure.sh` "M2" rows | see the targets table below |
| M3 | How many places still read the original's data tables by address through the front-end and sound readers? | `measure.sh` "M3" | lower after draft 1; 0 only with its full scope |
| M4 | Which crates could move into a toolkit today, depending on nothing of Superfrog's (toolkit crates may depend on each other) and naming nothing of it in code? | `measure.sh` "M4/M5" (`PASS` lines) | ≥ 3 |
| M5 | How many lines sit in those crates? | the `PASS` lines' counts | grows with each step |

Targets for the M2 rows:

| Row | Target | Moved by |
|---|---|---|
| kbd_poll definitions | 1 | draft 3 |
| key-byte encode sites | 1 | draft 3 |
| blitter memory-effect implementations | 1 | draft 4 |
| BLTSIZE decodes | 3: the helper plus `story/bobs.rs`'s two ported lines, which compose a size rather than decode one | draft 4 |
| chip-memory containers | 2: one `AmigaMemory`, and `StoryMem` as a newtype over it | draft 4 |
| `is_paula` definitions | 1 | draft 4 |
| frame/line length constants | ≤ 3, all in the clock crate; the probes and tests import them | draft 2 |
| beam-position types and functions | each hit left is a client of one `BeamPos`; checked by reading the hits | draft 2 |
| hunk-0 loads in sf-core | 1 | draft 1 |

## Judgement questions (owner)

| # | Question |
|---|---|
| J1 | Is game two chosen, and are its disks at hand? A toolkit cut against one game is a guess at what the second needs; with the second game's disks, its first loader and oracle run can test the cut. |
| J2 | Given the next step's size (its draft) and what the toolkit would hold today (M4), is the toolkit worth another step, or should it be extracted now? Answer "extract now", "one more step: N", or "stop". |

**J2 after amiga-chips, owner 2026-10-27: one more step, the keyboard (draft 3 A+B) and the clock
(draft 2); then extract (draft 5).** Note for that column: amiga-chips merged with pir's final test
run red (app tests timing out in the full suite) and its slow sf-conform runs unrecorded
(`plans/amiga-chips/REPORT.md`); M1 for that step is unproven until they are rerun.

**J1, owner 2026-10-27: chosen; disks at hand, a cracked release (see the last bullet).** Game two is *Forest Dumb Forever*
(© 1998 L.K. Avalon, developer Eagle Soft; a Superfrog-like platformer), named via the WHDLoad
package `ForestDumbForever_v1.1_Pl.lha` (whdownload.com). Inspected 2026-10-27, nothing run and
nothing kept in the repo (copyrighted, like Superfrog's disks):

- Main program `data/FOREST_DUMB_FOREVER` in the WHDLoad package is RNC ProPack method 1 packed
  (119 132 → 521 640 bytes; the packing is the WHDLoad install's, the disk stores it unpacked),
  an AmigaDOS hunk executable with 5 hunks (code 376 424 bytes; four chip hunks, 368 040 bytes).
  No AMOS or Blitz signatures; about 400 absolute custom-chip references (`$dff0xx`/`$dff1xx`):
  hand-written 68k hitting the hardware, so the routine-port-and-compare method applies.
- Loads through `dos.library` from a floppy labelled `!EAGLE_SOFT!` (AmigaDOS files, not a
  trackloader); uses `graphics.library` and `topaz.font`. Needs 368 KB chip and 0.5 MB other
  memory, which the oracle's A500 configuration provides.
- Six worlds of level files (`_Mapa1-1` … `_Mapa6-1` with `.Mapa`, `_Gen*`, `_MapaBobow*`).
- The WHDLoad install patches the game (protection and disk check removed, custom-register writes
  and blitter waits fixed, files relocated). It is not the original; the oracle needs the original
  floppy image (`.adf`), which the owner supplies outside the repo.
- Owner's disk set, 2026-10-27 (outside the repo, `~/Documents/Amiga`): `Forest Dumb Forever!!!
  (1996)(L.K. Avalon)(PL)(Disk 1 of 2)[cr WT].zip` and `(Disk 2 of 2)`, two 901 120-byte OFS
  `.adf`s, both volumes `FOREVER`. TOSEC `[cr WT]`: a cracked release. Disk 1 boots (OS 1.3 boot
  block) a `startup-sequence` of `Show -o Kwach4`, `BillClintonToDupek` (the crack's picture and
  intro) and `forest_dumb_forever`; it also holds `Installer`, `Instaluj`, `f.s`. Disk 2 holds the
  data and the `!EAGLE_SOFT!` marker file. `FOREST_DUMB_FOREVER` (521 640 bytes, stored unpacked)
  and all 48 data files are byte-identical to the WHDLoad package's (its exe after RNC unpacking),
  so the WHDLoad install is built on this release and patches at run time. Whether the crack also
  changed the executable cannot be told without an uncracked image.

What it means for the toolkit: none of the Team 17 "might share" items (ATN!, T7MP, Player 4.1A)
apply. It needs one piece Superfrog never did: an AmigaDOS (OFS) file reader for `.adf` images,
since the game loads files through `dos.library` (sf-disk has none; amitools' `xdftool`, already in `requirements.txt`, reads
OFS for RE work). Those belong to the extraction plan, not to drafts 1–4.

## Log

Newest column on the right. A step's column is filled by its plan's last task (or its single run),
on its branch, with the values measured on that branch; they are the values that merge.

| Question | Baseline 2026-10-23 (`e196a2e`) | amiga-chips 2026-10-27 (`pir/amiga-chips-T02`) |
|---|---|---|
| M1 | not run by this session; last full `sf-conform all` was rust-engine T217 | DESIGN §4 sf-conform runs green (41 cases incl. `complete-w1-1`; story ×9; story music) and a regression subset of the test block; the whole block runs at the plan's end; `sf-conform all` not run |
| M2 kbd_poll | 2 | 2 |
| M2 key-byte encode | 4 | 4 |
| M2 blitter memory effect | 4 (`Blit::run`, `objects::blit`, `scroll::blit`, `blit_panel`) | 1 (`Blit::run`) |
| M2 BLTSIZE decodes | 9 | 3 |
| M2 chip-memory containers | 3 (`AmigaMemory` ×2, `StoryMem`) | 2 (`AmigaMemory`, `StoryMem`) |
| M2 `is_paula` | 2 | 1 |
| M2 frame/line constants | 12 (8 in sf-core, 4 in sf-conform probes and tests) | 12 (2 now in amiga-chips) |
| M2 beam-position types | 6 (`Clock`, `Beam`, `Cx` ×2, slot `spend`, `sim::beam`) | 5 (copper `Beam` moved to amiga-chips; measure.sh scans sf-core only) |
| M2 hunk-0 loads | 4 | 4 |
| M3 | 52 | 52 |
| M4 | 1 (`sf-paula`) | 2 (`sf-paula`, `amiga-chips`) |
| M5 | 838 | 1575 (838 + 737) |
| J1 | not asked yet | not asked |
| J2 | not asked yet | not asked |
