"""task_pack.py (plans/forest-dumb-level-1, owner amendment 2026-11-06). Builds real packs from the
committed plan with --no-prep, so nothing is extracted or run; listings show only where the main
checkout's build/translate has them."""
import os
import re
import sys
import tempfile
import tomllib
import unittest
from unittest import mock

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import task_pack as tp  # noqa: E402


def pending(prefix=''):
    """Tasks of the plan not yet ✅, from PROGRESS.md."""
    rows = re.findall(r'(?m)^\| (T\d+) \| ([\w-]+) \|[^|]*\| (\S+) \|', tp.read(os.path.join(tp.PLAN_DIR, 'PROGRESS.md')))
    return [t for t, slug, state in rows if state != '✅' and slug.startswith(prefix)]


class Pack(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        p = mock.patch.object(tp, 'OUT', self.tmp.name)
        p.start()
        self.addCleanup(p.stop)

    def build(self, task, **kw):
        res = tp.build(task, do_prep=False, **kw)
        return res, tp.read(os.path.join(tp.REPO, res['pack']) if not os.path.isabs(res['pack']) else res['pack'])

    def test_a_porting_pack_holds_the_opening_reads_and_every_routine(self):
        res, text = self.build('T22')
        self.assertEqual(res['kinds'], ['porting'])
        self.assertIn('# T22 — collectibles', text)
        self.assertIn('## PROGRESS.md', text)
        self.assertIn('## FINDINGS.md', text)
        self.assertIn('### 4.3 Translation by Muse', text)
        self.assertIn('## 5. Environment', text)
        rows = [r for r in tomllib.loads(tp.read(os.path.join(tp.GAME, 're', 'translate.toml')))['routine']
                if r.get('port_task') == 'T22']
        self.assertTrue(rows)
        for r in rows:
            self.assertIn(f"### `{r['name']}` h0+${r['offset']:x}", text)
        self.assertIn('## Muse writes the engine code', text)
        # Before T22 adds its table the pack shows the fields to fill; after, T22's own.
        self.assertTrue("re/reshape.toml` has no `[T22]` yet" in text or "re/reshape.toml` has `[T22]`" in text)
        # The opening reads and the porting sections always fit; a later section may be left out as
        # the tree grows past --max-kb (T22's files did once T22 was built), named with where to read it.
        for name in ('how', 'task', 'progress', 'findings', 'design', 'listings', 'reshape', 'rules'):
            self.assertIn(name, res['sections'])
        for name in res['omitted']:
            self.assertIn(f'- {name}: read `', text.split('## Omitted to fit the pack', 1)[1])

    def test_dependencies_are_this_plans_commits_only(self):
        _, text = self.build('T22')
        deps = text.split('## What the tasks it depends on did', 1)[1].split('\n## ', 1)[0]
        # T11 and T20 of the first game's plan touched sf- crates; this plan's never do.
        self.assertNotIn('sf-', deps)
        # A commit whose body mentions T21 is not T21's.
        t21 = deps.split('### T21', 1)[1].split('### ', 1)[0] if '### T21' in deps else ''
        self.assertNotIn('T49:', t21)

    def test_a_review_pack_leaves_findings_for_after_the_cold_read(self):
        res, text = self.build('T22', review=True)
        self.assertNotIn('## FINDINGS.md', text)
        self.assertIn('## This branch against pir/forest-dumb-level-1, files only', text)
        # The builder's account stays out until after the cold read: its Notes cell, its commit messages.
        self.assertRegex(text, r'(?m)^\| T22 \|.*\(left out of a review pack\) \|$')
        self.assertEqual(res['prep'], [])
        self.assertTrue(res['pack'].endswith('T22-review.md'))

    def test_what_does_not_fit_is_named_with_where_to_read_it(self):
        res, text = self.build('T22', max_kb=40)
        self.assertTrue(res['omitted'])
        self.assertIn('## Omitted to fit the pack', text)
        for name in res['omitted']:
            self.assertIn(f'- {name}: read', text)
        for required in ('how', 'task', 'progress', 'findings', 'listings', 'reshape'):
            self.assertIn(required, res['sections'])

    def test_every_pending_task_gets_a_pack_within_the_cap(self):
        for task in pending():
            with self.subTest(task=task):
                res, _ = self.build(task)
                self.assertLessEqual(res['bytes'], tp.MAX_KB * 1024 + 60_000, 'required sections alone stay near the cap')
        for task in pending('case-'):
            self.assertIn('frame-matching', self.build(task)[0]['kinds'])


class Paths(unittest.TestCase):
    def test_braces_expand(self):
        self.assertEqual(tp.expand('fd-core/src/objects/{mod.rs,scan.rs}'),
                         ['fd-core/src/objects/mod.rs', 'fd-core/src/objects/scan.rs'])

    def test_a_task_path_resolves_under_the_crates_or_is_new(self):
        self.assertTrue(tp.resolve('fd-core/src/game.rs').endswith('engine/crates/fd-core/src/game.rs'))
        self.assertIsNone(tp.resolve('fd-core/src/no_such_file.rs'))

    def test_design_sections_bring_their_subsections(self):
        text = tp.design_text(['4'])
        self.assertIn('### 4.1 Rules every porting task follows', text)
        self.assertNotIn('## 6. Recovery', text)


if __name__ == '__main__':
    unittest.main()
