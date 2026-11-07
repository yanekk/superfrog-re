"""reshape.py (plans/forest-dumb-level-1 T62). Model calls are stubbed: a test file never calls the
network. The rig is stubbed too, so this needs no disks; test_reshape_judge.py builds real answers
into the scratch copy and runs them on the rig."""
import json
import os
import re
import shutil
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import reshape as r  # noqa: E402


def fake_ports(root):
    """A passing literal port for every unit's routines, each naming itself."""
    for cfg in r.TASKS.values():
        for u in cfg['units']:
            for name, off in u['routines']:
                d = os.path.join(root, f'h0-{off:x}')
                os.makedirs(d, exist_ok=True)
                with open(os.path.join(d, 'result.json'), 'w') as f:
                    json.dump({'result': 'pass'}, f)
                with open(os.path.join(d, 'port.rs'), 'w') as f:
                    f.write(f'pub fn port(cpu: &mut Cpu) {{ /* literal {name} */ }}\n')


def src(task):
    return r.read(os.path.join(r.ENGINE, r.TASKS[task]['src']))


def test_src(task):
    return r.read(os.path.join(r.ENGINE, r.TASKS[task]['test']))


def body(text):
    """A function's text from its signature on: what must not leak even without its docs."""
    return text[text.index('fn '):]


