"""pcode_fix.cycles' data-dependent kinds (translator T12): multiply, divide, dbcc, movem, chk.

The fixture is a made-up program holding each encoding the tests need (`ENCODINGS`), exported through
Ghidra 12.0 into `engine/crates/fd-conform/tests/fixtures/cycles_dynamic.json`, so its P-code is Ghidra's
own. From it `pcode_rust.py --insn` writes `cycles_dynamic.rs`, and `cycles_dynamic.bin` holds the code.
`fd-conform/tests/insn_cycles_dynamic.rs` runs every site against Moira on 200 states, cycles included.
The shifts, bit ops and scc by a register are T09's fixture's, judged by `insn_cycles.rs`.

These tests need no Ghidra, cargo or disks. After a translator or correction change, run
`python3 tools/test_cycles_dynamic.py --regen` from `games/forest-dumb/` (needs Ghidra) and commit the
four fixtures."""
import importlib
import io
import json
import os
import sys
import tempfile
import unittest
from contextlib import redirect_stdout

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import pcode_rust as pr  # noqa: E402

cy = importlib.import_module('pcode_fix.cycles')
FIXTURES = os.path.join(os.path.dirname(HERE), 'engine', 'crates', 'fd-conform', 'tests', 'fixtures')
JSON = os.path.join(FIXTURES, 'cycles_dynamic.json')
BIN = os.path.join(FIXTURES, 'cycles_dynamic.bin')
RS = os.path.join(FIXTURES, 'cycles_dynamic.rs')
TEXTS_RS = os.path.join(FIXTURES, 'cycles_dynamic_texts.rs')

# One instruction each, laid out in this order from h0+$0. A dbcc branches to the instruction after
# it, so both its ways stay inside the program.
ENCODINGS = [
    ([0xc0c1], 'mulu.w d1,d0'),
    ([0xc1c1], 'muls.w d1,d0'),
    ([0xc0fc, 0x0000], 'mulu.w #0,d0'),
    ([0xc0fc, 0xffff], 'mulu.w #$ffff,d0'),
    ([0xc0fc, 0x5555], 'mulu.w #$5555,d0'),
    ([0xc1fc, 0xffff], 'muls.w #-1,d0'),
    ([0xc1fc, 0xaaaa], 'muls.w #$aaaa,d0'),
    ([0x80c1], 'divu.w d1,d0'),
    ([0x81c1], 'divs.w d1,d0'),
    ([0x80fc, 0x0007], 'divu.w #7,d0'),
    ([0x80fc, 0x0001], 'divu.w #1,d0'),
    ([0x81fc, 0xfffd], 'divs.w #-3,d0'),
    ([0x81fc, 0x0001], 'divs.w #1,d0'),
    ([0x51c8, 0x0002], 'dbf d0,next'),
    ([0x50c8, 0x0002], 'dbt d0,next'),
    ([0x57c8, 0x0002], 'dbeq d0,next'),
    ([0x5ec9, 0x0002], 'dbgt d1,next'),
    ([0x48e1, 0x8000], 'movem.l d0,-(a1)'),
    ([0x48d1, 0x00ff], 'movem.l d0-d7,(a1)'),
    ([0x48e7, 0x7ffe], 'movem.l d1-d7/a0-a6,-(a7)'),
    ([0x48a9, 0xffff, 0x0010], 'movem.w d0-d7/a0-a7,16(a1)'),
    ([0x4cd9, 0x0001], 'movem.l (a1)+,d0'),
    ([0x4cd9, 0x00ff], 'movem.l (a1)+,d0-d7'),
    ([0x4c99, 0xfdff], 'movem.w (a1)+,d0-d7/a0/a2-a7'),
    ([0x4cf9, 0x00ff, 0x0000, 0x4000], 'movem.l $4000.l,d0-d7'),
    ([0x4e75], 'rts'),
]


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


def render():
    # Every site, not only each opcode word's first: the immediates and masks differ.
    wanted = [pr.parse_addr(a, 'site') for a in sites()]
    return pr.render_insn(pr.load_export(JSON), 'cycles_dynamic.json', wanted)[0]


