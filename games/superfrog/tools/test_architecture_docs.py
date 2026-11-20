"""The architecture docs' drift check (plans/engine-architecture DESIGN §2.4): every tracked engine
file belongs to exactly one chapter, the front matter is valid and every cited path exists. It
proves structure, not truth; the prose is the reviewer's."""
import os, subprocess, sys, tempfile, unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import architecture as arch  # noqa: E402

# While the chapters were being written their kind, mix and reuse could be "tbd". T18 set this to
# False once every chapter was complete: from then on a "tbd" fails the check (DESIGN §2.4).
ALLOW_TBD = False

ROOT = arch.repo_root()


def chapter(cid, owns=(), **over):
    ch = dict(id=cid, title=cid, tier='subsystem', depth='deep', kind='game',
              mix=dict(amiga=30, game=60, data=10, shell=0), reuse='superfrog-only',
              owns=list(owns), path=f'games/superfrog/docs/architecture/{cid}.md')
    ch.update(over)
    return ch


class RealTree(unittest.TestCase):
    def test_every_engine_file_is_owned_once(self):
        files = arch.tracked_files(ROOT)
        self.assertTrue(files, 'no tracked files under games/superfrog/engine/crates/ or games/superfrog/engine/conform/')
        _, problems = arch.assign(files, arch.load_chapters(ROOT))
        self.assertEqual(problems, [])

    def test_every_toolkit_file_is_owned_once(self):
        files = [f for f in arch.tracked_files(ROOT) if f.startswith('toolkit/')]
        self.assertIn('toolkit/Cargo.toml', files)
        self.assertIn('toolkit/crates/amiga-paula/src/lib.rs', files)
        assignment, problems = arch.assign(files, arch.load_chapters(ROOT))
        self.assertEqual([p for p in problems if 'toolkit/' in p and 'stale' not in p], [])
        self.assertEqual(sorted(assignment), files)

    def test_the_six_hardware_chapters_are_in_both_trees(self):
        chapters = arch.load_chapters(ROOT)
        for cid in ('display-hardware', 'cpu-time-and-beam', 'frame-interrupts-input',
                    'memory-model', 'sound-chip', 'verification'):
            self.assertEqual(chapters[cid]['path'], f'{arch.DOCS}/{cid}.md')
            self.assertEqual(chapters['toolkit/' + cid]['path'], f'{arch.TOOLKIT_DOCS}/{cid}.md')

    def test_the_moved_table_points_at_real_folders_and_files(self):
        for old, new in arch.MOVED.items():
            self.assertFalse(os.path.exists(os.path.join(ROOT, old)), f'{old} still exists')
            # A folder moved whole ends in `/`; a file moved out of a folder that stays does not.
            exists = os.path.isdir if new.endswith('/') else os.path.isfile
            self.assertTrue(exists(os.path.join(ROOT, new)), f'{new} does not exist')

    def test_front_matter_is_valid(self):
        chapters = arch.load_chapters(ROOT)
        self.assertEqual(len(chapters), 23)
        self.assertEqual(arch.validate(chapters, ALLOW_TBD), [])

    def test_every_cited_path_exists(self):
        md = sorted(f'{docs}/{n}' for docs in (arch.DOCS, arch.TOOLKIT_DOCS)
                    for n in os.listdir(os.path.join(ROOT, docs)) if n.endswith('.md'))
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
        _, problems = arch.assign(['games/superfrog/engine/crates/a/x.rs', 'games/superfrog/engine/crates/b.rs'],
                                  {'a': chapter('a', ['games/superfrog/engine/crates/a/'])})
        self.assertEqual(problems, ["unowned file games/superfrog/engine/crates/b.rs: add it to the right chapter's owns (DESIGN §2.4)"])

    def test_the_same_prefix_in_two_chapters_is_reported(self):
        _, problems = arch.assign(['games/superfrog/engine/crates/a/x.rs'],
                                  {'a': chapter('a', ['games/superfrog/engine/crates/a/']), 'b': chapter('b', ['games/superfrog/engine/crates/a/'])})
        self.assertEqual(problems, ["duplicate prefix 'games/superfrog/engine/crates/a/': in a and b"])

    def test_a_prefix_matching_no_file_is_reported(self):
        _, problems = arch.assign(['games/superfrog/engine/crates/a/x.rs'],
                                  {'a': chapter('a', ['games/superfrog/engine/crates/a/', 'games/superfrog/engine/crates/gone.rs'])})
        self.assertEqual(problems, ["stale prefix 'games/superfrog/engine/crates/gone.rs' in a: matches no tracked file"])

    def test_a_file_prefix_matches_only_that_file(self):
        _, problems = arch.assign(['games/superfrog/engine/crates/a.rs2'], {'a': chapter('a', ['games/superfrog/engine/crates/a.rs'])})
        self.assertIn('unowned file games/superfrog/engine/crates/a.rs2', problems[0])

    def test_the_longest_prefix_wins(self):
        src = 'games/superfrog/engine/crates/sf-core/src/'
        files = [src + 'front/menu.rs', src + 'front/story/sim.rs', src + 'front/ending.rs']
        assignment, problems = arch.assign(files, {
            'front-end': chapter('front-end', [src + 'front/']),
            'story': chapter('story', [src + 'front/story/', src + 'front/ending.rs'])})
        self.assertEqual(problems, [])
        self.assertEqual(assignment, {files[0]: 'front-end', files[1]: 'story', files[2]: 'story'})

    def test_a_file_owned_across_the_toolkit_game_line_is_reported(self):
        toolkit = chapter('t', ['games/superfrog/engine/crates/a.rs'], path='toolkit/docs/architecture/t.md')
        game = chapter('g', ['toolkit/b.rs'])
        _, problems = arch.assign(['games/superfrog/engine/crates/a.rs', 'toolkit/b.rs'],
                                  {'toolkit/t': toolkit, 'g': game})
        self.assertEqual(problems, [
            'games/superfrog/engine/crates/a.rs is owned across the toolkit/game line, by toolkit/t',
            'toolkit/b.rs is owned across the toolkit/game line, by g'])

    def test_a_chapter_may_own_nothing(self):
        _, problems = arch.assign(['games/superfrog/engine/crates/a.rs'],
                                  {'a': chapter('a', ['games/superfrog/engine/crates/a.rs']), 'view': chapter('view')})
        self.assertEqual(problems, [])


