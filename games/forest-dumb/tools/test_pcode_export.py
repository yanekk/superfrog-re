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


# Translator T07: Ghidra 12.0's known defects, one instruction each, for the instruction check
# (fd-conform/tests/insn_defects.rs, on Moira). The bcd and compare forms the game lacks (nbcd in
# every alterable mode, cmpm, -(a7)) are here; the bra.w at $4a lands on another function, `other`,
# and test_pcode_export_ghidra.py marks it CALL_RETURN as Ghidra does on the game.
DEFECTS_H0 = bytes.fromhex(
    'C101' 'C109' 'CF0F'                    # $00 abcd d1,d0; abcd -(a1),-(a0); abcd -(a7),-(a7)
    '8101' '8109' '8F0F'                    # $06 sbcd, the same three
    '4800' '4810' '4818' '4820'             # $0c nbcd d0; (a0); (a0)+; -(a0)
    '481F' '4827'                           # $14 nbcd (a7)+; -(a7): steps 2
    '48280010' '48301002'                   # $18 nbcd 16(a0); 2(a0,d1.w)
    '48380400' '483900000004'               # $20 nbcd $400.w; h1+$4, relocated
    'B001' 'B050' 'B081' 'B0C1' 'B1C9'      # $2a cmp.b d1,d0; cmp.w (a0),d0; cmp.l; cmpa.w d1,a0; cmpa.l a1,a0
    '0C030001' '0C501234' '0C8012345678'    # $34 cmpi.b #1,d3; cmpi.w #$1234,(a0); cmpi.l #$12345678,d0
    'B109' 'B149' 'B189' 'BF0F'             # $42 cmpm.b/.w/.l (a1)+,(a0)+; cmpm.b (a7)+,(a7)+
    '60000006'                              # $4a bra.w $52, into `other`
    '4E75' '4E71'                           # $4e rts; padding, never reached
    '74024E75'                              # $52 other: moveq #2,d2; rts
    '4E71')                                 # $56 padding
DEFECTS_H1 = bytes(16)
DEFECTS_RELOC = 0x26                        # the long of nbcd h1+$4
DEFECTS_FUNCS = [(0, 0x0, 'start'), (0, 0x52, 'other')]
DEFECTS_TAIL = 'h0+$4a'


# Translator T43: a made-up program shaped as the game's load lists, each call to `loader` followed by a
# store to a relocated address. On the game Ghidra reads such bytes as data after the call and judges
# load_file non-returning; it does not on this small program, so test_pcode_export_ghidra.py marks
# `loader` non-returning first (SetNoReturn.java), as Ghidra did. `loader` loops for ever on one path
# and returns on the other. Hunk 1 is code the entry in hunk 0 never reaches, as hunk 4 is on the game.
NORETURN_H0 = bytes.fromhex('4E754E71')    # $00 rts; padding: the program's entry
NORETURN_H1 = bytes.fromhex(
    '6100001E' '23C000000000'               # $00 bsr.w loader; $04 move.l d0,h2+$0  relocated
    '61000014' '23C000000004'               # $0a bsr.w loader; $0e move.l d0,h2+$4  relocated
    '6100000A' '23C000000008'               # $14 bsr.w loader; $18 move.l d0,h2+$8  relocated
    '4E75'                                  # $1e rts
    '4A41' '6702' '60FE'                    # $20 loader: tst.w d1; beq.s $26; $24 bra.s $24, for ever
    '7001' '4E75' '4E71')                   # $26 moveq #1,d0; rts; padding
NORETURN_H2 = bytes(16)
NORETURN_RELOCS = (0x06, 0x10, 0x1a)
NORETURN_FUNCS = [(0, 0x0, 'start'), (1, 0x0, 'lists'), (1, 0x20, 'loader')]
NORETURN_CALLS = ['h1+$0', 'h1+$a', 'h1+$14']
NORETURN_LOADER = 'h1+$20'


