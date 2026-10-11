"""tools/check_no_disk_data.py finds copied disk data and passes the real tree (DESIGN §2.3)."""
import os, subprocess, sys, tempfile, unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import check_no_disk_data as check  # noqa: E402

SCRIPT = os.path.join(HERE, 'check_no_disk_data.py')


def run(*args):
    return subprocess.run([sys.executable, SCRIPT, *args], capture_output=True, text=True)


class CheckNoDiskData(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        # Fails, rather than skips, when the disks cannot be found (DESIGN §5).
        cls.adfs = check.find_adfs(check.default_disk_dir())

    def chunk_bytes(self, length):
        """`length` bytes from inside a decompressed chunk, from a stretch with plenty of variety."""
        for label, data in check.disk_sources(self.adfs):
            if label.endswith('.adf'):
                continue
            for off in range(4096, len(data) - length, 997):
                piece = data[off:off + length]
                if len(set(piece)) >= 48:
                    return label, piece
        self.fail('no varied stretch found in any chunk')

    def test_flags_200_bytes_copied_from_a_chunk(self):
        label, piece = self.chunk_bytes(200)
        with tempfile.TemporaryDirectory() as d:
            path = os.path.join(d, 'leak.bin')
            with open(path, 'wb') as f:
                f.write(b'harmless prefix ' * 3 + piece + b' harmless suffix')
            r = run(path)
        self.assertEqual(r.returncode, 1, r.stdout + r.stderr)
        self.assertIn(f'{path}:48 (200 bytes, same as ', r.stdout)
        self.assertIn('do not relax this check', r.stderr)

    def test_a_short_copy_is_not_flagged(self):
        _label, piece = self.chunk_bytes(100)
        with tempfile.TemporaryDirectory() as d:
            path = os.path.join(d, 'short.bin')
            with open(path, 'wb') as f:
                f.write(piece)
            r = run(path)
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)

    def test_the_real_tree_passes(self):
        r = run()
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertIn('no disk data in', r.stdout)


if __name__ == '__main__':
    unittest.main()
