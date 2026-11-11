# pcode-machine

The machine translated 68000 code runs on (translator DESIGN §2.8): memory as the oracle's A500
lays it out, the program's hunks at given bases, a `pcode_rt::Bus` routing custom-register and CIA
addresses to devices, and the beam from counted cycles. Pure; names no game.

## Calling it

```rust
let cfg = pcode_machine::Config::a500(hunk_bases.to_vec());   // 512 KB chip, 512 KB slow, 313 lines
let mut m = pcode_machine::Machine::new(cfg, &program_bytes)?; // hunks placed and relocated
// or, at a case's sync point: chip RAM then slow RAM, and the custom registers by reg / 2
let mut m = pcode_machine::Machine::from_snapshot(cfg, &chip_then_slow, &custom);
m.attach(0x0A0..0x0E0, Box::new(paula));   // custom registers $0A0..$0DE to a Device
m.attach_cia(pcode_machine::Cia::A, Box::new(cia_a));
// m is a pcode_rt::Bus: hand it to the generated code.
```

- `poll` (called at every translated block's start) takes the cycle count from `Regs`; every
  access until the next block is timed at it. It returns `Stop::Outside("$xxxxxx")` once any
  access has hit unmapped space: the bus methods cannot fail, so the first such address is kept
  (`fault()`) and the run ends at the next block.
- `beam(cycles)` is `(line, h)`, h in DMA slots (227 a line, 454 CPU cycles), wrapping at
  `frame_lines`; `sync_beam(cycles, line, h)` sets where the beam is at a given count.
  `from_snapshot` syncs it from the snapshot's VPOSR and VHPOSR at cycle 0.

## What is modelled

- **Memory map** (vAmiga `A500_OCS_1MB`): chip RAM mirrored up to `$1FFFFF`; CIAs at
  `$A00000..$BFFFFF`; slow RAM at `$C00000`, the custom registers mirrored past it up to `$D7FFFF`;
  custom registers at `$DE0000..$DFFFFF` (offset `a & $1FE`). Addresses are 24 bits. Everything
  else (ROM, its mirror, autoconfig, the clock chip's `$D80000..$DDFFFF`) is outside.
- **Custom registers:** a word is the unit; a byte write puts the byte on both halves, a byte read
  reads its half of the word. A write to a read-only register (`$000..$01E`, `$07C`) changes nothing.
  A read of a write-only register writes the data bus's last word into it and returns it (vAmiga
  `peekCustomFaulty16`; the list is `pcode_rt::ops::custom_write_only`). The bus's last word is the
  last word read or written through this bus. VPOSR and VHPOSR read the beam: LOF set for a
  313-line frame, chip id 0 (OCS). Other readable registers read their device, else their last
  written value (a snapshot's, at first). Every written value is kept in `custom()`.
- **CIAs:** register `(a >> 8) & $F`; A on odd bytes with bit 12 low, B on even bytes with bit 13
  low; a byte write reaches every selected chip, a word write gives A its low byte and B its high.
  A CIA with no device keeps the last byte written to each register.

## What is not

- **DMA contention:** no cycle is lost to bitplane, copper, blitter, sprite or audio DMA
  (DESIGN §2.6); the beam advances with the CPU's counted cycles alone.
- **Timing inside a block:** accesses are timed at the block's start (`poll`), not the
  instruction's, which can be one block early. vAmiga's 5-slot read-ahead of VHPOSR is not added.
- **Instruction fetches** do not go through the bus, so the last word is a data word, where the
  68000's bus may hold a prefetched instruction word (the translated `clr` loads it itself).
- **VPOSW/VHPOSW** are kept as values; they do not move the beam.
- **Odd word addresses**: a word access at an odd address is made at it, byte by byte; the 68000
  would take an address error.
- Chips, interrupts, sound, input and the OS: devices and `poll` added by translator T23 to T26.