def noreturn_program():
    return (longs(0x3F3, 0, 3, 0, 2, len(NORETURN_H0) // 4, len(NORETURN_H1) // 4, len(NORETURN_H2) // 4)
            + longs(0x3E9, len(NORETURN_H0) // 4) + NORETURN_H0 + longs(0x3F2)
            + longs(0x3E9, len(NORETURN_H1) // 4) + NORETURN_H1
            + longs(0x3EC, len(NORETURN_RELOCS), 2, *NORETURN_RELOCS, 0, 0x3F2)
            + longs(0x3EA, len(NORETURN_H2) // 4) + NORETURN_H2 + longs(0x3F2))


def defects_program():
    return (longs(0x3F3, 0, 2, 0, 1, len(DEFECTS_H0) // 4, len(DEFECTS_H1) // 4)
            + longs(0x3E9, len(DEFECTS_H0) // 4) + DEFECTS_H0 + longs(0x3EC, 1, 1, DEFECTS_RELOC, 0, 0x3F2)
            + longs(0x3EA, len(DEFECTS_H1) // 4) + DEFECTS_H1 + longs(0x3F2))


# Translator T10: the move and control forms the game's census lacks, one instruction each, for the
# instruction check (fd-conform/tests/insn_moves.rs, on Moira): SR and CCR in every direction, rte and
# rtr, move usp, movem with the register itself in the list, byte moves through a7, link and unlk, pea,
# exg, dbcc, jsr, jmp, bsr. One code hunk, no relocation; each instruction after one that does not fall
# through is a function of its own.
MOVES_H0 = bytes.fromhex(
    '4E73' '4E77'                           # $00 rte; $02 rtr
    '46C0' '46D0' '46FC2015'                # $04 move.w d0,sr; (a0),sr; #$2015,sr
    '40C0' '40E7'                           # $0c move.w sr,d0; sr,-(a7)
    '44C0' '44FC0015'                       # $10 move.w d0,ccr; #$15,ccr
    '027CF8FF' '007C0700' '0A7C0711'        # $16 andi.w #$f8ff,sr; ori.w #$700,sr; eori.w #$711,sr
    '023C00F0' '003C0005' '0A3C001F'        # $22 andi.b #$f0,ccr; ori.b #5,ccr; eori.b #$1f,ccr
    '4E60' '4E69'                           # $2e move.l a0,usp; move.l usp,a1
    '48E7FFFE' '4CDF7FFF'                   # $32 movem.l d0-a6,-(a7); (a7)+,d0-a6
    '48A18080' '4C980003'                   # $3a movem.w d0/a0,-(a1); (a0)+,d0-d1
    '48E0C080' '4CD80101'                   # $42 movem.l d0-d1/a0,-(a0); (a0)+,d0/a0
    '1F00' '101F' '1F1F' '1ED8'             # $4a move.b d0,-(a7); (a7)+,d0; (a7)+,-(a7); (a0)+,(a7)+
    '4E56FFF8' '4E5E'                       # $52 link a6,#-8; unlk a6
    '4850' '48781234' '487A0010'            # $58 pea (a0); $1234.w; 16(pc)
    'C188' 'C34F'                           # $62 exg d0,a0; exg a1,a7
    '57C8FFFE' '51C90004' '4E71'            # $66 dbeq d0,$66; dbf d1,$70; nop
    '4E90' '4EBA0004' '4ED0'                # $70 jsr (a0); jsr $78(pc); jmp (a0)
    '61000004' '6102' '4E75'                # $78 bsr.w $7e; bsr.b $80; $7e rts
    '6602' '4E71' '4E75' '4E71')            # $80 bne.b $84; nop; $84 rts; padding
MOVES_FUNCS = [(0, 0x0, 'rte_'), (0, 0x2, 'rtr_'), (0, 0x4, 'start'), (0, 0x78, 'calls')]


def moves_program():
    return longs(0x3F3, 0, 1, 0, 0, len(MOVES_H0) // 4) + longs(0x3E9, len(MOVES_H0) // 4) + MOVES_H0 + longs(0x3F2)


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

    def test_the_defects_programs_reloc32_entry_is_read_from_the_file(self):
        # Ghidra's Amiga loader records no relocations (a defect of Ghidra 12.0's): the file's own
        # HUNK_RELOC32 entry, with the long it relocates, is what the export matches operands by.
        h = P.read_hunks(defects_program())
        self.assertEqual(h.kinds, ['CODE', 'DATA'])
        self.assertEqual(h.relocs, [(0, DEFECTS_RELOC, 1, 4)])

    def test_the_noreturn_programs_three_relocated_stores(self):
        h = P.read_hunks(noreturn_program())
        self.assertEqual(h.kinds, ['CODE', 'CODE', 'DATA'])
        self.assertEqual(h.relocs, [(1, 0x06, 2, 0), (1, 0x10, 2, 4), (1, 0x1a, 2, 8)])

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
