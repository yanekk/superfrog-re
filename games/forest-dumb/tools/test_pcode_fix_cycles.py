"""pcode_fix.cycles (translator T11, T12): which `r.tick` lines the translator writes, and where. Whether
the times are right is judged on Moira: `fd-conform/tests/insn_cycles.rs`, `insn_check.rs`,
`insn_defects.rs`, and `insn_check.py --columns cycles` on the game's census.

    python3 -m unittest tools/test_pcode_fix_cycles.py      (from games/forest-dumb/)
"""
import importlib
import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import pcode_rust as pr  # noqa: E402
import test_shift_bit as tsb  # noqa: E402

cy = importlib.import_module('pcode_fix.cycles')
FIXTURES = os.path.join(os.path.dirname(HERE), 'engine', 'crates', 'pcode-rt', 'tests', 'fixtures')


def shift_bit(text):
    """The shift_bit fixture's instruction written `text`, in the translator's form."""
    at = next(a for a, t in tsb.sites().items() if t == text)
    return tsb.at(tsb.load(), at)


def tick(op, size, src, dst):
    args = f'cy::Op::{op}, cy::Size::{size}, cy::Ea::{src}, cy::Ea::{dst}'
    return f'r.tick(const {{ cy::of({args}) }}); r.off_bus(const {{ cy::off_of({args}) }});'


def rt(own, off):
    """A run-time tick and its cycles off the bus (T49)."""
    return f'r.tick({own}); r.off_bus({off});'


class Kinds(unittest.TestCase):
    def test_fixed_kinds_name_their_row(self):
        for text, want in [
            ('lsl.l #8,d0', ('Shift(8)', 'L', 'Imm', 'Dn')),
            ('asl.b #1,d0', ('Shift(1)', 'B', 'Imm', 'Dn')),
            ('asl.w (a0)', ('Shift(1)', 'W', 'Ind', 'None')),
            ('bset #33,d0', ('Bset(1)', 'L', 'Imm', 'Dn')),   # the bit number modulo 32
            ('bclr #9,(a0)', ('Bclr(9)', 'B', 'Imm', 'Ind')),
            ('btst d1,d0', ('Btst', 'L', 'Dn', 'Dn')),
            ('st (a0)', ('Scc', 'B', 'Ind', 'None')),         # a byte, though Ghidra writes no suffix
            ('tas (a0)', ('Tas', 'B', 'Ind', 'None')),
            ('swap d0', ('Swap', 'L', 'Dn', 'None')),
        ]:
            self.assertEqual(cy.kind(shift_bit(text)), want, text)

    def test_data_dependent_kinds_tick_from_the_registers_before(self):
        # Not fixed-time rows, so no `kind`; T12 counts them at run time (test_cycles_dynamic.py has
        # the multiply, divide, dbcc and movem forms).
        for text, want in [
            ('lsl.l d1,d0', rt(sh := 'cy::shift_cycles(cy::Size::L, r.d(1))', f'cy::one_word_off({sh})')),
            ('roxr.w d1,d0', rt(sh := 'cy::shift_cycles(cy::Size::W, r.d(1))', f'cy::one_word_off({sh})')),
            ('bset d1,d0', rt(f'cy::of({(b := "cy::Op::Bset(r.d(1) & 31), cy::Size::L, cy::Ea::Dn, cy::Ea::Dn")})',
                              f'cy::off_of({b})')),
            ('bchg d1,d0', rt(f'cy::of({b})', f'cy::off_of({b})')),
            ('bclr d1,d0', rt(f'cy::of({(c := "cy::Op::Bclr(r.d(1) & 31), cy::Size::L, cy::Ea::Dn, cy::Ea::Dn")})',
                              f'cy::off_of({c})')),
            ('st d0', rt(sc := 'cy::scc_reg(cy::cc_holds(0, r))', f'cy::one_word_off({sc})')),
            ('sne d0', rt(sc := 'cy::scc_reg(cy::cc_holds(6, r))', f'cy::one_word_off({sc})')),
        ]:
            ins = shift_bit(text)
            self.assertIsNone(cy.kind(ins), text)
            self.assertEqual(cy.cycles(ins), [want], text)
        self.assertTrue(cy.is_dynamic('mulu', '#imm', 'Dn'))
        self.assertTrue(cy.is_dynamic('dbf', 'Dn', 'label'))
        self.assertFalse(cy.is_dynamic('bne', 'label', None))

    def test_an_instruction_without_its_text_counts_nothing(self):
        ins = dict(shift_bit('swap d0'), operands=None)
        self.assertEqual(cy.cycles(ins), [])


class Emitted(unittest.TestCase):
    def test_a_corrected_shift_counts_first_as_any_instruction(self):
        lines, _ = pr.Emit({}, {}).instruction((0, 0x4c), shift_bit('lsl.l #8,d0'))
        self.assertIn('(corrected: pcode_fix.shift_bit)', lines[0])
        self.assertEqual(lines[2].strip(), tick('Shift(8)', 'L', 'Imm', 'Dn'))
        # A register count is emitted by shift_bit too, its tick computed from d1 before (T12).
        lines, _ = pr.Emit({}, {}).instruction((0, 0x48), shift_bit('lsl.l d1,d0'))
        sh = 'cy::shift_cycles(cy::Size::L, r.d(1))'
        self.assertEqual(lines[2].strip(), rt(sh, f'cy::one_word_off({sh})'))
        self.assertTrue(lines[3].strip().startswith('let cnt'))

    def test_a_conditional_branch_counts_each_path_on_its_own(self):
        doc = pr.load_export(os.path.join(FIXTURES, 'flags_call.json'))
        text, _ = pr.render(doc, 'flags_call.json')
        body = text[text.index('beq.b'):]
        taken = ("if r.get(0x45, 1) != 0 { r.tick(const { cy::bcc(true, cy::Size::B) }); "
                 "r.off_bus(const { cy::bcc_off(true) }); pc = (0, 0x8); continue 'b; }")
        self.assertEqual(body.split('\n')[2].strip(), taken)
        self.assertEqual(body.split('\n')[3].strip(),
                         'r.tick(const { cy::bcc(false, cy::Size::B) }); r.off_bus(const { cy::bcc_off(false) });')
        self.assertEqual(cy.branch_cycles({'mnem': 'bra.w', 'ops': []}), None)
        self.assertEqual(cy.branch_cycles({'mnem': 'bne.w', 'ops': []})[1],
                         'r.tick(const { cy::bcc(false, cy::Size::W) }); r.off_bus(const { cy::bcc_off(false) });')


if __name__ == '__main__':
    unittest.main()
