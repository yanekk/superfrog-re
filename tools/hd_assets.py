#!/usr/bin/env python3
"""HD asset pipeline: export a world's art for an image model, then rebuild
4x HD tiles, sprite sheets and level pictures from what the model returns.

  python tools/hd_assets.py prepare  [--world 1]   # build/hd/w1/src/*.png + manifest.json
  python tools/hd_assets.py assemble [--world 1]   # build/hd/w1/out/*.png -> build/hd/w1/final/

Every model input is exactly OUT_W x OUT_H (2752 x 1536, the 2k 16:9 size the
model returns), so an output maps back to its input by position alone: no
registration step. Level windows are 43 x 24 tiles at 4x (64 px per tile).
Sprite sheets go in halves at 8x for extra detail and are scaled back to 4x.

Tiles are restyled in context, never one by one: a tileset is fragments of
larger pictures (a tree spans dozens of tiles) that make no sense alone, and
tiles redrawn together come out joined. Each tile index is then taken from the
occurrence that downsamples closest to the original, and a seam pass evens out
the edges of tile pairs that the levels actually put next to each other.

Inputs come from build/decomp (tools/unpack.py). Nothing here touches the engine.
"""
import argparse, json, os, struct, sys
import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import t7mp
from render_levels import decode_tiles

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
# Worktrees have no build/decomp of their own; fall back to the main checkout's.
DECOMP = next(p for p in [os.path.join(ROOT, 'build', 'decomp'),
                          os.path.join(ROOT, '..', '..', '..', 'build', 'decomp')]
              if os.path.isdir(p))

OUT_W, OUT_H = 2752, 1536
S = 4                     # HD scale
T = 16                    # original tile size
WIN_W, WIN_H = OUT_W // (T * S), OUT_H // (T * S)   # 43 x 24 tiles
MARGIN = 2                # window-edge tiles the model sees but we do not trust
PATCH = 5                 # rare tiles go in packed 5x5-tile context patches
KEY = (255, 0, 255)       # transparent colour shown to the model

# Chunks per world (docs/10, docs/08). Only W1 is wired up for the pilot.
WORLDS = {
    1: dict(lp='d1_c17', bm='d1_c18', bo='d1_c19',
            levels=['d1_c21', 'd1_c22', 'd1_c23', 'd1_c24'], bob_table=0x2ed2c),
}
EXE = 'd1_c03'
HERO = 'd1_c04'
HERO_RECTS, HERO_N = 0x1a6f4, 96          # records 96..191 are mirror images
SHARED_RECTS, SHARED_N = 0x2c9b4, 64
WORLD_N = 128


def chunk(prefix):
    f = next(x for x in os.listdir(DECOMP) if x.startswith(prefix + '_'))
    return open(os.path.join(DECOMP, f), 'rb').read()


def palette(world):
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


def exe_rects(off, n):
    """Frame records from the game program: x, y, w, h, hotspot dx, dy.
    Hunk 0's data starts at file offset 0x2c (docs/08)."""
    d = chunk(EXE)
    return [dict(zip('x y w h dx dy'.split(), struct.unpack_from('>4H2h', d, 0x2c + off + 12 * i)))
            for i in range(n)]


def levels(world):
    out = {}
    for i, c in enumerate(WORLDS[world]['levels']):
        tags = {t: p for t, _, _, p in t7mp.parse(chunk(c))}
        w = struct.unpack('>I', tags['XBLK'])[0]
        h = struct.unpack('>I', tags['YBLK'])[0]
        out[f'W{world}-{i + 1}'] = np.array(struct.unpack(f'>{w * h}H', tags['BODY'][:w * h * 2])
                                            ).reshape(h, w) & 0x3FF
    return out


def tiles_rgb(world):
    return palette(world)[decode_tiles(chunk(WORLDS[world]['bm']))]   # (n,16,16,3)


def render_grid(grid, tiles):
    h, w = grid.shape
    return tiles[grid].transpose(0, 2, 1, 3, 4).reshape(h * T, w * T, 3)


def up(a, k):
    return np.repeat(np.repeat(a, k, 0), k, 1)


