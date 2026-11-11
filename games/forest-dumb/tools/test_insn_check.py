"""The instruction check's Python half (translator T06): `pcode_fix` loads and still corrects what the
trial corrected, `pcode_rust.py --insn` writes one function per opcode word, and `insn_check.py` runs
its steps with a stub build and judge, on the made-up export the Rust rig test uses
(`engine/crates/fd-conform/tests/fixtures/insn.json`). The rig itself, on Moira, is
`fd-conform/tests/insn_check.rs`. No disks, no Ghidra, no cargo."""
import json, os, re, subprocess, sys, tempfile, textwrap, unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import insn_check as ic  # noqa: E402
import pcode_fix  # noqa: E402
import pcode_rust as pr  # noqa: E402

GAME = os.path.dirname(HERE)
FIXTURE = os.path.join(GAME, 'engine', 'crates', 'fd-conform', 'tests', 'fixtures', 'insn.json')
FIXTURE_RS = FIXTURE[:-5] + '.rs'
BCD = os.path.join(GAME, 'engine', 'crates', 'pcode-rt', 'tests', 'fixtures', 'bcd.json')


def load(path):
    with open(path, encoding='utf-8') as f:
        return json.load(f)


def dump(obj, path):
    with open(path, 'w', encoding='utf-8') as f:
        json.dump(obj, f)


def read(path):
    with open(path, encoding='utf-8') as f:
        return f.read()


def rust(argv):
    return subprocess.run([sys.executable, '-I', os.path.join(HERE, 'pcode_rust.py')] + argv,
                          capture_output=True, text=True)


class Fixes(unittest.TestCase):
    def test_every_family_module_loads(self):
        self.assertEqual([m.__name__.split('.')[-1] for m in pcode_fix.MODULES],
                         ['bcd', 'compare', 'arith', 'shift_bit', 'move_control', 'cycles'])
        for m in pcode_fix.MODULES:
            self.assertTrue(m.__doc__, m.__name__)
        self.assertEqual(pcode_fix.cycles({'mnem': 'nop', 'ops': []}), [])

    def test_the_trials_two_corrections_still_apply(self):
        doc = pr.load_export(BCD)
        abcd = doc['insns'][(0, 4)]
        lines, family = pcode_fix.emit(pcode_fix.correct(abcd))
        self.assertEqual(family, 'bcd')
        self.assertIn('pcode_rt::bcd::bcd_add', lines[0])
        cmpi = doc['insns'][(0, 0)]
        fixed = pcode_fix.correct(cmpi)
        x_copy = {'op': 'COPY', 'out': ['register', 67, 1], 'in': [['register', 71, 1]]}
        self.assertIn(x_copy, cmpi['ops'])
        self.assertNotIn(x_copy, fixed['ops'])
        self.assertEqual(pcode_fix.emit(fixed), (None, None))
        # Through the translator: the trial's routines use the same path.
        text = '\n'.join(pr.Emit({}, {}).instruction((0, 4), abcd)[0])
        self.assertIn('(corrected: pcode_fix.bcd)', text)

    def test_the_fix_modules_name_no_game_and_no_address(self):
        d = os.path.join(HERE, 'pcode_fix')
        for name in os.listdir(d):
            if name.endswith('.py'):
                with open(os.path.join(d, name)) as f:
                    src = f.read().lower()
                for word in ('forest', 'dumb', 'superfrog'):
                    self.assertNotIn(word, src, name)
                self.assertEqual(re.findall(r'h\d+\+\$[0-9a-f]+', src), [], name)


