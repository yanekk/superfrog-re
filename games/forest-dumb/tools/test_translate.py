"""translate.py (plans/forest-dumb-level-1 T33). Model calls are stubbed: a test file never calls the
network. The listing and the judge are stubbed too, so this needs no disks; the judge itself is
tested in fd-conform's tests/judge.rs, and with a real build in test_translate_judge.py."""
import io
import json
import os
import subprocess
import sys
import tempfile
import unittest
import urllib.error
from unittest import mock

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import translate as t  # noqa: E402

TOOL = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'translate.py')
GOOD = 'pub fn port(cpu: &mut Cpu) { cpu.d[0] = 5; }\n'
WRONG = 'pub fn port(cpu: &mut Cpu) { cpu.d[0] = 6; }\n'
FIRST_CASE = 'case 7:\nentry registers: d=[...]\ndifferences (1 in all):\n  d0: original $00000005, port $00000006'


def write(path, text):
    with open(path, 'w') as f:
        f.write(text)


def read(path):
    with open(path) as f:
        return f.read()


def answer(code):
    return f'Here it is.\n\n```rust\n{code}```\n'


def usage(cost=0.001):
    return {'in': 1000, 'out': 2000, 'reasoning': 1500, 'cost': cost, 'finish': 'stop'}


class Service(unittest.TestCase):
    """translate() with every outside piece replaced: the key, the listing, the price, the model, the
    judge."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.out = self.tmp.name
        self.calls = []  # the messages each model call was sent
        self.answers = []
        self.survey = {'cases': 2000, 'passed': 2000, 'paths': 4, 'hangs': 0, 'faults': 0}
        self.judged = []

        def call_model(messages, key):
            self.calls.append([dict(m) for m in messages])
            nxt = self.answers.pop(0)
            if isinstance(nxt, Exception):
                raise nxt
            return nxt

        def judge_answer(path, hunk, off, gen, out):
            src = read(path)
            self.judged.append(src)
            if 'd[0] = 5' in src:
                return {'outcome': 'pass', 'paths': 4}
            return {'outcome': 'fail', 'failures': 2000,
                    'feedback': f'The judge ran your port against the original: it differs on 2000 of 2000 '
                                f'states. The first:\n{FIRST_CASE}\n\nReturn the whole corrected file.'}

        for name, value in [
            ('find_key', lambda: ('sk-test', 'nowhere')),
            ('flow_text', lambda h, o, stops=(): ('00005462  4e75  rts', 34)),
            ('split_at', lambda h, o: []),
            ('price', lambda key: (0.06e-6, 0.2e-6)),
            ('call_model', call_model),
            ('judge_answer', judge_answer),
            ('survey', lambda h, o, gen, out: self.survey),
            ('symbols', lambda: [{'name': 'char_sprite_position', 'hunk': 0, 'offset': 0x5462}]),
        ]:
            p = mock.patch.object(t, name, value)
            p.start()
            self.addCleanup(p.stop)
        self.stderr = io.StringIO()
        p = mock.patch.object(sys, 'stderr', self.stderr)
        p.start()
        self.addCleanup(p.stop)

    def tearDown(self):
        self.tmp.cleanup()

    def run_it(self, **kw):
        return t.translate('h0+$5462', 'char_sprite_position', out=self.out, **kw)

    def log(self):
        with open(os.path.join(self.out, 'calls.jsonl')) as f:
            return [json.loads(l) for l in f]

    def test_a_right_answer_passes_and_its_port_is_written(self):
        self.answers = [(answer(GOOD), usage())]
        res = self.run_it()
        self.assertEqual((res['result'], res['tries'], res['paths']), ('pass', 1, 4))
        self.assertEqual((res['tokens_in'], res['tokens_out'], res['usd']), (1000, 2000, 0.001))
        self.assertEqual(read(res['port']), GOOD)
        self.assertTrue(res['port'].endswith(os.path.join('h0-5462', 'port.rs')))
        self.assertIn('char_sprite_position', self.calls[0][1]['content'])
        self.assertEqual(self.calls[0][0]['role'], 'system')
        self.assertEqual([r['outcome'] for r in self.log()], ['pass'])

    def test_a_wrong_answer_gets_its_first_failing_case_back(self):
        self.answers = [(answer(WRONG), usage()), (answer(GOOD), usage())]
        res = self.run_it()
        self.assertEqual((res['result'], res['tries']), ('pass', 2))
        second = self.calls[1][-1]
        self.assertEqual(second['role'], 'user')
        self.assertIn(FIRST_CASE, second['content'])
        # The conversation carries on: the wrong answer is in it, before the feedback.
        self.assertEqual(self.calls[1][-2]['role'], 'assistant')

    def test_three_wrong_answers_fail_with_no_fourth_call(self):
        self.answers = [(answer(WRONG), usage())] * 3 + [AssertionError('a fourth call')]
        res = self.run_it()
        self.assertEqual((res['result'], res['tries'], res['port']), ('fail', 3, None))
        self.assertEqual(len(self.calls), 3)
        self.assertEqual(len(self.answers), 1)
        self.assertEqual([r['outcome'] for r in self.log()], ['fail'] * 3)
        self.assertFalse(os.path.exists(os.path.join(self.out, 'h0-5462', 'port.rs')))
        self.assertEqual(json.loads(read(os.path.join(self.out, 'h0-5462', 'result.json')))['result'], 'fail')

    def test_an_answer_cut_off_with_no_code_counts_as_a_try(self):
        self.answers = [('I am still reasoning about d0 and', dict(usage(), finish='length')),
                        (answer(GOOD), usage())]
        res = self.run_it()
        self.assertEqual((res['result'], res['tries']), ('pass', 2))
        self.assertIn('no ```rust block', self.calls[1][-1]['content'])
        self.assertEqual([r['outcome'] for r in self.log()], ['no-code', 'pass'])
        self.assertEqual(self.log()[0]['finish'], 'length')

    def test_three_cut_off_answers_fail(self):
        self.answers = [('thinking', usage())] * 3
        res = self.run_it()
        self.assertEqual((res['result'], res['tries'], len(self.calls)), ('fail', 3, 3))

    def test_a_failed_call_is_a_lost_try(self):
        self.answers = [t.CallError('HTTP 400: bad'), (answer(GOOD), usage())]
        res = self.run_it()
        self.assertEqual((res['result'], res['tries']), ('pass', 2))
        # The question is asked again, not stacked twice.
        self.assertEqual([m['role'] for m in self.calls[1]], ['system', 'user'])
        self.assertEqual(self.log()[0]['outcome'], 'error')

    def test_max_usd_stops_before_a_call_that_would_pass_it(self):
        # A full answer costs 96 000 x $0.2/M = $0.0192 at worst: $0.03 allows one call, not two.
        self.answers = [(answer(WRONG), usage(0.012)), AssertionError('a second call')]
        res = self.run_it(max_usd=0.03)
        self.assertEqual((res['result'], res['tries'], res['stopped']), ('fail', 1, 'max-usd'))
        self.assertEqual(len(self.calls), 1)
        self.assertIn('--max-usd', self.stderr.getvalue())

    def test_max_usd_below_one_call_makes_none(self):
        self.answers = [AssertionError('a call')]
        res = self.run_it(max_usd=0.01)
        self.assertEqual((res['result'], res['tries'], res['stopped']), ('fail', 0, 'max-usd'))
        self.assertEqual(self.calls, [])

    def test_a_routine_the_original_never_returns_from_is_no_oracle(self):
        self.survey = dict(self.survey, passed=0, paths=0, hangs=2000)
        self.answers = [AssertionError('a call')]
        res = self.run_it()
        self.assertEqual((res['result'], res['tries']), ('no-oracle', 0))
        self.assertEqual(self.calls, [])
        self.assertIn('2000 hang(s)', self.stderr.getvalue())

    def test_a_routine_starting_a_line_or_fill_blit_is_no_oracle(self):
        self.survey = dict(self.survey, unmodelled_blits=3)
        self.answers = [AssertionError('a call')]
        res = self.run_it()
        self.assertEqual((res['result'], res['tries']), ('no-oracle', 0))
        self.assertEqual(self.calls, [])
        self.assertIn('line or fill blit', self.stderr.getvalue())

    def test_a_routine_over_180_instructions_is_too_big(self):
        self.answers = [AssertionError('a call')]
        with mock.patch.object(t, 'flow_text', lambda h, o, stops=(): ('...', 181)):
            res = self.run_it()
        self.assertEqual((res['result'], res['tries']), ('too-big', 0))
        self.assertEqual(self.calls, [])
        with mock.patch.object(t, 'flow_text', lambda h, o, stops=(): ('...', 180)):
            self.answers = [(answer(GOOD), usage())]
            self.assertEqual(self.run_it()['result'], 'pass')

    def test_a_split_routine_needs_its_callees_passing_ports(self):
        with mock.patch.object(t, 'split_at', lambda h, o: [(0, 0x774)]):
            with self.assertRaises(t.Usage) as e:
                self.run_it()
            self.assertIn('h0+$774', str(e.exception))
            self.assertIn('translate it first', str(e.exception))
            callee = os.path.join(self.out, 'h0-774')
            os.makedirs(callee)
            write(os.path.join(callee, 'port.rs'), 'pub fn port(cpu: &mut Cpu) { cpu.d[1] = 1; }\n')
            write(os.path.join(callee, 'result.json'), json.dumps({'result': 'pass', 'split_at': []}))
            self.answers = [(answer('pub fn port(cpu: &mut Cpu) { port_h0_774(cpu); cpu.d[0] = 5; }\n'), usage())]
            res = self.run_it()
        self.assertEqual(res['result'], 'pass')
        built = self.judged[0]
        self.assertIn('pub mod callee_h0_774', built)
        self.assertIn('pub use callee_h0_774::port as port_h0_774;', built)
        self.assertIn('cpu.d[1] = 1', built)
        self.assertIn('port_h0_774(cpu)', self.calls[0][1]['content'])
        # The builder's port is the answer alone; the callees are in their own folders.
        self.assertNotIn('callee_h0_774', read(res['port']))


class Calls(unittest.TestCase):
    """call_model's waiting, with urllib replaced."""

    def test_429_502_503_are_waited_out_not_counted(self):
        ok = io.BytesIO(json.dumps({'choices': [{'message': {'content': 'x'}, 'finish_reason': 'stop'}],
                                    'usage': {'prompt_tokens': 3, 'completion_tokens': 4, 'cost': 0.5}}).encode())
        seq = [urllib.error.HTTPError('u', c, 'busy', {}, io.BytesIO(b'busy')) for c in (429, 502, 503)] + [ok]

        class Resp:
            def __init__(self, body):
                self.body = body

            def __enter__(self):
                return self.body

            def __exit__(self, *a):
                return False

        def urlopen(req, timeout):
            nxt = seq.pop(0)
            if isinstance(nxt, Exception):
                raise nxt
            return Resp(nxt)

        errors = [e for e in seq if isinstance(e, Exception)]
        for e in errors:
            self.addCleanup(e.close)
        waits = []
        with mock.patch.object(t.urllib.request, 'urlopen', urlopen), mock.patch.object(t, 'sleep', waits.append):
            text, u = t.call_model([{'role': 'user', 'content': 'q'}], 'k')
        self.assertEqual((text, u['in'], u['out'], u['cost']), ('x', 3, 4, 0.5))
        self.assertEqual(waits, list(t.WAITS[:3]))

    def test_another_error_is_raised(self):
        err = urllib.error.HTTPError('u', 400, 'bad', {}, io.BytesIO(b'bad request'))
        self.addCleanup(err.close)

        def urlopen(req, timeout):
            raise err

        with mock.patch.object(t.urllib.request, 'urlopen', urlopen), mock.patch.object(t, 'sleep', lambda s: None):
            with self.assertRaises(t.CallError) as e:
                t.call_model([], 'k')
        self.assertIn('HTTP 400', str(e.exception))