def cover_windows(lv):
    """Greedy set cover: few windows whose trusted interior shows every tile used."""
    need = set()
    for g in lv.values():
        need |= set(np.unique(g).tolist())
    cands = []
    for k, g in lv.items():
        h, w = g.shape
        for y in range(0, h - WIN_H + 1, 2):
            for x in range(0, w - WIN_W + 1, 2):
                inner = g[y + MARGIN:y + WIN_H - MARGIN, x + MARGIN:x + WIN_W - MARGIN]
                cands.append((k, x, y, set(np.unique(inner).tolist())))
    left, picks = set(need), []
    while left:
        k, x, y, s = max(cands, key=lambda c: len(c[3] & left))
        gain = s & left
        # A window that adds only a few tiles is cheaper as patches in a packed sheet.
        if len(gain) < 12:
            break
        picks.append((k, x, y))
        left -= gain
    return picks, sorted(left)


def find_occurrence(lv, t):
    """A spot where tile t sits with a full PATCH context around it, preferring
    spots surrounded by non-sky so the model sees what the tile belongs to."""
    r = PATCH // 2
    best = None
    for k, g in lv.items():
        ys, xs = np.nonzero(g == t)
        for y, x in zip(ys, xs):
            if r <= y < g.shape[0] - r and r <= x < g.shape[1] - r:
                ctx = g[y - r:y + r + 1, x - r:x + r + 1]
                score = len(np.unique(ctx))
                if best is None or score > best[0]:
                    best = (score, k, int(x), int(y))
    return best[1:] if best else None


def load_overrides(world):
    path = os.path.join(ROOT, 'tools', 'hd_overrides', f'w{world}.json')
    return json.load(open(path)) if os.path.exists(path) else {}


