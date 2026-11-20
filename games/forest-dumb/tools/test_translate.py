"""translate.py, the translator's one command (translator T18, its DESIGN §2.12). Export, build and
judge are stub commands (FD_TRIAL_EXPORT, FD_TRIAL_BUILD, FD_TRIAL_JUDGE); the translation is the real
pcode_rust.py on pcode-rt's made-up `shared_call` fixture (h0+$0 calls $10 and $20, both call $30).
Nothing here touches build/, Ghidra, cargo or the oracle. The run on the game is test_translate_game.py."""
import json
import os
import shlex
import shutil
import sys
import tempfile
import textwrap
import unittest
from unittest import mock

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import translate as t  # noqa: E402

FIXTURES = os.path.join(os.path.dirname(HERE), 'engine', 'crates', 'pcode-rt', 'tests', 'fixtures')

# Counts its runs in FAKE_EXPORTS; fails when FAKE_EXPORT_FAIL is set.
EXPORT = '''
import os, shutil, sys
if os.environ.get('FAKE_EXPORT_FAIL'):
    print('ghidra: no JDK found', file=sys.stderr)
    sys.exit(2)
with open(os.environ['FAKE_EXPORTS'], 'a') as f:
    f.write('x')
shutil.copy(sys.argv[1], sys.argv[sys.argv.index('--out') + 1] + '/pcode.json')
print('{"instructions":11}')
'''

# Records the tree it was handed, as cargo would build it in. With FAKE_BUILD_ERROR set to a routine's
# function name, fails as rustc --message-format short would, on the line after that function's head
# in whichever part of the folder tree holds it.
BUILD = '''
import os, sys
tree = os.environ['FD_TRANSLATED']
open(os.environ['FAKE_BUILT'], 'w').write(tree)
fn = os.environ.get('FAKE_BUILD_ERROR')
if fn:
    for part in sorted(os.listdir(tree)):
        lines = open(os.path.join(tree, part)).read().splitlines()
        for i, l in enumerate(lines, 1):
            if l.startswith(f'pub fn {fn}('):
                print(f'{tree}/{part}:{i + 1}:5: error[E0425]: cannot find value `x`', file=sys.stderr)
                sys.exit(101)
    sys.exit('no such function')
'''

# Answers from FAKE_ANSWERS by mode and routine; each call is appended to FAKE_CALLS.
JUDGE = '''
import json, os, sys
with open(os.environ['FAKE_CALLS'], 'a') as f:
    f.write(json.dumps(sys.argv[1:]) + '\\n')
ans = json.load(open(os.environ['FAKE_ANSWERS']))
if sys.argv[1:] == ['--generators']:
    print(json.dumps({'generators': ans['generators']}))
    sys.exit(0)
routine, gen, cases, mode = sys.argv[1], sys.argv[3], int(sys.argv[5]), sys.argv[-1]
if gen not in ans['generators']:
    print(f'fd-judge: no generator `{gen}` in fd-conform/src/gen/', file=sys.stderr)
    sys.exit(2)
base = {'routine': routine, 'gen': gen, 'cases': cases, 'passed': cases, 'paths': 3, 'hangs': 0,
        'faults': 0, 'unmodelled_blits': 0, 'failures': 0, 'pass': True, 'first_failure': ''}
if mode == '--survey':
    base.update(ans.get('survey', {}).get(routine, {}))
else:
    base.update(untranslated=0, write_into_code=0, unsupported=0, stopped_hangs=0, first_stop='')
    base.update(ans.get('translated', {}).get(routine, {}))
print(json.dumps(base))
'''

SYMBOLS = '''
[[sym]]
name = "first_routine"
hunk = 0
offset = 0x0
kind = "func"

[[sym]]
name = "view_tile"
hunk = 0
offset = 0x10
kind = "func"

[[sym]]
name = "view_tile"
exe = "some_other_program"
hunk = 0
offset = 0x999
kind = "func"
'''

INVENTORY = '''
[[routine]]
name = "first_routine"
hunk = 0
offset = 0x0
port_task = "T08"

[[routine]]
name = "h0_20"
hunk = 0
offset = 0x20
port_task = "T08"
gen_task = "T56"
'''

ANSWERS = {
    'generators': ['first_routine', 'view_tile', 'h0_20', 'spare'],
    'survey': {},
    'translated': {},
}


