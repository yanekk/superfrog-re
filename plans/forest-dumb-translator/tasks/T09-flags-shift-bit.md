# T09 — flags-shift-bit

**Phase:** 1 · **Depends on:** T06 · **Weight:** medium

## Goal

Make every census kind in this family match Moira in registers, condition codes and memory written: asl, asr, lsl, lsr, rol, ror, roxl, roxr (register and memory forms, counts 0, 1..8 and by register up to 63), btst, bset, bclr, bchg, scc, tas, swap, as many of them as the census holds. Corrections are by mnemonic or P-code pattern, never by address (DESIGN §2.2), each with a test.

## Design sections this implements

DESIGN §2.2, §2.6.

## Files

- `tools/pcode_fix/shift_bit.py` and its tests; `crates/pcode-rt/src/ops.rs` for a helper the family needs, with unit tests.
- No other `pcode_fix` module: the other families are other tasks, running at the same time.

## Tests

- [ ] Every census kind in the family passes `insn_check.py --kinds <family> --columns regs,ccr,mem` on 500 states.
- [ ] Shift by a register count of 0 clears C and keeps X; `roxl` by 0 copies X to C.
- [ ] `asl` sets V when the sign changes at any step.
- [ ] `btst` on Dn uses bit mod 32, on memory mod 8.
- [ ] Each correction has a made-up-encoding test that fails without it.

## Done when

- [ ] `insn.md` shows the family at 0 fails in regs, ccr and mem; the counts before and after are in FINDINGS.
- [ ] The trial's 84 routines still pass (`translator_trial.py --skip-export`).
