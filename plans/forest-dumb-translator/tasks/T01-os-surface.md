# T01 — os-surface

**Phase:** 0 · **Depends on:** — · **Weight:** medium

## Goal

Measure on the oracle every place the program leans on the Amiga's OS, so T26 answers exactly those and nothing is guessed. The program loads every file through `dos.library`, the OS's interrupts and keyboard handler are live through the title, and `system_on`/`system_off` hand the machine back and forth (`docs/07-executable-map.md`).

## Design sections this implements

DESIGN §2.8 (the OS), §2.1.

## Files

- `games/forest-dumb/engine/crates/fd-conform/src/os_surface.rs` and a `fd-conform os-surface` subcommand (new).
- `games/forest-dumb/re/os-calls.toml` (new, committed: library names, vector offsets, call sites as `hN+$off`, counts; no listing text).
- `games/forest-dumb/docs/15-os-surface.md` (new), indexed in `games/forest-dumb/CLAUDE.md` Notes.

## Interface

```
fd-conform os-surface [--frames N] [--worlds 1,2]   # writes re/os-calls.toml (sorted, stable)
[[call]]  library = "dos.library"  lvo = -30  name = "Open"  sites = ["h0+$8ee0"]  count = 84
[[read]]  what = "ExecBase"  addr = "$4"  sites = [...]
[[irq]]   level = 3  owner = "os" | "program"  when = "title" | "play"
[[cia]]   reg = "ciaa_sdr"  sites = [...]
```

## How

Run the oracle from power-on through the title, Start, level 1's load and its first frames, and a world 2 load (poke `world_index` before `load_world`, the same poke T33 uses), with a PC watch: every transfer from a program hunk to an address outside the hunks (a ROM entry), with `a6` and the vector offset; every read of OS memory (`$4`, library bases, structures off them); every interrupt taken and whose handler ran; every CIA access by the program. Name each library call by its LVO.

## Tests

- [ ] `os_surface` unit tests on a made-up program calling a fake library vector table: call, LVO and site found.
- [ ] Full: the run on the game writes the same `re/os-calls.toml` twice (stable order).
- [ ] The four file routines of `docs/07` appear as `Open`/`Read`/`Close` sites.

## Done when

- [ ] `re/os-calls.toml` and `docs/15-os-surface.md` are committed; the doc says, per call, what the program relies on its result for.
- [ ] Anything surprising (a call not in `docs/07`, OS code the program jumps into for a non-library reason) is a FINDINGS row.
