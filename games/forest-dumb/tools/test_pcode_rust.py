"""pcode_rust.py: each P-code op's Rust, unsupported ops, hints, bad inputs, determinism, and the
committed fixture translations against what the translator writes now (translator-trial T03).

The fixtures are made-up programs (engine/crates/pcode-rt/tests/fixtures); `cargo test -p pcode-rt
--test translated` runs their committed translations. Regenerate them after a translator change with
`python3 tools/test_pcode_rust.py --regen` and commit the result."""
import json
import os
import re
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import pcode_rust as pr

TOOL = os.path.join(HERE, 'pcode_rust.py')
FIXTURES = os.path.join(HERE, '..', 'engine', 'crates', 'pcode-rt', 'tests', 'fixtures')
# (export, hints or None, committed translation)
TRANSLATIONS = [(n + '.json', None, n + '.rs') for n in (
    'code_write', 'dbf_loop', 'endless', 'flags_call', 'jump_table', 'nested_call', 'pushed_return',
    'relative', 'unsupported')] + [('jump_table.json', 'jump_table.hints.toml', 'jump_table_hinted.rs')]

D0 = ['register', 0, 4]
D1 = ['register', 4, 4]
D0W = ['register', 2, 2]
U = ['unique', 0x100, 4]
RTS = {'len': 2, 'mnem': 'rts', 'ops': [
    {'op': 'LOAD', 'out': ['register', 80, 4], 'in': [['const', 433, 8], ['register', 60, 4]]},
    {'op': 'INT_ADD', 'out': ['register', 60, 4], 'in': [['register', 60, 4], ['const', 4, 4]]},
    {'op': 'RETURN', 'in': [['register', 80, 4]]}]}


def export(insns, entries=('h0+$0',), sizes=(0x100,)):
    return {'hunks': [{'index': i, 'size': s} for i, s in enumerate(sizes)],
            'entries': list(entries), 'insns': insns}


def run(args):
    return subprocess.run([sys.executable, '-I', TOOL] + args, capture_output=True, text=True)


def translate_doc(doc, hints=None):
    """(Rust text, stats) for an export given as a dict."""
    with tempfile.TemporaryDirectory() as d:
        p = os.path.join(d, 'pcode.json')
        with open(p, 'w') as f:
            json.dump(doc, f)
        args = [p, '--out', os.path.join(d, 'tree.rs')]
        if hints is not None:
            h = os.path.join(d, 'hints.toml')
            with open(h, 'w') as f:
                f.write(hints)
            args += ['--hints', h]
        r = run(args)
        if r.returncode != 0:
            raise AssertionError(r.stderr)
        with open(os.path.join(d, 'tree.rs')) as f:
            return f.read(), json.loads(r.stdout)


def one_op(op):
    """The Rust lines one op in an instruction of its own becomes."""
    em = pr.Emit({}, {})
    ins = {'len': 2, 'mnem': '', 'ops': [op], 'targets': []}
    lines, stop = em.instruction((0, 0x10), ins)
    return '\n'.join(lines), stop