class Validate(unittest.TestCase):
    def problems(self, allow_tbd=True, **over):
        return arch.validate({'c': chapter('c', ['x'], **over)}, allow_tbd)

    def test_a_good_chapter_passes(self):
        self.assertEqual(self.problems(), [])

    def test_a_bad_tier_kind_or_reuse_is_reported(self):
        self.assertEqual(self.problems(tier='layer'),
                         ["games/superfrog/docs/architecture/c.md: tier 'layer' is not one of mechanism, subsystem, component"])
        self.assertEqual(self.problems(kind='hardware'),
                         ["games/superfrog/docs/architecture/c.md: kind 'hardware' is not one of amiga, game, data, shell"])
        self.assertEqual(self.problems(reuse='maybe'),
                         ["games/superfrog/docs/architecture/c.md: reuse 'maybe' is not one of as-is, small-changes, big-changes, superfrog-only"])
        self.assertEqual(self.problems(depth='shallow'),
                         ["games/superfrog/docs/architecture/c.md: depth 'shallow' is not one of deep, light"])

    def test_a_missing_key_is_reported(self):
        ch = chapter('c', ['x'])
        del ch['reuse']
        self.assertEqual(arch.validate({'c': ch}, True), ["games/superfrog/docs/architecture/c.md: missing key 'reuse'"])

    def test_an_id_unlike_the_file_name_is_reported(self):
        self.assertEqual(self.problems(path='games/superfrog/docs/architecture/other.md'),
                         ["games/superfrog/docs/architecture/other.md: id 'c' does not match the file name"])

    def test_a_mix_summing_to_99_is_reported(self):
        self.assertEqual(self.problems(mix=dict(amiga=30, game=59, data=10, shell=0)),
                         ['games/superfrog/docs/architecture/c.md: mix sums to 99, not 100'])

    def test_a_mix_missing_a_kind_is_reported(self):
        self.assertEqual(self.problems(mix=dict(amiga=40, game=60)),
                         ['games/superfrog/docs/architecture/c.md: mix must give exactly amiga, game, data, shell'])

    def test_tbd_passes_only_while_allowed(self):
        tbd = dict(kind='tbd', mix='tbd', reuse='tbd')
        self.assertEqual(self.problems(True, **tbd), [])
        self.assertEqual(self.problems(False, **tbd), [
            'games/superfrog/docs/architecture/c.md: kind is still "tbd"',
            'games/superfrog/docs/architecture/c.md: reuse is still "tbd"',
            'games/superfrog/docs/architecture/c.md: mix is still "tbd"'])

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
                    f.write(f'# {name}\n\nText with `games/superfrog/engine/` in it.\n')
            with open(os.path.join(folder, 'level.md'), 'w') as f:
                f.write('+++\nid = "level"\n+++\n# The level\n')
            chapters = arch.load_chapters(d)
        self.assertEqual(list(chapters), ['level'])
        self.assertEqual(chapters['level']['path'], 'games/superfrog/docs/architecture/level.md')


