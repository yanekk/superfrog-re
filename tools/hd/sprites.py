"""Sprite sheets (frog + Spud, the world's bobs: enemies, pickups, effects) in HD.

Each sheet goes to the model in two halves at 8x on flat magenta, centred in a 324 x 192
canvas (2592 x 1536 at 8x, GPT Image's native 27:16). The result is registered onto the
original like a level window, reduced to 4x, and given alpha from the ORIGINAL silhouette
(smoothed), so the outline the game collides against cannot change.
"""
import json
import os

import numpy as np
from PIL import Image

from . import register as rg
from . import source as s
from .prepare import out_dir
from .register import blur

KEY = np.array((255, 0, 255), np.uint8)
K = 8                                   # model input scale
S = 4                                   # output scale (matches the tiles)
CW, CH = 324, 192                       # canvas, original pixels


def sheets(world):
    # the frog sheet is the same in every world: drawn once, with World 1
    out = {'hero': s.hero()} if world == 1 else {}
    out[f'bobs{world}'] = s.bobs(world)
    return out


def halves(idx):
    h = idx.shape[0] // 2
    return [(0, idx[:h]), (h, idx[h:])]


def canvas(part):
    c = np.zeros((CH, CW), np.uint8)
    y0 = (CH - part.shape[0]) // 2; x0 = (CW - part.shape[1]) // 2
    c[y0:y0 + part.shape[0], x0:x0 + part.shape[1]] = part
    return c, (x0, y0)


def rgb(world, idx):
    out = s.palette(world)[idx]
    out[idx == 0] = KEY
    return out


def prepare(world):
    d = out_dir(world)
    os.makedirs(os.path.join(d, 'src'), exist_ok=True)
    man = []
    for name, idx in sheets(world).items():
        for i, (row, part) in enumerate(halves(idx)):
            c, (x0, y0) = canvas(part)
            fn = f'{name}_{i}'
            Image.fromarray(rgb(world, c)).resize((CW * K, CH * K), Image.NEAREST).save(
                os.path.join(d, 'src', fn + '.png'))
            np.save(os.path.join(d, 'src', fn + '_idx.npy'), c)
            man.append(dict(name=fn, sheet=name, row=row, rows=part.shape[0], x0=x0, y0=y0))
    with open(os.path.join(d, 'sprites.json'), 'w') as fh:
        json.dump(man, fh, indent=1)
    return man


def fill_from_neighbours(img, holes, iters=12):
    """Replace hole pixels by the mean of their non-hole neighbours, growing inward."""
    img = img.copy(); holes = holes.copy()
    for _ in range(iters):
        if not holes.any():
            break
        ok = (~holes).astype(np.float32)
        num = sum(np.roll(np.roll(img * ok[..., None], dy, 0), dx, 1)
                  for dy in (-1, 0, 1) for dx in (-1, 0, 1))
        den = sum(np.roll(np.roll(ok, dy, 0), dx, 1) for dy in (-1, 0, 1) for dx in (-1, 0, 1))
        new = holes & (den > 0)
        img[new] = num[new] / den[new][:, None]
        holes &= ~new
    return img


def frame_rects(world, sheet):
    if sheet == 'hero':
        rs = s.exe_rects(s.HERO_RECTS, s.HERO_N)
    else:
        rs = s.exe_rects(s.SHARED_RECTS, s.SHARED_N) + s.world_rects(world)
    return sorted({(r['x'], r['y'], r['w'], r['h']) for r in rs if r['w'] and r['h']})


