"""Tests for tools/empty_memory.sh: its seatbelt refuses, and a good run leaves only the pointer."""
import os, re, subprocess, tempfile, unittest

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPT = os.path.join(HERE, 'empty_memory.sh')
with open(SCRIPT) as f:
    NOTES = re.search(r'NOTES="(.*?)"', f.read(), re.S).group(1).split()
BODY = 'trackloader cruncher decompressor emulation routine sector chunks header\n'


class Empty(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.repo = os.path.join(self.tmp.name, 'repo')
        self.memory = os.path.join(self.tmp.name, 'memory')
        os.makedirs(self.memory)
        os.makedirs(self.repo)
        self.git('init', '-q')
        for note in NOTES:
            self.write(os.path.join(self.repo, note), BODY)
            self.write(os.path.join(self.memory, os.path.basename(note)),
                       '---\nname: x\nmetadata:\n  originSessionId: abcdefgh\n---\n' + BODY)
        self.write(os.path.join(self.memory, 'MEMORY.md'), 'old index\n')
        self.commit()

    def write(self, path, text):
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, 'w') as f:
            f.write(text)

    def git(self, *args):
        subprocess.run(['git', *args], cwd=self.repo, check=True)

    def commit(self):
        self.git('add', '-A')
        self.git('-c', 'user.name=t', '-c', 'user.email=t@t', 'commit', '-q', '-m', 'x', '--allow-empty')

    def run_script(self):
        env = dict(os.environ, MEMORY_DIR=self.memory, REPO_ROOT=self.repo)
        return subprocess.run([SCRIPT], env=env, capture_output=True, text=True)

    def assert_nothing_deleted(self, result, says):
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn(says, result.stderr)
        self.assertIn('nothing deleted', result.stderr)
        self.assertEqual(len(os.listdir(self.memory)), 13)
        with open(os.path.join(self.memory, 'MEMORY.md')) as f:
            self.assertEqual(f.read(), 'old index\n')

    def test_twelve_notes(self):
        self.assertEqual(len(NOTES), 12)
        self.assertEqual(len({os.path.basename(n) for n in NOTES}), 12)

    def test_a_note_missing_from_the_repo_refuses(self):
        self.git('rm', '-q', NOTES[3])
        self.commit()
        self.assert_nothing_deleted(self.run_script(), 'does not hold')

    def test_a_note_not_committed_refuses(self):
        self.git('rm', '-q', '--cached', NOTES[3])
        self.git('-c', 'user.name=t', '-c', 'user.email=t@t', 'commit', '-q', '-m', 'x')
        self.assert_nothing_deleted(self.run_script(), 'does not hold')

    def test_a_note_with_uncommitted_changes_refuses(self):
        self.write(os.path.join(self.repo, NOTES[3]), BODY + 'more\n')
        self.assert_nothing_deleted(self.run_script(), 'uncommitted')

    def test_a_stub_note_refuses(self):
        self.write(os.path.join(self.repo, NOTES[3]), 'stub\n')
        self.commit()
        self.assert_nothing_deleted(self.run_script(), 'holds only')

    def test_a_note_reworded_but_holding_most_of_it_passes(self):
        self.write(os.path.join(self.repo, NOTES[3]),
                   'corrected trackloader cruncher decompressor emulation routine\n')
        self.commit()
        self.assertEqual(self.run_script().returncode, 0)

    def test_an_unknown_memory_file_refuses(self):
        self.write(os.path.join(self.memory, 'new-note.md'), 'written by another session\n')
        result = self.run_script()
        self.assertEqual(result.returncode, 1)
        self.assertIn('new-note.md', result.stderr)
        self.assertEqual(len(os.listdir(self.memory)), 14)

    def test_a_good_run_leaves_only_the_pointer_and_is_idempotent(self):
        self.assertEqual(self.run_script().returncode, 0)
        self.assertEqual(os.listdir(self.memory), ['MEMORY.md'])
        with open(os.path.join(self.memory, 'MEMORY.md')) as f:
            first = f.read()
        self.assertIn('knowledge/', first)
        self.assertIn('2026-10-30', first)
        self.assertEqual(self.run_script().returncode, 0)
        with open(os.path.join(self.memory, 'MEMORY.md')) as f:
            self.assertEqual(f.read(), first)


if __name__ == '__main__':
    unittest.main()