class Command(unittest.TestCase):
    def test_a_missing_key_gives_no_key_and_names_where_it_looked(self):
        with tempfile.TemporaryDirectory() as home:
            write(os.path.join(home, '.zshrc'), 'export PATH=/bin\n')
            env = {k: v for k, v in os.environ.items() if k != 'OPENROUTER_API_KEY'}
            env['HOME'] = home
            r = subprocess.run([sys.executable, '-I', TOOL, 'h0+$5462', '--gen', 'char_sprite_position',
                                '--out', os.path.join(home, 'out')], capture_output=True, text=True, env=env)
        self.assertNotEqual(r.returncode, 0)
        res = json.loads(r.stdout)
        self.assertEqual((res['result'], res['tries'], res['usd']), ('no-key', 0, 0.0))
        self.assertIn('OPENROUTER_API_KEY', r.stderr)
        self.assertIn(os.path.join(home, '.zshrc'), r.stderr)

    def test_the_key_is_read_from_zshrc(self):
        with tempfile.TemporaryDirectory() as home:
            write(os.path.join(home, '.zshrc'), '# x\nexport OPENROUTER_API_KEY="sk-or-abc"\n')
            with mock.patch.dict(os.environ, {'HOME': home}), mock.patch.dict(os.environ, {}, clear=False):
                os.environ.pop('OPENROUTER_API_KEY', None)
                self.assertEqual(t.find_key()[0], 'sk-or-abc')

    def test_an_unknown_routine_is_a_usage_error(self):
        with tempfile.TemporaryDirectory() as home:
            env = dict(os.environ, HOME=home, OPENROUTER_API_KEY='sk-test')
            r = subprocess.run([sys.executable, '-I', TOOL, 'no_such_routine', '--gen', 'x'],
                               capture_output=True, text=True, env=env)
        self.assertEqual((r.returncode, r.stdout), (2, ''))
        self.assertIn('no_such_routine', r.stderr)


