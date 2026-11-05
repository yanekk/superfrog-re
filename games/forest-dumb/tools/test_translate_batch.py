"""translate_batch.py (plans/forest-dumb-level-1 T40). The per-routine runner is stubbed: a test never
calls the network, the listing or the judge. translate.py itself is tested in test_translate.py."""
import json
import os
import sys
import tempfile
import threading
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import translate_batch as b  # noqa: E402


def entry(off, name=None, split=(), gen=True, task='T08'):
    label = f'h0+${off:x}'
    return {'label': label, 'name': name or f'h0_{off:x}', 'hunk': 0, 'offset': off,
            'gen': (name or f'h0_{off:x}') if gen else None, 'port_task': task if gen else None,
            'subsystem': 'character' if gen else 'sound', 'split_at': list(split)}


class Stub:
    """A runner answering by label: `outcomes[label]` is a result or a dict; records each call."""

    def __init__(self, out, outcomes=None, usd=0.01):
        self.out, self.outcomes, self.usd = out, outcomes or {}, usd
        self.calls, self.lock = [], threading.Lock()

    def __call__(self, e, max_usd, out):
        with self.lock:
            self.calls.append((e['label'], max_usd))
        o = self.outcomes.get(e['label'], 'pass')
        if isinstance(o, dict):
            return o
        res = {'routine': e['label'], 'result': o, 'tries': 1 if o in ('pass', 'fail') else 0,
               'usd': min(self.usd, max_usd) if o in ('pass', 'fail') else 0.0, 'paths': 3, 'port': None}
        if o == 'pass':
            d = os.path.join(out, e['label'].replace('+$', '-'))
            os.makedirs(d, exist_ok=True)
            port = os.path.join(d, 'port.rs')
            with open(port, 'w') as f:
                f.write('pub fn port(cpu: &mut Cpu) {}\n')
            res['port'] = os.path.relpath(port, b.REPO)
        return res


