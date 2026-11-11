"""pcode_fix.shift_bit (translator T09): shifts, rotates, bit ops, scc, tas and swap.

The fixture is a made-up program holding every encoding of the family the tests need (`ENCODINGS`),
exported through Ghidra 12.0 into `engine/crates/fd-conform/tests/fixtures/shift_bit.json`, so its
P-code is Ghidra's own, not hand-written. From it, `pcode_rust.py --insn` writes `shift_bit.rs` (with
every correction) and `shift_bit_raw.rs` (without this module's), and `shift_bit.bin` holds the
program's code. `fd-conform/tests/insn_shift_bit.rs` runs both against Moira: the corrected one
passes every state, the raw one fails where each correction says.

These tests need no Ghidra, cargo or disks. After a translator or correction change, or to add an
encoding, run `python3 tools/test_shift_bit.py --regen` from `games/forest-dumb/` (needs Ghidra) and
commit the five fixtures."""
import io, json, os, sys, tempfile, unittest
from contextlib import redirect_stdout

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import pcode_fix  # noqa: E402
import pcode_rust as pr  # noqa: E402
from pcode_fix import shift_bit as sb  # noqa: E402

FIXTURES = os.path.join(os.path.dirname(HERE), 'engine', 'crates', 'fd-conform', 'tests', 'fixtures')
JSON = os.path.join(FIXTURES, 'shift_bit.json')
BIN = os.path.join(FIXTURES, 'shift_bit.bin')
RS = os.path.join(FIXTURES, 'shift_bit.rs')
RAW_RS = os.path.join(FIXTURES, 'shift_bit_raw.rs')
TEXTS_RS = os.path.join(FIXTURES, 'shift_bit_texts.rs')

TYPES = ('as', 'ls', 'rox', 'ro')
SIZES = ('b', 'w', 'l')


def _encodings():
    """[(words, text)]: one instruction each, laid out in this order from h0+$0."""
    out = []
    for tt, t in enumerate(TYPES):
        for d, dn in ((0, 'r'), (1, 'l')):
            for ss, s in enumerate(SIZES):
                op = 0xe000 | (d << 8) | (ss << 6) | (tt << 3)
                out.append(([op | (1 << 9) | (1 << 5)], f'{t}{dn}.{s} d1,d0'))   # count d1 mod 64
                out.append(([op | (1 << 9)], f'{t}{dn}.{s} #1,d0'))
                out.append(([op], f'{t}{dn}.{s} #8,d0'))                        # count field 0 is 8
            out.append(([0xe0d0 | (tt << 9) | (d << 8)], f'{t}{dn}.w (a0)'))     # memory: word, by 1
    for tt, t in enumerate(('btst', 'bchg', 'bclr', 'bset')):
        out.append(([0x0300 | (tt << 6)], f'{t} d1,d0'))                          # bit d1 mod 32
        out.append(([0x0310 | (tt << 6)], f'{t} d1,(a0)'))                        # bit d1 mod 8
        out.append(([0x0800 | (tt << 6), 33], f'{t} #33,d0'))
        out.append(([0x0810 | (tt << 6), 9], f'{t} #9,(a0)'))
    ccs = ('t', 'f', 'hi', 'ls', 'cc', 'cs', 'ne', 'eq', 'vc', 'vs', 'pl', 'mi', 'ge', 'lt', 'gt', 'le')
    for cc, c in enumerate(ccs):
        out.append(([0x50c0 | (cc << 8)], f's{c} d0'))
    out.append(([0x50d0], 'st (a0)'))
    out.append(([0x56d0], 'sne (a0)'))
    out.append(([0x4ac0], 'tas d0'))
    out.append(([0x4ad0], 'tas (a0)'))
    out.append(([0x4840], 'swap d0'))
    out.append(([0x4e75], 'rts'))
    return out


ENCODINGS = _encodings()


def code_bytes():
    """The program's one code hunk, padded to a long."""
    b = b''.join(w.to_bytes(2, 'big') for ws, _ in ENCODINGS for w in ws)
    return b + b'\0' * (-len(b) % 4)


def sites():
    """{'hN+$off': text} of every encoding."""
    out, off = {}, 0
    for ws, text in ENCODINGS:
        out[pr.fmt_addr((0, off))] = text
        off += 2 * len(ws)
    return out


def without_shift_bit():
    """A context in which pcode_fix runs every module but this one."""
    class Ctx:
        def __enter__(self):
            self.saved = pcode_fix.MODULES
            pcode_fix.MODULES = [m for m in pcode_fix.MODULES if m is not sb]

        def __exit__(self, *a):
            pcode_fix.MODULES = self.saved
    return Ctx()


def render(raw=False):
    doc = pr.load_export(JSON)
    if raw:
        with without_shift_bit():
            return pr.render_insn(doc, 'shift_bit.json (shift_bit corrections off)')[0]
    return pr.render_insn(doc, 'shift_bit.json')[0]


def texts_rs():
    """`TEXTS`, each encoding's text in site order, for the Rust test to find a site by."""
    return ('// Written by tools/test_shift_bit.py --regen: each site\'s text, in SITES order.\n'
            'const TEXTS: &[&str] = &[\n' + ''.join(f'    {json.dumps(t)},\n' for _, t in ENCODINGS) + '];\n')


