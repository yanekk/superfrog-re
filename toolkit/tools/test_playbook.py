"""toolkit/PLAYBOOK.md and toolkit/template/ (monorepo-and-knowledge DESIGN §2.6): every path they cite
is real, the playbook's steps are in order, and a copy of the template is complete and game-free."""
import os, re, shutil, subprocess, sys, tempfile, unittest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, os.path.join(ROOT, 'tools'))
import check_tier_names
import smoke_core

PLAYBOOK = 'toolkit/PLAYBOOK.md'
TEMPLATE = 'toolkit/template'
PIECES = {'CLAUDE.md', 'docs/01-disk-location.md', 'docs/README.md', '.gitignore',
          'conform/case-stub.txt', 'smoke.toml'}
STEPS = ['Disks and how the game boots', 'The packer', 'The file formats', 'The executable map',
         'The oracle and the first conformance case', 'Porting routine by routine against it',
         'The app', 'The recorder', 'The planner']
# Starts at a whole path, so `~/x/toolkit/y` and `../../toolkit/y` are not read as repo paths, and
# placeholders (`games/<game>`, `plans/{slug}`) are not paths at all.
CITED = re.compile(r'(?<![\w./~-])(?:games/[a-z0-9-]+(?:/[\w.-]+)*|(?:toolkit|plans|knowledge)(?:/[\w.-]+)+)')


def cited_paths(text):
    return {m.group().rstrip('.') for m in CITED.finditer(text)}


def known_paths(root):
    """Files git tracks or would track, and their folders: a git-ignored path is not on the branch,
    so a citation of build output cannot pass on the machine that happens to have it."""
    out = subprocess.run(['git', 'ls-files', '--cached', '--others', '--exclude-standard', '-z'],
                         cwd=root, capture_output=True, check=True).stdout.decode()
    known = set()
    for f in filter(None, out.split('\0')):
        known.add(f)
        while '/' in f:
            f = f.rsplit('/', 1)[0]
            known.add(f)
    return known


def read(rel):
    with open(os.path.join(ROOT, rel), encoding='utf-8') as f:
        return f.read()


def template_files():
    return [f'{TEMPLATE}/{p}' for p in sorted(PIECES)]


class CitedPaths(unittest.TestCase):
    def test_the_pattern_finds_paths_and_skips_placeholders(self):
        text = ('`games/one/docs/01-a.md`, toolkit/tools/x.py. (plans/p/DESIGN.md) knowledge/k.md '
                'games/<game>/test.sh plans/{slug}/x <repo>/games/<game> ~/c/toolkit/y ../toolkit/z '
                "toolkit's")
        self.assertEqual(cited_paths(text), {'games/one/docs/01-a.md', 'toolkit/tools/x.py',
                                             'plans/p/DESIGN.md', 'knowledge/k.md'})

    def test_every_cited_path_is_on_the_branch(self):
        known = known_paths(ROOT)
        for rel in (PLAYBOOK, *template_files()):
            with self.subTest(rel):
                self.assertEqual(sorted(p.rstrip('/') for p in cited_paths(read(rel))
                                        if p.rstrip('/') not in known), [])


class Steps(unittest.TestCase):
    def sections(self):
        parts = re.split(r'^## (\d+)\. (.+)$', read(PLAYBOOK), flags=re.M)
        return [(int(parts[i]), parts[i + 1], parts[i + 2]) for i in range(1, len(parts), 3)]

    def test_the_nine_steps_are_in_order(self):
        self.assertEqual([(n, title) for n, title, _ in self.sections()],
                         list(enumerate(STEPS, start=1)))

    def test_each_step_has_artifacts_and_a_lesson(self):
        for n, _, body in self.sections():
            with self.subTest(step=n):
                self.assertIn('Lesson: ', body)
                self.assertTrue(cited_paths(body), 'cites no artifact')

    def test_the_game_two_note_is_there(self):
        note = next(l for l in read(PLAYBOOK).splitlines() if 'OFS' in l)
        self.assertIn('AmigaDOS (OFS) `.adf` reader', note)
        self.assertIn('learned on:', note)


class Template(unittest.TestCase):
    def test_a_copy_is_the_six_pieces_and_names_no_game(self):
        with tempfile.TemporaryDirectory() as scratch:
            copy = os.path.join(scratch, 'game')
            shutil.copytree(os.path.join(ROOT, TEMPLATE), copy)
            found = {os.path.relpath(os.path.join(d, f), copy)
                     for d, _, files in os.walk(copy) for f in files}
            self.assertEqual(found, PIECES)
            # Stricter than the name check: a template has no use for a `learned on:` line.
            patterns = check_tier_names.banned_patterns(check_tier_names.game_dirs(ROOT))
            for rel in sorted(found):
                with open(os.path.join(copy, rel), encoding='utf-8') as f:
                    text = f.read()
                for pattern in patterns:
                    with self.subTest(file=rel, token=pattern.pattern):
                        self.assertIsNone(pattern.search(text))

    def test_the_smoke_stub_passes_the_label_check_in_a_new_game(self):
        project = smoke_core.Project('games/new', read(f'{TEMPLATE}/smoke.toml'), sorted(PIECES), {})
        self.assertEqual(smoke_core.check(project), [])

    def test_git_keeps_every_piece(self):
        # A root ignore rule swallowing a piece would leave a template incomplete on a fresh clone.
        for rel in template_files():
            with self.subTest(rel):
                ignored = subprocess.run(['git', 'check-ignore', '-q', rel], cwd=ROOT)
                self.assertEqual(ignored.returncode, 1)


if __name__ == '__main__':
    unittest.main()
