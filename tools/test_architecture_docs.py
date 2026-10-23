"""The architecture docs' drift check (plans/engine-architecture DESIGN §2.4): every tracked engine
file belongs to exactly one chapter, the front matter is valid and every cited path exists. It
proves structure, not truth; the prose is the reviewer's."""
import os, subprocess, sys, tempfile, unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import architecture as arch  # noqa: E402

# While the chapters are being written their kind, mix and reuse may be "tbd". T18 sets this to
# False once every chapter is complete; from then on a "tbd" fails the check.
ALLOW_TBD = True

ROOT = arch.repo_root()


def chapter(cid, owns=(), **over):
    ch = dict(id=cid, title=cid, tier='subsystem', depth='deep', kind='game',
              mix=dict(amiga=30, game=60, data=10, shell=0), reuse='superfrog-only',
              owns=list(owns), path=f'docs/architecture/{cid}.md')
    ch.update(over)
    return ch


class RealTree(unittest.TestCase):
    def test_every_engine_file_is_owned_once(self):
        files = arch.tracked_files(ROOT)
        self.assertTrue(files, 'no tracked files under engine/crates/ or engine/conform/')
        _, problems = arch.assign(files, arch.load_chapters(ROOT))
        self.assertEqual(problems, [])

    def test_front_matter_is_valid(self):
        chapters = arch.load_chapters(ROOT)
        self.assertEqual(len(chapters), 17)
        self.assertEqual(arch.validate(chapters, ALLOW_TBD), [])

    def test_every_cited_path_exists(self):
        md = sorted(f'{arch.DOCS}/{n}' for n in os.listdir(os.path.join(ROOT, arch.DOCS)) if n.endswith('.md'))
        self.assertEqual(arch.check_paths(ROOT, md), [])

    def test_totals_prints_a_row_per_chapter_and_exits_0(self):
        r = subprocess.run([sys.executable, os.path.join(HERE, 'architecture.py'), 'totals'],
                           capture_output=True, text=True)
        self.assertEqual(r.returncode, 0, r.stderr)
        for cid in arch.load_chapters(ROOT):
            self.assertIn(f'| `{cid}` |', r.stdout)
        self.assertIn('| (uncounted', r.stdout)


class Assign(unittest.TestCase):
    def test_an_unowned_file_is_reported(self):
        _, problems = arch.assign(['engine/crates/a/x.rs', 'engine/crates/b.rs'],
                                  {'a': chapter('a', ['engine/crates/a/'])})
        self.assertEqual(problems, ["unowned file engine/crates/b.rs: add it to the right chapter's owns (DESIGN §2.4)"])

    def test_the_same_prefix_in_two_chapters_is_reported(self):
        _, problems = arch.assign(['engine/crates/a/x.rs'],
                                  {'a': chapter('a', ['engine/crates/a/']), 'b': chapter('b', ['engine/crates/a/'])})
        self.assertEqual(problems, ["duplicate prefix 'engine/crates/a/': in a and b"])

    def test_a_prefix_matching_no_file_is_reported(self):
        _, problems = arch.assign(['engine/crates/a/x.rs'],
                                  {'a': chapter('a', ['engine/crates/a/', 'engine/crates/gone.rs'])})
        self.assertEqual(problems, ["stale prefix 'engine/crates/gone.rs' in a: matches no tracked file"])

    def test_a_file_prefix_matches_only_that_file(self):
        _, problems = arch.assign(['engine/crates/a.rs2'], {'a': chapter('a', ['engine/crates/a.rs'])})
        self.assertIn('unowned file engine/crates/a.rs2', problems[0])

    def test_the_longest_prefix_wins(self):
        src = 'engine/crates/sf-core/src/'
        files = [src + 'front/menu.rs', src + 'front/story/sim.rs', src + 'front/ending.rs']
        assignment, problems = arch.assign(files, {
            'front-end': chapter('front-end', [src + 'front/']),
            'story': chapter('story', [src + 'front/story/', src + 'front/ending.rs'])})
        self.assertEqual(problems, [])
        self.assertEqual(assignment, {files[0]: 'front-end', files[1]: 'story', files[2]: 'story'})

    def test_a_chapter_may_own_nothing(self):
        _, problems = arch.assign(['engine/crates/a.rs'],
                                  {'a': chapter('a', ['engine/crates/a.rs']), 'view': chapter('view')})
        self.assertEqual(problems, [])


