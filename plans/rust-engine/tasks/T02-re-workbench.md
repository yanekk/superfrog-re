# T02 — re-workbench

**Phase:** 1 · **Depends on:** — · **Weight:** light

## Goal

Give every reverse-engineering task one way to read the game's code: a regenerated, labelled listing
(and, if it works here, decompilation) of each executable, driven by a committed file of our own
names and notes. The listing is derived from copyrighted code and stays outside git; the names are
ours and are committed.

## Design sections this implements

DESIGN §2.3, §3.5 (`re/symbols.toml`, `~/.cache/superfrog-re/re/`).

## Files

- `re/symbols.toml` — schema below, seeded with what docs/02–03 already know.
- `tools/re_listing.py` — hunk parse + capstone m68k disassembly of a decompressed exe, applying
  symbols, emitting a listing with labels, comments and cross-references to
  `~/.cache/superfrog-re/re/<chunk>.lst`.
- `tools/test_re_listing.py`
- Optionally `tools/setup/install-ghidra.sh` (brew openjdk + ghidra cask) and
  `tools/re_ghidra_import.py` (headless import + symbol round-trip), only if Ghidra imports an Amiga
  hunk exe here in under an hour of trying; otherwise record why not in FINDINGS.
- `docs/RE-WORKBENCH.md` — how to regenerate and query.

## Interface

```toml
[[sym]]
exe = "d1_c03"          # chunk id
hunk = 0                # hunk index
offset = 0x1234         # offset within hunk
name = "frog_update"
kind = "func"           # func | data | table | var
size = 4                # bytes, for data/var/table
note = "one line"
```

```
tools/re_listing.py <chunk-id> [--out DIR] [--xrefs]
```

## Tests

- [ ] hunk parse of all five exes gives hunk count, types and sizes that sum to the chunk.
- [ ] relocation entries are applied, so absolute references show as `hunk+offset` labels.
- [ ] a symbol in `symbols.toml` appears as a label at its offset and in xrefs of a referencing instruction.
- [ ] a duplicate name or overlapping data symbol is rejected with the line number.
- [ ] the listing is written outside the repo and the repo tree is unchanged after a run.

## Done when

- [ ] `tools/re_listing.py d1_c03 --xrefs` produces a labelled listing of the main executable.
- [ ] The ATN! routine (docs/02) appears under its symbol name in the `d1_c01`/stage listing it lives in.
- [ ] `docs/RE-WORKBENCH.md` says how to regenerate, and FINDINGS says whether Ghidra was adopted.

## Outside actions

- Install Ghidra (only if adopted) — `worker`