class Batch(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.out = os.path.join(self.tmp.name, 'translate')

    def tearDown(self):
        self.tmp.cleanup()

    def results(self, batch):
        return {k: r['result'] for k, r in batch['rows'].items()}

    def test_resume_redoes_only_unfinished_routines(self):
        todo = [entry(0x10), entry(0x20), entry(0x30), entry(0x40), entry(0x50)]
        # First run: one passes, one fails, one is too big, one errs, and the cap leaves the last.
        first = Stub(self.out, {'h0+$20': 'fail', 'h0+$30': 'too-big',
                                'h0+$40': {'result': 'error', 'note': 'fd-judge does not build'},
                                'h0+$50': {'result': 'fail', 'tries': 0, 'stopped': 'max-usd', 'usd': 0.0}})
        b.run_batch(todo, self.out, max_usd=5, parallel=1, runner=first)
        with open(os.path.join(self.out, 'batch.json')) as f:
            saved = json.load(f)
        self.assertEqual(self.results(saved),
                         {'h0+$10': 'pass', 'h0+$20': 'fail', 'h0+$30': 'too-big', 'h0+$40': 'error',
                          'h0+$50': 'not-attempted'})
        second = Stub(self.out)
        batch = b.run_batch(todo, self.out, max_usd=5, parallel=4, resume=True, runner=second)
        self.assertEqual(sorted(l for l, _ in second.calls), ['h0+$40', 'h0+$50'])
        self.assertEqual(self.results(batch), {'h0+$10': 'pass', 'h0+$20': 'fail', 'h0+$30': 'too-big',
                                               'h0+$40': 'pass', 'h0+$50': 'pass'})
        self.assertEqual(batch['kept'], 3)
        self.assertEqual(batch['spent'], 0.02)

    def test_a_pass_whose_port_is_gone_is_redone(self):
        todo = [entry(0x10)]
        batch = b.run_batch(todo, self.out, 5, runner=Stub(self.out))
        os.remove(os.path.join(b.GAME, batch['rows']['h0+$10']['port']))
        again = Stub(self.out)
        b.run_batch(todo, self.out, 5, resume=True, runner=again)
        self.assertEqual(again.calls, [('h0+$10', 0.10)])

    def test_without_resume_everything_runs_again(self):
        todo = [entry(0x10), entry(0x20)]
        b.run_batch(todo, self.out, 5, runner=Stub(self.out))
        again = Stub(self.out)
        b.run_batch(todo, self.out, 5, runner=again)
        self.assertEqual(len(again.calls), 2)

    def test_the_cap_stops_the_run_and_the_table_names_what_was_not_attempted(self):
        todo = [entry(off) for off in (0x10, 0x20, 0x30, 0x40)]
        stub = Stub(self.out, usd=0.10)
        batch = b.run_batch(todo, self.out, max_usd=0.25, parallel=1, runner=stub, min_call=0.02)
        # 0.10 + 0.10 spent; 0.05 left is enough for a call, not for the fourth.
        self.assertEqual([round(u, 6) for _, u in stub.calls], [0.10, 0.10, 0.05])
        self.assertLessEqual(batch['spent'], 0.25)
        self.assertEqual(self.results(batch)['h0+$40'], 'not-attempted')
        text = b.table(batch)
        self.assertIn('| `h0+$40` `h0_40` | not-attempted |', text)
        self.assertIn('Not attempted (the cap stopped the run): `h0_40`.', text)

    def test_parallel_routines_never_reserve_past_the_cap(self):
        todo = [entry(0x10 * i) for i in range(1, 9)]
        stub = Stub(self.out, usd=0.10)
        batch = b.run_batch(todo, self.out, max_usd=0.35, parallel=8, runner=stub, min_call=0.02)
        self.assertLessEqual(sum(u for _, u in stub.calls), 0.35 + 1e-9)
        self.assertEqual(len(stub.calls), 4)
        self.assertEqual(sum(1 for r in batch['rows'].values() if r['result'] == 'not-attempted'), 4)

    def test_a_call_translate_refuses_for_the_cap_is_not_attempted(self):
        stub = Stub(self.out, {'h0+$10': {'result': 'fail', 'tries': 0, 'stopped': 'max-usd', 'usd': 0.0}})
        batch = b.run_batch([entry(0x10), entry(0x20)], self.out, 5, parallel=1, runner=stub)
        self.assertEqual(self.results(batch), {'h0+$10': 'not-attempted', 'h0+$20': 'not-attempted'})
        self.assertEqual(len(stub.calls), 1)

    def test_a_split_routine_runs_after_its_callees_pass(self):
        todo = [entry(0x10, split=['h0+$30', 'h0+$40']), entry(0x30, split=['h0+$40']), entry(0x40)]
        stub = Stub(self.out)
        batch = b.run_batch(todo, self.out, 5, parallel=8, runner=stub)
        self.assertEqual([l for l, _ in stub.calls], ['h0+$40', 'h0+$30', 'h0+$10'])
        self.assertEqual(set(self.results(batch).values()), {'pass'})

    def test_a_callee_that_fails_blocks_its_caller(self):
        todo = [entry(0x10, split=['h0+$20']), entry(0x20), entry(0x30, split=['h0+$10'])]
        stub = Stub(self.out, {'h0+$20': 'no-oracle'})
        batch = b.run_batch(todo, self.out, 5, runner=stub)
        self.assertEqual(self.results(batch), {'h0+$10': 'blocked', 'h0+$20': 'no-oracle', 'h0+$30': 'blocked'})
        self.assertEqual(batch['rows']['h0+$10']['note'], 'split callee h0+$20 is no-oracle')
        self.assertEqual([l for l, _ in stub.calls], ['h0+$20'])

    def test_sound_has_no_generator_and_makes_no_call(self):
        stub = Stub(self.out)
        batch = b.run_batch([entry(0x10, gen=False)], self.out, 5, runner=stub)
        self.assertEqual(self.results(batch), {'h0+$10': 'no-gen'})
        self.assertEqual(stub.calls, [])

    def test_a_runner_that_raises_is_an_error_row_not_a_stopped_run(self):
        def broken(e, max_usd, out):
            raise RuntimeError('boom')
        batch = b.run_batch([entry(0x10)], self.out, 5, runner=broken)
        self.assertEqual(batch['rows']['h0+$10']['result'], 'error')

    def test_the_table_lists_by_hand_routines_under_their_porting_task(self):
        todo = [entry(0x10, task='T08'), entry(0x20, task='T25'), entry(0x30, task='T08'), entry(0x40, task='T09')]
        stub = Stub(self.out, {'h0+$10': 'too-big', 'h0+$20': {'result': 'no-oracle', 'note': 'a line blit'},
                               'h0+$30': 'fail'})
        text = b.table(b.run_batch(todo, self.out, 5, runner=stub))
        self.assertIn('- T08: `h0_10` (too-big); `h0_30` (fail)', text)
        self.assertIn('- T25: `h0_20` (no-oracle: a line blit)', text)
        self.assertNotIn('- T09', text)
        self.assertRegex(text, r'\| `h0\+\$40` `h0_40` \| pass \| T09 \| `[^`]*/h0-40/port\.rs` \|')


class Runner(unittest.TestCase):
    """run_one reads translate.py's exit and lines; here translate.py is a stand-in script."""

    def run_with(self, code, stdout, stderr):
        with tempfile.TemporaryDirectory() as d:
            fake = os.path.join(d, 'translate.py')
            with open(fake, 'w') as f:
                f.write(f'import sys\nsys.stdout.write({stdout!r})\nsys.stderr.write({stderr!r})\nsys.exit({code})\n')
            old, b.TOOL = b.TOOL, fake
            try:
                return b.run_one(entry(0x10), 0.1, d)
            finally:
                b.TOOL = old

    def test_an_unknown_generator_is_no_gen(self):
        r = self.run_with(2, '', 'translate: fd-judge: no generator `h0_10` in fd-conform/src/gen/\n')
        self.assertEqual(r['result'], 'no-gen')

    def test_another_usage_error_is_an_error_with_its_reason(self):
        r = self.run_with(2, '', 'translate: h0+$10 has no label in the listing\n')
        self.assertEqual(r, {'result': 'error', 'note': 'translate: h0+$10 has no label in the listing'})

    def test_a_result_line_is_read_with_stderr_as_its_note(self):
        r = self.run_with(1, '{"routine":"h0+$10","result":"too-big","tries":0}\n', 'translate: 200 instructions\n')
        self.assertEqual(r['result'], 'too-big')
        self.assertEqual(r['note'], 'translate: 200 instructions')


if __name__ == '__main__':
    unittest.main()
