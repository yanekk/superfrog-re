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
m.insert(pcode_machine::Disks { df0: adf });              // the disk every Open reads
m.attach(0x0A0..0x0E0, Box::new(paula));   // custom registers $0A0..$0DE to a Device
m.attach_cia(pcode_machine::Cia::A, Box::new(cia_a));
// m is a pcode_rt::Bus: hand it to the generated code.
m.set_input(&pcode_machine::Input::from(frame_input));     // each frame, at its start
let paula = m.take_paula_writes();                          // each frame, at its end: (cycle, PaulaWrite)
let intreq = m.take_cia_interrupts();                       // after each block: INTREQ bits to set
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
  `$A00000..$BFFFFF`; slow RAM at `$C00000`, the custom registers mirrored past it up to `$DBFFFF`
  (the A500 bank map); custom registers at `$DE0000..$DFFFFF` (offset `a & $1FE`). Addresses are
  24 bits. Everything else (ROM, its mirror, autoconfig, the clock chip's `$DC0000`, the reserved
  `$DD0000`) is outside.
- **Custom registers:** a word is the unit; a byte write puts the byte on both halves, a byte read
  reads its half of the word. A write to a read-only register (`$000..$01E`, `$07C`) changes nothing.
  A read of a write-only register writes the data bus's last word into it and returns it (vAmiga
  `peekCustomFaulty16`; the list is `pcode_rt::ops::custom_write_only`). The bus's last word is the
  last word read or written through this bus. VPOSR and VHPOSR read the beam: LOF set for a
  313-line frame, chip id 0 (OCS). Other readable registers read their device, else their last
  written value (a snapshot's, at first). Every written value is kept in `custom()`.
- **CIAs:** register `(a >> 8) & $F`; A on odd bytes with bit 12 low, B on even bytes with bit 13
  low; a byte write reaches every selected chip, a word write gives A its low byte and B its high.
  An attached device replaces either chip's model.
- **CIA-A** (`cia.rs`), what the program uses (translator T01's OS surface names the sites): PRA reads its outputs' latch and, on inputs, fire (bit 7, port 2) and the left mouse
  button (bit 6, port 1) low when held, the OVL, LED and disk lines high; PRB, DDRA, DDRB as
  registers; CRA reads bits 4 and 7 as 0, CRB bit 4 (vAmiga). ICR: a write with bit 7 sets mask
  bits, without clears them; a read returns the flags (bit 7 if the line is driven) and clears them.
  Only that read releases the line: masking a raised interrupt and unmasking it gives no new edge. The line falling is PORTS (`$0008`) from
  `take_cia_interrupts`, for the interrupt logic to set in INTREQ. The keyboard (`amiga-input`
  `key_bytes`): each frame's changes queue a byte each; a byte lands in SDR with the serial flag, and
  the next waits for the handshake (CRA bit 6 set then cleared) plus `KEY_BYTE_CYCLES` (3 405, the
  manual's 8 bits at 60 µs, unmeasured). A keyboard never acknowledged waits for ever; the real one
  resyncs. Reset state: all masked, all inputs; at a snapshot set the mask (`cia_a_mut`), since the
  oracle's snapshot holds no CIA state. TOD registers keep their byte.
- **CIA-B:** each register keeps its last byte written; the program only latches the drive motor off
  through PRB.
- **Input:** `set_input` sets JOY1DAT (read-only to the program), the fire buttons and the keys
  held; `Input::from(FrameInput)` is the hand engine's input (`Joy::joydat`).
- **Paula's writes** (`paula.rs`): every write to DMACON, INTENA, INTREQ, ADKCON and `$0A0..$0DE`
  (`amiga_chips::is_paula`), in order, with its cycle count and beam position, including the one a
  read of a write-only audio register makes. The host empties the list each frame.

## The OS (translator T26)

No Kickstart code runs (DESIGN §2.8). ExecBase, the libraries and their jump tables are made-up
structures in Kickstart's ROM window (`$F80000`, `os::layout`); a `jsr` through a vector leaves the
hunks, so the generated code hands it to `Bus::call_outside`, the machine answers it in Rust
(`os/`), pops the return address into `PC` as `rts` would, and the caller's return check goes on.
Nothing is checked or guessed beyond what is listed: any other vector stops the run as
`os: unknown <lib> LVO <n> at <site>` (`Stop::Os`), the site being the `jsr` read back from the
return address; an argument an answer cannot honour stops naming it. Calls take no cycles and loads
finish at once (owner, 2026-11-11). d1, a0 and a1, scratch to the real OS, are left as they were.

| Call | Answer | Why a program needs it (`games/…/docs/15-os-surface.md`) |
|---|---|---|
| exec `OldOpenLibrary` (-408) | `d0` = the base of `exec.library`, `graphics.library` or `dos.library`; any other name stops | graphics' base is read and called at start; dos' is `a5` for every file call |
| exec `OpenLibrary` (-552) | as `OldOpenLibrary`; `d0` = 0 for a version over 34 (Kickstart 1.3) | not used by the measured program; its newer form |
| exec `CloseLibrary` (-414) | nothing; a base that is no library's stops | after each file |
| graphics `OpenFont` (-72) | `d0` = a TextFont for `topaz.font` size 8, any other stops | `tf_CharData` (+$22) and `tf_Modulo` (+$26) are copied, never used |
| graphics `CloseFont` (-78) | nothing | at start |
| graphics `FreeSprite` (-414) | nothing | called where `CloseLibrary` was meant; changes nothing a program sees |
| dos `Open` (-30) | `d0` = a handle (a BPTR into the ROM window), or 0 for a file df0's disk lacks; the name may start `df0:` or `<volume>:`; another device, a mode but `MODE_OLDFILE` ($3ED), more than 16 open, or no OFS disk stops | every load; zero sends a program to its insert-disk prompt |
| dos `Read` (-42) | the next bytes, at most `d3`, into RAM at `d2`; `d0` = how many (0 at the end). A buffer over translated code stops as `write-into-code`, outside RAM, an unknown handle or a negative length as `os:` | `d0` must be the file's exact length: the unpacker finds the data's end by it |
| dos `Close` (-36) | frees the handle; an unknown one stops | after each file |

OS memory: `$4` holds ExecBase, set by `new` (power-on) and by `from_snapshot` (whose own ExecBase
would lead into a Kickstart not here; a program reads `$4` afresh before its calls). `new` also points
the autovectors `$64..$7C` at the OS's interrupt handlers (`os::irq_handler(n)`), so a program can
save them and put them back; entering one stops (`os: the OS's level n interrupt handler, not
modelled`): the measured program takes every interrupt it needs through its own handlers, and the
OS's run only during loads, which finish at once. GfxBase's `copinit` (+$26) and `LOFlist` (+$32) point
at a copper list that only ends. The structures are readable; a write into the ROM window, or a read
past the structures, is `outside-memory`. Values a program copies from them (the library bases, the
handle, the font's fields, the saved vectors) are the machine's, not Kickstart's.

Not answered: a `jmp` (rather than `jsr`) through a vector, and a `jsr` to an absolute ROM address,
stop as `untranslated`; the measured program makes neither.

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
- **CIA timers:** neither CIA-A timer is modelled; reading or writing a timer register, or setting
  CRA's or CRB's START bit, stops the run (`unsupported: CIA-A timer at $bfe401`). The program
  touches no timer (T01). CIA-B's timers are not checked.
- **Audio interrupts:** none is raised; the program never enables level 4 (T01). Sound is played
  from the writes by `amiga-paula` in the app, not here.
- The chips and interrupts: translator T23 and T24.
