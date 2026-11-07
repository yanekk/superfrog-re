"""pcode_export.py through Ghidra: a hand-written two-hunk program, then the game (full set: about 20 s).

Needs Ghidra and its JDK (translator DESIGN §5); without them the test fails naming the path, never
skips (§2.8).
"""
import io, os, sys, tempfile, unittest
from contextlib import redirect_stdout

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import pcode_export as P  # noqa: E402
from test_pcode_export import ENTRIES, two_hunk_program  # noqa: E402


class GhidraExport(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        exe = os.path.join(cls.tmp.name, 'p.exe')
        with open(exe, 'wb') as f:
            f.write(two_hunk_program())
        P.check_inputs(exe, P.SYMBOLS, P.TRANSLATE)   # a missing Ghidra fails here, naming the path
        syms = [P.Sym(0, 0, 'start', 'func', 0, ''), P.Sym(0, 0x18, 'callee', 'func', 0, ''),
                P.Sym(0, 0x1c, 'other', 'func', 0, ''), P.Sym(0, 0x24, 'tail_a', 'func', 0, ''),
                P.Sym(0, 0x28, 'tail_b', 'func', 0, '')]
        with redirect_stdout(io.StringIO()):
            cls.doc = P.export(exe, syms, ENTRIES, os.path.join(cls.tmp.name, 'out'), name='p')
        cls.insns = cls.doc['insns']

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def flow(self, at):
        return self.insns[at]['flow']

    def test_every_instruction_by_direct_flow_hunk_relative(self):
        self.assertEqual(self.doc['entries'], ['h0+$0', 'h0+$1c', 'h0+$24', 'h0+$28', 'h0+$30'])
        self.assertEqual(sorted(self.insns, key=lambda a: int(a.split('$')[1], 16)),
                         ['h0+$0', 'h0+$6', 'h0+$a', 'h0+$c', 'h0+$e', 'h0+$12', 'h0+$14', 'h0+$16',
                          'h0+$18', 'h0+$1a', 'h0+$1c', 'h0+$20', 'h0+$24', 'h0+$28', 'h0+$2c', 'h0+$2e',
                          'h0+$30', 'h0+$36'])
        self.assertEqual([h['kind'] for h in self.doc['hunks']], ['CODE', 'DATA'])
        self.assertEqual(self.insns['h0+$0']['len'], 6)
        self.assertEqual(self.insns['h0+$6']['mnem'], 'bsr.w')

    def test_the_lea_to_the_other_hunk_is_a_relocated_constant(self):
        i = self.insns['h0+$0']
        self.assertEqual(i['relocs'], [{'in_op': 0, 'slot': 0, 'target': 'h1+$10'}])
        self.assertEqual(i['ops'][0]['in'][0], ['const', 'h1+$10', 4])

    def test_flow_kinds_and_targets(self):
        self.assertEqual(self.flow('h0+$6'), {'kind': 'call', 'targets': ['h0+$18']})
        self.assertEqual(self.flow('h0+$e'), {'kind': 'cbranch', 'targets': ['h0+$c']})
        self.assertEqual(self.flow('h0+$a'), {'kind': 'fallthrough', 'targets': []})
        self.assertEqual(self.flow('h0+$16'), {'kind': 'return', 'targets': []})
        # the bsr's own P-code: its target and the return address it pushes, both hunk-relative
        ops = self.insns['h0+$6']['ops']
        self.assertEqual(ops[-1], {'op': 'CALL', 'out': None, 'in': [['ram', 'h0+$18', 4]]})
        self.assertIn(['const', 'h0+$a', 4], ops[1]['in'])

    def test_a_branch_into_a_shared_tail_is_a_branch(self):
        # Ghidra overrides such a branch into a tail call on the game (GameExport), not on this program
        for at in ('h0+$24', 'h0+$28'):
            self.assertEqual(self.flow(at), {'kind': 'branch', 'targets': ['h0+$2c']})
            self.assertEqual(self.insns[at]['ops'], [{'op': 'BRANCH', 'out': None, 'in': [['ram', 'h0+$2c', 4]]}])
            self.assertEqual(P.reach_counts(self.doc)[at], 3)

    def test_computed_jumps_ghidra_cannot_resolve_are_empty_and_counted(self):
        self.assertEqual(self.flow('h0+$14'), {'kind': 'call', 'computed': True, 'targets': []})
        self.assertEqual(self.flow('h0+$20'), {'kind': 'computed', 'targets': []})
        self.assertEqual(self.doc['check']['unresolved'], 2)

    def test_a_pc_relative_address_is_hunk_relative_and_explained(self):
        # Ghidra's P-code adds the displacement to the instruction's own address at run time
        self.assertEqual(self.insns['h0+$1c']['ops'][0]['in'], [['const', 'h0+$1c', 4], ['const', 2, 4]])
        self.assertEqual(self.insns['h0+$1c']['relocs'], [])
        c = self.doc['check']
        self.assertEqual(c['pcrel'], 1)
        self.assertEqual((c['unmatched'], c['mismatched'], c['undecoded']), ([], [], []))
        self.assertEqual((c['relocs'], c['relocs_matched'], c['relocs_in_file']), (1, 1, 2))

    def test_an_unrelocated_number_in_hunk_range_stays_a_number_and_is_reported(self):
        # Ghidra loads hunk 0 at $21f000; reading this as an address would be the guess DESIGN §7 rejects
        self.assertEqual(self.doc['ghidra_bases'][0], 0x21f000)
        self.assertEqual(self.insns['h0+$30']['ops'][0]['in'], [['const', 0x21f004, 4]])
        self.assertEqual(self.insns['h0+$30']['relocs'], [])
        self.assertEqual(self.doc['check']['unexplained'], ['h0+$30 const h0+$4'])


class GameExport(unittest.TestCase):
    """The game's program from the extracted disk (`fd-tools extract`), into a scratch project."""

    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        P.check_inputs(P.EXE, P.SYMBOLS, P.TRANSLATE)   # names the program's path when it is missing
        with redirect_stdout(io.StringIO()):
            cls.doc = P.export(P.EXE, P.load_symbols(P.SYMBOLS), P.load_entries(P.TRANSLATE),
                               os.path.join(cls.tmp.name, 'out'))

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def test_each_routine_reaches_what_the_inventory_counts(self):
        # Exact today; fails if a branch Ghidra calls a tail call is read as a call that falls
        # through, or if a pointer the loader marked inside an instruction stops its decoding.
        want = {P.rel(h, off): n for h, off, n in P.load_entries(P.TRANSLATE)}
        self.assertEqual(P.reach_counts(self.doc), want)

    def test_every_relocation_inside_the_code_lands_on_an_operand(self):
        c = self.doc['check']
        self.assertEqual(c['relocs'], c['relocs_matched'])
        self.assertEqual((c['unmatched'], c['mismatched'], c['unexplained'], c['undecoded']), ([], [], [], []))


if __name__ == '__main__':
    unittest.main()
