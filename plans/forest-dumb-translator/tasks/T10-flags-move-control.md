# T10 — flags-move-control

**Phase:** 1 · **Depends on:** T06 · **Weight:** medium

## Goal

Make every census kind in this family match Moira in registers, condition codes and memory written: move, movea, moveq, movem (both directions, every mode the census holds), lea, pea, link, unlk, exg, move to and from SR and CCR, move usp, jmp, jsr, bsr, rts, rte, rtr, dbcc, bcc, trap, as many of them as the census holds. Corrections are by mnemonic or P-code pattern, never by address (DESIGN §2.2), each with a test.

## Design sections this implements

DESIGN §2.2, §2.6.

## Files

- `tools/pcode_fix/move_control.py` and its tests; `crates/pcode-rt/src/ops.rs` for a helper the family needs, with unit tests.
- No other `pcode_fix` module: the other families are other tasks, running at the same time.

## Tests

- [ ] Every census kind in the family passes `insn_check.py --kinds <family> --columns regs,ccr,mem` on 500 states.
- [ ] `movem` to `-(An)` stores in reverse order and the written An is the decremented one.
- [ ] `move.b` to and from `(a7)+` and `-(a7)` steps 2.
- [ ] `move to SR` and `rte` restore the interrupt mask; `move to CCR` touches only the low byte.
- [ ] Each correction has a made-up-encoding test that fails without it.

## Done when

- [ ] `insn.md` shows the family at 0 fails in regs, ccr and mem; the counts before and after are in FINDINGS.
- [ ] The trial's 84 routines still pass (`translator_trial.py --skip-export`).
