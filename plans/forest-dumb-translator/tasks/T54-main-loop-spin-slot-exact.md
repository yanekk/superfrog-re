# T54 — main-loop-spin-slot-exact

**Split from:** T50 (owner, 2026-11-19); carries T50's "All 18 cases pass" for `route-W1-1` and `route-both-W1-1`.

**Phase:** 5 · **Depends on:** T50; blocks T42 · **Weight:** heavy

## Goal

Make `route-W1-1` and `route-both-W1-1` pass with `--engine translated`. On T50's machine (interrupts at instruction boundaries) both pass from the sync point to pass 4160 and first differ at pass 4161 (oracle frame 10463): `edge_at_down_x` 4 lower on the engine, 8 lower a pass later on both.

What T50 measured (temporary probes, not committed: the oracle's line samples under `set_profile`, the machine's function entries and pass ends by beam):

- Frames 10459 to 10461: after each pass both sides spin in `main_loop`'s wait (`h0+$252` `move.w irq_count,d0`, `andi.l #$1f,d0`, `bne.w`, `$2a0` `tst.w`, `bne.w`, `$2aa` `bra.w`) from about line 141 to the pass at line 239, some 500 iterations.
- The pass at line 239 of frame 10460 meets that loop at a different instruction on the two sides. On the original `$252` then reads the new `irq_count`, whose low five bits are 0, and runs the status redraw (`h0_6a96`, lines 157 to 193 of frame 10461) before the edge redraws, which the next pass cuts. The machine had made that read already, so it runs the edge redraws first (`h0+$df6`, lines 157 to 229) and the status redraw from line 233. The redraws' progress then differs at the pass end.
- The machine's bus is within 6 slots of the oracle's on `tests_dma.rs`' readings (10 000 to 21 000 slots); over this spin a few slots move the interrupt to another instruction. The loop's own reading has not been taken.

Measure the spin's slots on the oracle (a reading as `amiga-oracle/tests/contention.rs` takes them, with the game's display and its sprites, audio and copper as in frame 10460), compare with the machine, and model what differs generally (DESIGN §2.2).

**For T36 (owner, 2026-11-19):** both cases pass translated up to pass 4161, so with level 1 mounted any difference before that pass is the seam's, and one from it on is this task's.

## Design sections this implements

DESIGN §2.2, §2.7, §2.8.

## Files

- `crates/pcode-machine` (`dma.rs`, `lib.rs`), `crates/pcode-rt` (`cycles.rs`), `tools/pcode_fix/cycles.py`, wherever the cause is; fixtures regenerated as T49 did.
- Case files only for notes; a case's checks are never loosened.

## Interface

```
cargo run -p fd-conform -- run cases/route-W1-1.toml --engine translated
cargo run -p fd-conform -- run cases/route-both-W1-1.toml --engine translated
```

## Tests

- [ ] A machine unit test of the spin loop's slots, its value measured on the oracle first (as `tests_dma.rs`' readings).
- [ ] Full: both route cases pass with `--engine translated`, and the other 40 level cases T50 ran still do.

## Done when

- [ ] Both route cases pass on the translated program; every fix is general (DESIGN §2.2).
- [ ] T50's `T50 (2026-11-19)` note is gone from both case files.
- [ ] The routine gate on the machine (`translator_trial.py --whole --machine`) is no worse than 89 of 100, `MACHINE_PINNED` updated where a pin moves.