class OpMapping(unittest.TestCase):
    def test_each_op_maps_to_its_helper(self):
        cases = [
            ({'op': 'COPY', 'out': D0, 'in': [['const', 3, 4]]}, 'let v = 0x3u64;\n    r.set(0x0, 4, v);'),
            ({'op': 'COPY', 'out': D0, 'in': [['const', -1, 4]]}, None),
            ({'op': 'LOAD', 'out': D0, 'in': [['const', 433, 8], D1]}, 'load(bus, r.get(0x4, 4) as u32, 4)'),
            ({'op': 'STORE', 'in': [['const', 433, 8], D1, D0W]},
             'let v = r.get(0x2, 2);\n    store(bus, img, r.get(0x4, 4) as u32, 2, v)?;'),
            ({'op': 'INT_SEXT', 'out': D0, 'in': [D0W]}, 'int_sext(r.get(0x2, 2), 2, 4)'),
            ({'op': 'INT_ZEXT', 'out': D0, 'in': [D0W]}, 'int_zext(r.get(0x2, 2), 2)'),
            ({'op': 'SUBPIECE', 'out': D0W, 'in': [D0, ['const', 2, 4]]}, 'subpiece(r.get(0x0, 4), 4, 2, 2)'),
            ({'op': 'PIECE', 'out': D0, 'in': [D0W, ['register', 6, 2]]},
             'piece(r.get(0x2, 2), r.get(0x6, 2), 2, 4)'),
            ({'op': 'INT_LEFT', 'out': D0, 'in': [D0, ['const', 3, 4]]}, 'int_left(r.get(0x0, 4), 0x3u64, 4)'),
            ({'op': 'INT_SRIGHT', 'out': D0W, 'in': [D0W, ['register', 7, 1]]},
             'int_sright(r.get(0x2, 2), r.get(0x7, 1), 2)'),
            ({'op': 'BOOL_NEGATE', 'out': ['register', 69, 1], 'in': [['register', 69, 1]]},
             'bool_negate(r.get(0x45, 1))'),
            ({'op': 'BOOL_AND', 'out': U, 'in': [['register', 69, 1], ['register', 70, 1]]},
             'bool_and(r.get(0x45, 1), r.get(0x46, 1))'),
            ({'op': 'INT_2COMP', 'out': D0, 'in': [D0]}, 'int_2comp(r.get(0x0, 4), 4)'),
            ({'op': 'POPCOUNT', 'out': ['register', 69, 1], 'in': [D0]}, 'popcount(r.get(0x0, 4), 4)'),
            ({'op': 'COPY', 'out': ['ram', 'h1+$4', 4], 'in': [D0]},
             'store(bus, img, img.h(1, 0x4), 4, v)?;'),
            ({'op': 'COPY', 'out': ['ram', 0xdff096, 2], 'in': [D0W]}, 'store(bus, img, 0xdff096, 2, v)?;'),
            ({'op': 'COPY', 'out': D0, 'in': [['ram', 4, 4]]}, 'load(bus, 0x4, 4)'),
            ({'op': 'COPY', 'out': D0, 'in': [['const', 'h2+$1a4', 4]]}, '(img.h(2, 0x1a4) as u64)'),
            ({'op': 'COPY', 'out': ['unique', 0x80, 2], 'in': [D0W]}, 'u_80 = v & 0xffff;'),
            ({'op': 'CBRANCH', 'in': [['ram', 'h0+$20', 4], ['register', 69, 1]]},
             "if r.get(0x45, 1) != 0 { pc = (0, 0x20); continue 'b; }"),
            ({'op': 'BRANCH', 'in': [['ram', 0xfc0000, 4]]},
             'return Err(Stop::Untranslated("$fc0000".into()));'),
            ({'op': 'BRANCHIND', 'in': [D0]}, "Some(k) => { pc = k; continue 'b; }"),
            ({'op': 'RETURN', 'in': [['register', 80, 4]]}, 'r.set(PC, 4, v);\n    return Ok(());'),
            ({'op': 'CALL', 'in': [['ram', 'h0+$40', 4]]}, 'return Err(Stop::Untranslated("h0+$40".into()));'),
            ({'op': 'CALLIND', 'in': [D0]}, 'match img.hunk_of(a).and_then(by_addr)'),
        ]
        for name, helper in pr.BINARY.items():
            cases.append(({'op': name, 'out': D0, 'in': [D0, D1]}, f'{helper}(r.get(0x0, 4), r.get(0x4, 4), 4)'))
        for op, want in cases:
            with self.subTest(op=op['op']):
                text, stop = one_op(op)
                self.assertFalse(stop, text)
                if want:
                    self.assertIn(want, text)

    def test_a_negative_constant_is_its_bytes(self):
        text, _ = one_op({'op': 'COPY', 'out': D0W, 'in': [['const', -1, 2]]})
        self.assertIn('0xffffu64', text)

    def test_unknown_ops_and_varnodes_become_one_stop_each(self):
        for op, name in [
            ({'op': 'FLOAT_ADD', 'out': D0, 'in': [D0, D1]}, 'FLOAT_ADD'),
            ({'op': 'CALLOTHER', 'in': [['const', 2, 4]]}, 'CALLOTHER'),
            ({'op': 'CALLOTHER', 'name': 'reset', 'in': [['const', 5, 4]]}, 'CALLOTHER reset'),
            ({'op': 'COPY', 'out': ['register', 0x100, 10], 'in': [['register', 0x10a, 10]]}, '10-byte register'),
            ({'op': 'COPY', 'out': D0, 'in': [['stack', 4, 4]]}, 'stack space'),
            ({'op': 'MULTIEQUAL', 'out': D0, 'in': [D0, D1]}, 'MULTIEQUAL'),
        ]:
            with self.subTest(op=name):
                text, stop = one_op(op)
                self.assertTrue(stop)
                self.assertIn(f'Stop::Unsupported {{ op: "{name}".into(), at: "h0+$10".into() }}', text)

    def test_a_negative_constant_from_the_export_is_its_bytes(self):
        # Ghidra's varnode offsets are Java longs, so a 64-bit constant may arrive negative.
        text, stats = translate_doc(export({'h0+$0': {'len': 2, 'mnem': 'x', 'ops': [
            {'op': 'COPY', 'out': ['register', 0, 8], 'in': [['const', -1, 8]]}]}, 'h0+$2': RTS}))
        self.assertEqual(stats['unsupported'], 0)
        self.assertIn('0xffffffffffffffffu64', text)

    def test_a_branch_to_a_relocated_constant_is_one_stop(self):
        # A relocation landing on a P-code-relative branch makes no sense; that instruction stops.
        text, stop = one_op({'op': 'BRANCH', 'in': [['const', 'h0+$2', 4]]})
        self.assertTrue(stop)
        self.assertIn('BRANCH to a relocated constant', text)
        text, stop = one_op({'op': 'BRANCH', 'in': [['const', 0xffff, 2]]})
        self.assertTrue(stop)
        self.assertIn("first op", text)

    def test_translation_continues_past_an_unsupported_instruction(self):
        doc = export({
            'h0+$0': {'len': 2, 'mnem': 'made-up', 'ops': [{'op': 'FLOAT_SQRT', 'out': D0, 'in': [D0]}]},
            'h0+$2': {'len': 2, 'mnem': 'moveq #1,d1', 'ops': [{'op': 'COPY', 'out': D1, 'in': [['const', 1, 4]]}]},
            'h0+$4': RTS})
        text, stats = translate_doc(doc)
        self.assertEqual(stats['unsupported'], 1)
        self.assertEqual(text.count('Stop::Unsupported'), 1)
        self.assertIn('// h0+$2 moveq #1,d1', text)
        self.assertIn('r.set(0x4, 4, v);', text)


