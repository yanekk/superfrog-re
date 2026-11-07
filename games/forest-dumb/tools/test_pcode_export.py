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
    '4E714E71')     # $24
# hunk 1, DATA: a pointer to h0+$18 at $10, relocated to hunk 0
DATA = bytes(16) + struct.pack('>I', 0x18) + bytes(12)
ENTRIES = [(0, 0x0, 10), (0, 0x1c, 2)]


def longs(*xs):
    return b''.join(struct.pack('>I', x) for x in xs)


def two_hunk_program():
    return (longs(0x3F3, 0, 2, 0, 1, len(CODE) // 4, len(DATA) // 4)
            + longs(0x3E9, len(CODE) // 4) + CODE + longs(0x3EC, 1, 1, 2, 0, 0x3F2)
            + longs(0x3EA, len(DATA) // 4) + DATA + longs(0x3EC, 1, 0, 0x10, 0, 0x3F2))


class HunkWalk(unittest.TestCase):
    def test_sizes_kinds_and_every_reloc32_with_the_long_it_relocates(self):
        h = P.read_hunks(two_hunk_program())
        self.assertEqual(h.sizes, [len(CODE), len(DATA)])
        self.assertEqual(h.kinds, ['CODE', 'DATA'])
        self.assertEqual(h.relocs, [(0, 2, 1, 0x10), (1, 0x10, 0, 0x18)])

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


class Reach(unittest.TestCase):
    def test_reach_follows_fallthrough_branches_and_calls(self):
        def i(kind, targets=(), n=2):
            return {'len': n, 'flow': {'kind': kind, 'targets': list(targets)}}
        doc = {'entries': ['h0+$0'], 'insns': {
            'h0+$0': i('call', ['h0+$8']), 'h0+$2': i('cbranch', ['h0+$6']), 'h0+$4': i('return'),
            'h0+$6': i('branch', ['h0+$4']), 'h0+$8': i('return'), 'h0+$a': i('return')}}
        self.assertEqual(P.reach_counts(doc), {'h0+$0': 5})


if __name__ == '__main__':
    unittest.main()
