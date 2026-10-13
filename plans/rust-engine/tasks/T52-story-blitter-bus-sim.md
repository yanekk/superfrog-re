# T52 — story-blitter-bus-sim

**Phase:** 2 · **Depends on:** T51; blocks T53 · **Weight:** medium

**Split from:** T46.

## Goal

`front::story::sim`: the CPU and the blitter on the bus, cycle by cycle, as vAmiga runs them, checked
against the oracle's BLTSIZE writes and blitter events (T51).

## Model (T46 draft, discarded; rebuild from this)

- A cycle goes to the fixed DMA (refresh at $01, $03, $05, $E2; the copper's cycles from
  `copper::run_bus`, which T46 committed; the bitplanes, `Fetch::takes`), then the blitter, then the CPU.
- Blitter, vAmiga `SlowBlitter` level 2: BLTSIZE written at cycle w, the register lands at w+1,
  `BLT_STRT1` at w+2 (sets busy; waits for a cycle no DMA has, without taking it), `STRT2` the
  same, the program from the cycle after. Programs per word (no fill): F `A B C D`, D `A B D`, 9
  `A D`; D's write is locked on the first word (needs a free cycle, takes none). Busy clears at the
  last word's last step; then `HOLD_D` and D's last write (needs the bus), then the end.
- With BLTPRI set the blitter never yields; without it, to a CPU that has waited two cycles (BLS).
- CPU: `cpu.rs`'s access letters; an access waits for a free cycle. DMACON lands two cycles after
  its write.

## Interface

`front::story::sim` (new): the bus model T53's `sim::Prog` runs on.

## Files

`engine/crates/sf-core/src/front/story/` (`sim.rs` new, `cpu.rs`); `render/copper.rs`.

## Tests

- [ ] Unit tests of the programs' cycle counts on a free bus (ABCD 4n+..., AD 2n+...).
- [ ] An oracle test: the scene-1 masks' blits (frames 532–536, BLTSIZE writes known) and a
  restore pass start and end on the original's cycles.

## Done when

- [ ] The model's blit begin/end times match the oracle's for scene 1's masks and one draw's
  restores, given the original's BLTSIZE times.