class Flow(unittest.TestCase):
    JMP = {'h0+$0': {'len': 2, 'mnem': 'jmp (a1)', 'flow': {'kind': 'computed', 'targets': []},
                     'ops': [{'op': 'BRANCHIND', 'in': [['register', 36, 4]]}]},
           'h0+$10': RTS, 'h0+$20': RTS}

    def test_without_hints_nothing_is_assumed_about_a_computed_jump(self):
        text, stats = translate_doc(export(self.JMP))
        self.assertEqual(stats, {'functions': 1, 'blocks': 1, 'unsupported': 0, 'computed_unresolved': 1})
        self.assertNotIn('(0, 0x10) =>', text)

    def test_a_targets_hint_makes_them_blocks(self):
        hints = '[[hint]]\nat = "h0+$0"\nkind = "targets"\ntargets = ["h0+$10", "h0+$20"]\nwhy = "t"\n'
        text, stats = translate_doc(export(self.JMP), hints)
        self.assertEqual(stats, {'functions': 1, 'blocks': 3, 'unsupported': 0, 'computed_unresolved': 0})
        self.assertIn('(0, 0x10) =>', text)
        self.assertIn('(0, 0x20) =>', text)

    def test_ghidras_own_resolved_targets_count_as_resolved(self):
        doc = export(dict(self.JMP, **{'h0+$0': dict(self.JMP['h0+$0'], flow={'kind': 'computed',
                                                                              'targets': ['h0+$10']})}))
        text, stats = translate_doc(doc)
        self.assertEqual(stats['computed_unresolved'], 0)
        self.assertIn('(0, 0x10) =>', text)

    def test_a_data_hint_removes_the_instructions_it_covers(self):
        doc = export({'h0+$0': {'len': 2, 'mnem': 'nop', 'ops': []},
                      'h0+$2': {'len': 4, 'mnem': 'made-up', 'ops': []}, 'h0+$6': RTS})
        hints = '[[hint]]\nat = "h0+$4"\nkind = "data"\nlen = 1\nwhy = "t"\n'
        text, _ = translate_doc(doc, hints)
        self.assertNotIn('// h0+$2', text)
        self.assertIn('(0, 0x0, 0x2),', text)
        self.assertNotIn('(0, 0x2, ', text)

    def test_a_computed_call_with_a_hint_adds_the_function(self):
        doc = export({'h0+$0': {'len': 2, 'mnem': 'jsr (a0)', 'flow': {'kind': 'computed', 'targets': []},
                                'ops': [{'op': 'CALLIND', 'in': [['register', 32, 4]]}]},
                      'h0+$2': RTS, 'h0+$30': RTS})
        _, stats = translate_doc(doc)
        self.assertEqual((stats['functions'], stats['computed_unresolved']), (1, 1))
        hints = '[[hint]]\nat = "h0+$0"\nkind = "targets"\ntargets = ["h0+$30"]\nwhy = "t"\n'
        text, stats = translate_doc(doc, hints)
        self.assertEqual((stats['functions'], stats['computed_unresolved']), (2, 0))
        self.assertIn('"h0+$30" => Some(h0_30),', text)

    def test_a_reloc_entry_makes_its_operand_hunk_relative(self):
        doc = export({'h0+$0': {'len': 6, 'mnem': 'lea', 'relocs': [{'in_op': 0, 'slot': 0, 'target': 'h1+$8'}],
                                'ops': [{'op': 'COPY', 'out': ['register', 32, 4],
                                         'in': [['const', 0x21f408, 4]]}]}, 'h0+$6': RTS},
                     sizes=(0x10, 0x10))
        text, _ = translate_doc(doc)
        self.assertIn('let v = (img.h(1, 0x8) as u64);', text)
        self.assertNotIn('21f408', text)

    def test_a_branch_into_the_middle_of_a_run_splits_its_block(self):
        doc = export({'h0+$0': {'len': 2, 'mnem': 'a', 'ops': []},
                      'h0+$2': {'len': 2, 'mnem': 'b', 'ops': []},
                      'h0+$4': {'len': 2, 'mnem': 'bne', 'ops': [
                          {'op': 'CBRANCH', 'in': [['ram', 'h0+$2', 4], ['register', 69, 1]]}]},
                      'h0+$6': RTS})
        text, stats = translate_doc(doc)
        self.assertEqual(stats['blocks'], 3)
        self.assertEqual(re.findall(r'^            (\(0, 0x[0-9a-f]+\)) =>', text, re.M),
                         ['(0, 0x0)', '(0, 0x2)', '(0, 0x6)'])


