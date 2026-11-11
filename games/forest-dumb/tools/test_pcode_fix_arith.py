"""The arith family's corrections (translator T08, `pcode_fix/arith.py`) on made-up encodings.

The P-code below is written in Ghidra 12.0's shape (68000.sinc's addx, subx, negx, divu, divs and clr
constructors, as the whole export holds them). It is the export the Rust half runs on Moira,
`engine/crates/fd-conform/tests/insn_arith.rs`, through two generated modules: `arith.rs` with every
correction and `arith_raw.rs` without arith's, so each correction has a test that fails without it.

    python3 -I games/forest-dumb/tools/test_pcode_fix_arith.py --write   # regenerate the three fixtures

No disks, no Ghidra, no cargo; standard library only.
"""
import json, os, sys, unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import pcode_fix  # noqa: E402
import pcode_fix.arith as arith  # noqa: E402
import pcode_rust as pr  # noqa: E402

GAME = os.path.dirname(HERE)
FIX = os.path.join(GAME, 'engine', 'crates', 'fd-conform', 'tests', 'fixtures')
JSON, RS, RAW = (os.path.join(FIX, n) for n in ('arith.json', 'arith.rs', 'arith_raw.rs'))

XF, NF, ZF, VF, CF = 67, 68, 69, 70, 71
D0, D1 = 0, 4


def reg(off, size=4):
    return ['register', off, size]


def u(off, size=4):
    return ['unique', off, size]


def c(v, size=4):
    return ['const', v, size]


def op(o, out, *ins):
    return {'op': o, 'out': out, 'in': list(ins)}


def extended_result(dst):
    """The sinc's `extendedResultFlags`: N from the result, Z only ever cleared."""
    return [op('INT_SLESS', reg(NF, 1), dst, c(0)),
            op('INT_EQUAL', u(0xe600, 1), dst, c(0)),
            op('INT_EQUAL', u(0xe680, 1), reg(ZF, 1), c(1, 1)),
            op('BOOL_AND', reg(ZF, 1), u(0xe600, 1), u(0xe680, 1))]


def addx_l(ty, tx):
    return [op('INT_ADD', u(0xaf80), reg(ty), reg(tx)),
            op('INT_CARRY', u(0xb000, 1), reg(ty), reg(tx)),
            op('INT_ZEXT', u(0xb080), reg(XF, 1)),
            op('INT_CARRY', u(0xb100, 1), u(0xaf80), u(0xb080)),
            op('BOOL_OR', reg(CF, 1), u(0xb000, 1), u(0xb100, 1)),
            op('INT_SCARRY', u(0xb200, 1), reg(ty), reg(tx)),
            op('INT_ZEXT', u(0xb280), reg(XF, 1)),
            op('INT_SCARRY', u(0xb300, 1), u(0xaf80), u(0xb280)),
            op('BOOL_XOR', reg(VF, 1), u(0xb200, 1), u(0xb300, 1)),
            op('INT_ADD', u(0x14480), reg(ty), reg(tx)),
            op('INT_ZEXT', u(0x14500), reg(XF, 1)),
            op('INT_ADD', reg(tx), u(0x14480), u(0x14500))] + extended_result(reg(tx))


def subx_l(ty, tx):
    return [op('INT_ZEXT', u(0xc000), reg(XF, 1)),
            op('INT_LESS', u(0xc080, 1), reg(tx), reg(ty)),
            op('INT_EQUAL', u(0xc100, 1), reg(XF, 1), c(1, 1)),
            op('INT_EQUAL', u(0xc180, 1), reg(tx), reg(ty)),
            op('BOOL_AND', u(0xc200, 1), u(0xc100, 1), u(0xc180, 1)),
            op('BOOL_OR', reg(CF, 1), u(0xc080, 1), u(0xc200, 1)),
            op('INT_SBORROW', reg(VF, 1), reg(tx), reg(ty)),
            op('COPY', reg(XF, 1), reg(CF, 1)),
            op('INT_ADD', u(0xc280), u(0xc000), reg(ty)),
            op('INT_SUB', reg(tx), reg(tx), u(0xc280))] + extended_result(reg(tx))


def negx_l(dst):
    return [op('INT_ZEXT', u(0xd000), reg(XF, 1)),
            op('INT_ADD', u(0xd080), reg(dst), u(0xd000)),
            op('INT_SLESS', reg(CF, 1), c(0), u(0xd080)),
            op('INT_SBORROW', reg(VF, 1), c(0), u(0xd080)),
            op('COPY', reg(XF, 1), reg(CF, 1)),
            op('INT_2COMP', u(0xd080), u(0xd080)),
            op('COPY', reg(dst), u(0xd080))] + extended_result(reg(dst))


