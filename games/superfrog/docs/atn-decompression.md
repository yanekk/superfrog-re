# ATN! decompression

Moved from Claude's memory 2026-10-30; checked against games/superfrog/docs/03-atn-codec.md and the code. Corrected:
the note said emulation was the only working decoder and warned against a hand port. A Rust port now
exists and is the product decoder; emulation is its reference.

Superfrog disks are custom trackloader images (not AmigaDOS). All game data is a
sequence of sector-aligned **`ATN!` chunks**: `magic "ATN!"` + u32 decomp_size +
u32 comp_size (incl. 12-byte header) + compressed stream + ~46-byte trailer, padded
to 512 B.

**`ATN!` is a custom cruncher, NOT in `ancient` or any public tool.** Two decoders:

- The product decoder is the Rust port, `games/superfrog/engine/crates/sf-disk/src/atn.rs`.
- The reference runs the game's own 68k decompressor routine under Musashi emulation (`machine68k`).
  The routine is at disk-1 raw code offset `0x400+0x2DA`, identified by `cmpi.l #$41544E21,(a0)+`.
  `games/superfrog/tools/atn.py` (ATNDecompressor) does this: load code seg at 0x1000, chunk at
  0x400000, a0=src/a1=dst, call +0x2DA, STOP at the return address to halt. Success =
  d0==0xFFFFFFFF and len==decomp_size.

103/106 decode as carved, identically in both; the other three need the next sector for their trailer.

(A separate, simpler *backward bit-LZ* at load addr 0x40024 only unpacks the boot loader; don't
confuse the two.)

See [t7mp-level-format.md](t7mp-level-format.md). Full detail in [03-atn-codec.md](03-atn-codec.md).
