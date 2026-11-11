"""generate.py (plans/forest-dumb-level-1 T61, translator T17). The model is test_model's fake `claude`
on PATH: a test file never calls the network. The listing and the judge are stubbed too, so this needs no disks; fd-judge's `--cover` is
run for real in test_generate_judge.py."""
import io
import json
import os
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import generate as g  # noqa: E402
import judge_run as jr  # noqa: E402
import model  # noqa: E402
from test_model import FakeClaude  # noqa: E402

TOOL = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'generate.py')
# A made-up routine: two branches, the second guarded by a compare of a variable.
LISTING = """h0+$5462:
00005462  4a80                  tst.l    d0
00005464  6704                  beq.s    h0+$546a
00005466  7201                  moveq    #$1, d1
00005468  4e75                  rts
h0+$546a:
0000546a  0c790005c0775000      cmpi.w   #$5, h0+$5000
00005470  6e02                  bgt.s    h0+$5474
00005472  7202                  moveq    #$2, d1
h0+$5474:
00005474  4e75                  rts"""
FULL = 'pub const ALL: &[(&str, Generator)] = &[("h0_5462", G)]; // covers both\n'
HALF = 'pub const ALL: &[(&str, Generator)] = &[("h0_5462", G)]; // never var > 5\n'
HANG = 'pub const ALL: &[(&str, Generator)] = &[("h0_5462", G)]; // d0 = loop forever\n'
WILD = 'pub const ALL: &[(&str, Generator)] = &[("h0_5462", G)]; // pokes code\n'


def cover(branches, passed=2000, hangs=0, faults=0, outside=0, first='', blits=0, base=False):
    return {'routine': 'h0+$5462', 'gen': 'h0_5462', 'cases': 2000, 'passed': passed, 'hangs': hangs,
            'faults': faults, 'unmodelled_blits': blits, 'base': base, 'branches': branches,
            'outside': outside, 'first_outside': first}


BOTH = [['h0+$5464', 900, 1100], ['h0+$5470', 500, 600]]
# h0+$5470 is never taken: no state has the variable above 5.
ONE_WAY = [['h0+$5464', 900, 1100], ['h0+$5470', 0, 1100]]


def answer(code, after=''):
    return f'Here it is.\n\n```rust\n{code}```\n{after}'


def usage(cost=0.001):
    return {'in': 1000, 'out': 2000, 'reasoning': 1500, 'cost': cost, 'finish': 'stop'}


def read(path):
    with open(path) as f:
        return f.read()


def queue(answers):
    """The tests' answers as the fake's: (text, usage) an answer, an exception a failed run."""
    return [{'exit': 1, 'stderr': str(a)} if isinstance(a, Exception) else a[0] for a in answers]


