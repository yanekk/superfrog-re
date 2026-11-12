# T43 — call-fallthrough

**Phase:** 2 · **Depends on:** T03; blocks T16, T28 · **Weight:** medium

## Goal

The whole-program export keeps the code after every call: Ghidra 12.0 marks `load_file` (`h0+$8ec4`) non-returning, so each of its 8 callers, hunk 4's load lists (`h4+$19fda`, `$1a0f0`, `$1a2d8`, `$1a94c`, `$1afbe`, `$1b630`, `$1bca2`, `$1c314`), loses its fall-through, and power-on stops `untranslated: h4+$1a0f6` right after `_NAPIS` loads (found by T26, FINDINGS 2026-11-12). On the original every one of them returns.

**The fix is general, by pattern in the export or Ghidra's analysis (DESIGN §2.2): never a hint in `re/translator-hints.toml`, never a special case for an address or for `load_file`.** For example: a 68000 `jsr`/`bsr` falls through to the next instruction whatever Ghidra concluded about its callee (no-return analysis off, or the flow override ignored, for every call), so any routine Ghidra wrongly judges non-returning is covered, in this game and the next.

## Design sections this implements

DESIGN §2.1, §2.2, §2.3.

## Files

- `games/forest-dumb/tools/ghidra/ExportPcode.java`, `tools/pcode_export.py`, `tools/test_pcode_export_ghidra.py` (a made-up program whose callee loops for ever on one path and returns on another: the call's next instruction is exported).
- The instruction count and census the whole export changes, wherever a test asserts them (`fd-conform/tests/insn_gate.rs` asserts 6 684 instructions and 290 kinds); `docs/14-translator.md` if its readings change. Re-run T13's gate if new instruction kinds appear.

## Interface

No new interface: `pcode_export.py --whole` writes the same `build/translator/pcode.json` shape, with the instruction after every `flow.kind == "call"` present in `insns`. T26's `fd-judge --os-run power-on` line is the end-to-end check.

## Tests

- [ ] Made-up program on Ghidra: a call to a routine Ghidra marks non-returning still has its fall-through exported and translated.
- [ ] Whole export: every call's next address is an exported instruction (none of `flow.kind == "call"` lacks its fall-through), checked in `test_pcode_export_ghidra.py` or a full test.
- [ ] Full: `fd-conform/tests/os_run.rs` `power_on_reaches_the_title_with_no_stop` (T26) passes: power-on reaches the title's first fire poll, opening `!EAGLE_SOFT!`, `_NAPIS`, `_CZOLOWKA` and `_OPCJE`, with no stop. If it then stops for another reason outside this task's reach, record it in FINDINGS and say so in PROGRESS.

## Done when

- [ ] The whole export has no call without its fall-through, by a general rule.
- [ ] `insn_gate.rs` and every count the export change touches are updated and green; the trial (`translator_trial.py --skip-export`) still passes 84 of 85.
- [ ] Full: T26's power-on check passes, or its next stop is named in FINDINGS.