class Pieces(unittest.TestCase):
    def test_routines_resolve_by_address_or_name(self):
        syms = [{'name': 'redraw_edge_a', 'hunk': 0, 'offset': 0x844}]
        self.assertEqual(t.resolve('h4+$18870', syms), (4, 0x18870))
        self.assertEqual(t.resolve('redraw_edge_a', syms), (0, 0x844))
        for bad in ('h5+$10', 'h0+5462', '$5462', 'nope'):
            with self.assertRaises(t.Usage):
                t.resolve(bad, syms)
        self.assertEqual((t.label(0, 0x5462), t.slug(0, 0x5462)), ('h0+$5462', 'h0-5462'))

    def test_split_at_is_read_from_the_inventory(self):
        with tempfile.TemporaryDirectory() as d:
            inv = os.path.join(d, 'translate.toml')
            write(inv, '[[routine]]\nname = "big"\nhunk = 0\noffset = 0xb20\n'
                                 'split_at = ["h0+$774", "redraw_edge_a"]\n')
            with mock.patch.object(t, 'INVENTORY', inv), \
                    mock.patch.object(t, 'symbols', lambda: [{'name': 'redraw_edge_a', 'hunk': 0, 'offset': 0x844}]):
                self.assertEqual(t.split_at(0, 0xb20), [(0, 0x774), (0, 0x844)])
                self.assertEqual(t.split_at(0, 0x5462), [])
        with mock.patch.object(t, 'INVENTORY', '/no/such/file'):
            self.assertEqual(t.split_at(0, 0xb20), [])


