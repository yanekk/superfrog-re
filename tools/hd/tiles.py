"""Registered model redraws -> one HD tile per tile id that joins its neighbours.

Why the joints hold (W1 shares of all joint occurrences in the four levels, 2026-10-24):
  sky|sky 36%     flat sky is pinned to one exact colour per palette index -> exact.
  family  52%     interchangeable fill tiles (analysis.families: dirt, cave, leaves). The model
                  is told to keep every stone of the original in place (a texture-preserving
                  redraw), so they join wherever the original's tiles join.
  rigid           a side that always meets the same tile (trees, signs, boulders): the stamp
                  is cut from one place of one painting -> exact.
  all             which copy of each tile is used is chosen jointly (choose_jointly) so that
                  touching edges agree over every joint the levels use, weighted by frequency.
A colour-levelling pass (least squares on border offsets) was tried and removed: it fought
real edges in the art (a mortar line on a tile border) and smeared them.
"""
import json
import os
from collections import defaultdict

import numpy as np
from PIL import Image

from . import analysis as an
from . import source as s
from .prepare import out_dir
from .register import blur

S = 4
TS = s.T * S          # 64
MIN_MATCH = 0.2       # registration score below which a redraw is rejected


def load_sources(world):
    d = out_dir(world)
    man = json.load(open(os.path.join(d, 'manifest.json')))
    reg = json.load(open(os.path.join(d, 'registration.json')))
    out = []
    for src in man['sources']:
        p = os.path.join(d, 'reg', src['name'] + '.png')
        if not os.path.exists(p):
            continue
        # A redraw that does not match its source at all is not a redraw of it: on the bonus
        # level's abstract background the model drew the style reference instead (score ~0;
        # real redraws score 0.33-0.85, 2026-10-24). Its tiles fall back to other copies.
        if reg.get(src['name'], {}).get('global_ncc', 1) < MIN_MATCH:
            continue
        img = np.asarray(Image.open(p).convert('RGB')).astype(np.float32)
        idx = np.load(os.path.join(d, 'src', src['name'] + '_idx.npy'))
        out.append(dict(src, img=img, idx=idx))
    return out


def up(a, k=S):
    return np.repeat(np.repeat(a, k, 0), k, 1)


def flat_mask(idx, k):
    """HD pixels deep inside a flat area of palette index k (3x3 at 1x, then 2 HD px in)."""
    m = idx == k
    e = m.copy()
    e[1:] &= m[:-1]; e[:-1] &= m[1:]; e[:, 1:] &= m[:, :-1]; e[:, :-1] &= m[:, 1:]
    return up(e)


