"""Model inputs for one world: level windows + a sheet of rare tiles in context.

Writes build/hd2/w<N>/src/*.png (4x, nearest) and manifest.json. Each source entry records
where every tile of the image sits, so assembly can cut tiles back out after registration.
"""
import json
import os

import numpy as np
from PIL import Image

from . import source as s
from . import windows as w

S = 4
PATCH = 5                       # rare tiles are shown in a 5 x 5-tile patch of their level
PACK_COLS, PACK_ROWS = 8, 4     # 8 x 80 = 640 <= 648, 4 x 80 = 320 <= 384


def out_dir(world):
    return os.path.join(s.ROOT, 'build', 'hd2', f'w{world}')


def find_occurrence(levels, t):
    """(level, row, col) of a central occurrence of tile t (best interior position)."""
    best = None
    for name, g in levels.items():
        ys, xs = np.nonzero((g & 0x3FF) == t)
        h, wd = g.shape
        for y, x in zip(ys, xs):
            score = min(y, h - 1 - y, x, wd - 1 - x)
            if best is None or score > best[0]:
                best = (score, name, int(y), int(x))
    return best[1:]


def prepare(world):
    d = out_dir(world)
    os.makedirs(os.path.join(d, 'src'), exist_ok=True)
    pal = s.palette(world); tl = s.tiles(world); lv = s.levels(world)
    imgs = {k: s.render_grid(g, tl) for k, g in lv.items()}
    chosen, missing = w.cover(lv, need=1)
    # Drop windows that add fewer than 8 new tiles; their tiles go on the pack sheet.
    seen, keep = set(), []
    for name, tx, ty in chosen:
        new = set(np.unique(w.inner(lv[name], tx, ty)).tolist()) - seen
        if len(new) >= 8:
            keep.append((name, tx, ty)); seen |= new
    used = set()
    for g in lv.values():
        used |= set(np.unique(g & 0x3FF).tolist())
    rare = sorted(used - seen)
    man = dict(world=world, scale=S, sources=[])
    for i, (name, tx, ty) in enumerate(keep):
        x, y = tx * s.T, ty * s.T
        idx = imgs[name][y:y + w.WIN_H, x:x + w.WIN_W]
        fn = f'win{i:02d}'
        Image.fromarray(pal[idx]).resize((w.WIN_W * S, w.WIN_H * S), Image.NEAREST).save(
            os.path.join(d, 'src', fn + '.png'))
        np.save(os.path.join(d, 'src', fn + '_idx.npy'), idx)
        # tiles: (tile, x, y) in original pixels of this image; trusted = away from the edge
        tiles = []
        for r in range(w.HT):
            for c in range(w.WT - 1):
                t = int(lv[name][ty + r, tx + c] & 0x3FF)
                trusted = w.MARGIN <= r < w.HT - w.MARGIN and w.MARGIN <= c < w.WT - 1 - w.MARGIN
                tiles.append([t, c * s.T, r * s.T, int(trusted)])
        man['sources'].append(dict(name=fn, kind='window', level=name, tx=tx, ty=ty, tiles=tiles))
    # pack sheet(s) for the rest
    per = PACK_COLS * PACK_ROWS
    for k in range(0, len(rare), per):
        sheet = np.full((w.WIN_H, w.WIN_W), tl[0][0, 0], np.uint8)   # sky
        tiles = []
        for j, t in enumerate(rare[k:k + per]):
            name, ry, rx = find_occurrence(lv, t)
            g = lv[name] & 0x3FF
            h, wd = g.shape
            y0 = min(max(ry - PATCH // 2, 0), h - PATCH); x0 = min(max(rx - PATCH // 2, 0), wd - PATCH)
            patch = s.render_grid(g[y0:y0 + PATCH, x0:x0 + PATCH], tl)
            px, py = (j % PACK_COLS) * PATCH * s.T + 4, (j // PACK_COLS) * PATCH * s.T + 32
            sheet[py:py + PATCH * s.T, px:px + PATCH * s.T] = patch
            for r in range(PATCH):
                for c in range(PATCH):
                    tiles.append([int(g[y0 + r, x0 + c]), px + c * s.T, py + r * s.T,
                                  int((y0 + r, x0 + c) == (ry, rx))])
        fn = f'pack{k // per:02d}'
        Image.fromarray(pal[sheet]).resize((w.WIN_W * S, w.WIN_H * S), Image.NEAREST).save(
            os.path.join(d, 'src', fn + '.png'))
        np.save(os.path.join(d, 'src', fn + '_idx.npy'), sheet)
        man['sources'].append(dict(name=fn, kind='pack', tiles=tiles))
    man['rare'] = rare
    with open(os.path.join(d, 'manifest.json'), 'w') as fh:
        json.dump(man, fh)
    return man
