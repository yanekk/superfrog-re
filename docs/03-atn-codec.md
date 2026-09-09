# 03 — The `ATN!` codec

`ATN!` is a custom Team 17 cruncher. It is **not** recognised by `ancient`
(Teemu Suutari's decompression library) or other public tools, so there is no
off-the-shelf decoder.

## How we decode it: emulation

Rather than risk a hand-port of its variable-length code table + trailer
handling, we run **the game's own decompressor routine** on a real 68000 core
(`machine68k`, Musashi). This is exact by construction.

- The routine lives in the disk-1 stage-1.5 loader, raw code at file offset
  `0x400..0x1600`, entry at `+0x2DA`. It is identified unambiguously by its first
  real instruction: `cmpi.l #$41544E21,(a0)+` ("ATN!").
- `tools/atn.py` loads that code segment at `0x1000`, places a full carved chunk
  at `0x400000`, points `a0`→source and `a1`→dest, and calls `+0x2DA`. A `STOP`
  instruction at the fake return address halts the CPU cleanly on `rts`.
- Success is confirmed by the routine's own return code in `d0` (`0xFFFFFFFF`)
  **and** the output length matching `decomp_size`.

Result: **103 / 106 chunks decode exactly.** The 3 stragglers
(`d1_c16`, `d2_c39`, `d4_c16`) hit an edge case in the harness (they read past
the loaded window) and are the only ones flagged `X` — revisit by widening the
emulation window / trailer capture.

> **`d4_c16` is exactly 53,588 bytes — the T7MP level size** — but is classified
> `binary/gfx` because it failed to decompress. It is very likely a **level** that
> did not unpack (possibly a hidden/30th level or a duplicate), not graphics.
> Getting it to decode is the way to confirm. (`d1_c16` and `d2_c39` are other sizes.)

## Algorithm (from the disassembly)

Backward, byte-wise LZ. Pointers: `a3` = source (read backward via `-(a3)`),
`a4` = dest (written backward via `-(a4)`), `a5` = dest base (stop condition).

```
a3 = src + comp_size          ; end of compressed region
a4 = dst + decomp_size         ; end of output
copy 12 trailer bytes over the source header; load d2 (long), d3 (word) from trailer
copy 7 longs from trailer into a stack table (offset bases)
loop until a4 == a5:
    bit reader: MSB-first, `add.b d3,d3`; refill `move.b -(a3),d3 / addx.b d3,d3`
    a prefix code selects: literal / short match / long match, with the copy
    length and offset-bit-count driven by small PC-relative tables at +0x42A/+0x42E
    matches copy backward: `move.b -(a2),-(a4)` with a2 = a4 + base + extra
```

The exact grammar is not needed for extraction (we emulate), but the disassembly
is preserved in git history / `docs` notes for anyone porting a native decoder.

## Bonus: the stage-2 *backward bit-LZ* (different codec)

The boot self-decompressor at load address `0x40024` is a distinct, simpler
codec (3-long header `[srclen, destlen, checksum]`, 32-bit bit buffer consumed
LSB-first, XOR checksum). It was hand-ported and **verified** (checksum XORs to
0; output begins with valid m68k `bra.w`). It only unpacks the loader, not game
data, but it was the first foothold into the boot chain.