class Translate(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        d = self.d = tmp.name
        stubs = {}
        for name, src in (('export', EXPORT), ('build', BUILD), ('judge', JUDGE)):
            stubs[name] = self.write(f'stubs/{name}.py', textwrap.dedent(src))
        self.write('symbols.toml', SYMBOLS)
        self.write('inventory.toml', INVENTORY)
        self.write('program', 'the program, version 1')
        # The translator's sources, as tool_version hashes them, in a game folder of their own.
        self.write('game/tools/pcode_rust.py', '# translator v1\n')
        self.answers = json.loads(json.dumps(ANSWERS))
        self.save_answers()
        py = shlex.quote(sys.executable)
        env = {'FD_TRIAL_EXPORT': f"{py} {shlex.quote(stubs['export'])} {shlex.quote(os.path.join(FIXTURES, 'shared_call.json'))}",
               'FD_TRIAL_BUILD': f"{py} {shlex.quote(stubs['build'])}",
               'FD_TRIAL_JUDGE': f"{py} {shlex.quote(stubs['judge'])}",
               'FAKE_EXPORTS': os.path.join(d, 'exports'), 'FAKE_BUILT': os.path.join(d, 'built'),
               'FAKE_CALLS': os.path.join(d, 'calls'), 'FAKE_ANSWERS': os.path.join(d, 'answers.json')}
        patcher = mock.patch.dict(os.environ, env)
        patcher.start()
        self.addCleanup(patcher.stop)
        os.environ.pop('FAKE_EXPORT_FAIL', None)
        os.environ.pop('FAKE_BUILD_ERROR', None)
        self.paths = t.Paths(translator=os.path.join(d, 'translator'), out=os.path.join(d, 'translate'),
                             program=os.path.join(d, 'program'), inventory=os.path.join(d, 'inventory.toml'),
                             symbols=os.path.join(d, 'symbols.toml'), plans=os.path.join(d, 'plans'),
                             game=os.path.join(d, 'game'), root=os.path.join(d, 'elsewhere'))

    def write(self, name, text):
        p = os.path.join(self.d, name)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, 'w') as f:
            f.write(text)
        return p

    def save_answers(self):
        self.write('answers.json', json.dumps(self.answers))

    def read(self, path):
        with open(path) as f:
            return f.read()

    def run_cmd(self, *argv):
        out, err = [], []
        with mock.patch('builtins.print', lambda *a, file=None, **k: (err if file is sys.stderr else out).append(' '.join(map(str, a)))):
            code = t.main(list(argv), paths=self.paths)
        return code, [json.loads(line) for line in out], '\n'.join(err)

    def exports(self):
        try:
            with open(os.path.join(self.d, 'exports')) as f:
                return len(f.read())
        except FileNotFoundError:
            return 0

    def calls(self):
        try:
            with open(os.path.join(self.d, 'calls')) as f:
                return [json.loads(line) for line in f]
        except FileNotFoundError:
            return []

    # --- results ---

    def test_pass_gives_the_line_and_the_routine_with_its_callees(self):
        code, lines, err = self.run_cmd('h0+$0')
        self.assertEqual(code, 0, err)
        self.assertEqual(lines, [{'routine': 'h0+$0', 'name': 'first_routine', 'result': 'pass', 'first': '',
                                  'states': 2000, 'rust': lines[0]['rust']}])
        rust = lines[0]['rust']
        self.assertEqual(rust, os.path.join(self.paths.out, 'h0-0', 'translated.rs'))
        text = self.read(rust)
        # Routine first, then its callees depth first in the order called; each once.
        order = [text.index(f'pub fn h0_{x}(') for x in ('0', '10', '30', '20')]
        self.assertEqual(order, sorted(order))
        # Each routine is its seam entry and its `NAME_from`, the body (translator T28, T35): 4 routines, 8 fns.
        for x in ('0', '10', '30', '20'):
            self.assertEqual((text.count(f'pub fn h0_{x}('), text.count(f'pub fn h0_{x}_from(')), (1, 1), x)
        self.assertEqual(text.count('pub fn '), 8)
        self.assertNotIn('super::p', text)
        self.assertNotIn('#[allow', text)
        self.assertTrue(text.startswith('// h0+$0 `first_routine`'))
        # Built optimised with the folder tree; the routine's own generator, surveyed then judged.
        self.assertEqual(self.read(os.path.join(self.d, 'built')), self.paths.tree)
        self.assertEqual([c[-1] for c in self.calls()], ['--generators', '--survey', '--translated'])
        self.assertEqual(self.calls()[1][:5], ['h0+$0', '--gen', 'first_routine', '--cases', '2000'])

    def test_a_callee_alone_holds_only_itself_and_its_callees(self):
        code, lines, err = self.run_cmd('h0+$20')
        self.assertEqual(code, 0, err)
        text = self.read(lines[0]['rust'])
        self.assertIn('pub fn h0_20(', text)
        self.assertIn('pub fn h0_30(', text)
        self.assertNotIn('pub fn h0_0(', text)
        self.assertEqual(lines[0]['name'], 'h0_20')

    def test_fail_names_the_first_difference(self):
        self.answers['translated']['h0+$0'] = {'pass': False, 'passed': 1990, 'failures': 10,
                                               'first_failure': 'state 4: d0 0x1 against 0x2'}
        self.save_answers()
        code, lines, _ = self.run_cmd('h0+$0', '--cases', '2000')
        self.assertEqual(code, 1)
        self.assertEqual((lines[0]['result'], lines[0]['first'], lines[0]['states']),
                         ('fail', 'state 4: d0 0x1 against 0x2', 2000))

    def test_a_stop_is_its_own_result_naming_the_stop(self):
        self.answers['translated']['h0+$0'] = {'pass': False, 'passed': 0, 'failures': 2000, 'untranslated': 2000,
                                               'first_failure': 'state 0: the port stopped',
                                               'first_stop': 'untranslated: h0+$44'}
        self.save_answers()
        code, lines, _ = self.run_cmd('h0+$0')
        self.assertEqual(code, 1)
        self.assertEqual((lines[0]['result'], lines[0]['first']), ('stop', 'untranslated: h0+$44'))

    def test_no_generator_registered_is_no_gen_and_judges_nothing(self):
        self.answers['generators'] = ['spare']
        self.save_answers()
        code, lines, _ = self.run_cmd('h0+$30')
        self.assertEqual(code, 1)
        self.assertEqual(lines[0]['result'], 'no-gen')
        self.assertTrue(os.path.exists(lines[0]['rust']))
        self.assertEqual([c[-1] for c in self.calls()], ['--generators'])

    def test_gen_names_the_generator_and_an_unregistered_one_is_no_gen(self):
        code, lines, err = self.run_cmd('h0+$30', '--gen', 'spare')
        self.assertEqual(code, 0, err)
        self.assertEqual(self.calls()[0][:3], ['h0+$30', '--gen', 'spare'])
        self.assertNotIn(['--generators'], self.calls())
        code, lines, _ = self.run_cmd('h0+$30', '--gen', 'nonesuch')
        self.assertEqual((code, lines[0]['result']), (1, 'no-gen'))

    def test_no_oracle_when_the_original_never_returns_or_blits_lines(self):
        self.answers['survey']['h0+$0'] = {'passed': 0, 'hangs': 2000}
        self.answers['survey']['h0+$20'] = {'passed': 1990, 'unmodelled_blits': 4}
        self.save_answers()
        self.assertEqual(self.run_cmd('h0+$0')[1][0]['result'], 'no-oracle')
        line = self.run_cmd('h0+$20')[1][0]
        self.assertEqual((line['result'], line['first']), ('no-oracle', 'a line or fill blit on 4 of 1990 states'))

    def test_a_symbol_is_looked_up_and_its_generator_found_by_name(self):
        code, lines, err = self.run_cmd('view_tile')
        self.assertEqual(code, 0, err)
        self.assertEqual((lines[0]['routine'], lines[0]['name']), ('h0+$10', 'view_tile'))
        self.assertEqual(self.calls()[1][:3], ['h0+$10', '--gen', 'view_tile'])
        self.assertTrue(lines[0]['rust'].endswith(os.path.join('h0-10', 'translated.rs')))

    def test_a_generator_named_by_address_serves_its_routine(self):
        self.answers['generators'] = ['h0_30']
        self.save_answers()
        code, lines, err = self.run_cmd('h0+$30')
        self.assertEqual(code, 0, err)
        self.assertEqual(self.calls()[1][:3], ['h0+$30', '--gen', 'h0_30'])

    # --- --task ---

    def test_task_judges_each_routine_of_the_inventory(self):
        self.answers['translated']['h0+$20'] = {'pass': False, 'failures': 1, 'passed': 1999, 'first_failure': 'x'}
        self.save_answers()
        code, lines, _ = self.run_cmd('--task', 'T08')
        self.assertEqual(code, 1)
        self.assertEqual([(x['routine'], x['result']) for x in lines], [('h0+$0', 'pass'), ('h0+$20', 'fail')])
        # gen_task counts too; one export and build serve every routine.
        code, lines, _ = self.run_cmd('--task', 'T56')
        self.assertEqual([x['routine'] for x in lines], ['h0+$20'])
        self.assertEqual(self.exports(), 1)

    def test_task_not_in_the_inventory_reads_its_doc_routines_section(self):
        self.write('plans/some-plan/tasks/T42-port-things.md', textwrap.dedent('''\
            # T42
            Mentions h0+$30 above the section, which does not count.
            ## Routines
            - `view_tile`, then h0+$20, then `view_tile` again and `not_a_symbol`.
            ## Tests
            - h0+$0
            '''))
        code, lines, err = self.run_cmd('--task', 'T42')
        self.assertEqual(code, 0, err)
        self.assertEqual([x['routine'] for x in lines], ['h0+$10', 'h0+$20'])

    def test_task_in_several_plans_needs_the_doc_named(self):
        for plan in ('a', 'b'):
            self.write(f'plans/{plan}/tasks/T42-x.md', '## Routines\n- h0+$20\n')
        with mock.patch.object(t, 'current_plan', return_value=None):
            code, _, err = self.run_cmd('--task', 'T42')
        self.assertEqual(code, 2)
        self.assertIn('plans/a/tasks/T42-x.md', err)
        with mock.patch.object(t, 'current_plan', return_value='b'):
            self.assertEqual(self.run_cmd('--task', 'T42')[0], 0)
        code, lines, _ = self.run_cmd('--task', os.path.join(self.d, 'plans/a/tasks/T42-x.md'))
        self.assertEqual((code, lines[0]['routine']), (0, 'h0+$20'))

    def test_task_with_no_routine_is_usage(self):
        code, _, err = self.run_cmd('--task', 'T99')
        self.assertEqual(code, 2)
        self.assertIn('T99', err)

    # --- --build and the cache ---

    def test_build_writes_the_tree_and_judges_nothing(self):
        code, lines, err = self.run_cmd('--build')
        self.assertEqual(code, 0, err)
        self.assertEqual(lines[0]['functions'], 4)
        self.assertTrue(os.path.exists(os.path.join(self.paths.tree, 'mod.rs')))
        self.assertEqual(self.calls(), [])
        self.assertFalse(os.path.exists(os.path.join(self.d, 'built')))

    def test_the_tree_is_cached_on_the_program_and_the_tool(self):
        self.assertEqual(self.run_cmd('--build')[0], 0)
        self.assertEqual(self.run_cmd('h0+$0')[0], 0)
        self.assertEqual(self.exports(), 1)
        self.write('program', 'the program, version 2')
        self.assertEqual(self.run_cmd('--build')[0], 0)
        self.assertEqual(self.exports(), 2)
        self.write('game/tools/pcode_rust.py', '# translator v2\n')
        self.assertEqual(self.run_cmd('--build')[0], 0)
        self.assertEqual(self.exports(), 3)
        self.assertEqual(self.run_cmd('--build', '--fresh')[0], 0)
        self.assertEqual(self.exports(), 4)
        # A tree gone from disk is made again, whatever the stamp says.
        shutil.rmtree(self.paths.tree)
        self.assertEqual(self.run_cmd('--build')[0], 0)
        self.assertEqual(self.exports(), 5)

    def test_a_failed_export_is_named_and_leaves_no_stamp(self):
        os.environ['FAKE_EXPORT_FAIL'] = '1'
        code, lines, err = self.run_cmd('h0+$0')
        self.assertEqual((code, lines), (1, []))
        self.assertIn('export:', err)
        self.assertIn('no JDK', err)
        self.assertFalse(os.path.exists(self.paths.stamp))

    def test_a_compile_error_in_the_folder_tree_names_its_routine(self):
        os.environ['FAKE_BUILD_ERROR'] = 'h0_20'
        code, lines, err = self.run_cmd('h0+$0')
        self.assertEqual((code, lines), (1, []))
        self.assertIn('does not compile, in h0+$20:', err)

    # --- usage ---

    def test_usage(self):
        for argv in ((), ('h0+$0', '--task', 'T08'), ('h0+$0', '--build'), ('--build', '--gen', 'x'),
                     ('--task', 'T08', '--gen', 'x'), ('h0+$0', '--cases', '0'), ('nonesuch',), ('h9+$0',),
                     ('--task', 'eight')):
            code, lines, err = self.run_cmd(*argv)
            self.assertEqual((code, lines), (2, []), argv)
            self.assertTrue(err, argv)
        self.assertEqual(self.calls(), [])
        self.assertEqual(self.exports(), 0)

    def test_no_program_is_usage_naming_the_extract(self):
        os.remove(self.paths.program)
        code, _, err = self.run_cmd('h0+$0')
        self.assertEqual(code, 2)
        self.assertIn('fd-tools -- extract', err)

    def test_a_routine_outside_the_tree_is_a_stop(self):
        code, lines, _ = self.run_cmd('h0+$4')
        self.assertEqual(code, 1)
        self.assertEqual((lines[0]['result'], lines[0]['rust']), ('stop', None))


if __name__ == '__main__':
    unittest.main()