def prepare(world):
    base = os.path.join(ROOT, 'build', 'hd', f'w{world}')
    src = os.path.join(base, 'src')
    os.makedirs(src, exist_ok=True)
    pal = palette(world)
    tiles = tiles_rgb(world)
    lv = levels(world)
    man = dict(world=world, out_size=[OUT_W, OUT_H], scale=S, inputs=[])

    picks, rare = cover_windows(lv)
    for i, (k, x, y) in enumerate(picks):
        img = render_grid(lv[k][y:y + WIN_H, x:x + WIN_W], tiles)
        name = f'win{i:02d}'
        Image.fromarray(up(img, S)).save(os.path.join(src, name + '.png'))
        man['inputs'].append(dict(name=name, kind='window', level=k, x=x, y=y))

    # Packed sheets: 8 x 4 patches of 5x5 tiles fill 40x20 of the 43x24 canvas.
    per_row, per_col = WIN_W // PATCH, WIN_H // PATCH
    spots = [(t, find_occurrence(lv, t)) for t in rare]
    missing = [t for t, s in spots if s is None]
    spots = [(t, s) for t, s in spots if s is not None]
    r = PATCH // 2
    for p in range(0, len(spots), per_row * per_col):
        canvas = np.zeros((WIN_H * T, WIN_W * T, 3), np.uint8)
        canvas[:] = pal[0]
        cells = []
        for j, (t, (k, x, y)) in enumerate(spots[p:p + per_row * per_col]):
            cx, cy = (j % per_row) * PATCH, (j // per_row) * PATCH
            canvas[cy * T:(cy + PATCH) * T, cx * T:(cx + PATCH) * T] = \
                render_grid(lv[k][y - r:y + r + 1, x - r:x + r + 1], tiles)
            cells.append(dict(tile=int(t), col=cx + r, row=cy + r))
        name = f'pack{p // (per_row * per_col):02d}'
        Image.fromarray(up(canvas, S)).save(os.path.join(src, name + '.png'))
        man['inputs'].append(dict(name=name, kind='pack', cells=cells))

    # Sprite sheets in halves at 8x on the key colour, padded to the canvas.
    hero = planar(chunk(HERO), 320, 384, 4)
    hero_rgb = pal[hero + 16]
    hero_rgb[hero == 0] = KEY
    bo = planar(chunk(WORLDS[world]['bo']), 320, 320, 5)
    bo_rgb = pal[bo]
    bo_rgb[bo == 0] = KEY
    for sheet, rgb in (('hero', hero_rgb), (f'bobs{world}', bo_rgb)):
        h = rgb.shape[0] // 2
        for half in range(2):
            part = up(rgb[half * h:(half + 1) * h], 2 * S)
            canvas = np.zeros((OUT_H, OUT_W, 3), np.uint8)
            canvas[:] = KEY
            canvas[:part.shape[0], :part.shape[1]] = part
            name = f'{sheet}_{half}'
            Image.fromarray(canvas).save(os.path.join(src, name + '.png'))
            man['inputs'].append(dict(name=name, kind='sheet', sheet=sheet, half=half,
                                      rows=h, scale=2 * S))
    # Detail sheets: small tiles that must match each other (the letter coins),
    # drawn together at 8x so the model can read them, each on a 3x3-tile block
    # of its own background colour. Listed in tools/hd_overrides/w<N>.json.
    idx = decode_tiles(chunk(WORLDS[world]['bm']))
    for d in load_overrides(world).get('detail', []):
        k = d.get('scale', 2 * S)
        block = 3 * T * k
        per_row = OUT_W // block
        canvas = np.zeros((OUT_H, OUT_W, 3), np.uint8)
        canvas[:] = pal[idx[d['tiles'][0]][0, 0]]
        cells = []
        for j, t in enumerate(d['tiles']):
            by, bx = (j // per_row) * block, (j % per_row) * block
            canvas[by:by + block, bx:bx + block] = pal[idx[t][0, 0]]
            y, x = by + T * k, bx + T * k
            canvas[y:y + T * k, x:x + T * k] = up(tiles[t], k)
            cells.append(dict(tile=int(t), y=y, x=x))
        Image.fromarray(canvas).save(os.path.join(src, d['name'] + '.png'))
        man['inputs'].append(dict(name=d['name'], kind='detail', scale=k, cells=cells))
    man['missing_tiles'] = [int(t) for t in missing]
    json.dump(man, open(os.path.join(base, 'manifest.json'), 'w'), indent=1)
    print(f'{len(picks)} windows, {len(spots)} packed tiles in '
          f'{sum(i["kind"] == "pack" for i in man["inputs"])} sheets, '
          f'{len(missing)} without context, 4 sprite halves -> {src}')


def register(out, src, k=4):
    """The model keeps the layout but not the framing exactly: outputs come back
    about 1% scaled and a few HD pixels shifted. Fit x/y scale and shift
    (out = src * s + t, per axis) by coarse-to-fine search on a k-times
    downsampled copy, so tiles are cut from where they really landed."""
    small = lambda a: np.array(Image.fromarray(a.astype(np.uint8)).resize(
        (a.shape[1] // k, a.shape[0] // k), Image.BOX)).astype(np.float32)
    s_src, s_out = small(src), small(out)
    h, w = s_src.shape[:2]
    best = (1e9, 1.0, 1.0, 0.0, 0.0)
    for step_s, step_t, span in ((0.005, 2.0, 4), (0.001, 0.5, 4), (0.0005, 0.25, 3)):
        _, sx0, sy0, tx0, ty0 = best
        for sx in sx0 + step_s * np.arange(-span, span + 1):
            for sy in sy0 + step_s * np.arange(-span, span + 1):
                for tx in tx0 + step_t * np.arange(-span, span + 1):
                    for ty in ty0 + step_t * np.arange(-span, span + 1):
                        im = Image.fromarray(s_out.astype(np.uint8)).transform(
                            (w, h), Image.AFFINE, (sx, 0, tx, 0, sy, ty), Image.BILINEAR)
                        e = np.abs(np.array(im, np.float32) - s_src)[8:-8, 8:-8].mean()
                        if e < best[0]:
                            best = (e, sx, sy, tx, ty)
    _, sx, sy, tx, ty = best
    return (float(sx), float(sy), float(tx * k), float(ty * k))


def warp(out, a):
    sx, sy, tx, ty = a
    return np.array(Image.fromarray(out.astype(np.uint8)).transform(
        (out.shape[1], out.shape[0]), Image.AFFINE, (sx, 0, tx, 0, sy, ty), Image.BICUBIC)
    ).astype(np.float32)


def load_registered(base, name):
    """A model output, aligned onto its input. Fits are cached in
    registration.json because the search takes most of a minute per image."""
    cache_path = os.path.join(base, 'registration.json')
    cache = json.load(open(cache_path)) if os.path.exists(cache_path) else {}
    out = np.array(Image.open(os.path.join(base, 'out', name + '.png')).convert('RGB')
                   .resize((OUT_W, OUT_H), Image.LANCZOS)).astype(np.float32)
    if name not in cache:
        src = np.array(Image.open(os.path.join(base, 'src', name + '.png')).convert('RGB'))
        cache[name] = register(out, src.astype(np.float32))
        json.dump(cache, open(cache_path, 'w'), indent=1)
        print('registered', name, cache[name])
    return warp(out, cache[name])


def down(a, k):
    h, w = a.shape[0] // k, a.shape[1] // k
    return a[:h * k, :w * k].reshape(h, k, w, k, -1).mean((1, 3))


def blur(a, sigma, axes=(0, 1)):
    """Separable Gaussian blur with edge padding (no scipy in the venv)."""
    r = max(1, int(3 * sigma))
    k = np.exp(-0.5 * (np.arange(-r, r + 1) / sigma) ** 2)
    k /= k.sum()
    out = a.astype(np.float32)
    for ax in axes:
        pad = [(0, 0)] * out.ndim
        pad[ax] = (r, r)
        q = np.pad(out, pad, mode='edge')
        out = sum(w * np.take(q, range(i, i + out.shape[ax]), axis=ax) for i, w in enumerate(k))
    return out


def normalise(patch, orig, idx):
    """Pull a redrawn tile back onto the original's colours without undoing the redraw.
    The model repaints flat areas a slightly different shade in each image, so
    tiles cut from different images showed as darker or lighter boxes in the sky
    around coins and apples. Two steps:
    - Flat areas (an original pixel whose 3x3 neighbourhood is one palette
      index) are set to that exact colour, feathered 2 px so redrawn edges stay.
    - Remaining drift is corrected at low frequency: the original minus the
      redraw shrunk to 16 px, blurred, enlarged back and added."""
    patch = patch.astype(np.float32)
    o4 = up(orig, S).astype(np.float32)
    q = np.pad(idx, 1, mode='edge')
    flat = np.ones(idx.shape, bool)
    for dy in range(3):
        for dx in range(3):
            flat &= q[dy:dy + T, dx:dx + T] == idx
    w = blur(up(flat.astype(np.float32), S), 2.0)[..., None]
    patch = patch * (1 - w) + o4 * w
    diff = blur(orig.astype(np.float32) - down(patch, S), 1.0)
    diff_up = np.stack([np.array(Image.fromarray(diff[..., c]).resize((T * S, T * S), Image.BILINEAR))
                        for c in range(3)], -1)
    return np.clip(patch + diff_up, 0, 255)


def seam_fix(hd, lv, orig, idx, iters=3, band=8, along=4.0, clamp=4):
    """Make tile borders behave like the original's.

    For every border pixel, look at all the places the levels put two tiles
    side by side and ask whether the ORIGINAL art continues across that pixel
    (same palette index on both sides) or has an edge exactly there.

    - Where it continues (sky into sky, dirt into dirt) the two touching lines
      are evened toward their mean, smoothed `along` px along the border and
      faded over `band` px, so tiles redrawn in different images join up.
    - Where the original has an edge on the border (the mushroom cap's top
      outline against plain sky, a coin above the grass line), nothing is
      averaged: averaging blended outlines into the sky and the grass line
      into the coin. Instead the outer `clamp` px are pulled to the
      original's edge colours, so an outline the model drew half across the
      border is not lost.
    Both weights are the fraction of neighbour occurrences of each kind."""
    n, P = len(hd), T * S
    pairs = {}
    for g in lv.values():
        for a, b in zip(g[:, :-1].ravel(), g[:, 1:].ravel()):
            pairs[('h', int(a), int(b))] = pairs.get(('h', int(a), int(b)), 0) + 1
        for a, b in zip(g[:-1, :].ravel(), g[1:, :].ravel()):
            pairs[('v', int(a), int(b))] = pairs.get(('v', int(a), int(b)), 0) + 1
    # Original border lines (index and colour) per side: left, right, top, bottom.
    side_idx = lambda t, s: (idx[t][:, 0], idx[t][:, -1], idx[t][0], idx[t][-1])[s]
    # Edge colours come from a smooth (bicubic) enlargement of the original:
    # square-block colours turned the round left edge of coins and apples
    # into steps wherever the outline touches the border.
    def smooth(t):
        a = np.pad(orig[t].astype(np.uint8), ((2, 2), (2, 2), (0, 0)), mode='edge')
        b = np.array(Image.fromarray(a).resize(((T + 4) * S, (T + 4) * S), Image.BICUBIC))
        return b[2 * S:-2 * S, 2 * S:-2 * S].astype(np.float32)
    side_smooth = lambda t, s, o: (o[:, 0], o[:, -1], o[0], o[-1])[s]
    same = np.zeros((n, 4, T), np.float32)    # neighbour occurrences continuing the art
    total = np.zeros((n, 4, T), np.float32)
    for (d, a, b), c in pairs.items():
        sa, sb = (1, 0) if d == 'h' else (3, 2)
        eq = (side_idx(a, sa) == side_idx(b, sb)).astype(np.float32)
        same[a, sa] += c * eq; total[a, sa] += c
        same[b, sb] += c * eq; total[b, sb] += c
    cont = np.repeat(np.where(total > 0, same / np.maximum(total, 1), 1.0), S, axis=2)  # (n,4,P)
    ramp = np.clip(1 - np.arange(P) / band, 0, 1)
    cramp = np.clip(1 - np.arange(P) / clamp, 0, 1)

    def apply(t, s, d, r):
        if s == 0:
            hd[t] += d[:, None, :] * r[None, :, None]
        elif s == 1:
            hd[t] += d[:, None, :] * r[::-1][None, :, None]
        elif s == 2:
            hd[t] += d[None, :, :] * r[:, None, None]
        else:
            hd[t] += d[None, :, :] * r[::-1][:, None, None]

    hd = hd.astype(np.float32)
    line = lambda t, s: (hd[t][:, 0], hd[t][:, -1], hd[t][0], hd[t][-1])[s]
    # Edges in the original: pull the border band to the original's colours.
    for t in range(n):
        for s in range(4):
            w = (1 - cont[t, s])[:, None]
            if w.max() == 0:
                continue
            tgt = side_smooth(t, s, smooth(t))
            apply(t, s, w * (tgt - line(t, s)), cramp)
    # Continuous art: even the two sides toward each other.
    for _ in range(iters):
        acc = np.zeros((n, 4, P, 3), np.float32)
        wt = np.zeros((n, 4, P), np.float32)
        for (d, a, b), c in pairs.items():
            sa, sb = (1, 0) if d == 'h' else (3, 2)
            eq = np.repeat((side_idx(a, sa) == side_idx(b, sb)).astype(np.float32), S) * c
            m = (line(a, sa) + line(b, sb)) / 2
            acc[a, sa] += eq[:, None] * m; wt[a, sa] += eq
            acc[b, sb] += eq[:, None] * m; wt[b, sb] += eq
        for t in range(n):
            for s in range(4):
                k = wt[t, s] > 0
                if not k.any():
                    continue
                tgt = np.where(k[:, None], acc[t, s] / np.maximum(wt[t, s], 1e-6)[:, None], line(t, s))
                d = blur((tgt - line(t, s)) * cont[t, s][:, None], along, axes=(0,))
                apply(t, s, d, ramp)
    return np.clip(hd, 0, 255).astype(np.uint8)


def seam_score(hd, lv):
    """Mean colour step across tile edges divided by the mean step between
    neighbouring pixels inside tiles: 1.0 means seams are invisible."""
    cross, inner = [], []
    for g in lv.values():
        a, b = g[:, :-1].ravel(), g[:, 1:].ravel()
        cross.append(np.abs(hd[a][:, :, -1].astype(int) - hd[b][:, :, 0]).mean())
    inner = np.abs(np.diff(hd.astype(int), axis=2)).mean()
    return float(np.mean(cross) / inner)


def shortlist(cands, k=10):
    """The k most plausible HD versions of one tile, with a cost each.
    Neither rule alone works: lowest error against the original rewards copies
    the model left blocky, and the most typical copy (medoid) loses when the
    model guesses differently every time (a 16 px bonus letter came back as R,
    A, n, R, B). So the cost is typicality plus fidelity, less a small bonus
    for sitting far from a window edge, where drift is largest."""
    cands = cands[:60]
    errs = np.array([c[0] for c in cands])
    small = np.stack([down(c[2].astype(np.float32), 4) for c in cands])
    typ = np.abs(small[:, None] - small[None]).mean((2, 3, 4)).mean(1) if len(cands) > 1 \
        else np.zeros(1)
    cost = typ + errs - 0.3 * np.array([min(c[1], 6) for c in cands])
    order = np.argsort(cost)[:k]
    return [cands[i][2] for i in order], cost[order], errs[order]


def choose_jointly(short, lv, n, iters=6, lam=1.0):
    """Pick one version per tile so that tiles the levels put side by side
    have matching touching edges. Neighbours drawn in different model images
    can disagree about where an outline crosses the boundary, which shows as
    flecks and steps; this prefers combinations whose edges agree (often
    copies from the same image). Iterated conditional modes: start from each
    tile's cheapest version, then re-pick each tile given its neighbours'
    current picks until nothing changes."""
    pairs = {}
    for g in lv.values():
        for a, b in zip(g[:, :-1].ravel(), g[:, 1:].ravel()):
            pairs[('h', int(a), int(b))] = pairs.get(('h', int(a), int(b)), 0) + 1
        for a, b in zip(g[:-1, :].ravel(), g[1:, :].ravel()):
            pairs[('v', int(a), int(b))] = pairs.get(('v', int(a), int(b)), 0) + 1
    # Edge lines of every version: left, right, top, bottom (2 px deep).
    edges = {t: dict(L=np.stack([p[:, :2].mean(1) for p in ps]).astype(np.float32),
                     R=np.stack([p[:, -2:].mean(1) for p in ps]).astype(np.float32),
                     T=np.stack([p[:2].mean(0) for p in ps]).astype(np.float32),
                     B=np.stack([p[-2:].mean(0) for p in ps]).astype(np.float32))
             for t, (ps, _, _) in short.items()}
    # Per tile: (my side, other tile, other side, weight); weights sum to 1 per tile
    # so a tile with thousands of neighbours does not outvote its own cost.
    nb = {t: [] for t in short}
    for (d, a, b), c in pairs.items():
        if a in short and b in short:
            sa, sb = ('R', 'L') if d == 'h' else ('B', 'T')
            nb[a].append((sa, b, sb, c))
            nb[b].append((sb, a, sa, c))
    for t in nb:
        tot = sum(x[3] for x in nb[t]) or 1
        nb[t] = [(s, o, so, c / tot) for s, o, so, c in nb[t]]
    choice = {t: 0 for t in short}
    for _ in range(iters):
        changed = 0
        for t, (ps, cost, _) in short.items():
            total = cost.copy()
            for s, o, so, w in nb[t]:
                other = edges[o][so][choice[o]]
                total += lam * w * np.abs(edges[t][s] - other).mean((1, 2))
            j = int(np.argmin(total))
            changed += j != choice[t]
            choice[t] = j
        if not changed:
            break
    return choice


def assemble(world):
    base = os.path.join(ROOT, 'build', 'hd', f'w{world}')
    man = json.load(open(os.path.join(base, 'manifest.json')))
    outd, fin = os.path.join(base, 'out'), os.path.join(base, 'final')
    os.makedirs(fin, exist_ok=True)
    tiles = tiles_rgb(world).astype(np.float32)
    lv = levels(world)
    P = T * S
    cand = {}          # tile -> [(error vs original, centrality, hd patch)]

    def take(t, im, y, x, central, src):
        """Cut tile t at (y, x), nudged up to 6 px to where it best matches the
        original. Whole-image registration leaves local drift of a few pixels,
        and at that size drift outweighs real differences: a misdrawn coin
        letter scored better than correct ones that sat 4 px off."""
        best = None
        for dy in range(-6, 7, 2):
            for dx in range(-6, 7, 2):
                yy, xx = y + dy, x + dx
                if yy < 0 or xx < 0 or yy + P > im.shape[0] or xx + P > im.shape[1]:
                    continue
                patch = im[yy:yy + P, xx:xx + P]
                err = float(np.abs(down(patch.astype(np.float32), S) - tiles[t]).mean())
                if best is None or err < best[0]:
                    best = (err, patch)
        cand.setdefault(t, []).append((best[0], central, best[1], src))

    def take_fitted(t, im, y, x, src):
        """Like take, but also fits size. A letter coin fills its tile edge to
        edge, and the model drew it ~2% larger, so a plain cut sliced its left
        edge flat; shrinking the cut box to the fitted size keeps it round."""
        best = None
        cy, cx = y + P / 2, x + P / 2
        for sc in np.arange(0.92, 1.041, 0.01):
            half = P / sc / 2
            for dy in range(-6, 7, 2):
                for dx in range(-6, 7, 2):
                    box = (cx + dx - half, cy + dy - half, cx + dx + half, cy + dy + half)
                    patch = np.array(Image.fromarray(im).resize((P, P), Image.LANCZOS, box=box))
                    err = float(np.abs(down(patch.astype(np.float32), S) - tiles[t]).mean())
                    if best is None or err < best[0]:
                        best = (err, patch)
        cand.setdefault(t, []).append((best[0], 0, best[1], src))

    for inp in man['inputs']:
        path = os.path.join(outd, inp['name'] + '.png')
        if not os.path.exists(path):
            print('missing model output', inp['name']); continue
        im = load_registered(base, inp['name']).astype(np.uint8)
        if inp['kind'] == 'window':
            g = lv[inp['level']][inp['y']:inp['y'] + WIN_H, inp['x']:inp['x'] + WIN_W]
            for r in range(MARGIN, WIN_H - MARGIN):
                for c in range(MARGIN, WIN_W - MARGIN):
                    central = min(r, c, WIN_H - 1 - r, WIN_W - 1 - c)
                    take(int(g[r, c]), im, r * P, c * P, central, inp['name'])
        elif inp['kind'] == 'pack':
            for cell in inp['cells']:
                take(cell['tile'], im, cell['row'] * P, cell['col'] * P, 0, inp['name'])
        elif inp['kind'] == 'detail':
            f = inp['scale'] // S
            small = np.array(Image.fromarray(im).resize((OUT_W // f, OUT_H // f), Image.LANCZOS))
            for cell in inp['cells']:
                take_fitted(cell['tile'], small, cell['y'] // f, cell['x'] // f, inp['name'])

    n = len(tiles)
    hd = np.zeros((n, P, P, 3), np.uint8)
    used = set()
    for g in lv.values():
        used |= set(np.unique(g).tolist())
    fallback, errs = [], []
    # Hand-picked sources for tiles no automatic rule gets right: tiny bonus
    # letters, where the model guesses a different letter each time and the
    # wrong one can match the original's colours best.
    overrides = load_overrides(world)
    forced = dict(overrides.get('tile_source', {}))
    detail_names = {d['name'] for d in overrides.get('detail', [])}
    regular = {}           # detail tiles' ordinary copies, for their background
    for d in overrides.get('detail', []):
        forced.update({str(t): d['name'] for t in d['tiles']})
        for t in d['tiles']:
            regular[t] = [c for c in cand.get(t, []) if c[3] not in detail_names]
    for t, src in forced.items():
        only = [c for c in cand.get(int(t), []) if c[3] == src]
        if only:
            cand[int(t)] = only
        else:
            print('override ignored, no copy of tile', t, 'in', src)
    short = {t: shortlist(c) for t, c in cand.items()}
    choice = choose_jointly(short, lv, n)
    for t in range(n):
        if t in short:
            ps, _, e = short[t]
            hd[t] = ps[choice[t]]
            errs.append(float(e[choice[t]]))
        else:
            hd[t] = up(tiles[t].astype(np.uint8), S)
            if t in used:
                fallback.append(t)
    # A detail sheet sets each tile on a flat block of its corner colour, so the
    # model returns a flat background even where the original had texture (an
    # R coin set in a dirt wall came back on plain brown). Keep only the object
    # from the detail sheet and take the background from the tile's ordinary
    # best copy.
    for t, reg in regular.items():
        if t in short and reg:
            obj = hd[t].astype(np.float32)
            bg = obj[:3, :3].reshape(-1, 3).mean(0)
            m = blur((np.abs(obj - bg).sum(-1) > 40).astype(np.float32), 1.5)[..., None]
            back = shortlist(reg)[0][0].astype(np.float32)
            hd[t] = (obj * m + back * (1 - m)).astype(np.uint8)
    idx = decode_tiles(chunk(WORLDS[world]['bm']))
    for t in short:
        hd[t] = normalise(hd[t], tiles[t], idx[t]).astype(np.uint8)
    before = seam_score(hd, lv)
    hd = seam_fix(hd, lv, tiles, idx)
    after = seam_score(hd, lv)

    cols = 40
    sheet = np.zeros((((n + cols - 1) // cols) * P, cols * P, 3), np.uint8)
    for t in range(n):
        sheet[(t // cols) * P:(t // cols + 1) * P, (t % cols) * P:(t % cols + 1) * P] = hd[t]
    Image.fromarray(sheet).save(os.path.join(fin, f'tiles_W{world}_hd.png'))
    for k, g in lv.items():
        img = hd[g].transpose(0, 2, 1, 3, 4).reshape(g.shape[0] * P, g.shape[1] * P, 3)
        Image.fromarray(img).save(os.path.join(fin, f'level_{k}_hd.png'))
        Image.fromarray(img).resize((img.shape[1] // 8, img.shape[0] // 8), Image.LANCZOS) \
            .save(os.path.join(fin, f'level_{k}_preview.png'))

    sprites = assemble_sprites(world, man, outd, fin)
    report = dict(tiles_from_model=len(cand), tiles_used=len(used), fallback_tiles=fallback,
                  mean_error_vs_original=float(np.mean(errs)) if errs else None,
                  seam_score_before=before, seam_score_after=after, sprites=sprites)
    json.dump(report, open(os.path.join(fin, 'report.json'), 'w'), indent=1)
    print(json.dumps({k: v for k, v in report.items() if k != 'sprites'}, indent=1))


def assemble_sprites(world, man, outd, fin):
    """Rejoin sheet halves at 4x with alpha. Alpha is the model's key colour
    cut, limited to a slightly grown original silhouette so the model cannot
    add stray shapes or change the outline the game collides against."""
    pal = palette(world)
    raw = dict(hero=planar(chunk(HERO), 320, 384, 4),
               **{f'bobs{world}': planar(chunk(WORLDS[world]['bo']), 320, 320, 5)})
    res = {}
    for sheet, idx in raw.items():
        halves = sorted((i for i in man['inputs'] if i.get('sheet') == sheet), key=lambda i: i['half'])
        if not all(os.path.exists(os.path.join(outd, i['name'] + '.png')) for i in halves):
            continue
        parts = []
        for i in halves:
            k = i['scale']
            im = load_registered(os.path.dirname(outd), i['name'])
            im = im[:i['rows'] * k, :idx.shape[1] * k]
            parts.append(np.array(Image.fromarray(im.astype(np.uint8)).resize(
                (idx.shape[1] * S, i['rows'] * S), Image.LANCZOS)).astype(np.float32))
        rgb = np.concatenate(parts, 0)
        key_dist = np.abs(rgb - np.array(KEY, np.float32)).sum(-1)
        a_model = np.clip((key_dist - 60) / 120, 0, 1)
        sil = np.array(Image.fromarray(up((idx > 0).astype(np.uint8) * 255, S))
                       .filter(__import__('PIL.ImageFilter', fromlist=['x']).MaxFilter(5))
                       .filter(__import__('PIL.ImageFilter', fromlist=['x']).GaussianBlur(1.5))) / 255
        alpha = (a_model * sil * 255).astype(np.uint8)
        # Pull key-coloured fringe toward the nearest real colour: unpremultiply-ish.
        rgba = np.dstack([rgb.astype(np.uint8), alpha])
        Image.fromarray(rgba).save(os.path.join(fin, f'{sheet}_hd.png'))
        res[sheet] = dict(size=[int(rgb.shape[1]), int(rgb.shape[0])])
    # Frame rectangles, scaled to HD, so a consumer can cut frames like the game does.
    frames = dict(scale=S,
                  hero=[r for r in exe_rects(HERO_RECTS, HERO_N)],
                  shared=[r for r in exe_rects(SHARED_RECTS, SHARED_N)],
                  world=[r for r in exe_rects(WORLDS[world]['bob_table'], WORLD_N)])
    json.dump(frames, open(os.path.join(fin, 'frames.json'), 'w'), indent=1)
    # One PNG per frame, cut the way the game cuts them. Shared and world
    # records both address the world's bob sheet; empty records are skipped.
    for table, sheet in (('hero', 'hero'), ('shared', f'bobs{world}'), ('world', f'bobs{world}')):
        path = os.path.join(fin, f'{sheet}_hd.png')
        if not os.path.exists(path):
            continue
        im = Image.open(path)
        d = os.path.join(fin, 'frames', table)
        os.makedirs(d, exist_ok=True)
        for i, r in enumerate(frames[table]):
            if r['w'] and r['h']:
                im.crop((r['x'] * S, r['y'] * S, (r['x'] + r['w']) * S, (r['y'] + r['h']) * S)) \
                  .save(os.path.join(d, f'{i:03d}.png'))
    return res


if __name__ == '__main__':
    ap = argparse.ArgumentParser()
    ap.add_argument('cmd', choices=['prepare', 'assemble'])
    ap.add_argument('--world', type=int, default=1)
    a = ap.parse_args()
    (prepare if a.cmd == 'prepare' else assemble)(a.world)
