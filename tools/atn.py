#!/usr/bin/env python3
"""ATN! container + decompressor for Superfrog (Team 17, 1993) trackloader data.

The four Superfrog floppies are NOT AmigaDOS disks. Disks 1 & 3 carry a standard
`DOS\\0` bootblock whose code is a custom trackloader; disks 2 & 4 have no
filesystem at all. All game data is stored as a back-to-back sequence of
sector-aligned "ATN!" chunks:

    struct ATNChunk {          # all integers big-endian
        char magic[4];         # "ATN!"
        u32  decomp_size;      # size of decompressed data
        u32  comp_size;        # size of the compressed region incl. this 12-byte header
        u8   data[...];        # compressed stream + an appended ~46-byte trailer
        u8   pad[...];         # zero padding to the next 512-byte sector
    };

The compression is a custom byte-wise backward LZ ("ATN!" cruncher). It is not
recognised by public tools (e.g. `ancient`). Rather than risk a hand-port of its
intricate variable-length code table + trailer handling, we decompress by
*emulating the game's own decompressor routine* (found at disk-1 file offset
0x400+0x2da, identified by its `cmpi.l #$41544e21,(a0)+` magic check) with the
Musashi 68000 core from `machine68k`. This is exact by construction.

See docs/03-atn-codec.md for the full analysis.
"""
import struct
from machine68k import Machine, CPUType

MAGIC = b'ATN!'

# ---------------------------------------------------------------------------
# Container: carve ATN! chunks out of a raw disk image
# ---------------------------------------------------------------------------

def carve_chunks(data):
    """Yield (offset, decomp_size, comp_size, chunk_bytes) for every ATN! chunk.

    `chunk_bytes` spans from the magic to the next 512-byte sector boundary, so
    it includes the compressed stream *and* its appended trailer (both needed by
    the decompressor). Sub-sector "ATN!" occurrences inside loader code are
    skipped because they fail the size sanity check or are not sector aligned.
    """
    o = 0
    n = len(data)
    while o + 12 <= n:
        if data[o:o + 4] == MAGIC:
            dsz, csz = struct.unpack_from('>II', data, o + 4)
            if 0 < csz < 900000 and 0 < dsz < 4_000_000 and o + 12 + csz <= n:
                end = o + ((12 + csz + 511) // 512) * 512
                yield (o, dsz, csz, data[o:end])
                o = end
                continue
        o += 512


# ---------------------------------------------------------------------------
# Decompressor: emulate the game's own routine
# ---------------------------------------------------------------------------

class ATNDecompressor:
    """Decompress ATN! chunks by running the game's 68k decompressor routine.

    The routine and its PC-relative length tables live in the disk-1 stage-1.5
    loader (raw code at file offset 0x400..0x1600). We load that segment at a
    fixed base and call the routine at +0x2da with a0=src, a1=dst.
    """
    CODE_LO   = 0x400
    CODE_HI   = 0x1600
    CODE_BASE = 0x1000
    ROUTINE   = 0x2da          # offset of the decompressor within the segment
    SRC       = 0x400000
    DST       = 0x600000
    STACK     = 0x300000
    SENTINEL  = 0x7F0000       # return address; a STOP here halts the CPU
    RAM_KIB   = 8192

    def __init__(self, disk1_adf_bytes):
        self.code = disk1_adf_bytes[self.CODE_LO:self.CODE_HI]
        self.m = Machine(CPUType.M68000, self.RAM_KIB)
        self.m.mem.w_block(self.CODE_BASE, self.code)
        self.m.mem.w_block(self.SENTINEL, bytes.fromhex('4e722700'))  # STOP #$2700
        # silence out-of-range reads from the (rare) chunk that mis-decodes
        try:
            self.m.mem.set_invalid_func(lambda *a, **k: 0)
        except Exception:
            pass

    def decompress(self, chunk_bytes, decomp_size):
        """Return decompressed bytes for one chunk (full carved chunk_bytes)."""
        mem, cpu = self.m.mem, self.m.cpu
        mem.clear_block(self.DST, decomp_size + 16, 0)
        mem.w_block(self.SRC, chunk_bytes)
        cpu.pulse_reset()
        cpu.w_reg(8, self.SRC)     # a0 = source
        cpu.w_reg(9, self.DST)     # a1 = dest
        cpu.w_sp(self.STACK)
        mem.w32(self.STACK, self.SENTINEL)     # return address
        cpu.w_pc(self.CODE_BASE + self.ROUTINE)
        self.m.execute(100_000_000)
        pc = cpu.r_pc()
        ok = (pc == self.SENTINEL + 4) and (cpu.r_reg(0) == 0xFFFFFFFF)
        out = bytes(mem.r_block(self.DST, decomp_size))
        return out, ok


if __name__ == '__main__':
    import sys, glob, os
    disks = sys.argv[1] if len(sys.argv) > 1 else 'build/adf'
    d1 = sorted(glob.glob(os.path.join(disks, '*Disk 1*.adf')))[0]
    dec = ATNDecompressor(open(d1, 'rb').read())
    for adf in sorted(glob.glob(os.path.join(disks, '*.adf'))):
        data = open(adf, 'rb').read()
        for i, (o, dsz, csz, chunk) in enumerate(carve_chunks(data), 1):
            out, ok = dec.decompress(chunk, dsz)
            print(f"{os.path.basename(adf)[:20]:20} chunk {i:2d} @0x{o:06x} "
                  f"{dsz:7d}<-{csz:6d} {'ok' if ok else 'FAIL':4} {out[:4]!r}")