class Prompt(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        fake_ports(self.tmp.name)
        self.dirs = [self.tmp.name]

    def test_a_routines_own_engine_function_and_rig_test_never_reach_its_prompt(self):
        for task, cfg in r.TASKS.items():
            for u in cfg['units']:
                with self.subTest(task=task, fn=u['fn']):
                    p = r.prompt(task, u, dirs=self.dirs)
                    for fn in r.removed(u):
                        self.assertNotIn(body(r.fn_text(src(task), fn)), p)
                        # Not its doc or signature either, except the target's bare signature.
                        self.assertNotIn(r.fn_text(src(task), fn).split('{')[0].strip(), p)
                    own = r.rig_text(cfg, u, test_src(task))
                    self.assertTrue(own, 'the rig text was found, so its absence below means something')
                    for t in own:
                        self.assertNotIn(t.strip(), p)
                    for name, _ in u['routines']:
                        self.assertIn(f'/* literal {name} */', p)
                    ex = r.example_unit(cfg, u)
                    self.assertNotEqual(ex['fn'], u['fn'])
                    self.assertIn(body(r.fn_text(src(task), ex['fn'])), p)
                    for t in r.rig_text(cfg, ex, test_src(task)):
                        self.assertIn(t.strip(), p)

    def test_the_prompt_names_the_signature_and_shows_the_rest_as_signatures(self):
        u = r.find_unit('T08', 'h0+$4274')[1]
        p = r.prompt('T08', u, dirs=self.dirs)
        self.assertIn("Write `pub fn rise_step(&mut self)` in `impl Character<'_> { ... }`", p)
        self.assertIn('pub fn fall_step(&mut self) { ... }', p)
        self.assertIn('pub struct Motion {', p)
        self.assertIn('var("char_jump_t", 0, 0x410a, Width::Byte),', p)
        self.assertNotIn('#[cfg(test)]', p)

    def test_a_routine_with_no_passing_literal_port_is_a_usage_error(self):
        u = r.find_unit('T08', 'h0_4274')[1]
        with open(os.path.join(self.tmp.name, 'h0-4274', 'result.json'), 'w') as f:
            json.dump({'result': 'fail'}, f)
        with self.assertRaises(r.Usage):
            r.prompt('T08', u, dirs=self.dirs)


class Units(unittest.TestCase):
    def test_every_rig_routine_with_an_engine_function_is_a_unit_once(self):
        for task, cfg in r.TASKS.items():
            t = test_src(task)
            names = [n for u in cfg['units'] for n, _ in u['routines']]
            self.assertEqual(len(names), len(set(names)))
            for u in cfg['units']:
                for name, off in u['routines']:
                    self.assertRegex(t, rf'\("{name}",\s*{off:#x},')
                    self.assertIs(r.find_unit(task, f'h0+${off:x}')[1], u)
                    self.assertIs(r.find_unit(task, name)[1], u)

    def test_a_removed_helper_is_called_by_nothing_left(self):
        for task, cfg in r.TASKS.items():
            for u in cfg['units']:
                text = src(task)
                for a, b in sorted(r.fn_spans(text, r.removed(u)), reverse=True):
                    text = text[:a] + text[b:]
                code = r.mask(text)
                for h in u['helpers']:
                    with self.subTest(task=task, helper=h):
                        self.assertIsNone(re.search(rf'\b{h}\s*\(', code))

    def test_an_unknown_routine_or_task_is_a_usage_error(self):
        with self.assertRaises(r.Usage):
            r.find_unit('T08', 'h0+$4d7e')  # camera_update: T09's, and no literal port
        with self.assertRaises(r.Usage):
            r.find_unit('T10', 'h0+$4274')


class Source(unittest.TestCase):
    def test_mask_blanks_comments_strings_and_chars_but_not_lifetimes(self):
        s = "fn a<'x>(c: char) { let s = \"{\"; let b = '}'; // {\n }"
        m = r.mask(s)
        self.assertEqual(len(m), len(s))
        self.assertEqual(m.count('{'), 1)
        self.assertEqual(m.count('}'), 1)
        self.assertIn("<'x>", m)

    def test_skeleton_keeps_types_whole_and_bodies_out(self):
        s = ('//! module\nuse a::{b, c};\n\n/// Doc.\npub struct S {\n    pub x: u8,\n}\n\nconst T: [u8; 2] = [1, 2];\n\n'
             'impl S {\n    /// Gone.\n    pub fn gone(&self) {\n        1;\n    }\n\n    /// Kept.\n    fn kept(&self) -> u8 {\n'
             '        2\n    }\n}\n\nimpl std::fmt::Debug for S {\n    fn fmt(&self) {}\n}\n\n#[cfg(test)]\nmod tests {\n    fn t() {}\n}\n')
        k = r.skeleton(s, drop=['gone'])
        self.assertIn('use a::{b, c};', k)
        self.assertIn('/// Doc.\npub struct S {\n    pub x: u8,\n}', k)
        self.assertIn('const T: [u8; 2] = [1, 2];', k)
        self.assertIn('    /// Kept.\n    fn kept(&self) -> u8 { ... }', k)
        for gone in ('gone', 'module', 'Debug', 'mod tests', '2\n'):
            self.assertNotIn(gone, k)


class Scratch(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        p = mock.patch.object(r, 'SCRATCH', self.tmp.name)
        p.start()
        self.addCleanup(p.stop)

    def test_the_copy_cuts_the_unit_appends_the_answer_and_limits_the_rig(self):
        before = src('T09')
        u = r.find_unit('T09', 'h0+$51fc')[1]
        eng = r.scratch('T09', u, 'impl Camera { /* the answer */ }\n')
        self.assertEqual(src('T09'), before, 'the committed source is never changed')
        copy = r.read(os.path.join(eng, r.TASKS['T09']['src']))
        self.assertNotIn(body(r.fn_text(before, 'scroll')), copy)
        self.assertNotIn(body(r.fn_text(before, 'set')), copy)
        self.assertIn(body(r.fn_text(before, 'follow')), copy)
        self.assertTrue(copy.rstrip().endswith('impl Camera { /* the answer */ }'))
        test = r.read(os.path.join(eng, r.TASKS['T09']['test']))
        self.assertIn('RESHAPE_ONLY', test)
        core = r.read(os.path.join(eng, 'crates', 'fd-core', 'Cargo.toml'))
        self.assertIn(f'path = "{os.path.normpath(os.path.join(r.ENGINE, "..", "..", "..", "toolkit"))}', core)
        self.assertIn('path = "../fd-assets"', core)
        self.assertTrue(os.path.isdir(os.path.join(self.tmp.name, 're')))
        self.assertFalse(os.path.exists(os.path.join(eng, 'target')))


def answer(code):
    return f'Here.\n\n```rust\n{code}```\n'


def usage(cost=0.002):
    return {'in': 1000, 'out': 3000, 'reasoning': 2000, 'cost': cost, 'finish': 'stop'}


class Service(unittest.TestCase):
    """reshape() with the key, the price, the model and the rig replaced."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.out = os.path.join(self.tmp.name, 'out')
        ports = os.path.join(self.tmp.name, 'ports')
        fake_ports(ports)
        self.dirs = [ports]
        self.sent, self.answers, self.verdicts, self.judged = [], [], [], []

        def call_model(messages, key):
            self.sent.append(messages[-1]['content'])
            return self.answers.pop(0), usage()

        def run_rig(task, u, code, root=r.ENGINE, out=None):
            self.judged.append(code)
            return self.verdicts.pop(0)

        for name, value in [('find_key', lambda: ('sk-test', 'nowhere')), ('price', lambda key: (1e-7, 2e-7)),
                            ('call_model', call_model)]:
            p = mock.patch.object(r.tr, name, value)
            p.start()
            self.addCleanup(p.stop)
        p = mock.patch.object(r, 'run_rig', run_rig)
        p.start()
        self.addCleanup(p.stop)

    def run_it(self, **kw):
        return r.reshape('h0+$4274', 'T08', out=self.out, dirs=self.dirs, **kw)

    def test_compile_errors_and_the_first_failing_case_go_back_then_a_pass(self):
        self.answers = [answer('impl Character<\'_> { nope }\n'), 'no code at all',
                        answer('impl Character<\'_> { wrong }\n')]
        self.verdicts = [{'outcome': 'compile', 'feedback': 'It did not compile:\nerror: expected `fn`'},
                         {'outcome': 'fail', 'feedback': 'differs:\ncase 7: char_y $70 vs $6f'}]
        res = self.run_it()
        self.assertEqual(res['result'], 'fail')
        self.assertEqual(res['tries'], 3)
        self.assertIn('Write `pub fn rise_step(&mut self)`', self.sent[0])
        self.assertEqual(self.sent[1], 'It did not compile:\nerror: expected `fn`')
        self.assertIn('no ```rust block', self.sent[2])
        self.assertEqual(len(self.judged), 2)
        self.assertEqual(res['usd'], 0.006)

        self.answers = [answer('impl Character<\'_> { wrong }\n'), answer('impl Character<\'_> { right }\n')]
        self.verdicts = [{'outcome': 'fail', 'feedback': 'differs:\ncase 7: char_y $70 vs $6f'},
                         {'outcome': 'pass', 'paths': 6}]
        self.sent = []
        res = self.run_it()
        self.assertEqual(self.sent[1], 'differs:\ncase 7: char_y $70 vs $6f')
        self.assertEqual((res['result'], res['tries'], res['paths'], res['routines'], res['fn']),
                         ('pass', 2, 6, ['h0_4274'], 'rise_step'))
        self.assertEqual(r.read(res['port']), "impl Character<'_> { right }\n")
        saved = json.loads(r.read(os.path.join(self.out, 'h0-4274', 'result.json')))
        self.assertEqual((saved['result'], saved['task']), ('pass', 'T08'))
        calls = [json.loads(l) for l in r.read(os.path.join(self.out, 'calls.jsonl')).splitlines()]
        self.assertEqual([c['outcome'] for c in calls], ['compile', 'no-code', 'fail', 'fail', 'pass'])

    def test_no_call_starts_past_the_cap(self):
        # 96 000 answer tokens at $2e-7 is $0.0192 a try: the cap allows none.
        res = self.run_it(max_usd=0.01)
        self.assertEqual((res['result'], res['tries'], res.get('stopped')), ('fail', 0, 'max-usd'))
        self.assertEqual(self.sent, [])

    def test_no_key_makes_no_call(self):
        with mock.patch.object(r.tr, 'find_key', lambda: (None, 'nowhere')):
            res = self.run_it()
        self.assertEqual(res['result'], 'no-key')
        self.assertEqual(self.sent, [])


class OpusCost(unittest.TestCase):
    def test_sums_every_result_event_of_each_tasks_sessions(self):
        with tempfile.TemporaryDirectory() as d:
            def session(name, costs):
                with open(os.path.join(d, f'{name}.ndjson'), 'w') as f:
                    f.write(json.dumps({'dir': 'out', 'kind': 'message', 'text': 'Task'}) + '\n')
                    f.write('not json\n')
                    for c in costs:
                        f.write(json.dumps({'dir': 'in', 'event': {'type': 'result', 'model_usage_since_last_result': {
                            'claude-opus-5-5': {'costUSD': c}, 'claude-haiku-5-5': {'costUSD': 0.5}}}}) + '\n')
                        f.write(json.dumps({'dir': 'in', 'event': {'type': 'assistant'}}) + '\n')
            session('T08-implement-1', [10.0, 2.0])
            session('T08-review-1', [1.5])
            session('T09-implement-1', [3.0])
            session('T09-implement-2', [4.0])
            session('T10-implement-1', [99.0])
            c = r.opus_cost(d)
        self.assertEqual(c['T08'], {'sessions': ['T08-implement-1', 'T08-review-1'], 'usd': 15.0, 'routines': 14,
                                    'usd_per_routine': round(15.0 / 14, 3)})
        self.assertEqual(c['T09']['usd'], 8.0)
        self.assertEqual(c['T51'], {'sessions': [], 'usd': 0.0, 'routines': 6, 'usd_per_routine': 0.0})


TIDY_CODE = """impl Character<'_> {
    /// Rises one step: the original's `h0_4274` (h0+$4274).
    pub fn rise_step(&mut self) {
        self.motion.y = self.motion.y.wrapping_sub(RISE);
    }
}
"""

ROUGH_CODE = """impl Character<'_> {
    pub fn rise_step(&mut self) {
        let t = h(0, 0x431e);
        if self.left {
            self.a = self.a.wrapping_add(3);
            self.b = self.b.wrapping_sub(1);
            self.c = 0x1234;
            self.d = 1;
        }
        if self.right {
            self.a = self.a.wrapping_add(4);
            self.b = self.b.wrapping_sub(2);
            self.c = 0x1234;
            self.d = 1;
        }
    }
}
"""


class Quality(unittest.TestCase):
    def test_a_rough_answer_gets_every_kind_of_issue(self):
        issues = r.quality(ROUGH_CODE, r.TASKS['T08'])
        text = '\n'.join(issues)
        self.assertIn('`fn rise_step` has no `///` doc comment', text)
        self.assertIn('memory by address', text)
        self.assertIn('`0x431e` is `RISE_TABLE`', text)
        self.assertIn('`0x1234`', text)
        self.assertIn('appear twice', text)

    def test_a_tidy_answer_and_named_table_loads_pass(self):
        self.assertEqual(r.quality(TIDY_CODE, r.TASKS['T08']), [])
        named = TIDY_CODE.replace('wrapping_sub(RISE);', 'wrapping_sub(RISE);\n        self.scratch.a[0] = h(0, RISE_TABLE);')
        self.assertEqual(r.quality(named, r.TASKS['T08']), [])

    def test_hand_written_engine_functions_mostly_pass(self):
        # The check's bar is the file's own code: most of what Opus wrote for T08 and T09 has no issue.
        clean = total = 0
        for cfg in r.TASKS.values():
            text = r.read(os.path.join(r.ENGINE, cfg['src']))
            for u in cfg['units']:
                total += 1
                clean += not r.quality(''.join(r.fn_text(text, f) for f in r.removed(u)), cfg)
        self.assertGreaterEqual(clean / total, 0.7)

    def test_compiler_warnings_on_the_answer_are_numbered_from_its_first_line(self):
        cfg = r.TASKS['T08']
        with tempfile.TemporaryDirectory() as eng:
            p = os.path.join(eng, cfg['src'])
            os.makedirs(os.path.dirname(p))
            with open(p, 'w') as f:
                f.write('fn a() {}\n\n' + r.MARK + '\nimpl X {\n    fn b() { let unused = 1; }\n}\n')
            err = (f"{cfg['src']}:5:22: warning: unused variable: `unused`\n"
                   f"{cfg['src']}:1:4: warning: function `a` is never used\n"
                   "crates/other.rs:9:1: warning: elsewhere\n")
            self.assertEqual(r.warnings(cfg, eng, err),
                             ['The compiler warns at line 2 of your answer: unused variable: `unused`'])


class Tidy(Service):
    """The rounds after a pass: the quality check's issues go back, and the rig judges every answer."""

    def test_a_rough_pass_is_sent_back_and_the_tidy_one_kept(self):
        self.answers = [answer(ROUGH_CODE), answer(TIDY_CODE)]
        self.verdicts = [{'outcome': 'pass', 'paths': 6, 'lint': ['The compiler warns at line 3 of your answer: x']},
                         {'outcome': 'pass', 'paths': 6, 'lint': []}]
        res = self.run_it()
        self.assertEqual((res['result'], res['tries'], res['tidy_rounds'], res['quality']), ('pass', 1, 1, []))
        self.assertIn('It passes the rig', self.sent[1])
        self.assertIn('no `///` doc comment', self.sent[1])
        self.assertIn('The compiler warns at line 3', self.sent[1])
        self.assertEqual(r.read(res['port']), TIDY_CODE)

    def test_a_tidy_answer_that_fails_the_rig_keeps_the_passing_one(self):
        self.answers = [answer(ROUGH_CODE), answer(TIDY_CODE), answer(TIDY_CODE)]
        self.verdicts = [{'outcome': 'pass', 'paths': 6},
                         {'outcome': 'fail', 'feedback': 'differs:\ncase 3'},
                         {'outcome': 'fail', 'feedback': 'differs:\ncase 4'}]
        res = self.run_it()
        self.assertEqual((res['result'], res['tidy_rounds'], res['paths']), ('pass', 2, 6))
        self.assertIn('no longer behaves as your passing one', self.sent[2])
        self.assertEqual(r.read(res['port']), ROUGH_CODE)
        self.assertTrue(res['quality'], 'what is left goes to the builder')

    def test_tidy_zero_takes_the_first_pass(self):
        self.answers = [answer(ROUGH_CODE)]
        self.verdicts = [{'outcome': 'pass', 'paths': 6}]
        res = self.run_it(tidy=0)
        self.assertEqual((res['result'], res['tidy_rounds'], len(self.sent)), ('pass', 0, 1))
        self.assertTrue(res['quality'])


