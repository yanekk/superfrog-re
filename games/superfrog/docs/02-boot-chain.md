# 02 — Boot chain (disk 1 / 3)

Traced by disassembling the raw (uncompressed) code regions with capstone (m68k).
Addresses are disk-1 file offsets unless a load address is given.

```
Bootblock (0x000–0x400, "DOS\0")
  0x00C  standard bootblock entry
         - bsr $300  (stage-2 setup)
         - AllocMem 0x1000, DoIO read 0x1000 bytes from disk 0x400, jmp buffer
  0x300  reads 0x5000 bytes from disk 0xD5400 -> mem 0x40000, jsr $40000

Stage 2  (disk 0xD5400 -> mem 0x40000)
  0x40024  backward self-decompressor:
           - header at 0x4010C = [srclen, destlen, checksum] (3 longs)
           - decompresses an embedded block to 0x58000, verifies checksum, jmp 0x58000
           - this is a DIFFERENT (backward) LZ from the ATN! chunk codec

Stage 3  (decompressed to 0x58000, entry via bra to +0x451E)
           - custom MFM trackloader: sets up blitter ($DFF040+) and CIA ($BFD100/$BFE001)
           - self-relocation table, reads raw tracks

Stage 1.5 (disk 0x400 -> AllocMem'd CHIP buffer, run by bootblock 0x00C)
  +0x2DA   *** the ATN! decompressor ***  (cmpi.l #$41544E21,(a0)+)
  +0x43A   Amiga hunk (0x3F3) loader/relocator
  +0x534   disk/track helper
```

## Key takeaways

- The **ATN! chunk decompressor** used for all game data is the routine at
  **stage-1.5 offset `0x2DA`** (i.e. disk-1 file offset `0x400 + 0x2DA = 0x6DA`).
  It is byte-wise and reads its source **backward**. We drive it directly under
  emulation — see [`03-atn-codec.md`](03-atn-codec.md).
- The stage-2 self-decompressor at `0x40024` is a separate, simpler *backward*
  bit-LZ (3-long header + XOR checksum). It was fully hand-ported and verified
  (checksum → 0), which is how the boot chain was first opened; it is documented
  for completeness but is **not** the codec used for game chunks.
