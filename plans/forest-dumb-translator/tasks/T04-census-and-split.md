# T04 — census-and-split

**Phase:** 1 · **Depends on:** T03 · **Weight:** medium

## Goal

Count every instruction kind the program uses, which is stage 1's work list, and make the generated whole program build in reasonable time by splitting it into modules. The trial's 9 000-instruction tree took 51 s optimised; the whole program is several times that.

## Design sections this implements

DESIGN §2.6, §3.4.

## Files

- `games/forest-dumb/tools/pcode_census.py`, `tools/test_pcode_census.py` (new).
- `tools/pcode_rust.py`, `tools/test_pcode_rust.py`: `--out` becomes a directory with `mod.rs` and one module per part.

## Interface

```
pcode_census.py build/translator/pcode.json --out build/translator/census.json
# census.json: [{"kind": "add.w Dn,(An)+", "mnemonic": "add", "size": 2, "src": "Dn", "dst": "(An)+",
#               "count": 412, "encodings": ["d158", ...], "sites": ["h0+$1234", ...first 5]}]
pcode_rust.py pcode.json --out build/translator/tree/ [--hints ...] [--parts N]
# tree/mod.rs: CODE, HUNK_SIZES, entry(), by_addr(), enter(); tree/p00.rs .. pNN.rs
```

Part boundaries follow call-graph components so a call across parts is rare; the split is chosen by measured build time, recorded.

## Tests

- [ ] Census on a made-up export: kinds grouped by mnemonic, size and both modes; counts and encodings right.
- [ ] A split made-up tree compiles and runs identically to the unsplit one (`pcode-rt` tests).
- [ ] Full: build time of the whole tree, dev and release, recorded in FINDINGS.

## Done when

- [ ] `census.json` for the whole program; kind count and the ten commonest kinds in FINDINGS.
- [ ] The whole tree builds; build times recorded, and `fd-judge --translated` builds with the directory form.
