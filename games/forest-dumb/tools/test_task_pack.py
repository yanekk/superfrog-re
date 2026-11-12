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
        self.assertIn('## Translate first: the procedure', text)
        # The opening reads and the porting sections always fit; a later section may be left out as
        # the tree grows past --max-kb (T22's files did once T22 was built), named with where to read it.
        for name in ('how', 'task', 'progress', 'findings', 'design', 'listings', 'procedure', 'rules'):
            self.assertIn(name, res['sections'])
        for name in res['omitted']:
            self.assertIn(f'- {name}: read `', text.split('## Omitted to fit the pack', 1)[1])

    def test_a_porting_pack_carries_the_translator_procedure_and_no_muse_procedure(self):
        # Translator DESIGN §2.12: every pack says translate first. The plan's own records (its task doc,
        # PROGRESS, FINDINGS, DESIGN, commits) may still name Muse as level 1's history; what the pack
        # itself writes (how to use it, the listings, the procedure, the rules) may not.
        _, text = self.build('T22')
        own = {}
        for chunk in re.split(r'(?m)^(?=## )', text):
            own[chunk.splitlines()[0] if chunk.startswith('## ') else 'how'] = chunk
        procedure = next(v for k, v in own.items() if k.startswith('## Translate first'))
        self.assertIn('translate.py --task T{nn}', procedure)
        self.assertIn('by hand', procedure)
        self.assertIn('fd-judge --seam', procedure)
        for key in ('how', '## Its routines (re/translate.toml), with their listings', procedure.splitlines()[0],
                    '## toolkit/docs/porting-routines.md'):
            with self.subTest(section=key):
                self.assertNotRegex(own.get(key, ''), r'Muse|reshape\.toml|translate_batch|port\.rs')
        self.assertIn('translate with the translator first', own['how'])
        # Each routine names its translation, or the command that writes it.
        self.assertRegex(own['## Its routines (re/translate.toml), with their listings'], r'Translation: (`|none yet)')

    def test_the_procedure_is_docs_11s_section(self):
        doc = tp.read(os.path.join(tp.GAME, 'docs', '11-translation.md'))
        self.assertIn(tp.procedure().splitlines()[0], doc.split('## The procedure', 1)[1])

    def test_dependencies_are_this_plans_commits_only(self):
        _, text = self.build('T22')
        if '## What the tasks it depends on did' not in text:
            # Once this plan is merged into main its commits sit behind the merge-base, so the section
            # is rightly empty: then no commit since the merge-base may carry a dependency's subject.
            since = tp.git('merge-base', 'HEAD', tp.BASE).strip()
            subjects = tp.git('log', '--format=%s', f'{since}..HEAD').splitlines()
            for t in tp.depends(tp.read(tp.task_doc('T22'))):
                self.assertFalse([s for s in subjects if re.match(rf'{t}(?:[: ]|$)', s)], t)
            return
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
        for required in ('how', 'task', 'progress', 'findings', 'listings', 'procedure'):
            self.assertIn(required, res['sections'])

    def test_every_pending_task_gets_a_pack_within_the_cap(self):
        for task in pending():
            with self.subTest(task=task):
                res, _ = self.build(task)
                self.assertLessEqual(res['bytes'], tp.MAX_KB * 1024 + 60_000, 'required sections alone stay near the cap')
        for task in pending('case-'):
            self.assertIn('frame-matching', self.build(task)[0]['kinds'])


class RoutineBlock(unittest.TestCase):
    """A routine's block on a made-up listing and build folder: its flow and its translation's path."""

    def test_the_block_holds_the_flow_and_names_the_translation(self):
        with tempfile.TemporaryDirectory() as tmp:
            lst = os.path.join(tmp, 'x.lst')
            with open(lst, 'w') as f:
                f.write('h0+$10:\n00000010  61000004   bsr.w h0+$16\n00000014  4e75   rts\n'
                        'h0+$16:\n00000016  7001   moveq #1,d0\n00000018  4e75   rts\n')
            folder = os.path.join(tmp, 'translate', 'h0-10')
            os.makedirs(folder)
            open(os.path.join(folder, 'translated.rs'), 'w').close()
            r = {'name': 'made_up', 'hunk': 0, 'offset': 0x10, 'own': 2, 'instructions': 4}
            with mock.patch.object(tp, 'listing_path', return_value=lst), \
                 mock.patch.object(tp, 'port_dirs', return_value=[os.path.join(tmp, 'translate')]):
                block = tp.routine_block(r)
                r2 = dict(r, offset=0x16)
                none_yet = tp.routine_block(r2)
            self.assertIn('bsr.w h0+$16', block)
            self.assertIn('moveq #1,d0', block)  # the callee is followed
            self.assertIn(os.path.join(folder, 'translated.rs'), block)
            self.assertIn("translate.py 'h0+$16'` writes it", none_yet)
            # judge_run's listing path is put back
            self.assertNotEqual(tp.reshape.jr.LISTING, lst)


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
