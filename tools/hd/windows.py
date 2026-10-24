"""Level windows sent to the model, and the set of windows that covers every used tile.

A window is 648 x 384 original pixels at a tile-aligned origin. At 4x that is 2592 x 1536,
which is GPT Image 2.5's native 27:16 size, so the model has no reason to reframe it.
"""
import numpy as np

from .source import T

WIN_W, WIN_H = 648, 384                       # original pixels
WT, HT = -(-WIN_W // T), WIN_H // T           # 41 x 24 tiles touched (the last one cut at 8 px)
MARGIN = 2                                    # edge tiles the model sees but we do not trust


def inner(grid, tx, ty):
    """Tile ids in the trusted interior of the window at tile (tx, ty)."""
    return grid[ty + MARGIN:ty + HT - MARGIN, tx + MARGIN:tx + WT - 1 - MARGIN] & 0x3FF


def cover(levels, need=1, step=4):
    """Greedy set cover: windows until every used tile has `need` trusted occurrences.

    Returns [(level, tx, ty)]. Candidates sit on a `step`-tile lattice.
    """
    used = set()
    for g in levels.values():
        used |= set(np.unique(g & 0x3FF).tolist())
    cands = []
    for name, g in levels.items():
        h, w = g.shape
        for ty in list(range(0, h - HT + 1, step)) + [h - HT]:
            for tx in list(range(0, w - WT + 1, step)) + [w - WT]:
                ids, n = np.unique(inner(g, tx, ty), return_counts=True)
                cands.append((name, tx, ty, dict(zip(ids.tolist(), n.tolist()))))
    have = {t: 0 for t in used}
    chosen = []
    while True:
        def gain(c):
            return sum(min(n, need - have[t]) for t, n in c[3].items() if have[t] < need)
        best = max(cands, key=gain)
        if gain(best) == 0:
            break
        chosen.append(best[:3])
        for t, n in best[3].items():
            have[t] += n
        cands.remove(best)
    missing = sorted(t for t, n in have.items() if n == 0)
    return chosen, missing