class Validate(unittest.TestCase):
    def problems(self, allow_tbd=True, **over):
        return arch.validate({'c': chapter('c', ['x'], **over)}, allow_tbd)

    def test_a_good_chapter_passes(self):
        self.assertEqual(self.problems(), [])

    def test_a_bad_tier_kind_or_reuse_is_reported(self):
        self.assertEqual(self.problems(tier='layer'),
                         ["docs/architecture/c.md: tier 'layer' is not one of mechanism, subsystem, component"])
        self.assertEqual(self.problems(kind='hardware'),
                         ["docs/architecture/c.md: kind 'hardware' is not one of amiga, game, data, shell"])
        self.assertEqual(self.problems(reuse='maybe'),
                         ["docs/architecture/c.md: reuse 'maybe' is not one of as-is, small-changes, big-changes, superfrog-only"])
        self.assertEqual(self.problems(depth='shallow'),
                         ["docs/architecture/c.md: depth 'shallow' is not one of deep, light"])

    def test_a_missing_key_is_reported(self):
        ch = chapter('c', ['x'])
        del ch['reuse']
        self.assertEqual(arch.validate({'c': ch}, True), ["docs/architecture/c.md: missing key 'reuse'"])

    def test_an_id_unlike_the_file_name_is_reported(self):
        self.assertEqual(self.problems(path='docs/architecture/other.md'),
                         ["docs/architecture/other.md: id 'c' does not match the file name"])

    def test_a_mix_summing_to_99_is_reported(self):
        self.assertEqual(self.problems(mix=dict(amiga=30, game=59, data=10, shell=0)),
                         ['docs/architecture/c.md: mix sums to 99, not 100'])

    def test_a_mix_missing_a_kind_is_reported(self):
        self.assertEqual(self.problems(mix=dict(amiga=40, game=60)),
                         ['docs/architecture/c.md: mix must give exactly amiga, game, data, shell'])

    def test_tbd_passes_only_while_allowed(self):
        tbd = dict(kind='tbd', mix='tbd', reuse='tbd')
        self.assertEqual(self.problems(True, **tbd), [])
        self.assertEqual(self.problems(False, **tbd), [
            'docs/architecture/c.md: kind is still "tbd"',
            'docs/architecture/c.md: reuse is still "tbd"',
            'docs/architecture/c.md: mix is still "tbd"'])

    def test_tbd_is_never_a_tier(self):
        self.assertEqual(len(self.problems(tier='tbd')), 1)


class FrontMatter(unittest.TestCase):
    def test_parses_toml_between_plus_lines(self):
        fm = arch.parse_front_matter('+++\nid = "level"\nmix = { amiga = 30, game = 70, data = 0, shell = 0 }\n+++\n\n# The level\n')
        self.assertEqual(fm, {'id': 'level', 'mix': {'amiga': 30, 'game': 70, 'data': 0, 'shell': 0}})

    def test_a_file_without_front_matter_is_not_a_chapter(self):
        self.assertIsNone(arch.parse_front_matter('# Glossary\n\n+++\nid = "x"\n+++\n'))
        with tempfile.TemporaryDirectory() as d:
            folder = os.path.join(d, arch.DOCS)
            os.makedirs(folder)
            for name in ('README.md', 'glossary.md', 'chapter-template.md'):
                with open(os.path.join(folder, name), 'w') as f:
                    f.write(f'# {name}\n\nText with `engine/` in it.\n')
            with open(os.path.join(folder, 'level.md'), 'w') as f:
                f.write('+++\nid = "level"\n+++\n# The level\n')
            chapters = arch.load_chapters(d)
        self.assertEqual(list(chapters), ['level'])
        self.assertEqual(chapters['level']['path'], 'docs/architecture/level.md')


