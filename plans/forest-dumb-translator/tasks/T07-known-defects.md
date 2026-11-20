# T07 — known-defects

**Phase:** 1 · **Depends on:** T06 · **Weight:** medium

## Goal

Turn the trial's four Ghidra 12.0 defects into instruction checks against Moira, and add `nbcd`, which the trial left unsupported. The trial corrected them by pattern but checked only the routines' registers and memory; this proves the corrections at the instruction level, flags included.

## Design sections this implements

DESIGN §2.2, §2.6.

## Files

- `tools/pcode_fix/bcd.py`, `tools/pcode_fix/compare.py`, `crates/pcode-rt/src/bcd.rs` (`nbcd`), tests beside each.
- `tools/test_insn_check.py` cases for the four defects.

## Tests

- [ ] `abcd`/`sbcd` Dn and `-(An)` forms, `nbcd` Dn and memory forms: every census encoding passes all columns but cycles.
- [ ] `cmp`, `cmpa`, `cmpi`, `cmpm` leave X unchanged on states with X set and clear.
- [ ] The `bra.w` into another function marked `CALL_RETURN` is translated as a branch (made-up program).
- [ ] Relocations: a made-up hunk file's `HUNK_RELOC32` entries are read from the file.

## Done when

- [ ] Every census kind in the bcd and compare families passes regs, CCR and memory.
- [ ] `unsupported` is 0 for `nbcd` in the whole-program translation.
