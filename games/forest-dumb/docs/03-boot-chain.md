# 03 — Boot chain

How the disks reach the title screen. The boot is standard AmigaDOS with a crack's two programs in
front, so nothing here is a loader to reverse. Disk facts are from the real images (`xdftool`,
`fd-tools extract`). Addresses are offsets into the main program's hunk 0, read statically; T06's map
takes them further. Frame numbers are the T00 oracle run (Kickstart 1.3, disk 1 alone in df0).

## Boot block

| | Disk 1 | Disk 2 |
|---|---|---|
| Dos type | `DOS\0` | `DOS\0` |
| Checksum | `0xaac8246d`, valid: bootable | stored `0xdfc575a5`, computes `0xbca37650`: not bootable |
| Root block | 880 | 880 |
| Code | 40 bytes | 1012 bytes, never run |

Disk 1's code is the stock boot: it finds `dos.library` with `FindResident` and returns its init
address to the ROM, which starts AmigaDOS and runs `s/startup-sequence`. Disk 2's bytes name
`dos.library` and `expansion.library` and end in repeated `FM30` filler; the bad checksum means
the ROM ignores them. Disk 1's root block dates the disk 06.11.1996, as the crack intro does.

## startup-sequence

54 bytes, word for word:

```
Show -o Kwach4
BillClintonToDupek
forest_dumb_forever
```

AmigaDOS ignores case, so the last line runs `FOREST_DUMB_FOREVER`.

## The chain

| Step | File | Size | What it is | Packed |
|---|---|---|---|---|
| 1 | `Show` | 9 508 | SuperView v3.0 by David Grothe, a public picture viewer. `-o` overrides AutoOverscan | no: an ordinary 1-hunk program with readable strings |
| 1 | `Kwach4` | 46 000 | The crack's picture: IFF ILBM, 640×512 interlaced, 4 planes | only the format's own ByteRun1 |
| 2 | `BillClintonToDupek` | 62 896 | The crack's intro, signed Wanted Team, cracked 06-11-96 | yes by its shape: a 76-byte code hunk, then about 62 kB of data at 7.91 bits a byte in a hunk sized 173 kB in memory; packer named by T04 |
| 3 | `FOREST_DUMB_FOREVER` | 521 640 | The game: 5 hunks | no: 2.96 bits a byte, strings readable |

Input each step waits for (T00): `Show` leaves only on Esc (sent frames 800-810), and the intro on
the left mouse button (1300-1310; a click at 1200 is ignored). Those inputs reach the title at
frame 4921; other timings move the count.

Also on disk 1, never run by the boot: the installer. `Instaluj` runs `Installer ForestDumbForever
f.s`; `f.s` is its copy script. `FOREVER!` is a one-line script naming `FOREST_DUMB_FOREVER`, run
from its icon. These scripts and `s/startup-sequence` are the only places that name the main
program: it does not hold its own name.

## When the game asks for disk 2, and why

Disk 1 holds the program, disk 2 the data, and the program opens data only through `dos.library`.
Its very first act decides the disk:

1. The entry (`0x0`) calls `0x7b70`, which **always** draws the prompt and waits for fire: `włóż
   oryginalną dyskietkę nr 2 do stacji dysków df0:` (insert original disk no. 2 in drive df0). It
   appears at frame 3461. No disk check comes first, so the prompt shows whatever is in the drives.
2. It then calls `0x8fd8`, which opens `df0:!EAGLE_SOFT!`. If the open fails it calls `0x7b70` again
   and retries, for ever.
3. On success it reads the file into a buffer and closes it. The routine does not compare the bytes:
   the marker's presence is the check. Its 400 bytes are Eagle Soft's text banner.

That explains T00's three observations. The name says `df0:`, so disk 2 in df1 from power-on still
prompts and is never looked at. Fire without the swap fails the open and prompts again. Disk 2 in df0
with fire goes on: swap at 3600, fire 3700-3710, disk reads at 3791-4916, title at 4921.

So the arrangement is one drive: disk 1 alone in df0, then disk 2 in df0. Both disks have the volume
name `FOREVER`, so a volume name could not tell them apart; a marker file can.

Once running, every load (`0x8ec4`, `0x8f58`) opens a name from the buffer at `0x90a2` and on failure
calls `0x7bf4`, which prompts `włóż dysk 2` (insert disk 2), then retries. A level's load list in hunk
4 keeps each name inline, jumped over by a `bra.w`, and copies it into that buffer before the call.

## Names against the inventory

`tools/find_file_names.py` scans the program for file names; `fd-tools`' `tests/boot_chain.rs` checks
them against [04-file-inventory.txt](04-file-inventory.txt), the way AmigaDOS compares: no device, case
ignored. The program spells many in capitals (`_MAPA1-1.Mapa` for the disk's `_Mapa1-1.Mapa`). It names
all 49 files on disk 2, and nothing on disk 1.

Three names it holds are on neither disk. The check lists them exactly, so a new one fails:

| Name | At | What it is |
|---|---|---|
| `FileName.exe` | `0x90a2` | The default text of the load buffer above. Hunk 4 overwrites it before each load, so it is never opened under this name |
| `DF0:ST` | `0x918c` | Opened at `0x944c` as an existing file, then 128 bytes of the high-score table at `0x84b6` are written to it. Never read. See below |
| `HD_ID` | `0x91f8`, among the name buffers | No reference found. Left for T06 |

`DF0:ST` is the one to watch. The routine holding it (from `0x92d8`: waits for the write-protect tab,
prompting `odbezpiecz dysk`, then saves) has no caller found by absolute call, `bsr.w` or relocation in
hunks 0 and 4. The file is on neither disk, and the open retries for ever behind the `włóż dysk 2`
prompt, so a call would hang the game until a disk holding `ST` appeared. Whether the crack removed
the call or the file, or the original never called it, cannot be told without an uncracked image.