def unify_frames(world, idx, hd, rects):
    """Frames with the same silhouette share one HD drawing, recoloured per frame.

    The model draws each copy separately and sometimes gets one wrong (it turned three of four
    '200' score sprites into '201'). The copy whose edges best match its original is used for
    all; each copy keeps its own colours through the per-pixel palette ratio, so colour-cycled
    frames (a fading score) still fade.
    """
    pal = s.palette(world).astype(np.float32)
    groups = {}
    for (x, y, w, h) in rects:
        c = idx[y:y + h, x:x + w]
        if (c > 0).sum() < 8:
            continue
        # same picture up to a one-to-one change of colours: canonical relabelling
        _, inv = np.unique(c, return_inverse=True)
        order = {}
        lab = np.array([order.setdefault(v, len(order)) for v in inv.ravel()]).reshape(c.shape)
        groups.setdefault((lab.tobytes(), w, h), []).append((x, y))
    n = 0
    for (mk, w, h), members in groups.items():
        if len(members) < 2:
            continue
        def score(m):
            x, y = m
            o = rg.features(np.repeat(np.repeat(pal[idx[y:y + h, x:x + w]], 2, 0), 2, 1))
            p = hd[y * S:(y + h) * S, x * S:(x + w) * S]
            f = rg.features(np.asarray(Image.fromarray(np.clip(p, 0, 255).astype(np.uint8)).resize(
                (w * 2, h * 2), Image.LANCZOS)))
            return rg.ncc(o, f)
        bx, by = max(members, key=score)
        ref = hd[by * S:(by + h) * S, bx * S:(bx + w) * S].copy()
        kr = np.repeat(np.repeat(idx[by:by + h, bx:bx + w], S, 0), S, 1)
        for (x, y) in members:
            if (x, y) == (bx, by):
                continue
            kt = np.repeat(np.repeat(idx[y:y + h, x:x + w], S, 0), S, 1)
            out = ref.copy()
            # each colour area keeps the reference's shading, moved to the colour that area has
            # in this frame (scaled by how the reference's area relates to its palette colour)
            for k in np.unique(kr):
                if k == 0:
                    continue
                m = kr == k
                t = int(kt[m][0])
                if t == k:
                    continue
                mean = ref[m].mean(0)
                want = pal[t] * (mean + 8) / (pal[k] + 8)
                out[m] = ref[m] - mean + want
            hd[y * S:(y + h) * S, x * S:(x + w) * S] = np.clip(out, 0, 255)
            n += 1
    return n


def crisp_upscale(world, idx):
    """Original pixels enlarged with smooth outlines: each colour's area is enlarged bicubically
    and thresholded. Used for text, where the model guesses characters."""
    pal = s.palette(world)
    h, w = idx.shape
    out = np.zeros((h * S, w * S, 3), np.float32); best = np.full((h * S, w * S), -1.0, np.float32)
    for k in np.unique(idx):
        m = np.asarray(Image.fromarray(((idx == k) * 255).astype(np.uint8)).resize(
            (w * S, h * S), Image.BICUBIC)).astype(np.float32)
        sel = m > best
        out[sel] = pal[k]; best[sel] = m[sel]
    return out


# Sheet areas drawn from the original instead of the model: text the model misreads.
# x, y, w, h: the 100 / 200 / 50 score popups, same place in every bob sheet but the bonus level's
TEXT = {w: {f'bobs{w}': [(0, 16, 64, 27)]} for w in (1, 2, 3, 4, 5, 6, 8)}


def despill(img, alpha, band=3):
    """Remove the magenta tint the key colour leaves along sprite edges."""
    inside = alpha > 0.99
    near = alpha > 0
    for _ in range(band):
        inside = inside & np.roll(inside, 1, 0) & np.roll(inside, -1, 0) & np.roll(inside, 1, 1) & np.roll(inside, -1, 1)
    edge = near & ~inside
    r, g, b = img[..., 0], img[..., 1], img[..., 2]
    spill = np.clip(np.minimum(r, b) - g, 0, None) * edge
    img[..., 0] -= spill; img[..., 2] -= spill
    return img