class All(Service):
    """--all over a new subsystem: examples stay by hand, a caller is retried once its callee passes."""

    def test_examples_are_skipped_and_a_failure_is_retried_after_another_pass(self):
        seen = []
        verdicts = {}

        def reshape(routine, task, tries, max_usd, out, tidy=2):
            u = r.find_unit(task, routine)[1]
            n = seen.count(u['fn'])
            seen.append(u['fn'])
            res = verdicts.get(u['fn'], ['pass'])[min(n, len(verdicts.get(u['fn'], ['pass'])) - 1)]
            return {'routine': routine, 'fn': u['fn'], 'result': res}

        verdicts['step'] = ['fail', 'pass']
        reported = []
        with mock.patch.object(r, 'reshape', reshape):
            res = r.reshape_all('T08', report=reported.append)
        units = [u['fn'] for u in r.TASKS['T08']['units'] if u['fn'] not in r.TASKS['T08']['examples']]
        self.assertEqual([x['fn'] for x in res], units)
        self.assertNotIn('land_or_fall', seen)
        self.assertEqual(seen.count('step'), 2)
        self.assertEqual(next(x for x in res if x['fn'] == 'step')['result'], 'pass')
        self.assertEqual(len(reported), len(units) + 1)

    def test_a_task_with_one_example_is_a_usage_error_not_a_crash(self):
        cfg = dict(r.TASKS['T08'], examples=['rise_step'])
        u = next(x for x in cfg['units'] if x['fn'] == 'rise_step')
        with self.assertRaises(r.Usage):
            r.example_unit(cfg, u)


