"""Refit tiles that do not fit their neighbours: redraw each in its own context.

A tile with one or two copies in the paintings cannot be chosen to fit (tiles.choose_jointly
has nothing to choose from). Here each such tile is shown in a 3 x 3-tile panel made of its
neighbours' finished HD tiles, with its own original pixels (4x) in the middle, and the model
edits only the masked middle square (Nano Banana inpainting), so the redraw continues the
neighbours it actually has.

    python tools/hd/refit.py prepare <world> [n]   -> src/refit00.png, refit00_mask.png, refit.json
    (model: refit00.png + mask -> out/refit00.png)
    python tools/hd/refit.py apply <world>         -> used by tiles.build via refit.json
"""
import json
import os
import sys
from collections import Counter, defaultdict

import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from hd import analysis as an, source as s, tiles as tz  # noqa: E402
from hd.prepare import out_dir  # noqa: E402

COLS, ROWS = 9, 5                 # 45 panels of 3 x 3 tiles: 1728 x 960 at 4x, 16:9
P = 3 * tz.TS


def mismatch(hd, jt):
    bad, tot = defaultdict(float), defaultdict(float)
    for (a, b, ax), n in jt.items():
        if a == b or a not in hd or b not in hd:
            continue
        sa, sb = (1, 0) if ax == 1 else (3, 2)
        d = float(np.abs(tz.edges(hd[a])[sa] - tz.edges(hd[b])[sb]).mean())
        for t in (a, b):
            bad[t] += n * d; tot[t] += n
    return {t: bad[t] / tot[t] for t in bad}


def context(levels, canon, t):
    """The 3 x 3 neighbourhood of t that occurs most often in the levels."""
    lut = np.array([canon[i] for i in range(max(canon) + 1)])
    c = Counter()
    for g in levels.values():
        g = lut[g & 0x3FF]
        ys, xs = np.nonzero(g[1:-1, 1:-1] == t)
        for y, x in zip(ys + 1, xs + 1):
            c[g[y - 1:y + 2, x - 1:x + 2].tobytes()] += 1
    k, _ = c.most_common(1)[0]
    return np.frombuffer(k, dtype=lut.dtype).reshape(3, 3)


def prepare(world, n=COLS * ROWS):
    d = out_dir(world)
    tl = s.tiles(world); lv = s.levels(world); pal = s.palette(world)
    canon = an.canonical(tl); jt = an.joints(lv, canon)
    hd, _ = tz.build(world, refit=False)
    mm = mismatch(hd, jt)
    worst = [t for t, _ in sorted(mm.items(), key=lambda kv: -kv[1])[:n]]
    sheet = np.zeros((ROWS * P, COLS * P, 3), np.uint8)
    mask = np.zeros((ROWS * P, COLS * P), np.uint8)
    panels = []
    for j, t in enumerate(worst):
        ctx = context(lv, canon, t)
        y0, x0 = (j // COLS) * P, (j % COLS) * P
        for r in range(3):
            for c in range(3):
                u = int(ctx[r, c])
                p = tz.up(pal[tl[u]]) if (r, c) == (1, 1) else hd[u]
                sheet[y0 + r * tz.TS:y0 + (r + 1) * tz.TS, x0 + c * tz.TS:x0 + (c + 1) * tz.TS] = np.clip(p, 0, 255)
        mask[y0 + tz.TS:y0 + 2 * tz.TS, x0 + tz.TS:x0 + 2 * tz.TS] = 255
        panels.append(dict(tile=t, x=x0 + tz.TS, y=y0 + tz.TS, mismatch=round(mm[t], 1)))
    Image.fromarray(sheet).save(os.path.join(d, 'src', 'refit00.png'))
    Image.fromarray(mask).save(os.path.join(d, 'src', 'refit00_mask.png'))
    with open(os.path.join(d, 'refit.json'), 'w') as fh:
        json.dump(dict(sheet='refit00', size=[COLS * P, ROWS * P], panels=panels), fh, indent=1)
    print(len(panels), 'tiles, worst mismatch', panels[0]['mismatch'], 'last', panels[-1]['mismatch'])


def load(world):
    """tile -> refitted 64x64 patch, if a refit result exists."""
    d = out_dir(world)
    p = os.path.join(d, 'refit.json')
    if not os.path.exists(p):
        return {}
    meta = json.load(open(p))
    o = os.path.join(d, 'out', meta['sheet'] + '.png')
    if not os.path.exists(o):
        return {}
    w, h = meta['size']
    img = np.asarray(Image.open(o).convert('RGB').resize((w, h), Image.LANCZOS)).astype(np.float32)
    return {pn['tile']: img[pn['y']:pn['y'] + tz.TS, pn['x']:pn['x'] + tz.TS] for pn in meta['panels']}


if __name__ == '__main__':
    if sys.argv[1] == 'prepare':
        prepare(int(sys.argv[2]), *(int(a) for a in sys.argv[3:]))