class BothTrees(unittest.TestCase):
    def test_chapters_load_from_both_trees_keyed_apart(self):
        with tempfile.TemporaryDirectory() as d:
            for folder in (arch.DOCS, arch.TOOLKIT_DOCS):
                os.makedirs(os.path.join(d, folder))
                with open(os.path.join(d, folder, 'level.md'), 'w') as f:
                    f.write('+++\nid = "level"\n+++\n# The level\n')
            chapters = arch.load_chapters(d)
        self.assertEqual(sorted(chapters), ['level', 'toolkit/level'])
        self.assertEqual(chapters['toolkit/level']['path'], 'toolkit/docs/architecture/level.md')


class CitedPaths(unittest.TestCase):
    def check(self, text, files=()):
        with tempfile.TemporaryDirectory() as d:
            for rel in ('games/superfrog/docs/architecture/x.md', 'games/superfrog/docs/08-executable-map.md', *files):
                os.makedirs(os.path.dirname(os.path.join(d, rel)), exist_ok=True)
                open(os.path.join(d, rel), 'w').close()
            with open(os.path.join(d, 'games/superfrog/docs/architecture/x.md'), 'w') as f:
                f.write(text)
            return arch.check_paths(d, ['games/superfrog/docs/architecture/x.md'])

    def test_a_missing_path_is_reported(self):
        self.assertEqual(self.check('See `games/superfrog/engine/crates/sf-core/src/gone.rs`.'),
                         ['games/superfrog/docs/architecture/x.md: cites games/superfrog/engine/crates/sf-core/src/gone.rs, which does not exist'])

    def test_a_missing_link_target_is_reported(self):
        self.assertEqual(self.check('[notes](../09-nope.md) and [ok](../08-executable-map.md#lag)'),
                         ['games/superfrog/docs/architecture/x.md: cites ../09-nope.md, which does not exist'])

    def test_a_bare_sibling_link_is_checked(self):
        # Chapters link each other as `[memory model](memory-model.md)`, with no `./`.
        self.assertEqual(self.check('[gone](gone.md) and [here](x.md)'),
                         ['games/superfrog/docs/architecture/x.md: cites gone.md, which does not exist'])

    def test_line_and_item_suffixes_resolve_to_the_file(self):
        self.assertEqual(self.check('`games/superfrog/engine/a/path.rs:120`, `games/superfrog/engine/a/path.rs::item`, `games/superfrog/engine/a/path.rs::State::probe`',
                                    ['games/superfrog/engine/a/path.rs']), [])

    def test_a_pattern_is_skipped(self):
        self.assertEqual(self.check('`games/superfrog/engine/conform/scripts/*.txt` and `games/superfrog/docs/architecture/{id}.md`'), [])

    def test_a_rust_path_is_not_a_repo_path(self):
        self.assertEqual(self.check('`sf_core::level`, `crate::mem::Ram`, `State::probe`'), [])

    def test_fenced_blocks_and_commands_are_skipped(self):
        self.assertEqual(self.check('```\ngames/superfrog/engine/gone.rs `games/superfrog/engine/gone.rs`\n```\n`python tools/gone.py totals`'), [])

    def test_a_cited_old_path_resolves_through_the_moved_table(self):
        files = ['toolkit/crates/amiga-paula/src/lib.rs']
        table = {'games/superfrog/engine/crates/sf-paula/': 'toolkit/crates/amiga-paula/'}
        with tempfile.TemporaryDirectory() as d:
            for rel in ('games/superfrog/docs/architecture/x.md', *files):
                os.makedirs(os.path.dirname(os.path.join(d, rel)), exist_ok=True)
                open(os.path.join(d, rel), 'w').close()
            with open(os.path.join(d, 'games/superfrog/docs/architecture/x.md'), 'w') as f:
                f.write('`games/superfrog/engine/crates/sf-paula/src/lib.rs:12`, `games/superfrog/engine/crates/sf-paula/`, '
                        '`games/superfrog/engine/crates/sf-paula`, `toolkit/crates/amiga-paula/src/lib.rs`, '
                        '`games/superfrog/engine/crates/sf-paula/src/gone.rs`, `games/superfrog/engine/crates/sf-gone/x.rs`')
            problems = arch.check_paths(d, ['games/superfrog/docs/architecture/x.md'], table)
        self.assertEqual(problems, [
            'games/superfrog/docs/architecture/x.md: cites games/superfrog/engine/crates/sf-paula/src/gone.rs, which does not exist',
            'games/superfrog/docs/architecture/x.md: cites games/superfrog/engine/crates/sf-gone/x.rs, which does not exist'])

    def test_moved_replaces_the_longest_old_prefix_only(self):
        table = {'games/superfrog/engine/a/': 'toolkit/a/', 'games/superfrog/engine/a/b/': 'toolkit/b/'}
        self.assertEqual(arch.moved('games/superfrog/engine/a/b/x.rs', table), 'toolkit/b/x.rs')
        self.assertEqual(arch.moved('games/superfrog/engine/a/x.rs', table), 'toolkit/a/x.rs')
        self.assertEqual(arch.moved('games/superfrog/engine/a', table), 'toolkit/a')
        self.assertEqual(arch.moved('games/superfrog/engine/ab/x.rs', table), 'games/superfrog/engine/ab/x.rs')

    def test_web_links_are_skipped(self):
        self.assertEqual(self.check('[vAmiga](https://github.com/dirkwhoffmann/vAmiga/engine/x)'), [])


class LineCounts(unittest.TestCase):
    def test_counts_code_lines_and_totals_the_rest_apart(self):
        files = {'games/superfrog/engine/a.rs': 'a\nb\n', 'games/superfrog/engine/b.cpp': 'x\n', 'games/superfrog/engine/c.h': 'y\nz', 'games/superfrog/engine/d.wgsl': '1\n2\n3\n',
                 'games/superfrog/engine/s.txt': '1\n' * 10, 'games/superfrog/engine/c.toml': 'k = 1\n', 'games/superfrog/engine/p.patch': '-\n+\n',
                 'games/superfrog/engine/R.md': '# r\n'}
        with tempfile.TemporaryDirectory() as d:
            for rel, text in files.items():
                os.makedirs(os.path.dirname(os.path.join(d, rel)), exist_ok=True)
                with open(os.path.join(d, rel), 'w') as f:
                    f.write(text)
            assignment = {rel: ('one' if rel < 'games/superfrog/engine/c' else 'two') for rel in files}
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
