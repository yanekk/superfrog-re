# T27 — routine-oracle-hw

**Phase:** 4 · **Depends on:** T16, T23, T24, T25 · **Weight:** medium

## Goal

Judge translated routines on the machine instead of the judge's hand-written hardware copy (`judge/mem.rs`), so their hardware effects (blits, copper, Paula, beam waits) come from the models the app will use. The brief's stage 3 proof: routine-oracle cases in hardware mode.

## Design sections this implements

DESIGN §2.8, §4.

## Files

- `fd-conform/src/judge/translated.rs` (`--machine`: `impl Bus` from `pcode-machine`), `fd-judge.rs`, `tools/translator_trial.py --machine`.

## Tests

- [ ] Full: every judgeable routine passes on the machine as on `judge::Mem`; any routine that differs is explained (a model difference) and fixed in the machine.
- [ ] Hardware reads the case gives (`hw_reads`) are fed to the machine's devices identically.

## Done when

- [ ] The whole-tree judge passes with `--machine`; the reading is in `docs/14-translator.md`.
