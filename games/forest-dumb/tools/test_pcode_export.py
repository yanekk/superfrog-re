"""pcode_export.py without Ghidra: the hunk walk, the missing-input exits, ghidra_headless's command.

The program is hand-written 68000 for the test, never taken from the game. test_pcode_export_ghidra.py
exports it through Ghidra (full set).
"""
import io, json, os, struct, subprocess, sys, tempfile, unittest
from contextlib import redirect_stderr
from unittest import mock

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import pcode_export as P  # noqa: E402
import ghidra_headless as G  # noqa: E402

# hunk 0, CODE
CODE = bytes.fromhex(
    '41F900000010'  # $00 lea h1+$10,a0        absolute, relocated to hunk 1
    '61000010'      # $06 bsr.w $18
    '7003'          # $0a moveq #3,d0
    '5240'          # $0c addq.w #1,d0
    '51C8FFFC'      # $0e dbf d0,$0c
    '2250'          # $12 movea.l (a0),a1     the table entry
    '4E91'          # $14 jsr (a1)            through the table
    '4E75'          # $16 rts
    '7201'          # $18 moveq #1,d1
    '4E75'          # $1a rts
    '45FA0006'      # $1c lea (6,pc),a2       = h0+$24, not relocated
    '4ED3'          # $20 jmp (a3)            nothing says where
    '4E71'          # $22 nop
    '60000006'      # $24 bra.w $2c           two routines share one tail: Ghidra makes $2c a function
    '60000002'      # $28 bra.w $2c           and calls these two branches tail calls
    '7402'          # $2c moveq #2,d2
    '4E75'          # $2e rts
    '203C0021F004'  # $30 move.l #$21f004,d0  a number that happens to fall in Ghidra's hunk 0: no reloc,
    '4E75')         # $36 rts                 no PC operand, so it must stay a number (DESIGN §7)
# hunk 1, DATA: a pointer to h0+$18 at $10, relocated to hunk 0
DATA = bytes(16) + struct.pack('>I', 0x18) + bytes(12)
ENTRIES = [(0, 0x0, 10), (0, 0x1c, 2), (0, 0x24, 3), (0, 0x28, 3), (0, 0x30, 2)]


# The whole-program export's made-up program (T03): an entry, a vector install, a bounded jump table
# through (d8,PC,Xn), a second code hunk and a computed jump nothing resolves.
WHOLE_H0 = bytes.fromhex(
    '21FC0000002C006C'  # $00 move.l #h0+$2c,$6c.w  installs the level 3 autovector, relocated
    '4EB900000000'      # $08 jsr h1+$0             relocated to hunk 1
    '0C400001'          # $0e cmpi.w #1,d0          d0 unknown: the table's bound
    '620E'              # $12 bhi.s $22
    'D040'              # $14 add.w d0,d0
    '303B0006'          # $16 move.w ($1e,pc,d0.w),d0
    '4EFB0002'          # $1a jmp ($1e,pc,d0.w)
    '0006000A'          # $1e the table: $24, $28 (data, never exported)
    '4E75'              # $22 rts
    '74024E75'          # $24 moveq #2,d2; rts       reached only through the table
    '74034E75'          # $28 moveq #3,d2; rts
    '4E73'              # $2c rte                    reached only through the vector
    '4E71')             # $2e padding, never reached
WHOLE_H1 = bytes.fromhex(
    '7605'              # $00 moveq #5,d3
    '6702'              # $02 beq.s $06
    '4ED0'              # $04 jmp (a0)               nothing says where
    '4E75')             # $06 rts


def longs(*xs):
    return b''.join(struct.pack('>I', x) for x in xs)


