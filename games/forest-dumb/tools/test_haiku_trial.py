"""haiku_trial.py (translator T19): the mark on fakes. No model call, no build: the two tools' result
lines are made up here."""
import json
import os
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import haiku_trial as h  # noqa: E402


def gen_rows(at_least):
    """Rows for T61's 5 generators, the first `at_least` covering every way Muse's did, the rest one short."""
    rows = []
    for i, m in enumerate(h.MUSE_GEN):
        un = [] if i < at_least else [{'at': 'h0+$4ea0', 'never': 'taken'}]
        rows.append(h.gen_row(m, {'result': 'covered' if not un else 'partial', 'tries': 1,
                                  'branches': m[3], 'uncovered': un}))
    return rows


def reshape_rows(passing):
    """Rows for T62's 15 units, routines passing until `passing` of the 21 do. Failing a four-routine
    unit fails four, so the single-routine units fail first."""
    order = sorted(h.MUSE_RESHAPE, key=lambda m: -len(m[2]))
    left, ok = passing, set()
    for m in order:
        if len(m[2]) <= left:
            ok.add(m[1])
            left -= len(m[2])
    assert left == 0, passing
    return [h.reshape_row(m, {'result': 'pass' if m[1] in ok else 'fail', 'tries': 1, 'paths': 3}, {})
            for m in h.MUSE_RESHAPE]


class Mark(unittest.TestCase):
    def test_the_sets_are_level_1s(self):
        self.assertEqual(len(h.MUSE_GEN), 5)
        self.assertEqual(sum(len(m[2]) for m in h.MUSE_RESHAPE), 21)
        self.assertEqual(len(h.MUSE_RESHAPE), 15)

    def test_the_mark_at_its_edges(self):
        for gen, reshape, met in ((5, 21, True), (5, 20, True), (5, 19, False), (4, 21, False), (4, 20, False)):
            with self.subTest(gen=gen, reshape=reshape):
                s = h.summarise(gen_rows(gen), reshape_rows(reshape))
                self.assertEqual((s['gen_at_least_muse'], s['reshape_pass'], s['mark_met']), (gen, reshape, met))

    def test_a_generator_is_at_least_muses_only_on_every_way_muses_took(self):
        m = h.MUSE_GEN[0]  # 5 branches
        self.assertTrue(h.gen_row(m, {'result': 'covered', 'branches': 5, 'uncovered': []})['at_least_muse'])
        # More branches counted than Muse's record, one way untaken: still 10 of Muse's 10.
        self.assertTrue(h.gen_row(m, {'result': 'partial', 'branches': 6,
                                      'uncovered': [{'at': 'x', 'never': 'taken'}]})['at_least_muse'])
        self.assertFalse(h.gen_row(m, {'result': 'partial', 'branches': 5,
                                       'uncovered': [{'at': 'x', 'never': 'either'}]})['at_least_muse'])
        self.assertEqual(h.ways(5, [{'never': 'either'}, {'never': 'not-taken'}]), 7)
        for result in ('fail', 'no-oracle', 'no-model', 'usage'):
            self.assertFalse(h.gen_row(m, {'result': result})['at_least_muse'])

    def test_no_answer_or_half_a_trial_is_no_verdict(self):
        g = gen_rows(5)
        g[0] = h.gen_row(h.MUSE_GEN[0], {'result': 'no-model'})
        s = h.summarise(g, reshape_rows(21))
        self.assertIsNone(s['mark_met'])
        self.assertEqual(s['no_model'], 1)
        self.assertIsNone(h.summarise(gen_rows(5), None)['mark_met'])
        self.assertIsNone(h.summarise(None, reshape_rows(21))['mark_met'])


class Timeout(unittest.TestCase):
    def test_an_item_whose_every_call_timed_out_is_a_failure_not_a_missing_verdict(self):
        with tempfile.TemporaryDirectory() as d:
            log = os.path.join(d, 'tries.jsonl')
            with open(log, 'w') as f:
                for fn, outcome in (('idle', 'pass'), ('left_pressed', 'timeout'), ('left_pressed', 'timeout'),
                                    ('move_x', 'no-model'), ('move_x', 'timeout')):
                    f.write(json.dumps({'fn': fn, 'outcome': outcome}) + '\n')
            self.assertEqual(h.timed_out({'result': 'no-model', 'tries': 2}, log, 'fn', 'left_pressed')['result'], 'timeout')
            self.assertEqual(h.timed_out({'result': 'no-model', 'tries': 2}, log, 'fn', 'move_x')['result'], 'no-model')
            self.assertEqual(h.timed_out({'result': 'fail', 'tries': 2}, log, 'fn', 'left_pressed')['result'], 'fail')
        rows = reshape_rows(21)
        rows[6] = h.reshape_row(h.MUSE_RESHAPE[6], {'result': 'timeout', 'tries': 3}, {})
        s = h.summarise(gen_rows(5), rows)
        self.assertEqual((s['reshape_pass'], s['no_model'], s['mark_met']), (20, 0, True))


class Run(unittest.TestCase):
    def test_main_writes_the_table_and_the_summary_and_exits_on_the_mark(self):
        for gen, reshape, code in ((5, 20, 0), (5, 19, 1)):
            with self.subTest(gen=gen, reshape=reshape), tempfile.TemporaryDirectory() as out, \
                    mock.patch.object(h, 'run_gen', return_value=gen_rows(gen)), \
                    mock.patch.object(h, 'run_reshape', return_value=reshape_rows(reshape)), \
                    mock.patch('sys.stdout') as stdout:
                self.assertEqual(h.main(['--out', out]), code)
                with open(os.path.join(out, 'summary.json')) as f:
                    s = json.load(f)
                self.assertEqual(s['reshape_pass'], reshape)
                with open(os.path.join(out, 'table.md')) as f:
                    t = f.read()
                self.assertIn(f'({reshape} of 21 routines pass)', t)
                self.assertIn('| `h0_4b0e` | character |', t)
                self.assertIn('| T09 | `follow` | `h0_4e9a`, `h0_4ea6`, `h0_4eb4`, `h0_4ec2` |', t)
                self.assertIn('level 1 T62', t)
                self.assertIn('"mark_met":', ''.join(c.args[0] for c in stdout.write.call_args_list))

    def test_only_one_half_runs_the_other_not_at_all(self):
        with tempfile.TemporaryDirectory() as out, \
                mock.patch.object(h, 'run_gen', return_value=gen_rows(5)) as g, \
                mock.patch.object(h, 'run_reshape') as r, mock.patch('sys.stdout'):
            self.assertEqual(h.main(['--only', 'gen', '--out', out]), 1)
            g.assert_called_once()
            r.assert_not_called()


if __name__ == '__main__':
    unittest.main()