def texts_rs():
    return ('// Written by tools/test_cycles_dynamic.py --regen: each site\'s text, in SITES order.\n'
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
    with open(RS, 'w') as f:
        f.write(render())
    print('wrote', ', '.join(os.path.basename(p) for p in (JSON, BIN, RS, TEXTS_RS)))


def load():
    return pr.load_export(JSON)['insns']


def by_text(text):
    a = next(a for a, t in sites().items() if t == text)
    return load()[pr.parse_addr(a, 'test')]


class Fixtures(unittest.TestCase):
    def test_the_fixtures_are_current(self):
        with open(BIN, 'rb') as f:
            self.assertEqual(f.read(), code_bytes(), 'cycles_dynamic.bin is stale: --regen')
        with open(TEXTS_RS) as f:
            self.assertEqual(f.read(), texts_rs(), 'cycles_dynamic_texts.rs is stale: --regen')
        insns = load()
        for a, text in sites().items():
            self.assertIn(pr.parse_addr(a, 't'), insns, f'{a} {text}: --regen')
        with open(RS) as f:
            self.assertEqual(f.read(), render(), 'cycles_dynamic.rs is stale: --regen')

    def test_ghidra_decoded_each_encoding_as_written(self):
        for a, text in sites().items():
            ins = load()[pr.parse_addr(a, 't')]
            self.assertEqual(ins['mnem'].split(' ')[0].split('.')[0].lower(), text.split(' ')[0].split('.')[0], a)


class Ticks(unittest.TestCase):
    def test_every_site_but_rts_ticks_once_before_its_own_lines(self):
        for _, text in ENCODINGS[:-1]:
            lines = cy.cycles(by_text(text))
            self.assertEqual(len(lines), 1, text)
            self.assertTrue(lines[0].startswith('r.tick('), text)

    def test_a_register_operand_is_read_at_run_time(self):
        self.assertEqual(cy.cycles(by_text('mulu.w d1,d0')), ['r.tick(cy::mulu_cycles(r.d(1) as u16));'])
        self.assertEqual(cy.cycles(by_text('divs.w d1,d0')),
                         ['r.tick(cy::divs_cycles(r.d(0) as i32, r.d(1) as i16));'])
        self.assertEqual(cy.cycles(by_text('dbgt d1,next')),
                         ['r.tick(cy::dbcc(cy::cc_holds(14, r), r.d(1) as u16));'])

    def test_an_immediate_multiply_and_a_movem_are_constants(self):
        self.assertEqual(cy.cycles(by_text('muls.w #$aaaa,d0')),
                         ['r.tick(const { cy::muls_cycles(0xaaaau16) + cy::ea_cycles(cy::Ea::Imm, cy::Size::W) });'])
        self.assertEqual(cy.cycles(by_text('movem.w (a1)+,d0-d7/a0/a2-a7')),
                         ['r.tick(const { cy::movem_cycles(cy::Dir::ToRegs, cy::Ea::PostInc, cy::Size::W, 15) });'])
        self.assertEqual(cy.cycles(by_text('movem.l d1-d7/a0-a6,-(a7)')),
                         ['r.tick(const { cy::movem_cycles(cy::Dir::ToMem, cy::Ea::PreDec, cy::Size::L, 14) });'])

    def test_an_immediate_divide_reads_its_dividend_at_run_time(self):
        self.assertEqual(cy.cycles(by_text('divu.w #7,d0')),
                         ['r.tick(cy::divu_cycles(r.d(0), 0x0007u16) + const { cy::ea_cycles(cy::Ea::Imm, cy::Size::W) });'])

    def test_a_memory_source_multiply_stops_rather_than_guess(self):
        # mulu.w (a0),d0 and divs.w 4(a2),d3: the operand would be read twice, so the run stops.
        mulu = {'mnem': 'mulu.w (A0),D0w', 'bytes': 'c0d0', 'operands': ['(A0)', 'D0w'], 'at': (0, 0x10)}
        self.assertEqual(cy.cycles(mulu), ['return Err(Stop::Unsupported { op: "cycles: mulu.w with a (An) '
                                           'source".into(), at: "h0+$10".into() });'])
        divs = {'mnem': 'divs.w (0x4,A2),D3', 'bytes': '87ea0004', 'operands': ['(0x4,A2)', 'D3'], 'at': (4, 0x2)}
        self.assertIn('cycles: divs.w with a (d16,An) source', cy.cycles(divs)[0])

    def test_the_rendered_fixture_ticks_each_site(self):
        with open(RS) as f:
            rs = f.read()
        self.assertEqual(rs.count('r.tick('), len(ENCODINGS))   # rts too, a fixed time


if __name__ == '__main__':
    if sys.argv[1:] == ['--regen']:
        regen()
    else:
        unittest.main()
