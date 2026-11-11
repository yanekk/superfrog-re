"""pcode_census.py on made-up exports: kinds by mnemonic, size and both modes; counts, encodings and
sites; the modes Ghidra's operand text hides taken from the opcode word; bad inputs (translator T04)."""
import json
import os
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import pcode_census as pc

TOOL = os.path.join(HERE, 'pcode_census.py')


def ins(mnem, operands, raw):
    return {'len': len(raw) // 2, 'mnem': mnem, 'operands': operands, 'bytes': raw, 'ops': []}


# Operand text as Ghidra 12.0 writes it (seen in the whole export).
INSNS = {
    'h0+$0': ins('add.w', ['D0w', '(A1)+'], 'd159'),
    'h0+$2': ins('add.w', ['D2w', '(A3)+'], 'd55b'),
    'h0+$4': ins('add.w', ['D0w', '(A1)+'], 'd159'),
    'h0+$6': ins('add.w', ['D0w', '-(A1)'], 'd161'),
    'h0+$8': ins('add.l', ['D0', '(A1)+'], 'd399'),
    'h0+$a': ins('rts', [], '4e75'),
    'h0+$c': ins('move.b', ['(0x0,A5,D1w*0x1)', 'D0b'], '10351000'),
    'h0+$10': ins('move.l', ['(0x00dff004).l', '(0x4,A0)'], '217900dff0040004'),
    'h0+$18': ins('movem.l', ['{  A6 A5 D1 D0}', 'SP'], '48e7c006'),
    'h0+$1c': ins('movem.l', ['SP', '{ D0 D1 A5 A6}'], '4cdf6003'),
    'h0+$20': ins('jsr', ['-0x198', 'A6'], '4eaefe68'),
    'h0+$24': ins('jsr', ['0x00223d04'], '4eb900223d04'),
    'h0+$2a': ins('bne.w', ['0x00223d04'], '6600000e'),
    'h0+$2e': ins('dbf', ['D0w', '0x00223d04'], '51c8fffe'),
    'h0+$32': ins('moveq', ['0x5', 'D1'], '7205'),
    'h0+$34': ins('lea', ['(0x24,PC)', 'A0'], '41fa0024'),
    'h0+$38': ins('andi.w', ['#0xfff', 'D0w'], '02400fff'),
    'h0+$3c': ins('move.w', ['(0x1234).w', 'SP'], '3e781234'),
    'h0+$40': ins('move.w', ['#-0x2', '-(SP)'], '3f3cfffe'),
    'h1+$0': ins('add.w', ['D0w', '(A1)+'], 'd159'),
}


def census(insns=INSNS):
    return {r['kind']: r for r in pc.census({'insns': insns})}


class Kinds(unittest.TestCase):
    def test_grouped_by_mnemonic_size_and_both_modes(self):
        k = census()
        add = k['add.w Dn,(An)+']
        self.assertEqual({x: add[x] for x in ('mnemonic', 'size', 'src', 'dst', 'count')},
                         {'mnemonic': 'add', 'size': 2, 'src': 'Dn', 'dst': '(An)+', 'count': 4})
        self.assertEqual(add['encodings'], ['d159', 'd55b'])
        self.assertEqual(add['sites'], ['h0+$0', 'h0+$2', 'h0+$4', 'h1+$0'])
        # Another mode or another size is another kind.
        self.assertEqual(k['add.w Dn,-(An)']['count'], 1)
        self.assertEqual(k['add.l Dn,(An)+']['size'], 4)
        self.assertEqual(len(k), 17)

    def test_no_operands_and_no_size(self):
        r = census()['rts']
        self.assertEqual((r['size'], r['src'], r['dst']), (None, None, None))

    def test_each_mode_from_ghidras_text(self):
        k = census()
        for kind in ('move.b (d8,An,Xn),Dn', 'move.l abs.l,(d16,An)', 'bne.w label', 'dbf Dn,label',
                     'moveq #imm,Dn', 'lea (d16,PC),An', 'andi.w #imm,Dn', 'move.w abs.w,An',
                     'move.w #imm,-(An)'):
            self.assertIn(kind, k)

    def test_modes_ghidra_hides_come_from_the_opcode_word(self):
        k = census()
        self.assertIn('movem.l list,-(An)', k)
        self.assertIn('movem.l (An)+,list', k)
        # Ghidra splits jsr -$198(a6) into a number and a register: one operand, (d16,An).
        self.assertEqual(k['jsr (d16,An)']['src'], '(d16,An)')
        self.assertIsNone(k['jsr (d16,An)']['dst'])
        self.assertIn('jsr abs.l', k)

    def test_commonest_first_then_by_name(self):
        rows = pc.census({'insns': INSNS})
        self.assertEqual(rows[0]['kind'], 'add.w Dn,(An)+')
        rest = [r['kind'] for r in rows if r['count'] == 1]
        self.assertEqual(rest, sorted(rest))

    def test_only_the_first_five_sites(self):
        many = {f'h0+${2 * i:x}': ins('nop', [], '4e71') for i in range(12)}
        r = census(many)['nop']
        self.assertEqual((r['count'], r['sites']), (12, ['h0+$0', 'h0+$2', 'h0+$4', 'h0+$6', 'h0+$8']))

    def test_sites_in_address_order_not_text_order(self):
        r = census({'h0+$10': ins('nop', [], '4e71'), 'h0+$8': ins('nop', [], '4e71')})['nop']
        self.assertEqual(r['sites'], ['h0+$8', 'h0+$10'])


class Bad(unittest.TestCase):
    def test_an_operand_with_no_mode_is_named(self):
        with self.assertRaisesRegex(pc.BadInput, r"h0\+\$0: operand 1 'what' of add.w"):
            pc.census({'insns': {'h0+$0': ins('add.w', ['D0w', 'what'], 'd159')}})
        # A bare number is a mode only where the mnemonic says what it is.
        with self.assertRaisesRegex(pc.BadInput, 'no addressing mode'):
            pc.census({'insns': {'h0+$0': ins('add.w', ['0x5', 'D0w'], 'd07c')}})

    def test_an_export_without_operands_is_named(self):
        with self.assertRaisesRegex(pc.BadInput, r'no "operands"'):
            pc.census({'insns': {'h0+$0': {'len': 2, 'mnem': 'rts', 'ops': []}}})

    def test_cli_exits_2_and_writes_nothing(self):
        with tempfile.TemporaryDirectory() as d:
            out = os.path.join(d, 'census.json')
            for name, body, says in [('missing.json', None, 'cannot read'), ('bad.json', '{', 'not JSON'),
                                     ('noinsns.json', '{}', 'no "insns"'),
                                     ('nobytes.json', json.dumps({'insns': {'h0+$0': dict(INSNS['h0+$a'], bytes='')}}),
                                      'no "bytes"')]:
                p = os.path.join(d, name)
                if body is not None:
                    with open(p, 'w') as f:
                        f.write(body)
                r = subprocess.run([sys.executable, '-I', TOOL, p, '--out', out], capture_output=True, text=True)
                self.assertEqual(r.returncode, 2, name)
                self.assertIn(p, r.stderr)
                self.assertIn(says, r.stderr)
            self.assertFalse(os.path.exists(out))

    def test_cli_writes_the_census_and_a_summary(self):
        with tempfile.TemporaryDirectory() as d:
            p, out = os.path.join(d, 'pcode.json'), os.path.join(d, 'census.json')
            with open(p, 'w') as f:
                json.dump({'insns': INSNS}, f)
            r = subprocess.run([sys.executable, '-I', TOOL, p, '--out', out], capture_output=True, text=True)
            self.assertEqual(r.returncode, 0, r.stderr)
            self.assertEqual(json.loads(r.stdout), {'kinds': 17, 'instructions': 20})
            with open(out) as f:
                self.assertEqual(json.load(f)[0]['kind'], 'add.w Dn,(An)+')


if __name__ == '__main__':
    unittest.main()