def div_w(signed, src, dst):
    ext, div, rem = ('INT_SEXT', 'INT_SDIV', 'INT_SREM') if signed else ('INT_ZEXT', 'INT_DIV', 'INT_REM')
    return [op(ext, u(0x22100), src),
            op('COPY', u(0x22180), reg(dst)),
            op(div, u(0x22280), u(0x22180), u(0x22100)),
            op(rem, u(0x22380), u(0x22180), u(0x22100)),
            op('COPY', reg(CF, 1), c(0, 1)),
            op('INT_SLESS', reg(NF, 1), u(0x22280), c(0)),
            op('INT_EQUAL', reg(ZF, 1), u(0x22280), c(0)),
            op('INT_LEFT', u(0x22400), u(0x22380), c(16)),
            op('INT_AND', u(0x22480), u(0x22280), c(0xffff)),
            op('INT_OR', reg(dst), u(0x22400), u(0x22480))]


def clr_flags():
    return [op('COPY', reg(NF, 1), c(0, 1)), op('COPY', reg(ZF, 1), c(1, 1)),
            op('COPY', reg(VF, 1), c(0, 1)), op('COPY', reg(CF, 1), c(0, 1))]


def clr_abs(addr, size):
    return [op('COPY', ['ram', addr, size], c(0, size))] + clr_flags()


def clr_ind(areg, size):
    return [op('COPY', u(0x4f00, size), c(0, size)),
            {'op': 'STORE', 'out': None, 'in': [c(417), reg(areg), u(0x4f00, size)]}] + clr_flags()


# (offset, mnemonic, operands, bytes, P-code). Words the instructions leave between them are laid
# into memory by the Rust test (the prefetch a clr of a write-only register writes).
INSNS = [
    (0x0, 'addx.l', ['D1', 'D0'], 'd181', addx_l(D1, D0)),
    (0x2, 'subx.l', ['D1', 'D0'], '9181', subx_l(D1, D0)),
    (0x4, 'negx.l', ['D0'], '4080', negx_l(D0)),
    (0x6, 'divu.w', ['D1w', 'D0'], '80c1', div_w(False, reg(D1 + 2, 2), D0)),
    (0x8, 'divs.w', ['#0x3', 'D0'], '81fc0003', div_w(True, c(3, 2), D0)),
    (0xc, 'divu.w', ['#0x0', 'D0'], '80fc0000', div_w(False, c(0, 2), D0)),
    (0x10, 'clr.w', ['(0x00dff0a8).l'], '427900dff0a8', clr_abs(0xdff0a8, 2)),
    (0x18, 'clr.l', ['(0x00dff0a8).l'], '42b900dff0a8', clr_abs(0xdff0a8, 4)),
    (0x20, 'clr.w', ['(A0)'], '4250', clr_ind(0x20, 2)),
]


def fixture():
    insns = {}
    for off, mnem, opers, raw, ops in INSNS:
        insns[f'h0+${off:x}'] = {'len': len(raw) // 2, 'mnem': mnem, 'bytes': raw, 'operands': opers,
                                 'flow': {'kind': 'fallthrough'}, 'ops': ops}
    return {'hunks': [{'index': 0, 'size': 0x30, 'kind': 'CODE'}, {'index': 1, 'size': 0x10, 'kind': 'DATA'}],
            'entries': [{'at': 'h0+$0', 'source': 'made-up'}], 'insns': insns}


def render(raw=False):
    """The --insn module of the fixture, with or without arith's corrections."""
    saved = pcode_fix.MODULES
    if raw:
        pcode_fix.MODULES = [m for m in saved if m is not arith]
    try:
        text, _ = pr.render_insn(pr.load_export(JSON), 'arith.json')
    finally:
        pcode_fix.MODULES = saved
    if raw:
        text = text.replace('pcode_rust.py --insn;', 'pcode_rust.py --insn without pcode_fix.arith;', 1)
    return text


def write():
    with open(JSON, 'w', encoding='utf-8') as f:
        json.dump(fixture(), f, indent=1)
        f.write('\n')
    for path, raw in ((RS, False), (RAW, True)):
        with open(path, 'w', encoding='utf-8') as f:
            f.write(render(raw))


def ins_of(off):
    return pr.load_export(JSON)['insns'][(0, off)]


