# T52 — camera-bottom-right-pass-477

**Split from:** T49 (owner, 2026-11-18); carries T49's "Every case above passes" for `camera-bottom-right`.

**Phase:** 5 · **Depends on:** T49; blocks T36 · **Weight:** heavy

## Goal

Make `camera-bottom-right` pass with `--engine translated`. With T49's DMA contention it passes its old first difference (pass 93) and every variable, Paula write and custom write over all 2 164 passes, and fails the picture at pass 477 (oracle frame 6779): three glitch frames in a row (6777 to 6779, 331, 228 and 106 pixels), first at (224, 17), where a flying creature is whole on the engine and missing or half-drawn on the original.

What T49 measured (temporary probes, not committed; the commit message of T49's last code commit names them):

- The original's 13th blit of the bob loop starting at line 201 of frame 6777 falls after the line 233 interrupt, which cuts it; the machine starts it at (231, 4), before. Every blit's begin and end on both sides agree to within a few slots up to line 111.
- The machine runs ahead over the frame: its pass end (`INTREQ=$4010`) is a median 42 slots early over the case, up to 450. In frame 6777 the gap grows from 207 to 630 slots over lines 168 to 200, about 6%, in the main loop after `level3_handler`'s 7-line wait (`redraw_edge_a`'s loop calling `h0_7cc`, then the scan at `h0+$98a`).
- Over that stretch both sides give the CPU the same bus share per line (84 to 91 accesses a line, the oracle's `set_dma_profile` owners against the machine's), so the original makes about 220 more accesses (2 763 against 2 539) in it. `h0_7cc` runs 18 times on the machine, 19 hits on the oracle's step; only levels 2 and 3 interrupt (docs/15). The cause is not found.

Find what the original's CPU does in those lines that the machine does not count, and model it generally. Then re-measure what T30's pass slack still has to admit: with `frames::PASS_LAG` 0, T49 saw 10 of the 24 cases fail on a pass ending a frame apart, both ways; `custom_carried` was not re-measured.

## Design sections this implements

DESIGN §2.2, §2.7, §2.8.

## Files

- `crates/pcode-machine` (`dma.rs`, `lib.rs`), `crates/pcode-rt` (`cycles.rs`), `tools/pcode_fix/cycles.py`, wherever the cause is; fixtures regenerated as T49 did.
- Case files only for notes; a case's checks are never loosened (owner, 2026-11-18: the difference is not accepted).

## Interface

```
cargo run -p fd-conform -- run cases/camera-bottom-right.toml --engine translated
```

## Tests

- [ ] A machine unit test of what was missing, its value measured on the oracle first (as `tests_dma.rs`'s readings).
- [ ] Full: `camera-bottom-right` passes with `--engine translated`, and T49's 23 other cases still do (`camera-*`, `character-*`, `stretch-*`, `picture-*`, `w1-idle`, `stub-level`).

## Done when

- [ ] `camera-bottom-right` passes on the translated program; every fix is general (DESIGN §2.2).
- [ ] The routine gate on the machine (`translator_trial.py --whole --machine`) is no worse than T49's 89 of 100, `MACHINE_PINNED` updated where a pin moves.
- [ ] What `PASS_LAG` and `custom_carried` still admit is measured and written in FINDINGS; changing that rule is the owner's.
