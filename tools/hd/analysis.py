"""What the levels say about the tileset: duplicates, joints, sky, interchangeable families.

All of it is derived from the original tiles and level grids, so it carries to other worlds.
"""
from collections import Counter, defaultdict

import numpy as np


def sky(tl, jt, min_uses=200):
    """Flat background colours: the colour of every single-colour tile the levels use a lot.
    W1: tile 0 (index 4) and tile 248 (index 3)."""
    uses = Counter()
    for (a, b, ax), n in jt.items():
        uses[a] += n; uses[b] += n
    return tuple(sorted({int(tl[t][0, 0]) for t, n in uses.items()
                         if n >= min_uses and (tl[t] == tl[t][0, 0]).all()}))


def canonical(tl):
    """tile -> lowest tile id with identical pixels."""
    first, canon = {}, {}
    for i in range(len(tl)):
        canon[i] = first.setdefault(tl[i].tobytes(), i)
    return canon


def joints(levels, canon):
    """Counter of (a, b, axis): b is right of a (axis 1) or below a (axis 0), canonical ids."""
    lut = np.array([canon[i] for i in range(max(canon) + 1)])
    c = Counter()
    for g in levels.values():
        t = lut[g & 0x3FF]
        for a, b in zip(t[:, :-1].ravel(), t[:, 1:].ravel()):
            c[(int(a), int(b), 1)] += 1
        for a, b in zip(t[:-1].ravel(), t[1:].ravel()):
            c[(int(a), int(b), 0)] += 1
    return c


def families(tl, jt, min_size=3, sky_idx=None):
    """Interchangeable fill tiles: no sky pixels, and joined to each other in both orders.

    Two such tiles are linked when a|b and b|a both occur on the same axis; a family is a
    connected group of linked tiles. Its members may sit next to each other in any order, so
    their HD copies only join if the redraw keeps the original's texture (docs/15).
    """
    sky_idx = sky(tl, jt) if sky_idx is None else sky_idx
    nosky = {t for t in range(len(tl)) if not np.isin(tl[t], sky_idx).any()}
    adj = defaultdict(set)
    for (a, b, ax), n in jt.items():
        if a != b and a in nosky and b in nosky and jt.get((b, a, ax), 0) > 0:
            adj[a].add(b); adj[b].add(a)
    # self-tiling also counts (a|a on both axes) -- such a tile joins its own family
    seen, out = set(), []
    for t in sorted(adj):
        if t in seen:
            continue
        comp, stack = set(), [t]
        while stack:
            x = stack.pop()
            if x in comp:
                continue
            comp.add(x); stack.extend(adj[x] - comp)
        seen |= comp
        if len(comp) >= min_size:
            out.append(sorted(comp))
    return out
