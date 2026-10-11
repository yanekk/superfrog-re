# RE workbench — reading the game's code

One way for every reverse-engineering task to read Superfrog's 68000 code: a labelled
listing per executable, and a Ghidra project with decompilation, both regenerated from
the player's disks and both driven by one committed file of our own names,
[`re/symbols.toml`](../re/symbols.toml).

**The listings and decompilation are derived from copyrighted code and never go in git**
(DESIGN §2.3). They are written to `~/.cache/superfrog-re/re/` (DESIGN §3.5). The names
and notes in `re/symbols.toml` are ours and are committed.

## What there is to read

| Chunk id | What | Hunks |
|---|---|---|
| `d1_boot` | disk 1 bootblock, disk bytes `0x000..0x400` (raw) | 1 raw |
| `d1_stage15` | stage-1.5 loader, disk 1 bytes `0x400..0x1600` (raw): ATN! decompressor, hunk loader | 1 raw |
| `d1_c01`, `d3_c01` | 15 KB executables | CODE |
| `d1_c03` | 331 KB executable | CODE, BSS, 2 absent |
| `d3_c02` | 182 KB executable | CODE |
| `d3_c04` | 343 KB executable | CODE, BSS, 2 absent |

Chunk ids are `d<disk>_c<n>`, 1-based in disk order, as in `docs/05-chunk-inventory.txt`.
Which executable does what is T07's job (exe-map).

"Absent" hunks are declared in the hunk header but not present in the file. The game's
own hunk loader (`d1_stage15:hunk_load`) still reserves room for them, so the tools
treat them as empty BSS.

## Regenerate

Needs the venv (`python3 -m venv venv && venv/bin/pip install -r requirements.txt`) and
the four disk zips: `SUPERFROG_DISKS=<folder>`, else the main checkout root. No
`build/` step is needed; chunks are decompressed in memory.

```sh
venv/bin/python tools/re_listing.py d1_c03 --xrefs     # -> ~/.cache/superfrog-re/re/d1_c03.lst
venv/bin/python tools/re_listing.py d1_stage15         # a raw region
venv/bin/python tools/re_listing.py d3_c04 --out /tmp/x
```

All five executables and both raw regions take a few seconds in total.

### Reading a listing

```
; func — ATN! chunk decompressor: a0 = chunk, a1 = dest; d0 = -1 on success (docs/03)
atn_decompress:
          ; xref: stage15_entry+$ac (branch), stage15_entry+$f4 (branch)
000002da  48e73c38              movem.l  d2-d5/a2-a4, -(a7)
...
0000cf3e  1a3b006a              move.b   atn_lenbase_tab(pc, d0.w), d5
```

- The address column is the **load offset**: hunks are placed back to back from 0, as
  the game's loader places them. `hN+$off` is hunk N, byte offset `off`, which is what
  `symbols.toml` uses.
- Every relocated address, branch target and pc-relative operand is printed as a name:
  the symbol, `sym+$n` inside a sized symbol, or `hN+$off` when nothing names it yet.
- `--xrefs` adds, under each label, every instruction that refers to it and how
  (`abs` relocated address, `branch`, `pcrel`). A referencing instruction is named as
  the nearest preceding `func` symbol plus an offset, else `hN+$off`.
- Code is a linear sweep, so data inside a code hunk decodes as nonsense until a
  `data`/`table`/`var` symbol covers it; it is then printed as `dc.b`. Words capstone
  cannot decode are `dc.w`. A label that lands mid-instruction is noted as
  `falls inside this instruction`; that is usually data, or a sweep out of step.

Search a listing with `grep -n '^atn_decompress:' ~/.cache/superfrog-re/re/*.lst`, or
`grep -n 'h1+\$9fe' …` for every use of an address.

## Ghidra (adopted)

Ghidra 12.0 (brew formula) and the `ghidra-amiga` loader extension import the hunk
executables headless. `tools/setup/install-ghidra.sh` installs both if missing.

```sh
venv/bin/python tools/re_ghidra_import.py d1_c03       # first run: import + analyse, ~30 s
venv/bin/python tools/re_ghidra_import.py d1_c03       # later: re-apply symbols, re-export
venv/bin/python tools/re_ghidra_import.py d1_c03 --reimport   # throw the program away
```

Writes to `~/.cache/superfrog-re/re/`:

- `ghidra/superfrog.gpr`: the project. Open it in the Ghidra GUI (`ghidraRun`) to
  browse; with the GhidraMCP plugin enabled there, Claude's `ghidra` MCP tools can
  read and rename in it.
- `<chunk>.c`: decompilation of every function, headed `// name @ hN+$off`.
- `<chunk>.ghidra-names.toml`: the round trip. Names you give in Ghidra that
  `symbols.toml` lacks, already in `[[sym]]` form; review them and paste them in.
- `bin/<chunk>.exe`: the executable Ghidra imported. For `d1_c03`/`d3_c04` it has the
  absent hunks appended as empty `HUNK_BSS`, because the Amiga loader refuses the file
  without them.

Raw regions are imported as plain 68000 binaries based at 0. Ghidra's own addresses
differ from the listing's (the Amiga loader bases hunk 0 at `0x21f000`); go by
`hN+$off`, which both print.

## `re/symbols.toml`

```toml
[[sym]]
exe = "d1_c03"          # chunk id
hunk = 0                # hunk index (0 for a raw region)
offset = 0x1234         # byte offset within the hunk
name = "frog_update"    # identifier, unique within one exe
kind = "func"           # func | data | table | var
size = 4                # bytes; required for data, table and var
note = "one line"
```

Both tools refuse the file, naming the line of the offending `[[sym]]`, when a name
repeats within one exe, two symbols sit at the same place, two data symbols overlap,
a data symbol has no size, or a kind or key is unknown. A symbol past the end of its
hunk is refused when that exe is listed. The same routine in several executables gets
one entry per exe (see `atn_decompress`).

## Tests

`tools/test_re_listing.py` and `tools/test_re_ghidra_import.py`, run by the test command.
They need the disks and fail with a message when they cannot find them; the Ghidra
import test is skipped when Ghidra is not installed.
