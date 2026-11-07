# T01 — pcode-export

**Phase:** 1 · **Depends on:** T00 · **Weight:** medium

## Goal

The translator takes each instruction's meaning from Ghidra's P-code rather than from a decoder of its own (brief, owner 2026-11-06). This task writes the Ghidra script and its driver that export every instruction of the trial's routines, with addresses and relocated constants made hunk-relative, so the generated code does not depend on where Ghidra or the oracle load the program.

## Design sections this implements

DESIGN §2.1, §3.2, §3.4, §5 (the measured Ghidra facts).

## Files

- `games/forest-dumb/tools/ghidra/ExportPcode.java` (new; names no game)
- `games/forest-dumb/tools/pcode_export.py` (new; the game's driver; reads the file's `HUNK_RELOC32` entries with the standard library, since `hunk_listing.py`'s amitools is not on `python3 -I`'s path)
- `toolkit/tools/ghidra_headless.py` (extend `run_import` to take extra script paths and post-scripts; Superfrog's callers unchanged)
- `games/forest-dumb/tools/test_pcode_export.py` (new)
- `games/forest-dumb/smoke.toml` (label)

## Interface

```
python3 -I games/forest-dumb/tools/pcode_export.py [--out games/forest-dumb/build/translator] [--reimport]
  → <out>/pcode.json, prints one line: {"routines":87,"instructions":N,"functions":N,"unresolved":N,"relocs":N}
  exit 2 with the path named on stderr: Ghidra, JDK, program or symbols missing
```

`pcode.json`:

```
{ "hunks": [{"index":0,"size":376424,"kind":"CODE"}, …],      // sizes only, never bytes
  "entries": ["h0+$1fec", …],                                    // from re/translate.toml
  "insns": { "h0+$1fec": {
      "len": 4, "mnem": "bsr.w",
      "ops": [ {"op":"INT_SUB","out":["register",60,4],"in":[["register",60,4],["const",4,4]]}, … ],
      "flow": {"kind":"call","targets":["h0+$9b2e"]},          // fallthrough|branch|cbranch|call|return|computed
      "relocs": [ {"in_op":1,"slot":0,"target":"h2+$1a4"} ] } } }  // a const or ram operand that is a relocated address
```

- Varnodes are `[space, offset, size]` with Ghidra's space names (`register`, `unique`, `const`, `ram`). A `ram` address or `const` operand whose bytes lie at an offset a `HUNK_RELOC32` entry of the program file relocates is rewritten as a hunk-relative reference to that entry's target hunk (DESIGN §3.4; Ghidra's own relocation table is empty for this loader); any other `ram` address inside a hunk is also written hunk-relative. An address outside every hunk (custom chips at `$dff000`, CIA, exec base at 4) stays absolute.
- Entries come from `re/translate.toml`; names and further functions from `re/symbols.toml` through `ApplySymbols.java`. The export follows direct flow from every entry. A computed target Ghidra resolved (a jump table it found) is listed in `targets`; one it did not is `"targets":[]` and counted in `unresolved`.
- Hints (`re/translator-hints.toml`, T06) are not read here.

Check on the game: every relocation entry inside an exported instruction lands on one operand, and every 32-bit operand whose value falls in Ghidra's loaded hunk range is either relocated or explained. If either fails in a way the export cannot resolve from the file, stop and ask the owner; do not fall back to guessing or to loading at the oracle's bases.

## Tests

- [ ] A hand-written 68000 program of a few dozen bytes as a two-hunk executable (bytes in the test, `HUNK_RELOC32` included, read by the driver's own walk: an `lea` to the other hunk, a `jsr` through a table, a `bsr`, a `dbf` loop) exports with the expected hunk-relative addresses, flow kinds, targets and relocs. Needs Ghidra: full set.
- [ ] A computed jump Ghidra cannot resolve is `"targets":[]` and counted.
- [ ] Missing Ghidra (`GHIDRA_HOME` pointing nowhere) exits 2 naming the path, without starting Java (smoke).
- [ ] `ghidra_headless.py`'s existing tests still pass, and its old call form is unchanged.

## Done when

- [ ] Run on the game, the export lists all 87 entries, and per entry its instruction count with callees is within 5% of `re/translate.toml`'s `instructions`, or each difference over that is explained in `FINDINGS.md`.
- [ ] The relocation check above holds on the game, its counts recorded in `FINDINGS.md`.
- [ ] `ExportPcode.java` names no game; tests labelled in `smoke.toml`.

## Outside actions

- Run Ghidra headless on the program — `worker`