class Stubs(unittest.TestCase):
    def test_another_units_stub_takes_its_passing_answer(self):
        cfg = r.TASKS['T08']
        units = {u['fn']: u for u in cfg['units']}
        src = ("impl Character<'_> {\n    /// a\n    pub fn rise_step(&mut self) { todo!() }\n\n"
               "    /// b\n    pub fn fall_step(&mut self) { todo!() }\n\n"
               "    /// c\n    pub fn move_x(&mut self) { self.x = 1; }\n}\n")
        with tempfile.TemporaryDirectory() as out:
            for fn in ('fall_step', 'move_x'):
                d = os.path.join(out, r.slug(units[fn]))
                os.makedirs(d)
                with open(os.path.join(d, 'result.json'), 'w') as f:
                    json.dump({'result': 'pass'}, f)
                with open(os.path.join(d, 'port.rs'), 'w') as f:
                    f.write(f"impl Character<'_> {{ pub fn {fn}(&mut self) {{ /* muse */ }} }}\n")
            got = r.fill_stubs(src, cfg, units['rise_step'], out)
        self.assertIn('pub fn rise_step(&mut self) { todo!() }', got, 'the unit itself is left to scratch()')
        self.assertNotIn('pub fn fall_step(&mut self) { todo!() }', got)
        self.assertIn('pub fn fall_step(&mut self) { /* muse */ }', got)
        self.assertIn('self.x = 1;', got, 'a written function is never replaced')
        self.assertNotIn('pub fn move_x(&mut self) { /* muse */ }', got)


class Masks(unittest.TestCase):
    def test_masks_and_shifts_are_not_bare_numbers_and_decimals_are_not_renamed(self):
        code = ("impl Camera {\n    /// Scrolls (h0+$51fc).\n    pub fn scroll(&mut self) {\n"
                "        let b = self.x & 0xfff0;\n        let c = self.y >> 12;\n        let n = 17154;\n    }\n}\n")
        self.assertEqual(r.quality(code, r.TASKS['T09']), [])


class Tasks(unittest.TestCase):
    def test_reshape_toml_loads_every_field_the_tool_reads(self):
        for task, cfg in r.TASKS.items():
            with self.subTest(task=task):
                for k in ('src', 'impl', 'test', 'test_fn', 'loop', 'state', 'harness', 'examples', 'compared', 'units'):
                    self.assertIn(k, cfg)
                self.assertTrue(all(isinstance(o, int) for u in cfg['units'] for _, o in u['routines']))
                self.assertTrue(set(cfg['examples']) <= {u['fn'] for u in cfg['units']})


if __name__ == '__main__':
    unittest.main()
