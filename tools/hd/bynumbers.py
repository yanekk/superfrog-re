"""Fill textures rendered by numbers: the original's geometry, the model's rendering.

A family of interchangeable fill tiles (analysis.families: a plank wall, dirt) only joins if every
copy keeps the original's layout to the pixel. On a fine regular texture the model does not: on
W3's plank wall it drew planks at its own pitch, so tiles cut from different places of its
paintings never lined up (owner, 2026-10-24).

Here each original pixel becomes a 4x4 HD block learned from the paintings: the mean of what the
model drew for the same palette index with the same four neighbours, inside the same family. A
tile's border pixels do not know the neighbouring tile, so the table is also learned with the
outside neighbour as a wildcard; both sides of a joint then render from their own pixels exactly
as the original does, and the joint is as clean as the original's.
"""
import numpy as np

S = 4
WILD = 32                      # neighbour unknown (outside the tile)
B = 33


def _keys(c, up, dn, lf, rt):
    return (((c * B + up) * B + dn) * B + lf) * B + rt


class Table:
    def __init__(self):
        self.sum = {}
        self.n = {}

    def add(self, keys, blocks):
        u, inv = np.unique(keys, return_inverse=True)
        sm = np.zeros((len(u),) + blocks.shape[1:]); np.add.at(sm, inv, blocks)
        cn = np.bincount(inv, minlength=len(u))
        for k, s_, n in zip(u.tolist(), sm, cn):
            if k in self.sum:
                self.sum[k] += s_; self.n[k] += n
            else:
                self.sum[k] = s_; self.n[k] = n

    def get(self, k):
        return self.sum[k] / self.n[k] if k in self.sum else None


# which neighbours are unknown, for every position class in a tile (interior, sides, corners)
VARIANTS = [(u, d, l, r) for u in (0, 1) for d in (0, 1) for l in (0, 1) for r in (0, 1)
            if not (u and d) and not (l and r)]


def learn(sources, family, canon):
    """Table from every trusted occurrence of a family tile in the registered paintings."""
    tab = Table()
    for src in sources:
        idx = src['idx'].astype(np.int64); img = src['img']
        h, w = idx.shape
        P = np.pad(idx, 1, mode='edge')
        ys, xs = [], []
        for t, x, y, trusted in src['tiles']:
            if not trusted or canon[t] not in family:
                continue
            yy, xx = np.mgrid[y:y + 16, x:x + 16]
            ys.append(yy.ravel()); xs.append(xx.ravel())
        if not ys:
            continue
        ys = np.concatenate(ys); xs = np.concatenate(xs)
        ok = (ys < h) & (xs < w)
        ys, xs = ys[ok], xs[ok]
        c = idx[ys, xs]
        nb = [P[ys, xs + 1], P[ys + 2, xs + 1], P[ys + 1, xs], P[ys + 1, xs + 2]]   # up dn lf rt
        blocks = img.reshape(h, S, w, S, 3).transpose(0, 2, 1, 3, 4)[ys, xs]           # (N,4,4,3)
        for var in VARIANTS:
            n = [np.full_like(c, WILD) if v else nb[i] for i, v in enumerate(var)]
            tab.add(_keys(c, *n), blocks)
        tab.add(_keys(c, *(np.full_like(c, WILD),) * 4), blocks)
    return tab


def context(tl, jt, t):
    """18x18 indices: tile t framed by its most frequent neighbour on each side (from the levels),
    so border pixels render with the context they usually have."""
    best = {}
    for (a, b, ax), n in jt.items():
        for side, other in (((('R' if ax == 1 else 'B'), b) if a == t else (None, None)),
                            ((('L' if ax == 1 else 'T'), a) if b == t else (None, None))):
            if side and n > best.get(side, (0, None))[0]:
                best[side] = (n, other)
    P = np.full((18, 18), WILD, np.int64)
    P[1:17, 1:17] = tl[t]
    if 'T' in best: P[0, 1:17] = tl[best['T'][1]][-1]
    if 'B' in best: P[17, 1:17] = tl[best['B'][1]][0]
    if 'L' in best: P[1:17, 0] = tl[best['L'][1]][:, -1]
    if 'R' in best: P[1:17, 17] = tl[best['R'][1]][:, 0]
    return P


def render(tab, P, pal):
    """64x64 HD tile from its 18x18 framed palette indices (context())."""
    out = np.empty((16 * S, 16 * S, 3), np.float32)
    for y in range(16):
        for x in range(16):
            c = int(P[y + 1, x + 1])
            n = [int(P[y, x + 1]), int(P[y + 2, x + 1]), int(P[y + 1, x]), int(P[y + 1, x + 2])]
            blk = None
            for drop in ((0, 0, 0, 0), (1, 0, 0, 0), (0, 1, 0, 0), (0, 0, 1, 0), (0, 0, 0, 1),
                         (1, 1, 0, 0), (0, 0, 1, 1), (1, 1, 1, 1)):
                m = [WILD if d else v for d, v in zip(drop, n)]
                blk = tab.get(_keys(c, *m))
                if blk is not None:
                    break
            if blk is None:
                blk = np.broadcast_to(pal[c].astype(np.float32), (S, S, 3))
            out[y * S:(y + 1) * S, x * S:(x + 1) * S] = blk
    return out