class InsnMode(unittest.TestCase):
    def test_the_committed_fixture_is_current(self):
        with tempfile.TemporaryDirectory() as d:
            out = os.path.join(d, 'insn.rs')
            r = rust([FIXTURE, '--insn', '--out', out])
            self.assertEqual(r.returncode, 0, r.stderr)
            with open(out) as a, open(FIXTURE_RS) as b:
                self.assertEqual(a.read(), b.read(), 'fixtures/insn.rs is stale: regenerate it (tests/insn_check.rs)')

    def render(self, wanted=None):
        return pr.render_insn(pr.load_export(FIXTURE), 'insn.json', wanted)

    def test_one_function_per_opcode_word_at_its_first_site(self):
        doc = load(FIXTURE)
        doc['insns']['h0+$14'] = dict(doc['insns']['h0+$0'])   # a second moveq #-1,d0
        with tempfile.TemporaryDirectory() as d:
            p = os.path.join(d, 'x.json')
            dump(dict(doc, hunks=[{'index': 0, 'size': 0x20}, {'index': 1, 'size': 0x10}]), p)
            text, stats = pr.render_insn(pr.load_export(p), 'x.json')
        self.assertEqual(stats, {'sites': 7, 'unsupported': 0})
        self.assertIn('("70ff", "h0+$0", 0, 0x0, 2, &[], i_h0_0),', text)
        self.assertNotIn('i_h0_14', text)

    def test_sites_choose_the_instructions(self):
        text, stats = self.render([(0, 0xe), (0, 2)])
        self.assertEqual(stats['sites'], 2)
        self.assertIn('i_h0_e', text)
        self.assertNotIn('fn i_h0_0(', text)

    def test_control_leaving_the_instruction_writes_pc_and_returns(self):
        text, _ = self.render()
        bne = text[text.index('fn i_h0_a('):]
        bne = bne[:bne.index('\n}\n')]
        self.assertIn('r.set(PC, 4, img.h(0, 0x0) as u64); return Ok(());', bne)
        self.assertIn('r.set(PC, 4, img.h(0, 0xc) as u64);', bne)   # not taken: the next instruction
        self.assertNotIn("continue 'b", text)

    def test_an_absolute_operand_is_a_seed_and_a_branch_target_is_not(self):
        doc = pr.load_export(FIXTURE)
        self.assertEqual(pr.seeds(doc['insns'][(0, 0xe)]), [(1, 4)])
        self.assertEqual(pr.seeds(doc['insns'][(0, 0xa)]), [])

    def test_bad_uses_exit_2_naming_them(self):
        with tempfile.TemporaryDirectory() as d:
            r = rust([FIXTURE, '--insn', '--out', os.path.join(d, 'tree')])
            self.assertEqual(r.returncode, 2)
            self.assertIn('--out must end in .rs', r.stderr)
            r = rust([FIXTURE, '--insn', '--sites', 'h0+$100', '--out', os.path.join(d, 'x.rs')])
            self.assertEqual(r.returncode, 2)
            self.assertIn('h0+$100 is no instruction', r.stderr)
            r = rust([FIXTURE, '--sites', 'h0+$0', '--out', os.path.join(d, 'x.rs')])
            self.assertEqual(r.returncode, 2)
            doc = load(FIXTURE)
            del doc['insns']['h0+$2']['bytes']
            p = os.path.join(d, 'old.json')
            dump(doc, p)
            r = rust([p, '--insn', '--out', os.path.join(d, 'x.rs')])
            self.assertEqual(r.returncode, 2)
            self.assertIn('h0+$2: no "bytes"', r.stderr)


# A judge stub: reads the sites of the module FD_INSN named (the build stub copied it to STUB_MODULE)
# and prints a line for each of its shard; the encoding in STUB_FAIL fails every state in ccr C.
JUDGE_STUB = textwrap.dedent('''
    import json, os, re, sys
    a = sys.argv[1:]
    k, n = map(int, a[a.index('--shard') + 1].split('/'))
    states = int(a[a.index('--states') + 1])
    cols = a[a.index('--columns') + 1].split(',')
    if os.environ.get('STUB_EXIT'):
        print('fd-judge: no disks', file=sys.stderr); sys.exit(2)
    with open(os.environ['STUB_MODULE']) as f:
        rows = re.findall(r'\\("([0-9a-f]{4})", "(h\\d\\+\\$[0-9a-f]+)"', f.read())
    for i, (enc, at) in enumerate(rows):
        if i % n != k:
            continue
        bad = enc == os.environ.get('STUB_FAIL')
        c = {x: (0 if x in cols else None) for x in ('regs', 'ccr', 'mem')}
        c['cycles'] = 'not-counted' if 'cycles' in cols else None
        c['stop'] = 0
        if bad:
            c['ccr'] = states
        print(json.dumps({'encoding': enc, 'at': at, 'states': states, 'pass': 0 if bad else states,
                          'fail': states if bad else 0, 'skipped': 0, 'columns': c,
                          'first': {'state': 0, 'column': 'ccr', 'what': 'C', 'moira': '0', 'translated': '1'} if bad else None,
                          'first_skip': ''}))
''')


