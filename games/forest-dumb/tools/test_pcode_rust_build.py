"""fd-judge builds with FD_TRANSLATED naming a directory tree (translator T04): the committed split
fixture, `mod.rs` and two parts, through fd-conform's build.rs. Full set: one cargo build, a few
seconds warm, and it leaves fd-conform built with the fixture until the next build without it."""
import os
import subprocess
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
ENGINE = os.path.join(HERE, '..', 'engine')
SPLIT = os.path.join(ENGINE, 'crates', 'pcode-rt', 'tests', 'fixtures', 'shared_call')


class DirectoryTree(unittest.TestCase):
    def test_fd_judge_builds_with_a_directory_tree(self):
        env = dict(os.environ, FD_TRANSLATED=os.path.abspath(SPLIT))
        env.pop('FD_PORT', None)
        r = subprocess.run(['cargo', 'build', '-q', '-p', 'fd-conform', '--bin', 'fd-judge',
                            '--message-format', 'short'],
                           cwd=ENGINE, env=env, capture_output=True, text=True, timeout=900)
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertNotIn('warning', r.stderr)


if __name__ == '__main__':
    unittest.main()