def two_hunk_program():
    return (longs(0x3F3, 0, 2, 0, 1, len(CODE) // 4, len(DATA) // 4)
            + longs(0x3E9, len(CODE) // 4) + CODE + longs(0x3EC, 1, 1, 2, 0, 0x3F2)
            + longs(0x3EA, len(DATA) // 4) + DATA + longs(0x3EC, 1, 0, 0x10, 0, 0x3F2))


def whole_program():
    return (longs(0x3F3, 0, 2, 0, 1, len(WHOLE_H0) // 4, len(WHOLE_H1) // 4)
            + longs(0x3E9, len(WHOLE_H0) // 4) + WHOLE_H0 + longs(0x3EC, 1, 0, 2, 1, 1, 0x0a, 0, 0x3F2)
            + longs(0x3E9, len(WHOLE_H1) // 4) + WHOLE_H1 + longs(0x3F2))


class HunkWalk(unittest.TestCase):
    def test_sizes_kinds_and_every_reloc32_with_the_long_it_relocates(self):
        h = P.read_hunks(two_hunk_program())
        self.assertEqual(h.sizes, [len(CODE), len(DATA)])
        self.assertEqual(h.kinds, ['CODE', 'DATA'])
        self.assertEqual(h.relocs, [(0, 2, 1, 0x10), (1, 0x10, 0, 0x18)])

    def test_the_whole_program_two_code_hunks_and_its_two_relocs(self):
        h = P.read_hunks(whole_program())
        self.assertEqual(h.kinds, ['CODE', 'CODE'])
        self.assertEqual(h.relocs, [(0, 2, 0, 0x2c), (0, 0xa, 1, 0)])

    def test_bss_and_a_declared_but_absent_hunk(self):
        data = longs(0x3F3, 0, 3, 0, 2, 1, 4, 2) + longs(0x3E9, 1, 0x4E754E75, 0x3F2) + longs(0x3EB, 4, 0x3F2)
        h = P.read_hunks(data)
        self.assertEqual(h.kinds, ['CODE', 'BSS', 'ABSENT'])
        self.assertEqual(h.sizes, [4, 16, 8])

    def test_not_a_hunk_file_is_refused(self):
        with self.assertRaises(ValueError):
            P.read_hunks(b'\0' * 16)


class MissingInputs(unittest.TestCase):
    def run_main(self, env, *args):
        with tempfile.TemporaryDirectory() as tmp:
            exe = os.path.join(tmp, 'p.exe')
            with open(exe, 'wb') as f:
                f.write(two_hunk_program())
            argv = ['--exe', exe, '--out', os.path.join(tmp, 'out'), *args]
            err = io.StringIO()
            with mock.patch.dict(os.environ, env), mock.patch.object(subprocess, 'run') as run, \
                    redirect_stderr(err):
                code = P.main(argv)
            return code, err.getvalue(), run

    def test_missing_ghidra_exits_2_naming_the_path_without_starting_java(self):
        code, err, run = self.run_main({'GHIDRA_HOME': '/nowhere/ghidra'})
        self.assertEqual(code, 2)
        self.assertIn('/nowhere/ghidra/support/analyzeHeadless', err)
        run.assert_not_called()

    def test_missing_program_exits_2_naming_it(self):
        code, err, run = self.run_main({}, '--exe', '/nowhere/FOREST')
        self.assertEqual(code, 2)
        self.assertIn('/nowhere/FOREST', err)
        run.assert_not_called()

    def test_missing_symbols_exits_2_naming_them(self):
        code, err, run = self.run_main({}, '--symbols', '/nowhere/symbols.toml')
        self.assertEqual(code, 2)
        self.assertIn('/nowhere/symbols.toml', err)
        run.assert_not_called()

    def test_missing_entries_exits_2_naming_them(self):
        code, err, run = self.run_main({}, '--entries', '/nowhere/entries.toml')
        self.assertEqual(code, 2)
        self.assertIn('/nowhere/entries.toml', err)
        run.assert_not_called()

    def test_missing_routines_exits_2_naming_them(self):
        code, err, run = self.run_main({}, '--routines', '/nowhere/translate.toml')
        self.assertEqual(code, 2)
        self.assertIn('/nowhere/translate.toml', err)
        run.assert_not_called()

    def test_a_malformed_entries_file_exits_2_naming_it_and_the_row(self):
        for row, says in (('at = "0x4"\nsource = "trace"', "'0x4'"), ('at = "h0+$4"\nsource = "guess"', 'guess'),
                          ('source = "trace"', 'None')):
            with tempfile.TemporaryDirectory() as tmp, self.subTest(row=row):
                path = os.path.join(tmp, 'entries.toml')
                with open(path, 'w') as f:
                    f.write(f'[[entry]]\n{row}\n')
                code, err, run = self.run_main({}, '--entries', path)
                self.assertEqual(code, 2)
                self.assertIn(path, err)
                self.assertIn(says, err)
                run.assert_not_called()

    def test_from_the_command_line_under_python_I(self):
        r = subprocess.run([sys.executable, '-I', os.path.join(HERE, 'pcode_export.py'), '--exe', '/nowhere/X'],
                           capture_output=True, text=True, timeout=30)
        self.assertEqual(r.returncode, 2, r.stderr)
        self.assertIn('/nowhere/X', r.stderr)


class HeadlessCommand(unittest.TestCase):
    """ghidra_headless.run_import's command: the old call form unchanged, post-scripts appended."""

    def command(self, **kw):
        with tempfile.TemporaryDirectory() as tmp:
            out = os.path.join(tmp, 'out')
            seen = []

            def fake(cmd, **_):
                seen.append(cmd)
                open(os.path.join(out, 'p.c'), 'w').close()
                return subprocess.CompletedProcess(cmd, 0, 'INFO  X.java> hello\n', '')

            with mock.patch.dict(os.environ, {'GHIDRA_HOME': '/g', 'JAVA_HOME': '/j'}), \
                    mock.patch.object(subprocess, 'run', fake), redirect_stderr(io.StringIO()), \
                    mock.patch('builtins.print'):
                G.run_import('p', CODE, [], out, 'proj', raw=True, **kw)
            tsv = seen[0][seen[0].index('ApplySymbols.java') + 1]
            return [c.replace(tsv, '<tsv>').replace(out, '<out>') for c in seen[0]]

    def test_the_old_call_form_is_unchanged(self):
        self.assertEqual(self.command(), [
            '/g/support/analyzeHeadless', '<out>/ghidra', 'proj', '-import', '<out>/bin/p.exe', '-overwrite',
            '-loader', 'BinaryLoader', '-processor', '68000:BE:32:default',
            '-scriptPath', os.path.join(G.HERE, 'ghidra'),
            '-postScript', 'ApplySymbols.java', '<tsv>', '<out>/p', 'p'])

    def test_extra_script_paths_and_post_scripts_follow_apply_symbols(self):
        cmd = self.command(script_paths=['/s'], post_scripts=[('X.java', ['a', 'b'])])
        self.assertEqual(cmd[cmd.index('-scriptPath') + 1],
                         os.path.join(G.HERE, 'ghidra') + ';/s')
        self.assertEqual(cmd[-9:], ['-postScript', 'ApplySymbols.java', '<tsv>', '<out>/p', 'p',
                                    '-postScript', 'X.java', 'a', 'b'])


class WholeSeeds(unittest.TestCase):
    """The whole program's seeds: the entry, every func symbol, re/entries.toml, one source an address."""

    def seeds(self, toml, symbols=()):
        with tempfile.TemporaryDirectory() as tmp:
            path = os.path.join(tmp, 'entries.toml')
            with open(path, 'w') as f:
                f.write(toml)
            return P.whole_seeds(list(symbols), path)

    def test_program_entry_then_symbol_then_vector_then_trace(self):
        syms = [P.Sym(0, 0x30, 'f', 'func', 0, ''), P.Sym(0, 0x40, 'v', 'var', 0, ''),
                P.Sym(4, 0x10, 'player', 'func', 0, ''), P.Sym(0, 0, 'start', 'func', 0, '')]
        toml = ''.join(f'[[entry]]\nat = "{a}"\nsource = "{s}"\n\n' for a, s in [
            ('h0+$0', 'symbol'), ('h0+$30', 'trace'), ('h0+$50', 'trace'), ('h0+$50', 'vector'),
            ('h4+$10', 'symbol'), ('h2+$8', 'trace')])
        self.assertEqual(self.seeds(toml, syms), [
            (0, 0, 'program-entry'), (0, 0x30, 'symbol'), (0, 0x50, 'vector'), (2, 8, 'trace'),
            (4, 0x10, 'symbol')])

    def test_the_program_entry_is_seeded_with_no_entries_file_rows(self):
        self.assertEqual(self.seeds(''), [(0, 0, 'program-entry')])

    def test_an_unknown_source_is_refused_naming_the_address(self):
        with self.assertRaisesRegex(P.Usage, r'h0\+\$4.*guess'):
            self.seeds('[[entry]]\nat = "h0+$4"\nsource = "guess"\n')

    def test_the_committed_entries_file_seeds_every_func_symbol(self):
        syms = P.load_symbols(P.SYMBOLS)
        seeds = P.whole_seeds(syms, P.ENTRIES)
        at = {(h, off) for h, off, _ in seeds}
        self.assertTrue({(s.hunk, s.offset) for s in syms if s.kind == 'func'} <= at)
        self.assertEqual(seeds[0], (0, 0, 'program-entry'))
        self.assertTrue({h for h, _, _ in seeds} >= {0, 4})


class Summary(unittest.TestCase):
    def test_the_summary_counts_entries_by_source_and_instructions_by_hunk(self):
        doc = {'hunks': [{}, {}, {}],
               'entries': [{'at': 'h0+$0', 'source': 'program-entry'}, {'at': 'h0+$8', 'source': 'symbol'},
                           {'at': 'h2+$0', 'source': 'symbol'}, {'at': 'h0+$20', 'source': 'vector'}],
               'insns': {'h0+$0': {'hunk': 0}, 'h0+$8': {'hunk': 0}, 'h2+$0': {'hunk': 2}},
               'check': {'instructions': 3, 'functions': 2, 'unresolved': 1, 'unresolved_computed': ['h2+$0'],
                         'relocs': 0, 'relocs_matched': 0, 'unmatched': [], 'mismatched': [],
                         'unexplained': [], 'undecoded': []}}
        s = P.summary(doc)
        self.assertEqual((s['instructions'], s['entries'], s['unresolved_computed'], s['hunks']), (3, 4, 1, [2, 0, 1]))
        self.assertEqual(s['by_source'], {'program-entry': 1, 'symbol': 2, 'vector': 1})


class Modes(unittest.TestCase):
    def test_whole_and_routines_are_exclusive(self):
        with redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as e:
            P.main(['--whole', '--routines', P.TRANSLATE])
        self.assertEqual(e.exception.code, 2)



class Reach(unittest.TestCase):
    def test_reach_follows_fallthrough_branches_and_calls(self):
        def i(kind, targets=(), n=2):
            return {'len': n, 'flow': {'kind': kind, 'targets': list(targets)}}
        doc = {'entries': ['h0+$0'], 'insns': {
            'h0+$0': i('call', ['h0+$8']), 'h0+$2': i('cbranch', ['h0+$6']), 'h0+$4': i('return'),
            'h0+$6': i('branch', ['h0+$4']), 'h0+$8': i('return'), 'h0+$a': i('return')}}
        self.assertEqual(P.reach_counts(doc), {'h0+$0': 5})
        doc['entries'] = [{'at': 'h0+$0', 'source': 'program-entry'}]
        self.assertEqual(P.reach_counts(doc), {'h0+$0': 5})


if __name__ == '__main__':
    unittest.main()
