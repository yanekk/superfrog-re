"""translator_trial.py (translator-trial T06). Export, build and judge are stub commands
(FD_TRIAL_EXPORT, FD_TRIAL_BUILD, FD_TRIAL_JUDGE); the translation is the real pcode_rust.py on
pcode-rt's made-up jump-table fixture, so the hinted tree differs from the plain one. Nothing here
touches build/, Ghidra, cargo or the oracle."""
import fcntl
import json
import os
import re
import shlex
import shutil
import sys
import tempfile
import textwrap
import unittest
from unittest import mock

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import translator_trial as t  # noqa: E402

FIXTURES = os.path.join(os.path.dirname(HERE), 'engine', 'crates', 'pcode-rt', 'tests', 'fixtures')

INVENTORY = '''
[[routine]]
name = "h0_0"
hunk = 0
offset = 0x0
instructions = 9
subsystem = "camera"
gen_task = "T56"

[[routine]]
name = "mt_music"
hunk = 0
offset = 0x40
instructions = 300
subsystem = "sound"

[[routine]]
name = "waits_forever"
hunk = 0
offset = 0x80
instructions = 12
subsystem = "display"
gen_task = "T37"

[[routine]]
name = "draws_lines"
hunk = 0
offset = 0xc0
instructions = 30
subsystem = "display"
gen_task = "T37"

[[routine]]
name = "unregistered"
hunk = 0
offset = 0x100
instructions = 5
subsystem = "character"
gen_task = "T35"
'''

EXPORT = '''
import shutil, sys
shutil.copy(sys.argv[1], sys.argv[sys.argv.index('--out') + 1] + '/pcode.json')
'''

# Records which tree is "built in", as cargo would; fails like rustc when FAKE_BUILD_ERROR names a
# generated function to put the error in (in the tree FAKE_BUILD_ERROR_IN names, any without it).
# rustc names fd-conform's build.rs copy, OUT_DIR/tree.rs, whatever FD_TRANSLATED is called; the lines
# are the same (measured at T06 review on the real build).
BUILD = '''
import os, re, sys
tree = os.environ['FD_TRANSLATED']
bad = os.environ.get('FAKE_BUILD_ERROR')
if bad and os.environ.get('FAKE_BUILD_ERROR_IN', os.path.basename(tree)) == os.path.basename(tree):
    lines = open(tree).read().splitlines()
    at = next(i for i, l in enumerate(lines, 1) if l.startswith(f'pub fn {bad}('))
    print(f'/x/target/debug/build/fd-conform-0123/out/tree.rs:{at + 3}:9: error[E0308]: mismatched types',
          file=sys.stderr)
    sys.exit(101)
open(os.environ['FAKE_STATE'], 'w').write(os.path.basename(tree))
'''

# Answers from FAKE_ANSWERS by mode and, for --translated, by the tree last built.
JUDGE = '''
import json, os, sys
routine, gen, mode = sys.argv[1], sys.argv[3], sys.argv[-1]
ans = json.load(open(os.environ['FAKE_ANSWERS']))
if gen == 'unregistered':
    print(f'fd-judge: no generator `{gen}` in fd-conform/src/gen/', file=sys.stderr)
    sys.exit(2)
base = {'routine': routine, 'gen': gen, 'cases': int(sys.argv[5]), 'passed': 2000, 'paths': 3, 'hangs': 0,
        'faults': 0, 'unmodelled_blits': 0, 'failures': 0, 'pass': False, 'first_failure': ''}
if mode == '--survey':
    base.update(ans.get('survey', {}).get(routine, {}))
else:
    tree = open(os.environ['FAKE_STATE']).read()
    base.update(untranslated=0, write_into_code=0, unsupported=0, stopped_hangs=0, first_stop='')
    base.update(ans[tree][routine])
print(json.dumps(base))
'''

FAIL_PLAIN = {'pass': False, 'failures': 2000, 'passed': 0, 'untranslated': 2000,
              'first_failure': 'state 0: the port stopped: untranslated: h0+$10',
              'first_stop': 'untranslated: h0+$10'}
ANSWERS = {
    'survey': {'h0+$80': {'passed': 0, 'hangs': 2000},
               'h0+$c0': {'passed': 1990, 'unmodelled_blits': 4}},
    'tree.rs': {'h0+$0': FAIL_PLAIN},
    'tree-hints.rs': {'h0+$0': {'pass': True}},
}


