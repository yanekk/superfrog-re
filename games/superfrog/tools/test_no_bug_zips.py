"""No bug zip, and nothing under games/superfrog/bugs/, is ever committed (bug-recorder DESIGN §2.5, D5).

A bug zip holds a screenshot rendered from the disks, so it is disk-derived (RE DESIGN §2.3); the
app writes them into the git-ignored games/superfrog/bugs/ folder. This fails when a tracked file is under games/superfrog/bugs/
or is a zip, whatever .gitignore says (a forced `git add -f` gets past it).
"""
import os
import subprocess
import unittest

ROOT = os.path.dirname(os.path.abspath(__file__))


def offending(paths):
    """The tracked paths that must not be: under a bugs folder, or ending in .zip."""
    return [p for p in paths if p.startswith(('bugs/', 'games/superfrog/bugs/')) or p.lower().endswith('.zip')]


def tracked():
    out = subprocess.run(['git', 'ls-files', '-z'], cwd=ROOT, capture_output=True, check=True)
    return [p for p in out.stdout.decode('utf-8').split('\0') if p]


class NoBugZips(unittest.TestCase):
    def test_nothing_tracked_is_a_bug_zip_or_under_bugs(self):
        bad = offending(tracked())
        self.assertEqual(bad, [], 'remove these from git (git rm --cached); never commit a bug zip')

    def test_the_check_flags_what_it_should_and_nothing_else(self):
        self.assertEqual(
            offending([
                'games/superfrog/bugs/bug-W1-1-2026-10-20-0905.zip',
                'games/superfrog/bugs/notes.txt',
                'games/superfrog/engine/conform/x.ZIP',
                'Superfrog (Disk 1 of 4).zip',
                'games/superfrog/docs/bugs/readme.md',
                'tools/zipfile.py',
                'games/superfrog/engine/crates/sf-app/src/recorder.rs',
            ]),
            ['games/superfrog/bugs/bug-W1-1-2026-10-20-0905.zip', 'games/superfrog/bugs/notes.txt', 'games/superfrog/engine/conform/x.ZIP',
             'Superfrog (Disk 1 of 4).zip'],
        )

    def test_bugs_is_ignored(self):
        r = subprocess.run(['git', 'check-ignore', '-q', 'games/superfrog/bugs/bug-W1-1-2026-10-20-0905.txt'], cwd=ROOT)
        self.assertEqual(r.returncode, 0, 'games/superfrog/bugs/ must be in .gitignore')


if __name__ == '__main__':
    unittest.main()
