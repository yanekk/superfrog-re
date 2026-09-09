#!/usr/bin/env python3
"""Find Superfrog "secret passages": map cells drawn with solid terrain tiles but
flagged non-solid, so the frog can walk through what looks like a wall.

Collision model (reverse-engineered from the BODY flag field, cell >> 10):
    a cell is SOLID  iff  (flag & 0b11000) == 0b11000   (bits 3 & 4)
A secret-passage cell is a "wall" tile (a terrain tile that is solid almost
everywhere it is used) placed with a non-solid flag. Flag 38 (0b100110) is the
game's dedicated "fake wall" value; flag 0 marks fully carved-open wall cells.

Outputs build/secrets/<level>.png (level with passages boxed) + SECRETS.md.
Run tools/unpack.py first.
"""
import os, sys
from collections import Counter, deque
import numpy as np
from PIL import Image, ImageDraw
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import t7mp
import render_levels as RL

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, 'build', 'secrets')
S = RL.TW  # 16 px/block

def is_solid(f):
    return (f & 0b11000) == 0b11000

def wall_tiles(tg, fg):
    """Terrain tiles that are solid in >=50% of placements (>=15 uses)."""
    out = set()
    for ti in np.unique(tg):
        ff = fg[tg == ti].ravel()
        if len(ff) >= 15 and np.mean([is_solid(int(x)) for x in ff]) > 0.5:
            out.add(int(ti))
    return out

def clusters(mask):
    seen = np.zeros_like(mask, bool)
    h, w = mask.shape
    comps = []
    for y in range(h):
        for x in range(w):
            if mask[y, x] and not seen[y, x]:
                q = deque([(y, x)]); seen[y, x] = True; cells = []
                while q:
                    cy, cx = q.popleft(); cells.append((cy, cx))
                    for dy in (-1, 0, 1):
                        for dx in (-1, 0, 1):
                            ny, nx = cy+dy, cx+dx
                            if 0 <= ny < h and 0 <= nx < w and mask[ny, nx] and not seen[ny, nx]:
                                seen[ny, nx] = True; q.append((ny, nx))
                comps.append(cells)
    return comps

SECRET_FLAG = 38   # 0b100110 — the game's "fake wall" value: drawn solid, passable

def find(level_bytes):
    w, h, tiles, flags = t7mp.level_grid(level_bytes)
    tg = np.array(tiles); fg = np.array(flags)
    # A secret passage cell is any cell carrying the fake-wall flag 38: it is drawn
    # as ordinary terrain but the frog walks through it. (Do NOT also require the
    # tile to be "usually solid" — vertical drop-shafts are built from tiles used
    # *only* for passages, so they never read as solid elsewhere.)
    mask = (fg == SECRET_FLAG)
    comps = [c for c in clusters(mask) if len(c) >= 3]   # drop 1-2 cell noise
    comps.sort(key=lambda c: (min(x for _, x in c), min(y for y, _ in c)))
    return w, h, mask, comps, fg

def main():
    os.makedirs(OUT, exist_ok=True)
    tiles_cache = {}
    report = ["# Superfrog secret passages\n",
              "Cells drawn as solid wall but flagged passable (the frog walks "
              "through). Coordinates are block (col,row); origin top-left.\n"]
    thumbs = []
    total = 0
    for f, world, tpath, pal in RL.associations():
        d = open(f, 'rb').read()
        w, h, mask, comps, fg = find(d)
        if tpath not in tiles_cache:
            tiles_cache[tpath] = RL.decode_tiles(open(tpath, 'rb').read())
        im = RL.render_level(d, tiles_cache[tpath], pal).convert('RGB')
        arr = np.array(im)
        # tint the actual fake-wall cells magenta so their true shape shows
        for cells in comps:
            for y, x in cells:
                blk = arr[y*S:(y+1)*S, x*S:(x+1)*S].astype(np.uint16)
                blk = (blk * 0.35 + np.array([255, 0, 255]) * 0.65).astype(np.uint8)
                arr[y*S:(y+1)*S, x*S:(x+1)*S] = blk
        im = Image.fromarray(arr)
        dr = ImageDraw.Draw(im)
        name = os.path.basename(f).replace('_53588.bin', '')
        report.append(f"\n## {name}  (world {world}) — {len(comps)} passage(s)")
        for cells in comps:
            xs = [x for _, x in cells]; ys = [y for y, _ in cells]
            x0, x1, y0, y1 = min(xs), max(xs), min(ys), max(ys)
            dr.rectangle([x0*S-1, y0*S-1, (x1+1)*S, (y1+1)*S],
                         outline=(255, 255, 0), width=2)
            report.append(f"- cols {x0}-{x1}, rows {y0}-{y1}  ({len(cells)} cells)")
        total += len(comps)
        im.save(os.path.join(OUT, name + '.png'))
        thumbs.append((name, im.resize((im.width//8, im.height//8), Image.NEAREST)))
        print(f"{name:16} world {world}: {len(comps)} passages, "
              f"{int(mask.sum())} cells")
    # montage
    tw = max(t.width for _, t in thumbs); th = max(t.height for _, t in thumbs)
    cols = 3; rows = (len(thumbs)+cols-1)//cols; pad = 6
    M = Image.new('RGB', (cols*(tw+pad)+pad, rows*(th+pad)+pad), (30, 30, 30))
    for i, (_, t) in enumerate(thumbs):
        r, c = divmod(i, cols)
        M.paste(t, (pad+c*(tw+pad), pad+r*(th+pad)))
    M.save(os.path.join(OUT, '_montage.png'))
    report.insert(2, f"\n**{total} secret passages across {len(thumbs)} levels.**\n")
    open(os.path.join(OUT, 'SECRETS.md'), 'w').write("\n".join(report))
    print(f"\n{total} passages total -> {os.path.relpath(OUT, ROOT)}/")

if __name__ == '__main__':
    main()