class Service(unittest.TestCase):
    """generate() with every outside piece replaced: the listing, the model (a fake `claude`), the
    judge."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.out = self.tmp.name
        self.answers = []
        self.judged = []
        self.fake = FakeClaude(self)

        def run_cover(path, hunk, off, name, out):
            src = read(path)
            self.judged.append(src)
            if 'loop forever' in src:
                return cover(BOTH, passed=1990, hangs=10)
            if 'pokes code' in src:
                return cover(BOTH, outside=2000, first='state 0 pokes $c077e8 (h0+$10), which the original reads on no state')
            if 'both' in src:
                return cover(BOTH)
            return cover(ONE_WAY)

        for name, value in [
            ('flow_text', lambda h, o, stops=(): (LISTING, 7)),
            ('symbols', lambda: [{'name': 'char_thing', 'hunk': 0, 'offset': 0x5462}]),
        ]:
            p = mock.patch.object(jr, name, value)
            p.start()
            self.addCleanup(p.stop)
        for name, value in [
            ('run_cover', run_cover),
            ('listing_bases', lambda: {0: 0, 1: 0x5be68}),
            ('examples', lambda h, o, n: [('camera.rs', '// an example\n')]),
        ]:
            p = mock.patch.object(g, name, value)
            p.start()
            self.addCleanup(p.stop)
        self.stderr = io.StringIO()
        p = mock.patch.object(sys, 'stderr', self.stderr)
        p.start()
        self.addCleanup(p.stop)

    def run_it(self, **kw):
        self.fake.answer(*queue(self.answers))
        return g.generate('h0+$5462', 'h0_5462', out=self.out, **kw)

    def ncalls(self):
        return len(self.fake.seen())

    def last(self, n):
        """What call `n` asked last: the whole prompt of a first try, else the end of the transcript."""
        return self.fake.prompts()[n].split('# Now answer this\n\n')[-1]

    def log(self):
        with open(os.path.join(self.out, 'tries.jsonl')) as f:
            return [json.loads(l) for l in f]

    def test_a_generator_covering_every_branch_is_covered_and_written(self):
        self.answers = [(answer(FULL), usage())]
        res = self.run_it()
        self.assertEqual((res['result'], res['tries'], res['branches'], res['returns'], res['uncovered']),
                         ('covered', 1, 2, 2000, []))
        self.assertEqual(read(res['gen']), FULL)
        self.assertTrue(res['gen'].endswith('h0_5462.rs'))
        prompt = self.last(0)
        self.assertIn('`h0_5462`', prompt)
        self.assertIn('char_thing', prompt)
        self.assertIn('// an example', prompt)
        self.assertIn('bgt.s', prompt)
        self.assertEqual([r['outcome'] for r in self.log()], ['covered'])

    def test_a_missing_branch_goes_back_with_its_listing_lines(self):
        self.answers = [(answer(HALF), usage()), (answer(FULL), usage())]
        res = self.run_it()
        self.assertEqual((res['result'], res['tries']), ('covered', 2))
        back = self.last(1)
        self.assertIn('h0+$5470: no state took it', back)
        # The branch line and the compare before it, not the other branch.
        self.assertIn('00005470  6e02                  bgt.s    h0+$5474', back)
        self.assertIn('cmpi.w   #$5, h0+$5000', back)
        self.assertNotIn('h0+$5464:', back)

    def test_three_answers_missing_a_branch_give_partial_naming_it(self):
        self.answers = [(answer(HALF), usage())] * 3 + [AssertionError('a fourth call')]
        res = self.run_it()
        self.assertEqual((res['result'], res['tries'], self.ncalls()), ('partial', 3, 3))
        self.assertEqual(res['uncovered'], [{'at': 'h0+$5470', 'never': 'taken'}])
        self.assertEqual(read(res['gen']), HALF)
        self.assertEqual([r['outcome'] for r in self.log()], ['partial'] * 3)
        self.assertEqual(json.loads(read(os.path.join(self.out, 'h0_5462', 'result.json')))['result'], 'partial')

    def test_a_listed_branch_the_rig_never_counted_is_uncovered(self):
        def run_cover(path, hunk, off, name, out):
            return cover([['h0+$5464', 900, 1100]])
        self.answers = [(answer(FULL), usage())]
        with mock.patch.object(g, 'run_cover', run_cover):
            res = self.run_it(tries=1)
        self.assertEqual(res['result'], 'partial')
        self.assertEqual(res['uncovered'], [{'at': 'h0+$5470', 'never': 'either'}])
        self.assertEqual(res['branches'], 2)

    def test_an_unreachable_claim_is_passed_through_and_not_asked_again(self):
        claim = 'UNREACHABLE h0+$5470 taken: the variable is set to 5 by its only writer\n'
        self.answers = [(answer(HALF, claim), usage()), AssertionError('a second call')]
        res = self.run_it()
        self.assertEqual((res['result'], res['tries']), ('partial', 1))
        self.assertEqual(res['uncovered'], [{'at': 'h0+$5470', 'never': 'taken',
                                             'reason': 'the variable is set to 5 by its only writer'}])

    def test_the_best_try_is_kept(self):
        self.answers = [(answer(FULL.replace('both', 'oops')), usage()), (answer(HANG), usage()),
                        (answer(WILD), usage())]
        res = self.run_it()
        # Only the first was valid; it missed one way.
        self.assertEqual((res['result'], res['tries']), ('partial', 3))
        self.assertIn('oops', read(res['gen']))

    def test_a_branch_every_state_takes_is_uncovered_not_taken(self):
        def run_cover(path, hunk, off, name, out):
            return cover([['h0+$5464', 900, 1100], ['h0+$5470', 1100, 0]])
        self.answers = [(answer(FULL), usage())] * 2
        with mock.patch.object(g, 'run_cover', run_cover):
            res = self.run_it(tries=2)
        self.assertEqual(res['uncovered'], [{'at': 'h0+$5470', 'never': 'not-taken'}])
        self.assertIn('h0+$5470: every state took it', self.last(1))

    def test_a_generator_hanging_the_original_is_a_failed_try_with_the_reason_back(self):
        self.answers = [(answer(HANG), usage()), (answer(FULL), usage())]
        res = self.run_it()
        self.assertEqual((res['result'], res['tries']), ('covered', 2))
        back = self.last(1)
        self.assertIn('never returned on 10 state(s)', back)
        self.assertEqual([r['outcome'] for r in self.log()], ['hang', 'covered'])

    def test_a_generator_poking_outside_the_reads_is_a_failed_try_with_the_reason_back(self):
        self.answers = [(answer(WILD), usage()), (answer(FULL), usage())]
        res = self.run_it()
        self.assertEqual((res['result'], res['tries']), ('covered', 2))
        back = self.last(1)
        self.assertIn('2000 poked byte(s)', back)
        self.assertIn('pokes $c077e8 (h0+$10)', back)
        self.assertEqual(self.log()[0]['outcome'], 'outside')

    def test_three_failed_tries_fail_with_no_generator(self):
        self.answers = [(answer(WILD), usage())] * 3
        res = self.run_it()
        self.assertEqual((res['result'], res['tries'], res['gen']), ('fail', 3, None))
        self.assertFalse(os.path.exists(os.path.join(self.out, 'h0_5462.rs')))

    def test_too_few_returns_and_line_blits_are_failed_tries(self):
        for j, want in ((cover(BOTH, passed=1899, faults=101), 'returned on 1899 of 2000 states (101 took an exception'),
                        (cover(BOTH, blits=4), 'line or fill blit')):
            with self.subTest(want=want):
                if os.path.exists(os.path.join(self.fake.dir, 'seen.jsonl')):
                    os.remove(os.path.join(self.fake.dir, 'seen.jsonl'))
                self.answers = [(answer(FULL), usage()), (answer(FULL), usage())]
                seq = [j, cover(BOTH)]
                with mock.patch.object(g, 'run_cover', lambda *a: seq.pop(0)):
                    res = self.run_it()
                self.assertEqual((res['result'], res['tries']), ('covered', 2))
                self.assertIn(want, self.last(1))

    def test_exactly_95_percent_returning_is_enough(self):
        self.answers = [(answer(FULL), usage())]
        with mock.patch.object(g, 'run_cover', lambda *a: cover(BOTH, passed=1900, faults=100)):
            self.assertEqual(self.run_it()['result'], 'covered')

    def test_compile_errors_and_a_missing_name_go_back(self):
        seq = [{'outcome': 'compile', 'feedback': 'It did not compile:\nerror[E0425]: nope'},
               {'outcome': 'unlisted', 'feedback': 'Your `ALL` does not list a generator named `h0_5462`.'},
               cover(BOTH)]
        self.answers = [(answer(FULL), usage())] * 3
        with mock.patch.object(g, 'run_cover', lambda *a: seq.pop(0)):
            res = self.run_it()
        self.assertEqual((res['result'], res['tries']), ('covered', 3))
        self.assertIn('E0425', self.last(1))
        self.assertIn('does not list', self.last(2))

    def test_an_original_returning_on_no_state_of_any_answer_is_no_oracle(self):
        self.answers = [(answer(FULL), usage())] * 2
        with mock.patch.object(g, 'run_cover', lambda *a: cover(BOTH, passed=0, hangs=2000)):
            res = self.run_it(tries=2)
        self.assertEqual(res['result'], 'no-oracle')
        self.assertIn('routine oracle', self.stderr.getvalue())

    def test_max_calls_stops_the_run(self):
        model.set_max_calls(1)
        self.answers = [(answer(HALF), usage()), AssertionError('a second call')]
        res = self.run_it()
        self.assertEqual((res['result'], res['tries'], res['stopped']), ('partial', 1, 'max-calls'))
        self.assertEqual(self.ncalls(), 1)
        self.assertIn('--max-calls 1', self.stderr.getvalue())

    def test_max_calls_spent_makes_no_call(self):
        model.set_max_calls(0)
        self.answers = [AssertionError('a call')]
        res = self.run_it()
        self.assertEqual((res['result'], res['tries'], res['stopped']), ('fail', 0, 'max-calls'))
        self.assertEqual(self.ncalls(), 0)

    def test_the_answer_comes_from_claude_p_with_the_system_prompt(self):
        self.answers = [(answer(FULL), usage())]
        res = self.run_it()
        self.assertEqual(res['usd'], 0.003)
        argv = self.fake.seen()[0]['argv']
        self.assertEqual(argv[argv.index('--system-prompt') + 1], g.SYSTEM)
        self.assertEqual(argv[argv.index('--model') + 1], 'haiku')
        with open(os.path.join(self.out, 'calls.jsonl')) as f:
            calls = [json.loads(l) for l in f]
        self.assertEqual([(c['result'], c['service'], c['try']) for c in calls], [('ok', 'generate', 1)])

    def test_a_failed_call_is_a_lost_try(self):
        self.answers = [RuntimeError('Not logged in'), (answer(FULL), usage())]
        res = self.run_it()
        self.assertEqual((res['result'], res['tries']), ('covered', 2))
        # The question is asked again as it was, with no failed exchange before it.
        self.assertEqual(self.fake.prompts()[1], self.fake.prompts()[0])
        self.assertEqual(self.log()[0]['outcome'], 'no-model')

    def test_a_claude_that_never_answers_is_no_model_not_fail(self):
        # Logged out: `claude` is on PATH but every call exits non-zero. Haiku was never asked, so the
        # result must not read as Haiku failing to write a generator (T19 counts `fail` against it).
        self.answers = [RuntimeError('Not logged in')] * 3
        res = self.run_it()
        self.assertEqual((res['result'], res['tries'], res['gen']), ('no-model', 3, None))
        self.assertIn('Not logged in', self.stderr.getvalue())


class Command(unittest.TestCase):
    def test_no_claude_on_path_gives_no_model_and_asks_nothing(self):
        with tempfile.TemporaryDirectory() as home:
            env = dict(os.environ, HOME=home, PATH=home)
            r = subprocess.run([sys.executable, '-I', TOOL, 'h0+$5462', '--name', 'h0_5462',
                                '--out', os.path.join(home, 'out')], capture_output=True, text=True, env=env)
        self.assertEqual(r.returncode, 1)
        res = json.loads(r.stdout)
        self.assertEqual((res['result'], res['tries'], res['usd'], res['gen']), ('no-model', 0, 0.0, None))
        self.assertIn('claude', r.stderr)

    def test_max_usd_is_gone(self):
        r = subprocess.run([sys.executable, '-I', TOOL, 'h0+$5462', '--name', 'x', '--max-usd', '1'],
                           capture_output=True, text=True)
        self.assertEqual(r.returncode, 2)
        self.assertIn('--max-usd', r.stderr)

    def test_an_unknown_routine_is_a_usage_error(self):
        with tempfile.TemporaryDirectory() as home:
            env = dict(os.environ, HOME=home)
            r = subprocess.run([sys.executable, '-I', TOOL, 'no_such_routine', '--name', 'x'],
                               capture_output=True, text=True, env=env)
        self.assertEqual((r.returncode, r.stdout), (2, ''))
        self.assertIn('no_such_routine', r.stderr)


class Pieces(unittest.TestCase):
    FILES = {
        'camera.rs': 'pub const ALL = &[("camera_update", g(a)), ("h0_513c", g(b))];',
        'edge_redraw.rs': 'pub const ALL = &[("h0_774", g(a)), ("h0_7cc", g(b))];' + ' ' * 500,
        'objects.rs': 'pub const ALL = &[("h0_9000", g(a))];',
        'spike.rs': 'pub const ALL = &[("h0+$24ce", g(a))];' + ' ' * 100,
        'rest.rs': 'pub const ALL = &[("h0_7cc_peer", g(a))];' + ' ' * 900,
    }
    INV = [{'name': 'h0_774', 'hunk': 0, 'offset': 0x774, 'subsystem': 'camera'},
           {'name': 'h0_7cc', 'hunk': 0, 'offset': 0x7cc, 'subsystem': 'camera'},
           {'name': 'camera_update', 'hunk': 0, 'offset': 0x4e00, 'subsystem': 'camera'},
           {'name': 'h0_513c', 'hunk': 0, 'offset': 0x513c, 'subsystem': 'camera'},
           {'name': 'h0_9000', 'hunk': 0, 'offset': 0x9000, 'subsystem': 'objects'}]

    def test_examples_come_from_the_subsystem_and_never_show_the_routine_s_own(self):
        shown = [f for f, _ in g.examples(0, 0x513c, 'h0_513c', self.FILES, self.INV)]
        # camera.rs registers h0_513c itself: it would be the answer.
        self.assertNotIn('camera.rs', shown)
        self.assertEqual(shown[0], 'edge_redraw.rs')
        self.assertEqual(len(shown), g.EXAMPLES)
        shown = [f for f, _ in g.examples(0, 0x9000, 'h0_9000', self.FILES, self.INV)]
        self.assertNotIn('objects.rs', shown)

    def test_the_routine_s_own_module_is_never_shown_whatever_its_name(self):
        # --name differs from the routine's re/translate.toml name: camera.rs still holds the answer.
        shown = [f for f, _ in g.examples(0, 0x4e00, 'my_name', self.FILES, self.INV)]
        self.assertNotIn('camera.rs', shown)

    def test_an_unknown_routine_gets_the_smallest_modules(self):
        shown = [f for f, _ in g.examples(0, 0xabc, 'h0_abc', self.FILES, self.INV)]
        self.assertEqual(shown, ['objects.rs', 'camera.rs', 'spike.rs'])

    def test_cond_branches_are_bcc_and_dbcc_not_bra_or_bsr(self):
        text = '\n'.join([
            '00000100  6704                  beq.s    h0+$106',
            '00000102  51c8fffc              dbra     d0, h0+$100',
            '00000106  6000                  bra.s    h0+$108',
            '00000108  6100                  bsr.w    h0+$200',
            '0000010a  0839                  btst.b   #$0, h0+$300',
            '0000010c  57c9fffc              dbeq     d1, h0+$100',
        ])
        self.assertEqual(sorted(g.cond_branches(text)), [0x100, 0x102, 0x10c])

    def test_listing_addresses_and_labels_by_hunk_base(self):
        bases = {0: 0, 1: 0x5be68, 4: 0x60000}
        self.assertEqual(g.to_listing('h1+$d4', bases), 0x5bf3c)
        self.assertEqual(g.to_label(0x5bf3c, bases), 'h1+$d4')
        self.assertEqual(g.to_label(0x5462, bases), 'h0+$5462')
        self.assertIsNone(g.to_listing('$7e0000', bases))

    @unittest.skipUnless(os.path.exists(jr.LISTING), "needs the listing from the owner's disks (re_listing.py)")
    def test_the_real_listing_s_bases(self):
        bases = g.listing_bases()
        self.assertEqual((bases[0], bases[1]), (0, 0x5be68))


if __name__ == '__main__':
    unittest.main()