class CitedPaths(unittest.TestCase):
    def check(self, text, files=()):
        with tempfile.TemporaryDirectory() as d:
            for rel in ('docs/architecture/x.md', 'docs/08-executable-map.md', *files):
                os.makedirs(os.path.dirname(os.path.join(d, rel)), exist_ok=True)
                open(os.path.join(d, rel), 'w').close()
            with open(os.path.join(d, 'docs/architecture/x.md'), 'w') as f:
                f.write(text)
            return arch.check_paths(d, ['docs/architecture/x.md'])

    def test_a_missing_path_is_reported(self):
        self.assertEqual(self.check('See `engine/crates/sf-core/src/gone.rs`.'),
                         ['docs/architecture/x.md: cites engine/crates/sf-core/src/gone.rs, which does not exist'])

    def test_a_missing_link_target_is_reported(self):
        self.assertEqual(self.check('[notes](../09-nope.md) and [ok](../08-executable-map.md#lag)'),
                         ['docs/architecture/x.md: cites ../09-nope.md, which does not exist'])

    def test_line_and_item_suffixes_resolve_to_the_file(self):
        self.assertEqual(self.check('`engine/a/path.rs:120`, `engine/a/path.rs::item`, `engine/a/path.rs::State::probe`',
                                    ['engine/a/path.rs']), [])

    def test_a_pattern_is_skipped(self):
        self.assertEqual(self.check('`engine/conform/scripts/*.txt` and `docs/architecture/{id}.md`'), [])

    def test_a_rust_path_is_not_a_repo_path(self):
        self.assertEqual(self.check('`sf_core::level`, `crate::mem::Ram`, `State::probe`'), [])

    def test_fenced_blocks_and_commands_are_skipped(self):
        self.assertEqual(self.check('```\nengine/gone.rs `engine/gone.rs`\n```\n`python tools/gone.py totals`'), [])

    def test_web_links_are_skipped(self):
        self.assertEqual(self.check('[vAmiga](https://github.com/dirkwhoffmann/vAmiga/engine/x)'), [])


class LineCounts(unittest.TestCase):
    def test_counts_code_lines_and_totals_the_rest_apart(self):
        files = {'engine/a.rs': 'a\nb\n', 'engine/b.cpp': 'x\n', 'engine/c.h': 'y\nz', 'engine/d.wgsl': '1\n2\n3\n',
                 'engine/s.txt': '1\n' * 10, 'engine/c.toml': 'k = 1\n', 'engine/p.patch': '-\n+\n',
                 'engine/R.md': '# r\n'}
        with tempfile.TemporaryDirectory() as d:
            for rel, text in files.items():
                os.makedirs(os.path.dirname(os.path.join(d, rel)), exist_ok=True)
                with open(os.path.join(d, rel), 'w') as f:
                    f.write(text)
            assignment = {rel: ('one' if rel < 'engine/c' else 'two') for rel in files}
            counts, uncounted = arch.line_counts(d, assignment)
        self.assertEqual(counts, {'one': 3, 'two': 5})
        self.assertEqual(uncounted, 10 + 1 + 2 + 1)

    def test_totals_splits_lines_by_mix_and_rating(self):
        chapters = {'a': chapter('a', mix=dict(amiga=50, game=50, data=0, shell=0), reuse='as-is'),
                    'b': chapter('b', kind='tbd', mix='tbd', reuse='tbd')}
        rows, kinds, ratings = arch.totals(chapters, {'a': 100, 'b': 40})
        self.assertEqual([r[0] for r in rows], ['a', 'b'])
        self.assertEqual(dict(kinds), {'amiga': 50, 'game': 50, 'data': 0, 'shell': 0, 'tbd': 40})
        self.assertEqual(dict(ratings)['as-is'], 100)
        self.assertEqual(dict(ratings)['tbd'], 40)


if __name__ == '__main__':
    unittest.main()
