#!/usr/bin/env python3
"""Render every T7MP level to a PNG using its world tileset + P41A palette.

Associations (verified against the disk layout):
  - tileset = the nearest preceding 134400-byte chunk (840 x 16x16x5bpp tiles)
  - palette = the nearest preceding P41A chunk (32-colour OCS palette inside it)
  - map cell (16-bit): tile index = cell & 0x3FF (0..839), flags = cell >> 10

Run tools/unpack.py first (needs build/decomp/*.bin). Outputs build/levels/*.png
and build/levels/_montage.png.
"""
import glob, os, struct, sys
import numpy as np
from PIL import Image
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import t7mp

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DECOMP = os.path.join(ROOT, 'build', 'decomp')
OUT = os.path.join(ROOT, 'build', 'levels')
TW = TH = 16
BPP = 5
TILE_MASK = 0x3FF

def chunk_key(path):
    b = os.path.basename(path)
    disk = b.split('_')[0]
    cno = int(b.split('_c')[1].split('_')[0])
    return (disk, cno)

GRAY32 = [(int(i * 255 / 31),) * 3 for i in range(32)]

# The game's base palettes carry a fixed grey shading ramp near the top of the
# 32 colours (indices ~26-30): 0x111, 0x222, 0x444, 0x0AAA in a row. This
# signature reliably picks the true base palette out of a sprite bank that also
# holds many colour-cycle / sub-palette variants. Verified against a real
# in-game screenshot (World 2 = grey stone castle) and all six worlds.
_RAMP = (0x0111, 0x0222, 0x0444, 0x0AAA)

def base_palette(chunk_bytes):
    """The 32-colour OCS base palette in a chunk, or None.

    Scans from the end (the base palette sits after the cycle variants, at a
    fixed offset ~0x11300 in the world sprite banks) for a 32-word run of valid
    12-bit colours with colour 0 = black that contains the grey-ramp signature."""
    d = chunk_bytes
    for off in range(len(d) - 64, -1, -2):
        ws = struct.unpack_from('>32H', d, off)
        if ws[0] != 0 or any(w > 0x0FFF for w in ws):
            continue
        if any(ws[i:i+4] == _RAMP for i in range(29)):
            return [(((w >> 8) & 0xF) * 17, ((w >> 4) & 0xF) * 17, (w & 0xF) * 17)
                    for w in ws]
    return None

def decode_tiles(data):
    tsize = (TW // 8) * TH * BPP
    n = len(data) // tsize
    tiles = np.zeros((n, TH, TW), dtype=np.uint8)
    for t in range(n):
        for p in range(BPP):
            for y in range(TH):
                o = t * tsize + (p * TH + y) * (TW // 8)
                bits = np.unpackbits(np.frombuffer(data[o:o + 2], dtype=np.uint8))
                if bits.size < TW: continue
                tiles[t, y] |= (bits[:TW] << p)
    return tiles

def render_level(level_bytes, tiles, pal):
    T = {t: p for t, _, _, p in t7mp.parse(level_bytes)}
    w = struct.unpack('>I', T['XBLK'])[0]
    h = struct.unpack('>I', T['YBLK'])[0]
    body = struct.unpack(f'>{w*h}H', T['BODY'][:w * h * 2])
    palarr = np.array(pal, dtype=np.uint8)
    img = np.zeros((h * TH, w * TW, 3), dtype=np.uint8)
    nt = len(tiles)
    for r in range(h):
        for c in range(w):
            idx = body[r * w + c] & TILE_MASK
            if idx < nt:
                img[r*TH:(r+1)*TH, c*TW:(c+1)*TW] = palarr[tiles[idx]]
    return Image.fromarray(img)

def world_id(level_bytes):
    """Extract the world id char from IFFP, e.g. 'SDIFF:L6BM-IFF' -> '6'."""
    T = {t: p for t, _, _, p in t7mp.parse(level_bytes)}
    ref = T.get('IFFP', b'').upper()
    i = ref.find(b'L')
    j = ref.find(b'BM', i)
    return ref[i+1:j].decode(errors='.') if 0 <= i < j else '?'

def associations(files=None):
    """Return [(level_path, world, tileset_path, palette), ...] for every level,
    resolving each level's world tileset and base palette. Shared by the renderer
    and the secret-passage finder."""
    if files is None:
        files = sorted(glob.glob(os.path.join(DECOMP, '*.bin')), key=chunk_key)
    info = []   # (path, disk, is_tileset, is_level, palette_or_None)
    for f in files:
        d = open(f, 'rb').read()
        disk = os.path.basename(f).split('_')[0]
        is_ts = (len(d) == 134400 and d[:4] != b'T7MP')
        is_lvl = (d[:4] == b'T7MP')
        pal = None if is_lvl else base_palette(d)
        info.append((f, disk, is_ts, is_lvl, pal))
    tilesets = [f for f, dk, ts, lv, p in info if ts]   # Nth tileset == world N

    def pick_tileset(world):
        order = {'1':0,'2':1,'3':2,'4':3,'5':4,'6':5,'W':6}
        if world in order and order[world] < len(tilesets):
            return tilesets[order[world]]
        if world == 'B':
            return tilesets[min(5, len(tilesets)-1)]
        return tilesets[0]

    def pick_palette(level_index, disk):
        # base palette lives in the world sprite bank; nearest one preceding the
        # level, same disk first.
        def scan(pred):
            for j in range(level_index - 1, -1, -1):
                _, dk, _, _, pal = info[j]
                if pal is not None and pred(dk):
                    return pal
            return None
        return scan(lambda dk: dk == disk) or scan(lambda dk: True) or GRAY32

    out = []
    for i, (f, disk, ts, is_lvl, _) in enumerate(info):
        if not is_lvl:
            continue
        world = world_id(open(f, 'rb').read())
        out.append((f, world, pick_tileset(world), pick_palette(i, disk)))
    return out

def main():
    os.makedirs(OUT, exist_ok=True)
    tiles_cache, thumbs = {}, []
    for f, world, tpath, pal in associations():
        d = open(f, 'rb').read()
        if tpath not in tiles_cache:
            tiles_cache[tpath] = decode_tiles(open(tpath, 'rb').read())
        im = render_level(d, tiles_cache[tpath], pal)
        name = os.path.basename(f).replace('.bin', '') + '.png'
        im.save(os.path.join(OUT, name))
        print(f"{name:40} {im.width}x{im.height}  world={world}  "
              f"tiles={os.path.basename(tpath)}")
        thumbs.append((name, im.resize((im.width // 8, im.height // 8), Image.NEAREST)))
    # montage
    if thumbs:
        tw = max(t.width for _, t in thumbs)
        th = max(t.height for _, t in thumbs)
        cols = 3
        rows = (len(thumbs) + cols - 1) // cols
        pad = 6
        M = Image.new('RGB', (cols*(tw+pad)+pad, rows*(th+pad)+pad), (40, 40, 40))
        for i, (_, t) in enumerate(thumbs):
            r, c = divmod(i, cols)
            M.paste(t, (pad + c*(tw+pad), pad + r*(th+pad)))
        M.save(os.path.join(OUT, '_montage.png'))
        print(f"\n{len(thumbs)} levels -> {os.path.relpath(OUT, ROOT)}/  (+ _montage.png)")

if __name__ == '__main__':
    main()
