# 02 — Disk format

Both disks are plain AmigaDOS volumes, so `fd-tools extract` reads them through the toolkit's OFS
reader; no trackloader is involved. The file list with sizes and SHA-1s is
[04-file-inventory.txt](04-file-inventory.txt). The numbers below are from `xdftool info`, `boot show`
and `root show` on the real images.

## Layout

| | Disk 1 | Disk 2 |
|---|---|---|
| Image | 901 120 bytes: 1760 blocks of 512 bytes (2 sides × 80 cylinders × 11 sectors) | same |
| Dos type | `DOS\0`, OFS: each data block carries a 24-byte header, 488 bytes of data | same |
| Volume name | `FOREVER` | `FOREVER` |
| Root block | 880, hash table of 72 slots | 880 |
| Boot block | checksum valid, 40 bytes of boot code: boots | checksum invalid: does not boot, still a good volume |
| Blocks used | 1365 of 1760 | 1349 of 1760 |
| Files | 12 | 49 |
| Directories | `s` | none |

A file's size is its real length, not rounded up to a block. Directory listings come in hash-slot
order, so the inventory sorts by name.

## What each disk holds

Disk 1 is the program disk: the main program `FOREST_DUMB_FOREVER` (521 640 bytes), the crack's
`Show`, `Kwach4` and `BillClintonToDupek`, `s/startup-sequence`, `f.s`, `FOREVER!` and the Polish
installer (`Installer`, `Instaluj`) with their `.info` icons.

Disk 2 is the data disk: the marker file `!EAGLE_SOFT!` and 48 data files. Thirty of those are the
six worlds' level files (`_Gen<W>-1`, `_Mapa<W>-1`, each with a `.Mapa` companion, and
`_MapaBobow<W>`). The other 18 are shared graphics and the like (`_BALLA`, `_CZOLOWKA`, `_ELEMENTY.*`,
`_END`, `_NAPIS`, `_OPCJE`, `_Plomien*`, `_WYBIJACZKA*`, `_ZNIKAJKA*`, `StworkiNew.*`).

## Where it lands

`fd-tools extract` writes `games/forest-dumb/build/disk<N>.adf` and
`games/forest-dumb/build/files/disk<N>/<name>`, git-ignored and regenerable. It refuses an output
folder that is not inside one named `build`, and reads both disks whole before writing anything.

Most files on disk 2 are packed: [08-packers.md](08-packers.md) classifies every file.
