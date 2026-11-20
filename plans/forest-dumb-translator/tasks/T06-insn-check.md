# T06 — insn-check

**Phase:** 1 · **Depends on:** T04, T05 · **Weight:** heavy

## Goal

The rig that compares each census instruction kind, translated alone, with Moira's step on generated states, in every column: registers, X N Z V C, bytes written, cycles. Every later stage 1 task fixes what it reports. It also sets up `tools/pcode_fix/`, so the four fix tasks after it each edit their own file.

## Design sections this implements

DESIGN §2.2, §2.6, §7 (`pcode_fix/`).

## Files

- `fd-conform/src/bin/fd-judge.rs` (`--insn`), `fd-conform/src/judge/insn.rs` (new).
- `tools/pcode_rust.py`: `--insn` mode emitting one function per encoding that runs one instruction and returns; `cycles` column reads `Regs` cycle counter (zero until T11, reported `not-counted`).
- `tools/pcode_fix/__init__.py`, `tools/pcode_fix/{bcd,compare,arith,shift_bit,move_control,cycles}.py` (new, the trial's two corrections moved into `bcd.py` and `compare.py`).
- `tools/insn_check.py`, `tools/test_insn_check.py` (new).

## Interface

```
insn_check.py [--kinds add.w,...] [--states 500] [--columns regs,ccr,mem,cycles]
# build/translator/insn.json: [{"kind","encodings","pass","fail","first":{"state","column","moira","translated"}}]
# build/translator/insn.md: one row per kind, sorted by fail count
fd-judge --insn <encoding-hex> --at 'hN+$off' --states N
```

States: random d/a registers, CCR with X random, memory around every effective address seeded, a7 in RAM. Operand memory is placed so no write reaches the instruction.

## Tests

- [ ] On made-up encodings (`moveq`, `add.l`, `abcd`): the rig passes a right translation and fails a deliberately wrong flag with the column named.
- [ ] `pcode_fix` loads every module and the trial's corrections still apply (the trial's 84 still pass).

## Done when

- [ ] `insn_check.py` runs the whole census and writes `insn.md`; the pass and fail counts per column are in FINDINGS.
- [ ] The fail list is grouped by the families T07 to T12 own.
