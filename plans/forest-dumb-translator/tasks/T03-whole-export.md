# T03 — whole-export

**Phase:** 1 · **Depends on:** T02 · **Weight:** heavy

## Goal

Extend the trial's export from 87 routines to the whole program: seeded from the program entry, every `func` in `re/symbols.toml`, the vectors the program installs, hunk 4, and `re/entries.toml`. This is the input every later task translates.

## Design sections this implements

DESIGN §2.1, §2.3.

## Files

- `games/forest-dumb/tools/pcode_export.py`, `tools/ghidra/ExportPcode.java`, `tools/test_pcode_export.py`, `tools/test_pcode_export_ghidra.py`.

## Interface

```
pcode_export.py [--out build/translator] [--entries re/entries.toml] [--routines re/translate.toml]
                [--whole]   # default after this task: whole program; --routines keeps the trial's set
# pcode.json gains: "entries": [{"at": "hN+$off", "source": "..."}], per insn "hunk" 0..4
# summary line: {"instructions": N, "entries": N, "by_source": {...}, "unresolved_computed": N, "hunks": [...]}
```

Vectors: the export finds stores of a relocated hunk address into `$64..$7c` (autovectors) in the program's code and seeds their targets as `vector`.

## Tests

- [ ] Made-up program (bytes, committed): an entry, a vector install, a jump table and a second hunk; every instruction exported, the vector seeded, the table's targets found.
- [ ] The trial's 87 routines' instructions are a subset of the whole export (full).
- [ ] Unresolved computed jumps are listed by address, never dropped.

## Done when

- [ ] A whole-program `pcode.json` from one command; its summary is in FINDINGS (instructions, entries by source, unresolved).
- [ ] The trial's `translator_trial.py --skip-export` still passes 84 of 85 on the new export.
