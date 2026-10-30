# Reuse for another game: toolkit history and backlog

Moved from Claude's memory 2026-10-30; checked against toolkit/, plans/ and the plan reports.
Corrected: the extraction the note proposed is done (plans/toolkit-extract: 11 tasks, all ✅), the
keyboard run and one-clock are done, and the crates it names were renamed or moved, listed below.

## What was measured

The owner asked 2026-10-15 what carries over to another game and wants it remembered.

- learned on: Superfrog, measured on `pir/rust-engine`, ~75k lines of Rust.
- learned on: Superfrog. Reusable for any Amiga game (~13k lines, ~20%): `sf-oracle` (vAmiga headless harness + Moira routine runner, ~3.7k), now amiga-oracle.
- learned on: Superfrog. `sf-paula` (Paula mixer, ~1.1k), now amiga-paula; `sf-conform/src` (cases, scripts, faithful checks, plays-alone, reports; some sync points are game-specific, ~5.5k), its game-free half now amiga-conform.
- learned on: Superfrog. `sf-app` shell (window, PAL pacing, audio, input, capture, ~1.7k), now amiga-app; `sf-e2e` (~1.1k), now amiga-e2e.
- learned on: Superfrog. Parts of `sf-disk`/`sf-assets` (ADF/zip/SHA-1, cache, launch) and `sf-core/src/render` (OCS bitplanes, copper, sprites) ~2k, now amiga-disk and amiga-chips.
- learned on: Superfrog. Per game (~80%): `sf-core` game logic (38k), asset formats incl. the custom trackloader and `ATN!` cruncher (~3.5k; other Team 17 games *might* share ATN!/T7MP: unchecked), all conform cases/scripts/route planners (~19.5k), the `LAYOUT` memory map.

**Also carries**: the pir method, the faithful bar (frame-exact cost ~5 days; start a new game on
faithful, see knowledge/self-testing-faithful-bar.md), oracle/RE setup scripts, and days saved by
having the harness from day one. Porting effort still scales with the new game's size.

## What was decided

**Decided 2026-10-23..27**: keep the Amiga-shaped engine (not a native rewrite) and prepare a toolkit
first; plan `plans/amiga-toolkit-prep/` (drafts + NORTH-STAR readiness questionnaire, `measure.sh`).
amiga-chips (draft 4) built 2026-10-27, then the keyboard single run, one-clock and toolkit-extract;
draft 6 is this monorepo.

## Backlog: game two

- learned on: Forest Dumb Forever. Game two (owner, 2026-10-27): © 1998 L.K. Avalon / Eagle Soft, a similar platformer.
- learned on: Forest Dumb Forever. Disks: `~/Documents/Amiga/Forest Dumb Forever!!! (1996)...(Disk N of 2)[cr WT].zip`, a cracked set (files identical to the WHDLoad package; crack adds a boot intro). Never committed, like the first game's.
- learned on: Forest Dumb Forever. Hand-written 68k, RNC-packed exe, AmigaDOS floppy: needs an OFS file reader in the toolkit (RNC was only the WHDLoad packing).

**How to apply:** when the owner talks about the next game or the toolkit, start from
plans/amiga-toolkit-prep/NORTH-STAR.md.
