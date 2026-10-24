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

# lp: picture with the palette at +$11300; bm tiles; bo bob sheet; bob_table and bob_n: the world's
# frame records (count = gap to the next world's table, engine level/load.rs and bobs.rs).
# Keys: 1..6 worlds, 7 the bonus level (LB, 640 tiles), 8 the final level (LW, W6's palette).
WORLDS = {
    1: dict(lp='d1_c17', bm='d1_c18', bo='d1_c19', bob_table=0x2ed2c, bob_n=90,
            levels=['d1_c21', 'd1_c22', 'd1_c23', 'd1_c24']),
    2: dict(lp='d1_c27', bm='d1_c28', bo='d1_c29', bob_table=0x2f164, bob_n=121,
            levels=['d1_c31', 'd1_c32', 'd1_c33', 'd1_c34']),
    3: dict(lp='d2_c08', bm='d2_c09', bo='d2_c10', bob_table=0x2f710, bob_n=105,
            levels=['d2_c12', 'd2_c13', 'd2_c14', 'd2_c15']),
    4: dict(lp='d2_c18', bm='d2_c19', bo='d2_c20', bob_table=0x2fbfc, bob_n=99,
            levels=['d2_c22', 'd2_c23', 'd2_c24', 'd2_c25']),
    5: dict(lp='d2_c28', bm='d2_c29', bo='d2_c30', bob_table=0x300a0, bob_n=106,
            levels=['d2_c32', 'd2_c33', 'd2_c34', 'd2_c35']),
    6: dict(lp='d2_c45', bm='d4_c07', bo='d4_c08', bob_table=0x30598, bob_n=92,
            # W6-5..7: the second map set (docs/10), same tiles with a few cells changed;
            # d4_c16 is a straggler without a readable header and is skipped (docs/LEVEL-INDEX)
            levels=['d4_c02', 'd4_c03', 'd4_c04', 'd4_c05', 'd4_c14', 'd4_c15', 'd4_c17']),
    7: dict(lp='d2_c39', bm='d2_c40', bo='d2_c41', bob_table=0x30a30, bob_n=None,
            levels=['d2_c42'], name='WB'),
    8: dict(lp='d2_c45', bm='d4_c10', bo='d4_c11', bob_table=0x309e8, bob_n=None,
            levels=['d4_c13'], name='WW'),
}
HERO = 'd1_c04'   # frog sheet, 320x384, 4 planes, drawn with palette 16..31
EXE = 'd1_c03'
# Frame records in the game program (x, y, w, h, hotspot dx, dy), hunk 0 data at file +0x2c.
HERO_RECTS, HERO_N = 0x1a6f4, 96          # records 96..191 are the mirror images
SHARED_RECTS, SHARED_N = 0x2c9b4, 64      # address the world's bob sheet
WORLD_N = 128                             # fallback when a world's count is unknown


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
        out[f"{WORLDS[world].get('name', f'W{world}')}-{i + 1}"] = np.array(
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


def world_rects(world):
    w = WORLDS[world]
    return exe_rects(w['bob_table'], w['bob_n'] or WORLD_N)


def render_grid(grid, tl):
    """Tile-index grid -> index image."""
    h, w = grid.shape
    return tl[grid & 0x3FF].transpose(0, 2, 1, 3).reshape(h * T, w * T)
