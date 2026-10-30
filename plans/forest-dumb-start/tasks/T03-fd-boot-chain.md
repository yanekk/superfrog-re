# T03 — fd-boot-chain

**Phase:** 1 · **Depends on:** T00, T02 · **Weight:** light

## Goal

Document how these disks boot, from the disks themselves, and prove the OFS reader's first
real-world claim: every file name the main program actually contains appears in the inventory
(DESIGN §2.4). The boot chain is AmigaDOS-standard, not Superfrog's trackloader, so the work is
reading and cross-checking, not disassembling a loader.

## Design sections this implements

DESIGN §2.4, §2.3 (the inventory it checks against).

## Files

- `games/forest-dumb/docs/03-boot-chain.md` — the boot block (dos type, checksum, root block),
  `startup-sequence` word for word, the crack's chain (`Show -o Kwach4`, `BillClintonToDupek`,
  `forest_dumb_forever`: what each is, its size, whether it is packed), and when the game asks
  for disk 2 and why (the `!EAGLE_SOFT!` marker), using T00's measured arrangement and frame
  counts.
- `games/forest-dumb/tools/find_file_names.py` — scans the main program's bytes for AmigaDOS
  file-name strings (printable, with the game's name characters), used here and by T06.
- `games/forest-dumb/tools/test_find_file_names.py` — its unittest.
- `games/forest-dumb/test.sh` — gains the Python tools line, Superfrog's shape (the unittest
  discover with its exit-5 guard), since this task creates the first Python tool.
- `games/forest-dumb/engine/crates/fd-tools/tests/boot_chain.rs` — the agreement check on the
  real files.
- `games/forest-dumb/CLAUDE.md` — the doc's index line.

## Interface

`python games/forest-dumb/tools/find_file_names.py <file>` prints one candidate name a line,
sorted. The agreement check asserts that every name the scan finds in `FOREST_DUMB_FOREVER`
appears in the inventory's names (the map task deepens this into the dos.library call table;
this task proves the reader and the inventory agree with what the game names).

## Tests

- [ ] The name scan finds `_Mapa1-1.Mapa`, `!EAGLE_SOFT!` and `FOREST_DUMB_FOREVER` in the real
      executable, and ignores binary noise (the unittest covers the scanner on synthetic bytes
      with and without candidate names).
- [ ] The agreement check is green on the real files: every scanned name is in the inventory.

## Done when

- [ ] `docs/03-boot-chain.md` is written and linked from the game's CLAUDE.md.
- [ ] The agreement check passes on the real executable and inventory, and the boot observations
      (arrangement, frame counts) in the doc match T00's FINDINGS rows.

## Outside actions

None.