class Check(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        d = self.tmp.name
        self.module_copy = os.path.join(d, 'module.rs')
        judge = os.path.join(d, 'judge.py')
        with open(judge, 'w') as f:
            f.write(JUDGE_STUB)
        build = os.path.join(d, 'build.py')
        with open(build, 'w') as f:
            f.write(f'import os, shutil\nshutil.copy(os.environ["FD_INSN"], {self.module_copy!r})\n')
        self.env = {'FD_INSN_BUILD': f'{sys.executable} {build}', 'FD_INSN_JUDGE': f'{sys.executable} {judge}',
                    'STUB_MODULE': self.module_copy}
        self.old_env = {k: os.environ.get(k) for k in list(self.env) + ['STUB_FAIL', 'STUB_EXIT']}
        os.environ.update(self.env)
        self.old_lock = ic.LOCK_DIR
        ic.LOCK_DIR = os.path.join(d, 'lock')
        self.out = os.path.join(d, 'out')

    def tearDown(self):
        for k, v in self.old_env.items():
            if v is None:
                os.environ.pop(k, None)
            else:
                os.environ[k] = v
        ic.LOCK_DIR = self.old_lock
        self.tmp.cleanup()

    def main(self, *argv):
        import io
        from contextlib import redirect_stderr, redirect_stdout
        out, err = io.StringIO(), io.StringIO()
        with redirect_stdout(out), redirect_stderr(err):
            code = ic.main(['--pcode', FIXTURE, '--out', self.out] + list(argv))
        return code, out.getvalue(), err.getvalue()

    def results(self):
        return {k['kind']: k for k in load(os.path.join(self.out, 'insn.json'))}

    def test_the_whole_census_passing(self):
        code, out, err = self.main('--states', '20')
        self.assertEqual(code, 0, err)
        s = json.loads(out)
        self.assertEqual((s['kinds'], s['sites'], s['failing_kinds'], s['states']), (7, 7, 0, 140))
        self.assertEqual(s['columns']['cycles'], 'not-counted')
        k = self.results()
        self.assertEqual(k['add.l Dn,Dn']['family'], 'arith')
        self.assertEqual(k['abcd Dn,Dn']['task'], 'T07')
        self.assertEqual(k['bne.b label']['cycles_task'], 'T12')
        self.assertEqual(k['moveq #imm,Dn']['cycles_task'], 'T11')
        md = read(os.path.join(self.out, 'insn.md'))
        self.assertIn('| arith | T08 | 1 | 0 |', md)

    def test_a_failing_encoding_tops_the_table_with_its_column(self):
        os.environ['STUB_FAIL'] = 'd081'
        code, out, err = self.main('--states', '10')
        self.assertEqual(code, 0, err)
        self.assertEqual(json.loads(out)['failing_kinds'], 1)
        add = self.results()['add.l Dn,Dn']
        self.assertEqual((add['fail'], add['columns']['ccr']), (10, 10))
        self.assertEqual(add['first']['at'], 'h0+$2')
        rows = [l for l in read(os.path.join(self.out, 'insn.md')).splitlines() if l.startswith('| `')]
        self.assertTrue(rows[0].startswith('| `add.l Dn,Dn` | arith |'), rows[0])
        self.assertIn('h0+$2 state 0: ccr C moira 0, translated 1', rows[0])

    def test_kinds_by_name_with_their_commas_by_mnemonic_and_by_family(self):
        code, out, err = self.main('--kinds', 'add.l Dn,Dn,moveq,bcd', '--columns', 'regs,ccr')
        self.assertEqual(code, 0, err)
        self.assertEqual(sorted(self.results()), ['abcd Dn,Dn', 'add.l Dn,Dn', 'moveq #imm,Dn'])
        self.assertEqual(self.results()['moveq #imm,Dn']['columns']['mem'], None)
        self.assertIn('fn i_h0_2(', read(self.module_copy))
        self.assertNotIn('fn i_h0_e(', read(self.module_copy))

    def test_usage_and_step_failures(self):
        code, _, err = self.main('--kinds', 'frobnicate')
        self.assertEqual(code, 2)
        self.assertIn('`frobnicate` is no census kind', err)
        code, _, err = self.main('--columns', 'regs,flags')
        self.assertEqual(code, 2)
        missing = os.path.join(self.tmp.name, 'none.json')
        code, _, err = self.main('--pcode', missing)
        self.assertEqual(code, 1)
        self.assertIn(f'census: {missing}', err)
        os.environ['STUB_EXIT'] = '1'
        code, _, err = self.main()
        self.assertEqual(code, 1)
        self.assertIn('judge: shard', err)
        self.assertIn('no disks', err)
        self.assertFalse(os.path.exists(os.path.join(self.out, 'insn.json')))

    def test_a_held_lock_is_refused_not_waited_for(self):
        import fcntl
        os.makedirs(ic.LOCK_DIR)
        with open(os.path.join(ic.LOCK_DIR, 'judge.lock'), 'w') as f:
            fcntl.flock(f, fcntl.LOCK_EX)
            code, _, err = self.main()
        self.assertEqual(code, 1)
        self.assertIn('is held by another judge run', err)


class Families(unittest.TestCase):
    def test_each_family_task_and_cycles_owner(self):
        self.assertEqual(ic.family_of('lsl'), ('shift_bit', 'T09'))
        self.assertEqual(ic.family_of('sne'), ('shift_bit', 'T09'))
        self.assertEqual(ic.family_of('dbf'), ('move_control', 'T10'))
        self.assertEqual(ic.family_of('ori', 'SR'), ('move_control', 'T10'))
        self.assertEqual(ic.family_of('ori', 'Dn'), ('arith', 'T08'))
        self.assertEqual(ic.family_of('cmpm'), ('compare', 'T07'))
        self.assertEqual(ic.family_of('nop'), ('other', 'T10'))
        self.assertEqual(ic.cycles_task('lsl', 'Dn'), 'T12')
        self.assertEqual(ic.cycles_task('lsl', '#imm'), 'T11')
        self.assertEqual(ic.cycles_task('mulu', 'Dn'), 'T12')


if __name__ == '__main__':
    unittest.main()