def assemble(world):
    d = out_dir(world)
    fin = os.path.join(d, 'final')
    os.makedirs(fin, exist_ok=True)
    man = json.load(open(os.path.join(d, 'sprites.json')))
    info = {}
    for name, idx in sheets(world).items():
        parts = [m for m in man if m['sheet'] == name]
        if not all(os.path.exists(os.path.join(d, 'out', m['name'] + '.png')) for m in parts):
            continue
        rows = []
        for m in sorted(parts, key=lambda m: m['row']):
            c = np.load(os.path.join(d, 'src', m['name'] + '_idx.npy'))
            out = np.asarray(Image.open(os.path.join(d, 'out', m['name'] + '.png')).convert('RGB'))
            warped, inf = rg.register(rgb(world, c), out, scale=S)
            info[m['name']] = {k: float(v) for k, v in inf.items()}
            x0, y0 = m['x0'] * S, m['y0'] * S
            rows.append(warped[y0:y0 + m['rows'] * S, x0:x0 + idx.shape[1] * S])
        hd = np.concatenate(rows, 0)
        # alpha: the original silhouette at 4x with a smoothed outline (same footprint)
        sil = np.repeat(np.repeat((idx > 0).astype(np.float32), S, 0), S, 1)
        alpha = np.clip((blur(sil, 1.2) - 0.25) / 0.5, 0, 1)
        # inside the silhouette, magenta the model left (or bled into edges) is filled inward;
        # a large area it painted magenta (the blue panel behind the spring) falls back to the
        # original's colours
        keyd = np.abs(hd - KEY.astype(np.float32)).sum(-1)
        holes = (alpha > 0) & (keyd < 260)
        hd = fill_from_neighbours(hd, holes)
        left = holes & (np.abs(hd - KEY.astype(np.float32)).sum(-1) < 260)
        orig = np.asarray(Image.fromarray(rgb(world, idx)).resize(
            (idx.shape[1] * S, idx.shape[0] * S), Image.BICUBIC)).astype(np.float32)
        big = left.copy()
        for _ in range(12):      # only areas the neighbour fill could not close
            big = big | (holes & (np.roll(big, 1, 0) | np.roll(big, -1, 0) | np.roll(big, 1, 1) | np.roll(big, -1, 1)))
        hd[big] = orig[big]
        info[name + '_unified'] = unify_frames(world, idx, hd, frame_rects(world, name))
        for (x, y, w, h) in TEXT.get(world, {}).get(name, []):
            hd[y * S:(y + h) * S, x * S:(x + w) * S] = crisp_upscale(world, idx[y:y + h, x:x + w])
        hd = despill(hd, alpha)
        rgba = np.dstack([np.clip(hd, 0, 255).round().astype(np.uint8), (alpha * 255).round().astype(np.uint8)])
        Image.fromarray(rgba).save(os.path.join(fin, f'{name}_hd.png'))
    frames = dict(scale=S,
                  hero=s.exe_rects(s.HERO_RECTS, s.HERO_N),
                  shared=s.exe_rects(s.SHARED_RECTS, s.SHARED_N),
                  world=s.world_rects(world))
    with open(os.path.join(fin, 'frames.json'), 'w') as fh:
        json.dump(frames, fh, indent=1)
    for table, sheet in (('hero', 'hero'), ('shared', f'bobs{world}'), ('world', f'bobs{world}')):
        p = os.path.join(fin, f'{sheet}_hd.png')
        if not os.path.exists(p):
            continue
        im = Image.open(p)
        fd = os.path.join(fin, 'frames', table)
        os.makedirs(fd, exist_ok=True)
        for i, r in enumerate(frames[table]):
            if r['w'] and r['h']:
                im.crop((r['x'] * S, r['y'] * S, (r['x'] + r['w']) * S, (r['y'] + r['h']) * S)).save(
                    os.path.join(fd, f'{i:03d}.png'))
    return info


def main(world):
    d = out_dir(world)
    if not os.path.exists(os.path.join(d, 'sprites.json')):
        print(prepare(world))
    else:
        print(assemble(world))