class Fixtures(unittest.TestCase):
    def test_the_committed_fixtures_are_current(self):
        with open(JSON, encoding='utf-8') as f:
            self.assertEqual(json.load(f), fixture(), 'arith.json is stale: run this file with --write')
        for path, raw in ((RS, False), (RAW, True)):
            with open(path, encoding='utf-8') as f:
                self.assertEqual(f.read(), render(raw), f'{os.path.basename(path)} is stale: --write')


class Addx(unittest.TestCase):
    def test_addx_gets_x_from_c(self):
        ins = ins_of(0x0)
        out = arith.correct(ins)
        self.assertEqual(out['ops'][-1], {'op': 'COPY', 'out': ['register', XF, 1], 'in': [['register', CF, 1]]})
        self.assertEqual(len(out['ops']), len(ins['ops']) + 1)

    def test_subx_and_negx_already_set_x_and_are_left_alone(self):
        for off in (0x2, 0x4):
            ins = ins_of(off)
            self.assertIs(arith.correct(ins), ins)

    def test_other_mnemonics_are_left_alone(self):
        ins = dict(ins_of(0x0), mnem='add.l')
        self.assertIs(arith.correct(ins), ins)


class Divide(unittest.TestCase):
    def test_divu_and_divs_call_the_helper_and_trap_at_their_address(self):
        for off, helper in ((0x6, 'divu_w'), (0x8, 'divs_w'), (0xc, 'divu_w')):
            lines = arith.emit(ins_of(off))
            text = '\n'.join(l for l in lines if isinstance(l, str))
            self.assertIn(f'pcode_rt::ops::{helper}(', text)
            self.assertIn(f'Stop::Trap {{ vector: 5, at: "h0+${off:x}".into() }}', text)
            # The operand fetch stays P-code, written by the translator.
            self.assertTrue(isinstance(lines[0], dict) and lines[0]['op'] in ('INT_ZEXT', 'INT_SEXT'))

    def test_a_shape_it_does_not_know_is_left_to_the_p_code(self):
        ins = ins_of(0x6)
        self.assertIsNone(arith.emit(dict(ins, ops=ins['ops'][:-1])))        # no INT_OR into Dn
        self.assertIsNone(arith.emit(dict(ins, mnem='divu.l D1,D0')))         # not the word form


class Clear(unittest.TestCase):
    def test_a_word_or_long_clr_of_memory_writes_the_prefetch_first(self):
        for off, words in ((0x10, 1), (0x18, 2), (0x20, 1)):
            ins = ins_of(off)
            lines = arith.emit(ins)
            text = '\n'.join(l for l in lines if isinstance(l, str))
            self.assertEqual(text.count('custom_write_only('), words)
            nxt = f'img.h(0, 0x{off + ins["len"]:x})'
            self.assertIn(nxt, text)
            # The faulty read's write comes before the store of 0; a long's store is low word first.
            first = next(i for i, l in enumerate(lines) if isinstance(l, str) and 'custom_write_only' in l)
            if words == 2:
                self.assertFalse(any(isinstance(l, dict) and l['op'] == 'COPY' and l['out'][0] == 'ram'
                                     for l in lines))
                low = lines.index('store(bus, img, a.wrapping_add(2), 2, 0)?;')
                self.assertEqual(lines[low + 1], 'store(bus, img, a, 2, 0)?;')
                self.assertLess(first, low)
            else:
                store = next(i for i, l in enumerate(lines) if isinstance(l, dict)
                             and (l['op'] == 'STORE' or (l['out'] or [None])[0] == 'ram'))
                self.assertLess(first, store)

    def test_a_byte_clr_or_a_clr_of_a_register_is_not_emitted(self):
        self.assertIsNone(arith.emit(dict(ins_of(0x10), mnem='clr.b',
                                          ops=clr_abs(0xdff0a8, 1))))
        self.assertIsNone(arith.emit(dict(ins_of(0x10), mnem='clr.w',
                                          ops=[op('COPY', reg(D0 + 2, 2), c(0, 2))] + clr_flags())))

    def test_the_generated_rust_carries_the_correction(self):
        rs, raw = render(), render(raw=True)
        self.assertIn('custom_write_only', rs)
        self.assertNotIn('custom_write_only', raw)
        self.assertIn('(corrected: pcode_fix.arith)', rs)


class Rules(unittest.TestCase):
    def test_no_address_special_case(self):
        """`at` is read only for the stop's name and the word after the instruction."""
        with open(arith.__file__, encoding='utf-8') as f:
            src = f.read()
        self.assertNotIn("ins['at'] ==", src)
        self.assertNotIn('h0+$', src.split('"""', 2)[2])


if __name__ == '__main__':
    if sys.argv[1:] == ['--write']:
        write()
    else:
        unittest.main()
