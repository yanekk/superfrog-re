"""Tests for the HD-asset pipeline's pure parts (tools/hd). No disks, no model: synthetic tiles,
grids and images. The end-to-end run needs the disks and Higgsfield (docs/15-hd-assets.md)."""
import os
import sys
import unittest

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from hd import analysis as an, register as rg, sprites as sp, tiles as tz, windows as w  # noqa: E402


def tiles_of(*fills):
    """(n, 16, 16) tiles; a fill is a palette index or a 16x16 array."""
    return np.stack([np.full((16, 16), f, np.uint8) if np.isscalar(f) else f for f in fills])


class Analysis(unittest.TestCase):
    def test_duplicates_map_to_the_lowest_id(self):
        tl = tiles_of(4, 7, 4, 7, 9)
        self.assertEqual(an.canonical(tl), {0: 0, 1: 1, 2: 0, 3: 1, 4: 4})

    def test_joints_count_right_and_below_neighbours_by_canonical_id(self):
        tl = tiles_of(4, 7, 4)
        g = np.array([[0, 1], [2, 1]])
        jt = an.joints({'L': g}, an.canonical(tl))
        self.assertEqual(jt[(0, 1, 1)], 2)        # 0|1 twice (tile 2 is tile 0)
        self.assertEqual(jt[(0, 0, 0)], 1)        # 0 above its duplicate
        self.assertEqual(jt[(1, 1, 0)], 1)

    def test_a_family_is_sky_free_tiles_joined_in_both_orders(self):
        rnd = np.random.default_rng(0)
        dirt = [rnd.integers(7, 12, (16, 16)).astype(np.uint8) for _ in range(3)]
        tl = tiles_of(4, *dirt, 9)                # 0 sky, 1-3 dirt, 4 a lone tile
        g = np.array([[1, 2, 3, 1, 3, 2, 1],
                      [0, 0, 0, 0, 4, 0, 0]])
        fams = an.families(tl, an.joints({'L': g}, an.canonical(tl)))
        self.assertEqual(fams, [[1, 2, 3]])


class Windows(unittest.TestCase):
    def test_cover_reaches_every_tile_and_stops(self):
        g = np.zeros((40, 120), int)
        g[20, 5] = 1; g[20, 100] = 2              # two tiles far apart
        chosen, missing = w.cover({'L': g}, need=1, step=4)
        seen = set()
        for _, tx, ty in chosen:
            seen |= set(np.unique(w.inner(g, tx, ty)).tolist())
        self.assertEqual(missing, [])
        self.assertTrue({0, 1, 2} <= seen)
        self.assertLessEqual(len(chosen), 3)


class Register(unittest.TestCase):
    def test_a_shifted_redraw_is_pulled_back_onto_the_grid(self):
        rnd = np.random.default_rng(1)
        idx = rnd.integers(0, 6, (48, 80))
        idx = np.repeat(np.repeat(idx, 2, 0), 2, 1)[:48, :80]           # 2 px blobs
        pal = rnd.integers(0, 255, (6, 3)).astype(np.uint8)
        orig = pal[idx]
        big = np.repeat(np.repeat(orig, 4, 0), 4, 1)
        moved = np.roll(np.roll(big, 6, 0), -5, 1)                     # ~1.5 px drift at 1x
        warped, info = rg.register(orig, moved, scale=4)
        inner = (slice(24, -24), slice(24, -24))
        before = np.abs(moved[inner].astype(float) - big[inner]).mean()
        after = np.abs(warped[inner] - big[inner]).mean()
        self.assertLess(after, before / 3)


class Tiles(unittest.TestCase):
    def test_rigid_joints_are_the_sides_that_only_ever_meet_each_other(self):
        jt = {(1, 2, 1): 5, (3, 4, 1): 2, (3, 5, 1): 1, (0, 0, 1): 9}
        self.assertEqual(tz.rigid_joints(jt), {(1, 2, 1)})

    def test_joint_choice_prefers_the_copy_whose_edges_meet(self):
        flat = lambda v: np.full((64, 64, 3), v, np.float32)
        src = [dict(img=np.concatenate([flat(100), flat(100), flat(160)], 1), tiles=[])]
        # tile 1 has one copy (value 100); tile 2 has copies 100 and 160; 1|2 joint
        cands = {1: [(0, 0, 0)], 2: [(0, 32, 0), (0, 16, 0)]}
        chosen = {1: (0, 0, 0), 2: (0, 32, 0)}                          # start on the bad one
        got = tz.choose_jointly(src, cands, chosen, {(1, 2, 1): 10}, lam=0.0)
        self.assertEqual(got[2], (0, 16, 0))

    def test_seam_score_is_one_without_seams_and_grows_with_them(self):
        rnd = np.random.default_rng(2)
        img = rnd.integers(100, 130, (128, 128, 3)).astype(np.uint8)
        self.assertAlmostEqual(tz.seam_score(img), 1.0, delta=0.15)
        img[:, 64:] += 40
        self.assertGreater(tz.seam_score(img), 1.5)


class Sprites(unittest.TestCase):
    def test_despill_removes_magenta_only_at_the_edge(self):
        img = np.full((20, 20, 3), (200, 60, 200), np.float32)
        alpha = np.zeros((20, 20)); alpha[4:16, 4:16] = 1
        out = sp.despill(img.copy(), alpha, band=2)
        self.assertLess(out[4, 10, 0], 100)                             # edge pixel cleaned
        self.assertEqual(out[10, 10, 0], 200)                           # deep inside untouched

    def test_holes_are_filled_from_their_neighbours(self):
        img = np.full((8, 8, 3), 50, np.float32); holes = np.zeros((8, 8), bool)
        img[3:5, 3:5] = (255, 0, 255); holes[3:5, 3:5] = True
        out = sp.fill_from_neighbours(img, holes)
        self.assertTrue(np.allclose(out[3:5, 3:5], 50))


if __name__ == '__main__':
    unittest.main()