def regen():
    import pcode_export as P
    from test_pcode_export import longs
    code = code_bytes()
    exe = (longs(0x3F3, 0, 1, 0, 0, len(code) // 4) + longs(0x3E9, len(code) // 4) + code + longs(0x3F2))
    with tempfile.TemporaryDirectory() as d:
        path = os.path.join(d, 'p.exe')
        with open(path, 'wb') as f:
            f.write(exe)
        with redirect_stdout(io.StringIO()):
            doc = P.export(path, [P.Sym(0, 0, 'start', 'func', 0, '')], [(0, 0, 'routine')],
                           os.path.join(d, 'out'), name='p')
    missing = set(sites()) - set(doc['insns'])
    if missing:
        raise SystemExit(f'Ghidra did not decode {sorted(missing)}')
    with open(JSON, 'w') as f:
        json.dump(doc, f, indent=None, sort_keys=True)
        f.write('\n')
    with open(BIN, 'wb') as f:
        f.write(code)
    with open(TEXTS_RS, 'w') as f:
        f.write(texts_rs())
    for path, raw in ((RS, False), (RAW_RS, True)):
        with open(path, 'w') as f:
            f.write(render(raw))
    print('wrote', ', '.join(os.path.basename(p) for p in (JSON, BIN, RS, RAW_RS, TEXTS_RS)))


def load():
    return pr.load_export(JSON)['insns']


def at(insns, a):
    return insns[pr.parse_addr(a, 'test')]


def by_text(text):
    return next(a for a, t in sites().items() if t == text)


class Fixtures(unittest.TestCase):
    def test_the_fixtures_are_current(self):
        with open(BIN, 'rb') as f:
            self.assertEqual(f.read(), code_bytes(), 'shift_bit.bin is stale: --regen')
        with open(TEXTS_RS) as f:
            self.assertEqual(f.read(), texts_rs(), 'shift_bit_texts.rs is stale: --regen')
        insns = load()
        for a, text in sites().items():
            self.assertIn(pr.parse_addr(a, 't'), insns, f'{a} {text}: --regen')
        for path, raw in ((RS, False), (RAW_RS, True)):
            with open(path) as f:
                self.assertEqual(f.read(), render(raw), f'{os.path.basename(path)} is stale: --regen')

    def test_ghidra_decoded_each_encoding_as_written(self):
        insns = load()
        for a, text in sites().items():
            self.assertEqual(at(insns, a)['mnem'].split('.')[0], text.split(' ')[0].split('.')[0], a)


REGISTER_SHIFTS = [t for _, t in ENCODINGS if t[0] in 'alr' and '.' in t and t.endswith(',d0')]


class Corrections(unittest.TestCase):
    def test_every_register_form_shift_is_emitted_from_its_opcode_word(self):
        insns = load()
        self.assertEqual(len(REGISTER_SHIFTS), 4 * 2 * 3 * 3)
        for a, text in sites().items():
            ins = at(insns, a)
            lines = sb.emit(ins)
            if text in REGISTER_SHIFTS:
                self.assertIsNotNone(lines, text)
                self.assertIn('pcode_rt::ops::shift68k(', lines[1], text)
            else:
                # Memory forms, bit ops, scc, tas, swap: Ghidra's P-code matches Moira (insn_shift_bit.rs).
                self.assertIsNone(lines, text)

    def test_the_operands_come_from_the_opcode_word(self):
        insns = load()
        lines = sb.emit(at(insns, by_text('roxl.w d1,d0')))
        self.assertEqual(lines[0], 'let cnt = r.get(0x4, 4) & 0x3f;')            # d1 mod 64
        self.assertIn('shift68k(2, true, 2, cnt, r.get(0x2, 2), r.get(0x43, 1))', lines[1])
        self.assertEqual(lines[2], 'r.set(0x2, 2, v);')                           # d0's low word
        lines = sb.emit(at(insns, by_text('asr.b #8,d0')))
        self.assertEqual(lines[0], 'let cnt = 8u64;')                            # count field 0 is 8
        self.assertIn('shift68k(0, false, 1, cnt, r.get(0x3, 1)', lines[1])
        lines = sb.emit(at(insns, by_text('lsl.l #1,d0')))
        self.assertIn('shift68k(1, true, 4, cnt, r.get(0x0, 4)', lines[1])

    def test_a_made_up_word_for_another_register_and_count(self):
        # rol.l d5,d3 and lsr.w #3,d7, words the fixture does not hold.
        rol = {'len': 2, 'mnem': 'rol.l', 'ops': [], 'bytes': 'ebbb'}
        self.assertEqual(sb.emit(rol)[0], 'let cnt = r.get(0x14, 4) & 0x3f;')
        self.assertIn('shift68k(3, true, 4, cnt, r.get(0xc, 4)', sb.emit(rol)[1])
        lsr = {'len': 2, 'mnem': 'lsr.w', 'ops': [], 'bytes': 'e64f'}
        self.assertEqual(sb.emit(lsr)[0], 'let cnt = 3u64;')
        self.assertIn('r.set(0x1e, 2, v);', sb.emit(lsr))

    def test_a_word_that_disagrees_with_the_mnemonic_or_has_no_bytes_keeps_its_p_code(self):
        ins = at(load(), by_text('asl.l d1,d0'))
        for odd in (dict(ins, mnem='lsl.l'), dict(ins, mnem='asl.w'), dict(ins, bytes=None),
                    dict(ins, bytes='e0d0'), dict(ins, len=4), dict(ins, bytes='4840')):
            self.assertIsNone(sb.emit(odd), odd['mnem'] + ' ' + str(odd['bytes']))

    def test_through_the_translator(self):
        text = '\n'.join(pr.Emit({}, {}).instruction((0, 0), at(load(), by_text('asl.l d1,d0')))[0])
        self.assertIn('(corrected: pcode_fix.shift_bit)', text)
        self.assertEqual(pcode_fix.emit(at(load(), by_text('btst d1,d0'))), (None, None))


if __name__ == '__main__':
    if sys.argv[1:] == ['--regen']:
        regen()
    else:
        unittest.main()
