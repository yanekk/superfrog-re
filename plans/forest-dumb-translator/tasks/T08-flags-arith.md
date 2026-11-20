# T08 — flags-arith

**Phase:** 1 · **Depends on:** T06 · **Weight:** medium

## Goal

Make every census kind in this family match Moira in registers, condition codes and memory written: add, addx, addq, addi, adda, sub, subx, subq, subi, suba, neg, negx, clr, mulu, muls, divu, divs (including divide-by-zero's trap and overflow), and, or, eor, not and their immediates, ext, tst, chk, as many of them as the census holds. Corrections are by mnemonic or P-code pattern, never by address (DESIGN §2.2), each with a test.

## Design sections this implements

DESIGN §2.2, §2.6.

## Files

- `tools/pcode_fix/arith.py` and its tests; `crates/pcode-rt/src/ops.rs` for a helper the family needs, with unit tests.
- No other `pcode_fix` module: the other families are other tasks, running at the same time.

## Tests

- [ ] Every census kind in the family passes `insn_check.py --kinds <family> --columns regs,ccr,mem` on 500 states.
- [ ] `addx`/`subx`/`negx` keep Z when the result is zero (Z only cleared, never set).
- [ ] `divu` overflow sets V and leaves the destination unchanged.
- [ ] Division by zero raises the trap stop the machine handles, not a panic.
- [ ] Each correction has a made-up-encoding test that fails without it.

## Done when

- [ ] `insn.md` shows the family at 0 fails in regs, ccr and mem; the counts before and after are in FINDINGS.
- [ ] The trial's 84 routines still pass (`translator_trial.py --skip-export`).