class FlowText(unittest.TestCase):
    """flow_text on a small made-up listing: real branches are followed, bit ops on a label are not (T58)."""

    LISTING = {
        0x100: 'btst.b   #$0, h0+$200', 0x102: 'bset.b   #$1, h0+$210', 0x104: 'bclr.b   #$2, h0+$220',
        0x106: 'bchg.b   #$3, h0+$230', 0x108: 'beq.s    h0+$140', 0x10a: 'dbra     d0, h0+$150',
        0x10c: 'bsr.w    h0+$160', 0x10e: 'jsr      h0+$170', 0x110: 'bra.s    h0+$180', 0x112: 'nop',
        0x140: 'nop', 0x142: 'rts', 0x150: 'nop', 0x152: 'rts', 0x160: 'rts', 0x170: 'rts',
        0x180: 'jmp      h0+$190', 0x182: 'nop', 0x190: 'rts',
        0x200: 'nop', 0x202: 'rts', 0x210: 'rts', 0x220: 'rts', 0x230: 'rts',
    }
    LABELS = (0x100, 0x140, 0x150, 0x160, 0x170, 0x180, 0x190, 0x200, 0x210, 0x220, 0x230)

    def flow(self):
        rows = []
        for a, txt in sorted(self.LISTING.items()):
            if a in self.LABELS:
                rows.append(f'h0+${a:x}:')
            rows.append(f'{a:08x}  4e71                  {txt}')
        with tempfile.TemporaryDirectory() as d:
            lst = os.path.join(d, 'x.lst')
            write(lst, '\n'.join(rows) + '\n')
            with mock.patch.object(t, 'LISTING', lst), mock.patch.object(t, 'symbols', lambda: []):
                text, n = t.flow_text(0, 0x100)
        return {int(l[:8], 16) for l in text.splitlines() if l[:8].isalnum() and len(l) > 8 and l[8] == ' '}, n

    def test_bit_ops_on_a_label_are_not_followed(self):
        seen, _ = self.flow()
        self.assertEqual(seen & {0x200, 0x202, 0x210, 0x220, 0x230}, set())

    def test_branches_dbcc_calls_and_jumps_are_followed(self):
        seen, n = self.flow()
        want = {0x100, 0x102, 0x104, 0x106, 0x108, 0x10a, 0x10c, 0x10e, 0x110,
                0x140, 0x142, 0x150, 0x152, 0x160, 0x170, 0x180, 0x190}
        self.assertEqual((seen, n), (want, len(want)))

    @unittest.skipUnless(os.path.exists(t.LISTING), 'needs the listing from the owner\'s disks (re_listing.py)')
    def test_real_sizes_match_their_unit_in_the_inventory(self):
        # The T35 and T53 routines T58 was measured on; before it, btst on h0+$4ed2 gave 371, 348, 348, 295.
        for off, unit in ((0x3ea8, 127), (0x4718, 104), (0x48be, 104), (0x4b0e, 51)):
            with self.subTest(routine=t.label(0, off)):
                self.assertEqual(t.flow_text(0, off, t.split_at(0, off))[1], unit)


if __name__ == '__main__':
    unittest.main()
