"""toolkit/tools/disk_data_scan.py on synthetic sources: no disks, no game."""
import os, random, sys, unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import disk_data_scan as S


def noise(seed, n):
    return random.Random(seed).randbytes(n)


class Scan(unittest.TestCase):
    def setUp(self):
        self.sources = [('a', noise(1, 4096)), ('b', noise(2, 4096))]
        self.index = S.build_index(self.sources)

    def test_flags_a_copied_run_with_where_it_came_from(self):
        data = b'prefix-' + self.sources[1][1][1000:1200] + b'-suffix'
        self.assertEqual(S.scan(data, self.index, self.sources), [(7, 200, 'b', 1000)])

    def test_a_run_below_the_minimum_is_not_flagged(self):
        self.assertEqual(S.scan(self.sources[0][1][64:64 + S.MIN_RUN - 1], self.index, self.sources), [])

    def test_a_block_at_two_places_is_measured_against_both(self):
        block = noise(3, S.BLOCK)
        first = noise(4, 64) + block + noise(5, 64)   # the copy's block, but a short shared run
        second = noise(6, 128) + noise(7, 64)
        second = second[:64] + block + noise(8, 136)
        sources = [('first', first), ('second', second)]
        data = second[0:200]
        self.assertEqual(S.scan(data, S.build_index(sources), sources), [(0, 200, 'second', 0)])

    def test_flat_blocks_are_not_indexed(self):
        sources = [('zero', bytes(1024))]
        self.assertEqual(S.build_index(sources), {})
        self.assertEqual(S.scan(bytes(1024), {}, sources), [])


if __name__ == '__main__':
    unittest.main()
