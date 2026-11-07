# T07 — trial-fixes

**Phase:** 2 · **Depends on:** T06 · **Weight:** heavy

## Goal

The first table will have failures, and most will share causes: an op mis-sized, a flag Ghidra models differently, a return pattern the translator did not expect. This task works through them by kind, fixing the translator, the runtime or the export in general ways, until no general fix is left. It is where the trial learns what translation costs; it never makes a routine pass by a hand edit (DESIGN §2.2).

## Design sections this implements

DESIGN §2.2, §2.3, §2.8.

## Files

`pcode_rust.py`, `pcode-rt`, `ExportPcode.java`, `pcode_export.py`, their tests and fixtures; `re/translator-hints.toml` (hints for the with-hints run only).

## Interface

None new. Each fix keeps the interfaces of T01 to T04, or changes them with the same change made to every caller in the same commit.

## Tests

- [ ] Every fix adds a test on a made-up program that fails without it, in the suite of the part it fixes.
- [ ] A fix that is about Ghidra's P-code being wrong for an instruction is a correction in the translator keyed by mnemonic and P-code pattern, never by address, with the instruction named in `FINDINGS.md` for stage 1's check against Moira.

## Done when

- [ ] Every failing or stopped routine is in a category in `FINDINGS.md` (fixed, Ghidra's P-code, needs a hint, needs stage 2, unexplained) with a count per category.
- [ ] The last full run's summary line is in `FINDINGS.md`, and no category marked fixable in general is left open.
- [ ] Any hint added is in `translator-hints.toml` with its `why`; none is used by the run that counts for the gate.

If the work clearly needs more than one session (past about 200k context), the worker stops at a clean point and asks the owner to add a follow-up task, rather than pressing on (level 1 DESIGN §4.4).