class Cli(unittest.TestCase):
    def test_bad_inputs_exit_2_naming_them(self):
        with tempfile.TemporaryDirectory() as d:
            out = os.path.join(d, 'tree.rs')
            missing = os.path.join(d, 'nowhere.json')
            r = run([missing, '--out', out])
            self.assertEqual(r.returncode, 2)
            self.assertIn(missing, r.stderr)
            bad = [('notjson.json', '{', 'not JSON'),
                   ('noinsns.json', '{"hunks": [], "entries": []}', 'no "insns"'),
                   ('badaddr.json', json.dumps(export({'0x1000': RTS})), "'0x1000' is not an address"),
                   ('badvn.json', json.dumps(export({'h0+$0': {'len': 2, 'ops': [
                       {'op': 'COPY', 'out': ['register', 0], 'in': []}]}})), 'is not a varnode'),
                   ('badhunk.json', json.dumps(export({'h3+$0': RTS})), 'hunk 3 is not in "hunks"'),
                   ('entries.json', json.dumps(dict(export({}), entries=5)), '"entries" must be a list'),
                   ('relocs.json', json.dumps(export({'h0+$0': dict(RTS, relocs=5)})), '"relocs" must be a list'),
                   ('flow.json', json.dumps(export({'h0+$0': dict(RTS, flow='x')})), '"flow" must be an object'),
                   ('targets.json', json.dumps(export({'h0+$0': dict(RTS, flow={'targets': 'h0+$2'})})),
                    '"targets" must be a list'),
                   ('arity.json', json.dumps(export({'h0+$0': {'len': 2, 'ops': [
                       {'op': 'INT_ADD', 'out': D0, 'in': [D0]}]}})), 'op 0 INT_ADD has too few inputs'),
                   ('noinput.json', json.dumps(export({'h0+$0': {'len': 2, 'ops': [{'op': 'BRANCH', 'in': []}]}})),
                    'op 0 BRANCH has too few inputs'),
                   ('negram.json', json.dumps(export({'h0+$0': {'len': 2, 'ops': [
                       {'op': 'COPY', 'out': D0, 'in': [['ram', -4, 4]]}]}})), 'has a bad offset')]
            for name, body, says in bad:
                p = os.path.join(d, name)
                with open(p, 'w') as f:
                    f.write(body)
                r = run([p, '--out', out])
                self.assertEqual(r.returncode, 2, name)
                self.assertIn(p, r.stderr)
                self.assertIn(says, r.stderr)
            good = os.path.join(d, 'good.json')
            with open(good, 'w') as f:
                json.dump(export({'h0+$0': RTS}), f)
            for hints, says in [('kind = 1\n[[', 'not TOML'),
                                ('[[hint]]\nat = "h0+$0"\nkind = "guess"\n', 'kind must be'),
                                ('[[hint]]\nat = "h0+$0"\nkind = "data"\n', 'positive "len"')]:
                h = os.path.join(d, 'h.toml')
                with open(h, 'w') as f:
                    f.write(hints)
                r = run([good, '--out', out, '--hints', h])
                self.assertEqual(r.returncode, 2, hints)
                self.assertIn(h, r.stderr)
                self.assertIn(says, r.stderr)
            self.assertFalse(os.path.exists(out), 'nothing written on a bad input')

    def test_two_runs_write_identical_files(self):
        src = os.path.join(FIXTURES, 'nested_call.json')
        with tempfile.TemporaryDirectory() as d:
            outs = []
            for i in range(2):
                out = os.path.join(d, f'{i}.rs')
                self.assertEqual(run([src, '--out', out]).returncode, 0)
                with open(out, 'rb') as f:
                    outs.append(f.read())
            self.assertEqual(outs[0], outs[1])

    def test_the_committed_fixture_translations_are_current(self):
        for src, hints, rs in TRANSLATIONS:
            with self.subTest(rs=rs), tempfile.TemporaryDirectory() as d:
                out = os.path.join(d, rs)
                args = [os.path.join(FIXTURES, src), '--out', out]
                if hints:
                    args += ['--hints', os.path.join(FIXTURES, hints)]
                r = run(args)
                self.assertEqual(r.returncode, 0, r.stderr)
                with open(out) as f, open(os.path.join(FIXTURES, rs)) as g:
                    self.assertEqual(f.read(), g.read(),
                                     f'{rs} is stale: run python3 tools/test_pcode_rust.py --regen')


class NoGame(unittest.TestCase):
    def test_the_translator_names_no_game_and_no_address(self):
        with open(TOOL) as f:
            src = f.read()
        for word in ('forest', 'dumb', 'superfrog', 'FOREST_DUMB'):
            self.assertNotIn(word, src.lower(), word)
        # No hex literal of four digits or more (an address); format masks are built from sizes.
        self.assertEqual(re.findall(r'0x[0-9a-fA-F]{4,}', src), [])
        self.assertEqual(re.findall(r"h\d+\+\$[0-9a-fA-F]+", src.replace('hN+$off', '')), [])


def regen():
    for src, hints, rs in TRANSLATIONS:
        args = [os.path.join(FIXTURES, src), '--out', os.path.join(FIXTURES, rs)]
        if hints:
            args += ['--hints', os.path.join(FIXTURES, hints)]
        r = run(args)
        sys.stdout.write(f'{rs}: {r.stdout or r.stderr}')


if __name__ == '__main__':
    if sys.argv[1:] == ['--regen']:
        regen()
    else:
        unittest.main()