class Trial(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        d = self.d = tmp.name
        stubs = {}
        for name, src in (('export', EXPORT), ('build', BUILD), ('judge', JUDGE)):
            p = os.path.join(d, f'{name}.py')
            with open(p, 'w') as f:
                f.write(textwrap.dedent(src))
            stubs[name] = p
        self.write('inventory.toml', INVENTORY)
        shutil.copy(os.path.join(FIXTURES, 'jump_table.hints.toml'), os.path.join(d, 'hints.toml'))
        self.write('answers.json', json.dumps(ANSWERS))
        py = shlex.quote(sys.executable)
        env = {'FD_TRIAL_EXPORT': f"{py} {shlex.quote(stubs['export'])} {shlex.quote(os.path.join(FIXTURES, 'jump_table.json'))}",
               'FD_TRIAL_BUILD': f"{py} {shlex.quote(stubs['build'])}",
               'FD_TRIAL_JUDGE': f"{py} {shlex.quote(stubs['judge'])}",
               'FAKE_STATE': os.path.join(d, 'built'), 'FAKE_ANSWERS': os.path.join(d, 'answers.json')}
        patcher = mock.patch.dict(os.environ, env)
        patcher.start()
        self.addCleanup(patcher.stop)
        self.paths = t.Paths(out=os.path.join(d, 'translator'), lock_dir=os.path.join(d, 'translate'),
                             inventory=os.path.join(d, 'inventory.toml'), hints=os.path.join(d, 'hints.toml'),
                             batches=[os.path.join(d, 'here', 'batch.json'), os.path.join(d, 'main', 'batch.json')])

    def write(self, name, text):
        p = os.path.join(self.d, name)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, 'w') as f:
            f.write(text)
        return p

    def run_trial(self, *argv):
        out, err = [], []
        with mock.patch('builtins.print', lambda *a, file=None, **k: (err if file is sys.stderr else out).append(' '.join(map(str, a)))):
            code = t.main(list(argv), paths=self.paths)
        return code, '\n'.join(out), '\n'.join(err)

    def table(self):
        with open(os.path.join(self.paths.out, 'table.md')) as f:
            return f.read()

    def test_table_columns_gate_and_unjudgeable_rows(self):
        code, out, err = self.run_trial()
        self.assertEqual(code, 0, err)
        s = json.loads(out)
        self.assertEqual(list(s), list(t.SUMMARY_KEYS))
        # One judgeable routine; the four others are out of the percentage.
        self.assertEqual(s, {'judgeable': 1, 'pass': 0, 'pass_pct': 0.0, 'pass_with_hints': 1, 'unjudgeable': 4,
                             'unsupported': 0, 'untranslated': 1, 'write_into_code': 0})
        tab = self.table()
        self.assertIn('| Routine | Subsystem | Instructions | Without hints | First difference or stop | Untranslated '
                      '| Write into code | Unsupported | With hints | Muse |', tab)
        self.assertIn('| `h0+$0` `h0_0` | camera | 9 | fail | untranslated: h0+$10 | 2000 | 0 | 0 | pass | none |', tab)
        rows = {m.group(1): m.group(0) for m in re.finditer(r'^\| `(h0\+\$\w+)`.*$', tab, re.M)}
        self.assertIn('unjudgeable: no generator (sound)', rows['h0+$40'])
        self.assertIn('unjudgeable: no-oracle: the original returned on none of 2000 states', rows['h0+$80'])
        self.assertIn('unjudgeable: no-oracle: a line or fill blit on 4 of 1990 states', rows['h0+$c0'])
        self.assertIn('unjudgeable: no generator registered (`unregistered`)', rows['h0+$100'])
        self.assertIn('Gate: 0 of 1 judgeable routines pass with no hand edit (0.0%), against 90%: not met.', tab)
        with open(os.path.join(self.paths.out, 'trial.json')) as f:
            self.assertEqual(json.load(f)['summary'], s)
        self.assertFalse(os.path.exists(os.path.join(self.paths.out, 'table.md.tmp')))

    def test_percentage_counts_only_judgeable_routines(self):
        rows = [{'unjudgeable': None, 'plain': {'verdict': 'pass'}, 'hints': {'verdict': 'pass'}}] * 9 + \
               [{'unjudgeable': None, 'plain': {'verdict': 'fail', 'unsupported': 3}, 'hints': {'verdict': 'fail'}}] + \
               [{'unjudgeable': 'no generator (sound)', 'plain': None, 'hints': None}] * 5
        s = t.summary(rows)
        self.assertEqual((s['judgeable'], s['pass'], s['pass_pct'], s['unjudgeable'], s['unsupported']),
                         (10, 9, 90.0, 5, 1))

    def test_routines_filter_and_skip_export(self):
        code, _, err = self.run_trial('--routines', 'h0_0,mt_music', '--cases', '50')
        self.assertEqual(code, 0, err)
        self.assertNotIn('waits_forever', self.table())
        self.assertIn('50 states a routine', self.table())
        os.environ['FD_TRIAL_EXPORT'] = 'false'   # would fail if run
        code, _, err = self.run_trial('--skip-export', '--routines', 'h0_0')
        self.assertEqual(code, 0, err)
        self.assertEqual(self.run_trial('--routines', 'nonesuch')[0], 2)

    def test_muse_column_reads_batch_json_then_the_main_checkout(self):
        self.write('main/batch.json', json.dumps({'rows': {'h0+$0': {'result': 'fail'}, 'h0+$40': {'result': 'no-gen'}}}))
        self.assertEqual(self.run_trial()[0], 0)
        self.assertRegex(self.table(), r'\| `h0\+\$0` .*\| 0 \| pass \| fail \|\n')
        self.assertRegex(self.table(), r'\| `h0\+\$40` `mt_music` .*\| no-gen \|\n')
        self.assertRegex(self.table(), r'\| `h0\+\$80` `waits_forever` .*\| none \|\n')
        # This checkout's batch wins over the main checkout's.
        self.write('here/batch.json', json.dumps({'rows': {'h0+$0': {'result': 'pass'}}}))
        self.assertEqual(self.run_trial()[0], 0)
        self.assertRegex(self.table(), r'\| `h0\+\$0` .*\| 0 \| pass \| pass \|\n')

    def test_a_second_run_while_the_lock_is_held_refuses_naming_it(self):
        os.makedirs(self.paths.lock_dir)
        lock = os.path.join(self.paths.lock_dir, 'judge.lock')
        with open(lock, 'w') as held:
            fcntl.flock(held, fcntl.LOCK_EX)
            code, _, err = self.run_trial()
        self.assertEqual(code, 1)
        self.assertIn(lock, err)
        self.assertFalse(os.path.exists(os.path.join(self.paths.out, 'table.md')))
        self.assertEqual(self.run_trial()[0], 0)   # released: the next run goes ahead

    def test_a_compile_failure_exits_1_naming_the_routine(self):
        os.environ['FAKE_BUILD_ERROR'] = 'h0_0'
        code, _, err = self.run_trial()
        self.assertEqual(code, 1)
        self.assertIn('does not compile, in h0+$0', err)
        self.assertIn('error[E0308]: mismatched types', err)
        self.assertFalse(os.path.exists(os.path.join(self.paths.out, 'table.md')))

    def test_a_compile_failure_of_the_hinted_tree_names_its_routine(self):
        os.environ.update(FAKE_BUILD_ERROR='h0_0', FAKE_BUILD_ERROR_IN='tree-hints.rs')
        code, _, err = self.run_trial()
        self.assertEqual(code, 1)
        self.assertIn('tree-hints.rs does not compile, in h0+$0', err)

    def test_a_failing_export_is_named(self):
        os.environ['FD_TRIAL_EXPORT'] = f'{shlex.quote(sys.executable)} -c "import sys; sys.exit(2)"'
        code, _, err = self.run_trial()
        self.assertEqual(code, 1)
        self.assertIn('export:', err)

    def test_a_judge_past_its_limit_fails_as_timeout(self):
        slow = self.write('slow.py', 'import time; time.sleep(5)\n')
        os.environ['FD_TRIAL_JUDGE'] = f'{shlex.quote(sys.executable)} {shlex.quote(slow)}'
        with mock.patch.object(t.translate, 'JUDGE_SECONDS', 0.2):
            v = t.verdict({'label': 'h0+$0', 'gen': 'h0_0'}, 10)
        self.assertEqual((v['verdict'], v['first']), ('fail', 'timeout (0.2 s)'))

    def test_a_crashing_judge_fails_the_routine_not_the_run(self):
        boom = self.write('boom.py', 'import os, signal; os.kill(os.getpid(), signal.SIGABRT)\n')
        os.environ['FD_TRIAL_JUDGE'] = f'{shlex.quote(sys.executable)} {shlex.quote(boom)}'
        v = t.verdict({'label': 'h0+$0', 'gen': 'h0_0'}, 10)
        self.assertEqual(v['verdict'], 'fail')
        self.assertIn('crash (signal 6)', v['first'])


if __name__ == '__main__':
    unittest.main()
