#!/usr/bin/env python3
"""Parse a decompressed T7MP Superfrog level file.

T7MP is a flat sequence of [4-byte ASCII tag][u32 big-endian length][payload].
See docs/04-t7mp-level-format.md.
"""
import struct, sys

# Known tags and their meaning (payload sizes are for the shipped levels).
KNOWN = {
    'T7MP': 'form id / total file size',
    'XBLK': 'map width in blocks',
    'YBLK': 'map height in blocks',
    'VERS': 'version string',
    'COM1': 'comment line 1',
    'COM2': 'comment line 2',
    'REMX': 'start/remembered X (blocks)',
    'REMY': 'start/remembered Y (blocks)',
    'IFFP': 'tileset (IFF) reference path',
    'PALA': 'palette A', 'PALB': 'palette B',
    'PALC': 'palette C', 'PALD': 'palette D',
    'COLS': 'colour count', 'CCCL': 'colour-cycling list',
    'IFFC': 'tile attribute / collision table',
    'BODY': 'tilemap: XBLK*YBLK u16 tile indices',
}

def parse(data):
    """Return list of (tag, offset, length, payload) tuples."""
    out, off = [], 0
    while off + 8 <= len(data):
        tag = data[off:off + 4]
        ln = struct.unpack_from('>I', data, off + 4)[0]
        # length-driven: some levels null out a tag name (e.g. VERS) but keep the
        # 8-byte header + payload, so trust the length and stop only on overrun.
        if ln > len(data) or off + 8 + ln > len(data):
            break
        payload = data[off + 8:off + 8 + ln]
        out.append((tag.decode(errors='.'), off, ln, payload))
        off += 8 + ln
    return out

# Each BODY cell is a 16-bit big-endian value:
#   tile index = cell & 0x03FF   (0..839; the world tileset has 840 tiles)
#   flags      = cell >> 10      (6 bits; collision/behaviour, cross-check IFFC)
TILE_MASK = 0x03FF
FLAG_SHIFT = 10

def level_grid(data):
    """Return (width, height, tiles, flags) grids from a T7MP level.

    tiles[r][c] = tile index (cell & 0x3FF); flags[r][c] = cell >> 10."""
    tags = {t: p for t, _, _, p in parse(data)}
    w = struct.unpack('>I', tags['XBLK'])[0]
    h = struct.unpack('>I', tags['YBLK'])[0]
    cells = struct.unpack(f'>{w*h}H', tags['BODY'][:w*h*2])
    tiles = [[cells[r*w+c] & TILE_MASK for c in range(w)] for r in range(h)]
    flags = [[cells[r*w+c] >> FLAG_SHIFT for c in range(w)] for r in range(h)]
    return w, h, tiles, flags

def main(path):
    data = open(path, 'rb').read()
    print(f"{path}: {len(data)} bytes")
    for tag, off, ln, payload in parse(data):
        desc = KNOWN.get(tag, '?')
        if ln <= 4 and ln > 0:
            val = int.from_bytes(payload, 'big')
            extra = f"= {val} (0x{val:x})"
        else:
            asc = ''.join(chr(b) if 32 <= b < 127 else '.' for b in payload[:20])
            extra = f"|{asc}|"
        print(f"  @{off:6x} {tag:5} {ln:6d}  {desc:38} {extra}")
    try:
        w, h, tiles, flags = level_grid(data)
        tv = [c for row in tiles for c in row]
        fv = [c for row in flags for c in row]
        print(f"\n  tilemap {w}x{h}, {len(tv)} cells, "
              f"tile idx: distinct={len(set(tv))} max={max(tv)}; "
              f"flags: distinct={len(set(fv))} values={sorted(set(fv))}")
    except Exception as e:
        print("  (no tilemap:", e, ")")

if __name__ == '__main__':
    if len(sys.argv) < 2:
        raise SystemExit("usage: t7mp.py <level.bin>")
    main(sys.argv[1])
