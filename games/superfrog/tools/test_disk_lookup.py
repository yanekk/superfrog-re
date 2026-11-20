"""The default disk lookup and move-disks.sh (monorepo-and-knowledge DESIGN §2.5), on fake folders."""
import os, subprocess, sys, tempfile, unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import disk_lookup

ZIP = 'Game (Disk 1 of 4).zip'


class Lookup(unittest.TestCase):
    def test_the_game_folder_comes_before_the_root(self):
        self.assertEqual(disk_lookup.checkout_dirs('/m'), ['/m/games/superfrog', '/m'])

    def test_a_folder_holds_disks_only_with_a_disk_image(self):
        with tempfile.TemporaryDirectory() as d:
            self.assertFalse(disk_lookup.holds_disks(d))
            Path(d, 'notes.zip').write_bytes(b'')
            self.assertFalse(disk_lookup.holds_disks(d))
            Path(d, ZIP).write_bytes(b'')
            self.assertTrue(disk_lookup.holds_disks(d))


class MoveDisks(unittest.TestCase):
    def run_in_repo(self, main):
        # The script finds the checkout from its own folder, so it runs from a copy inside the fake repo.
        tools = Path(main, 'games', 'superfrog', 'tools')
        tools.mkdir(parents=True, exist_ok=True)
        script = tools / 'move-disks.sh'
        script.write_text((HERE / 'move-disks.sh').read_text())
        return subprocess.run(['/bin/sh', str(script)], capture_output=True, text=True)

    def test_moves_disks_only_and_is_idempotent(self):
        with tempfile.TemporaryDirectory() as d:
            main = os.path.realpath(d)
            subprocess.run(['git', 'init', '-q', main], check=True)
            Path(main, ZIP).write_bytes(b'x')
            Path(main, 'other.zip').write_bytes(b'y')
            r = self.run_in_repo(main)
            self.assertEqual(r.returncode, 0, r.stderr)
            self.assertIn(f'moved: {ZIP}', r.stdout)
            self.assertTrue(Path(main, 'games/superfrog', ZIP).exists())
            self.assertFalse(Path(main, ZIP).exists())
            self.assertTrue(Path(main, 'other.zip').exists())
            again = self.run_in_repo(main)
            self.assertEqual(again.returncode, 0, again.stderr)
            self.assertIn('no disks to move', again.stdout)

    def test_never_overwrites_a_disk_already_there(self):
        with tempfile.TemporaryDirectory() as d:
            main = os.path.realpath(d)
            subprocess.run(['git', 'init', '-q', main], check=True)
            Path(main, 'games/superfrog').mkdir(parents=True)
            Path(main, 'games/superfrog', ZIP).write_bytes(b'kept')
            Path(main, ZIP).write_bytes(b'new')
            r = self.run_in_repo(main)
            self.assertEqual(r.returncode, 0, r.stderr)
            self.assertEqual(Path(main, 'games/superfrog', ZIP).read_bytes(), b'kept')
            self.assertTrue(Path(main, ZIP).exists())


if __name__ == '__main__':
    unittest.main()
