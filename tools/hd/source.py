"""Original World 1 art as arrays: palette, tiles, level grids, sprite sheets.

Everything here is read from the decompressed disk chunks (build/decomp, made by
tools/unpack.py); nothing is written. Chunk ids per world are in docs/10 and docs/08.
"""
import os
import struct
import sys

import numpy as np

TOOLS = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, TOOLS)
import t7mp  # noqa: E402
from render_levels import decode_tiles  # noqa: E402

ROOT = os.path.dirname(TOOLS)
# A worktree has no build/decomp of its own; fall back to the main checkout's.
DECOMP = next((p for p in [os.path.join(ROOT, 'build', 'decomp'),
                           os.path.join(ROOT, '..', '..', '..', 'build', 'decomp')]
               if os.path.isdir(p)), None)

T = 16   # original tile size

WORLDS = {
    1: dict(lp='d1_c17', bm='d1_c18', bo='d1_c19',
            levels=['d1_c21', 'd1_c22', 'd1_c23', 'd1_c24'], bob_table=0x2ed2c),
}
HERO = 'd1_c04'   # frog sheet, 320x384, 4 planes, drawn with palette 16..31
EXE = 'd1_c03'
# Frame records in the game program (x, y, w, h, hotspot dx, dy), hunk 0 data at file +0x2c.
HERO_RECTS, HERO_N = 0x1a6f4, 96          # records 96..191 are the mirror images
SHARED_RECTS, SHARED_N = 0x2c9b4, 64      # address the world's bob sheet
WORLD_N = 128


def chunk(prefix):
    if DECOMP is None:
        sys.exit('build/decomp missing: run tools/extract_disks.py then tools/unpack.py')
    f = next(x for x in os.listdir(DECOMP) if x.startswith(prefix + '_'))
    with open(os.path.join(DECOMP, f), 'rb') as fh:
        return fh.read()


def palette(world):
    """32 OCS colours as (32, 3) uint8."""
    ws = struct.unpack_from('>32H', chunk(WORLDS[world]['lp']), 0x11300)
    return np.array([(((w >> 8) & 15) * 17, ((w >> 4) & 15) * 17, (w & 15) * 17)
                     for w in ws], np.uint8)


def planar(b, w, h, planes):
    rb = w // 8
    a = np.zeros((h, w), np.uint8)
    for p in range(planes):
        bits = np.unpackbits(np.frombuffer(b[p * rb * h:(p + 1) * rb * h], np.uint8))
        a |= bits.reshape(h, w) << p
    return a


def tiles(world):
    """Palette indices, (840, 16, 16)."""
    return decode_tiles(chunk(WORLDS[world]['bm']))


def levels(world):
    """{'W1-1': (rows, cols) cell array, tile index | flags << 10}."""
    out = {}
    for i, c in enumerate(WORLDS[world]['levels']):
        tags = {t: p for t, _, _, p in t7mp.parse(chunk(c))}
        w = struct.unpack('>I', tags['XBLK'])[0]
        h = struct.unpack('>I', tags['YBLK'])[0]
        out[f'W{world}-{i + 1}'] = np.array(
            struct.unpack(f'>{w * h}H', tags['BODY'][:w * h * 2])).reshape(h, w)
    return out


def hero():
    """Frog sheet as palette indices 16..31; 0 = transparent."""
    a = planar(chunk(HERO), 320, 384, 4)
    return np.where(a == 0, 0, a + 16).astype(np.uint8)


def bobs(world):
    """World bob sheet (enemies, pickups, effects), indices 0..31; 0 = transparent."""
    return planar(chunk(WORLDS[world]['bo']), 320, 320, 5)


def exe_rects(off, n):
    d = chunk(EXE)
    return [dict(zip('x y w h dx dy'.split(), struct.unpack_from('>4H2h', d, 0x2c + off + 12 * i)))
            for i in range(n)]


def render_grid(grid, tl):
    """Tile-index grid -> index image."""
    h, w = grid.shape
    return tl[grid & 0x3FF].transpose(0, 2, 1, 3).reshape(h * T, w * T)