def harmonise(sources, npal=32):
    """Even out colour grading between paintings: a smooth per-painting correction pulls every
    flat palette area towards the median of that colour over all paintings."""
    samples = defaultdict(list)
    for src in sources:
        for k in np.unique(src['idx']):
            m = flat_mask(src['idx'], k)
            if m.sum() > 64:
                samples[int(k)].append(src['img'][m][::7])
    target = {k: np.median(np.concatenate(v), 0) for k, v in samples.items()}
    for src in sources:
        img = src['img']
        num = np.zeros(img.shape, np.float32); den = np.zeros(img.shape[:2], np.float32)
        for k, t in target.items():
            m = flat_mask(src['idx'], k)
            num[m] += t - img[m]; den[m] += 1
        # low-pass at 1/8 resolution (sigma ~48 HD px), normalised by coverage
        f = 8
        h, w = den.shape
        nd = num.reshape(h // f, f, w // f, f, 3).sum((1, 3)); dd = den.reshape(h // f, f, w // f, f).sum((1, 3))
        nd = np.stack([blur(nd[..., c], 6) for c in range(3)], -1); dd = blur(dd, 6)[..., None]
        corr = np.where(dd > 1e-3, nd / np.maximum(dd, 1e-3), 0)
        src['img'] = np.clip(img + up(corr, f), 0, 255)
    return target


def candidates(sources, canon):
    """tile -> [(score, patch)] ; trusted occurrences only unless a tile has none."""
    cands = defaultdict(list); loose = defaultdict(list)
    where = defaultdict(list)
    for si, src in enumerate(sources):
        for t, x, y, trusted in src['tiles']:
            t = canon[t]
            p = src['img'][y * S:y * S + TS, x * S:x * S + TS]
            if p.shape[:2] != (TS, TS):
                continue
            (cands if trusted else loose)[t].append((si, x, y))
            if trusted:
                where[(si, x, y)] = t
    for t in loose:
        if t not in cands:
            cands[t] = loose[t]
    return cands, where


def patch(sources, occ):
    si, x, y = occ
    return sources[si]['img'][y * S:y * S + TS, x * S:x * S + TS]


def medoid(sources, occs, limit=24):
    if len(occs) <= 2:
        return occs[0]
    occs = occs[:: max(1, len(occs) // limit)][:limit]
    P = np.stack([patch(sources, o) for o in occs]).reshape(len(occs), 16, 4, 16, 4, 3).mean((2, 4))
    D = np.abs(P[:, None] - P[None]).mean((2, 3, 4))
    return occs[int(D.sum(1).argmin())]


def rigid_joints(jt):
    """(a, b, axis) where a's side only ever meets b and b's side only ever meets a."""
    fwd = defaultdict(set); back = defaultdict(set)
    for (a, b, ax) in jt:
        fwd[(a, ax)].add(b); back[(b, ax)].add(a)
    return {(a, b, ax) for (a, b, ax) in jt
            if a != b and fwd[(a, ax)] == {b} and back[(b, ax)] == {a}}


def choose(sources, cands, rigid):
    """One occurrence per tile. Rigidly joined tiles (stamps) are taken from one place."""
    # stamps: connected through rigid joints, with fixed relative offsets
    nb = defaultdict(list)
    for a, b, ax in rigid:
        d = (1, 0) if ax == 1 else (0, 1)
        nb[a].append((b, d)); nb[b].append((a, (-d[0], -d[1])))
    chosen, done = {}, set()
    for t0 in sorted(cands, key=lambda t: -len(cands[t])):
        if t0 in done:
            continue
        # stamp layout by BFS
        lay, stack = {t0: (0, 0)}, [t0]
        while stack:
            a = stack.pop()
            for b, (dx, dy) in nb[a]:
                if b not in lay and b in cands:
                    lay[b] = (lay[a][0] + dx, lay[a][1] + dy); stack.append(b)
        # occurrence of the anchor that brings along the most stamp members
        occ_set = {o for t in lay for o in cands[t]}
        best, best_n = None, -1
        anchors = cands[t0] if len(lay) > 1 else [medoid(sources, cands[t0])]
        for (si, x, y) in anchors:
            n = sum((si, x + dx * s.T, y + dy * s.T) in occ_set for (dx, dy) in lay.values())
            if n > best_n:
                best, best_n = (si, x, y), n
        si, x, y = best
        for t, (dx, dy) in lay.items():
            o = (si, x + dx * s.T, y + dy * s.T)
            if t not in done:
                chosen[t] = o if o in occ_set else medoid(sources, cands[t])
                done.add(t)
    return chosen


def edges(p):
    """2-px-deep edge strips L, R, T, B of a 64x64 patch -> (4, 64, 3)."""
    return np.stack([p[:, :2].mean(1), p[:, -2:].mean(1), p[:2].mean(0), p[-2:].mean(0)])


def choose_jointly(sources, cands, chosen, jt, k=8, sweeps=6, lam=0.5):
    """Pick one copy per tile so that, over every joint the levels use (weighted by how often),
    the touching edges of the chosen copies agree; a copy far from the tile's typical look is
    penalised (lam x its distance to the medoid) so an odd copy does not win on edges alone.

    Iterated conditional modes over a short list per tile. Tiles of a stamp start from one
    painting, where their joints agree exactly, so ICM keeps them together unless a better fit
    exists.
    """
    short, E, U = {}, {}, {}
    for t, occs in cands.items():
        occs = list(dict.fromkeys([chosen[t]] + occs[:: max(1, len(occs) // (4 * k))]))
        P = np.stack([patch(sources, o) for o in occs])
        small = P.reshape(len(occs), 16, 4, 16, 4, 3).mean((2, 4))
        d = np.abs(small[:, None] - small[None]).mean((2, 3, 4)).mean(1)
        keep = np.argsort(d)[:k]
        if 0 not in keep:                       # keep the stamp's own copy in play
            keep = np.r_[0, keep[:-1]]
        short[t] = [occs[i] for i in keep]
        E[t] = np.stack([edges(P[i]) for i in keep])
        U[t] = lam * d[keep]
    pick = {t: 0 for t in short}
    nb = defaultdict(list)          # t -> [(other, my side, other side, weight)]
    for (a, b, ax), n in jt.items():
        if a in short and b in short:
            w = np.sqrt(n)
            sa, sb = (1, 0) if ax == 1 else (3, 2)
            nb[a].append((b, sa, sb, w)); nb[b].append((a, sb, sa, w))
    for _ in range(sweeps):
        changed = 0
        for t in short:
            cost = U[t].copy()
            for o, ms, os_, w in nb[t]:
                other = E[o][pick[o], os_] if o != t else None
                if other is None:   # tile next to itself: same copy on both sides
                    cost += w * np.abs(E[t][:, ms] - E[t][:, os_]).mean((1, 2))
                else:
                    cost += w * np.abs(E[t][:, ms] - other).mean((1, 2))
            best = int(cost.argmin())
            changed += best != pick[t]
            pick[t] = best
        if not changed:
            break
    return {t: short[t][pick[t]] for t in short}


def fit_cost(hd, jt, t, p):
    """Frequency-weighted mean edge mismatch of patch p as tile t against its neighbours."""
    e = edges(p); tot = cost = 0.0
    for (a, b, ax), n in jt.items():
        if a == b or (t not in (a, b)) or a not in hd or b not in hd:
            continue
        sa, sb = (1, 0) if ax == 1 else (3, 2)
        mine, other, os_ = (sa, b, sb) if a == t else (sb, a, sa)
        cost += n * float(np.abs(e[mine] - edges(hd[other])[os_]).mean()); tot += n
    return cost / tot if tot else 0.0


def build(world, refit=True):
    tl = s.tiles(world); lv = s.levels(world)
    canon = an.canonical(tl)
    jt = an.joints(lv, canon)
    rigid = rigid_joints(jt)
    sources = load_sources(world)
    target = harmonise(sources)
    cands, _ = candidates(sources, canon)
    chosen = choose_jointly(sources, cands, choose(sources, cands, rigid), jt)
    used = sorted({canon[int(t)] for g in lv.values() for t in np.unique(g & 0x3FF)})
    pal = s.palette(world)
    hd = {}
    for t in used:
        hd[t] = patch(sources, chosen[t]).copy() if t in chosen else up(pal[tl[t]]).astype(np.float32)

    # --- sky: exact colour per flat palette index (own pixels only, so both sides agree)
    if refit:
        # a refitted copy replaces the chosen one only where it fits the neighbours better
        from . import refit as rf
        kept = 0
        for t, p in rf.load(world).items():
            if t in hd and fit_cost(hd, jt, t, p) < fit_cost(hd, jt, t, hd[t]):
                hd[t] = p.copy(); kept += 1
    sky = {t: np.zeros((TS, TS), bool) for t in used}
    for t in used:
        for k in an.sky(tl, jt):
            if k in target:
                m = flat_mask(np.pad(tl[t], 1, mode='edge'), k)[S:-S, S:-S]
                hd[t][m] = target[k]
                sky[t] |= m

    return hd, dict(families=an.families(tl, jt), rigid=len(rigid), refit_kept=kept if refit else 0, target={k: v.tolist() for k, v in target.items()})


def compose(hd, grid, canon):
    h, w = grid.shape
    out = np.empty((h * TS, w * TS, 3), np.uint8)
    lut = {t: np.clip(p, 0, 255).round().astype(np.uint8) for t, p in hd.items()}
    for r in range(h):
        for c in range(w):
            out[r * TS:(r + 1) * TS, c * TS:(c + 1) * TS] = lut[canon[int(grid[r, c]) & 0x3FF]]
    return out


def seam_score(img, ts=TS):
    """Mean colour step across tile borders / mean step between neighbouring columns inside
    tiles. 1.0 means a border is no more visible than any other column of the picture."""
    a = img.astype(np.int16)
    dx = np.abs(np.diff(a, axis=1)).sum(-1)
    dy = np.abs(np.diff(a, axis=0)).sum(-1)
    xb = np.arange(dx.shape[1]) % ts == ts - 1
    yb = np.arange(dy.shape[0]) % ts == ts - 1
    border = (dx[:, xb].mean() + dy[yb].mean()) / 2
    inner = (dx[:, ~xb].mean() + dy[~yb].mean()) / 2
    return float(border / inner)
