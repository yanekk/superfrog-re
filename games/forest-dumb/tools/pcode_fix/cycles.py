"""The cycles each instruction adds to the counter (T11 the fixed part, T12 the data-dependent part).

The timing table is Rust's (`pcode_rt::cycles`), held once: this module only names an instruction's
kind there, from its mnemonic and the census's addressing modes (`pcode_census.modes_of`), and the
generated code evaluates `cy::of(..)` inside `const { }`, so each instruction's time is a constant
and a kind the table lacks fails the build instead of counting zero.

- `cycles(ins)`: the line run before the instruction's own, `r.tick(..)`, for every fixed-time kind
  but a conditional branch. Ticking first means an instruction leaving by a jump or a return has
  counted its time already.
- `branch_cycles(ins)`: for `bcc`, (taken, not taken) lines: the translator puts the first on the
  CBRANCH's jump and the second after the instruction, so each path counts its own time.

- The data-dependent kinds (`is_dynamic`, T12): `cycles(ins)` is still one `r.tick(..)` before the
  instruction, its time computed by a `pcode_rt::cycles` helper from the registers the instruction is
  about to read (`dynamic`): the source of `mulu`/`muls`, dividend and divisor of `divu`/`divs`, the
  count of a shift by a register, the bit number of a bit op by a register, the condition and
  counter of `dbcc`, the condition of `scc` on a register. `movem` is a constant by its mask.
  A multiply or divide with a memory source would need its operand read twice (a custom register's
  read is not free), so it stops the run, `Stop::Unsupported` naming its cycles, rather than count a
  guess; the game has none. `trap`, `trapv` and `chk` are counted by nothing: Ghidra writes their
  exception as a CALLOTHER, an unsupported stop, and the machine (T23) owns the exception's time.

Nothing for an instruction without `operands` and `bytes` (a hand-written fixture from before
translator T04).
"""
import os
import sys

from . import base_mnemonic

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import pcode_census  # noqa: E402

CC = ('hi', 'ls', 'cc', 'cs', 'ne', 'eq', 'vc', 'vs', 'pl', 'mi', 'ge', 'lt', 'gt', 'le', 'hs', 'lo')
SIZE = {'b': 'B', 'w': 'W', 'l': 'L'}
EA = {None: 'None', 'label': 'None', 'Dn': 'Dn', 'An': 'An', '(An)': 'Ind', '(An)+': 'PostInc',
      '-(An)': 'PreDec', '(d16,An)': 'D16An', '(d8,An,Xn)': 'D8AnXn', 'abs.w': 'AbsW', 'abs.l': 'AbsL',
      '(d16,PC)': 'D16Pc', '(d8,PC,Xn)': 'D8PcXn', '#imm': 'Imm'}
# Mnemonic -> timing class, where the class does not hang on the operands.
OPS = {'move': 'Move', 'movea': 'Movea', 'moveq': 'Moveq', 'cmp': 'Cmp', 'adda': 'Adda', 'suba': 'Adda',
       'cmpa': 'Cmpa', 'cmpm': 'Cmpm', 'addi': 'Imm', 'subi': 'Imm', 'ori': 'Imm', 'eori': 'Imm',
       'andi': 'Imm', 'cmpi': 'Cmpi', 'addq': 'Quick', 'subq': 'Quick', 'clr': 'Clr', 'neg': 'Clr',
       'negx': 'Clr', 'not': 'Clr', 'tst': 'Tst', 'addx': 'Addx', 'subx': 'Addx', 'abcd': 'Bcd',
       'sbcd': 'Bcd', 'nbcd': 'Nbcd', 'ext': 'Ext', 'swap': 'Swap', 'exg': 'Exg', 'nop': 'Nop',
       'lea': 'Lea', 'pea': 'Pea', 'jsr': 'Jsr', 'jmp': 'Jmp', 'bra': 'Bra', 'bsr': 'Bsr', 'rts': 'Rts',
       'rte': 'Rte', 'rtr': 'Rtr', 'link': 'Link', 'unlk': 'Unlk', 'tas': 'Tas'}
ALU = {'add', 'sub', 'and', 'or', 'eor'}
SHIFTS = {'asl', 'asr', 'lsl', 'lsr', 'rol', 'ror', 'roxl', 'roxr'}
SCC = {'st', 'sf'} | {'s' + c for c in CC}
BCC = {'b' + c for c in CC}
# Data-dependent whatever the operands (T12).
DYNAMIC = {'mulu', 'muls', 'divu', 'divs', 'movem', 'trap', 'trapv', 'chk', 'dbra', 'dbt', 'dbf'} | {
    'db' + c for c in CC}
# Operands naming a status register are move_control's own (T10) and outside this table.
SPECIAL = {'SR', 'CCR', 'USP'}


def is_dynamic(mnemonic, src, dst):
    """Whether a census kind's time depends on data (T12's), by mnemonic and modes."""
    m = mnemonic.lower()
    return (m in DYNAMIC or (m in SHIFTS and src == 'Dn') or (m in SCC and src == 'Dn')
            or (m in ('bset', 'bclr', 'bchg') and src == 'Dn' and dst == 'Dn'))


def modes(ins):
    """(src, dst) as the census names them, or None when the instruction lacks its text or bytes."""
    raw, ops = ins.get('bytes'), ins.get('operands')
    if not raw or len(raw) < 4 or not isinstance(ops, list):
        return None
    try:
        ms = pcode_census.modes_of(ins['mnem'], ops, int(raw[:4], 16), ins['mnem'])
    except (pcode_census.BadInput, ValueError):
        return None
    return (ms[0] if ms else None), (ms[1] if len(ms) > 1 else None)


def size_of(ins):
    parts = ins['mnem'].lower().split(' ')[0].split('.')
    return SIZE.get(parts[1] if len(parts) > 1 else 'l', 'L')


def kind(ins):
    """(op, size, src, dst) as Rust names them, or None for a kind this module does not count."""
    md = modes(ins)
    if md is None:
        return None
    src, dst = md
    m = base_mnemonic(ins)
    if src in SPECIAL or dst in SPECIAL or src not in EA or dst not in EA or is_dynamic(m, src, dst):
        return None
    size = size_of(ins)
    word = int(ins['bytes'][:4], 16)
    if m in OPS:
        op = OPS[m]
    elif m in ALU:
        op = 'AluToDn' if dst == 'Dn' else 'AluToMem'
    elif m in SHIFTS:
        if src == '#imm' and dst == 'Dn':
            op = f'Shift({(word >> 9) & 7 or 8})'
        elif dst is None:
            op, size = 'Shift(1)', 'W'
        else:
            return None
    elif m in ('btst', 'bset', 'bchg', 'bclr'):
        bit = 0
        if src == '#imm':
            if len(ins['bytes']) < 8:
                return None
            bit = int(ins['bytes'][4:8], 16) & 31
        op = {'btst': 'Btst', 'bclr': f'Bclr({bit})'}.get(m, f'Bset({bit})')
    elif m in SCC:
        op = 'Scc'
    else:
        return None
    if op in ('Scc', 'Nbcd', 'Tas'):
        size = 'B'   # byte ops Ghidra writes with no suffix
    return op, size, EA[src], EA[dst]


def tick(expr):
    return f'r.tick(const {{ {expr} }});'


def tick_rt(expr):
    """A tick whose time is computed at run time, before the instruction."""
    return f'r.tick({expr});'


MULDIV = {'mulu': 'mulu_cycles', 'muls': 'muls_cycles', 'divu': 'divu_cycles', 'divs': 'divs_cycles'}


def fmt_at(ins):
    h, off = ins['at']
    return f'h{h}+${off:x}'


def uncounted(ins, why):
    """A stop in place of a time this module cannot count without guessing."""
    return [f'return Err(Stop::Unsupported {{ op: "cycles: {why}".into(), at: "{fmt_at(ins)}".into() }});']


# movem's memory operand by the opcode word's mode and register fields; Ghidra's text for some modes
# (`16(a1)` as two operands, an absolute address) is more than `pcode_census.modes_of` reads.
MOVEM_EA = {2: 'Ind', 3: 'PostInc', 4: 'PreDec', 5: 'D16An', 6: 'D8AnXn',
            (7, 0): 'AbsW', (7, 1): 'AbsL', (7, 2): 'D16Pc', (7, 3): 'D8PcXn'}


def movem(ins):
    """movem's tick, a constant by its mask; None for anything else."""
    raw = ins.get('bytes') or ''
    if base_mnemonic(ins) != 'movem' or len(raw) < 8:
        return None
    w = int(raw[:4], 16)
    mode, reg = (w >> 3) & 7, w & 7
    ea = MOVEM_EA.get((7, reg) if mode == 7 else mode)
    if w & 0xfb80 != 0x4880 or ea is None:
        return None
    d = 'ToRegs' if (w >> 10) & 1 else 'ToMem'
    sz = 'L' if (w >> 6) & 1 else 'W'
    n = bin(int(raw[4:8], 16)).count('1')
    return [tick(f'cy::movem_cycles(cy::Dir::{d}, cy::Ea::{ea}, cy::Size::{sz}, {n})')]


def dynamic(ins):
    """The tick lines of a data-dependent kind, or None when `ins` is not one this module counts."""
    if base_mnemonic(ins) == 'movem':
        return movem(ins)
    md = modes(ins)
    if md is None:
        return None
    src, dst = md
    m = base_mnemonic(ins)
    if not is_dynamic(m, src, dst):
        return None
    raw = ins['bytes']
    w = int(raw[:4], 16)
    size = f'cy::Size::{size_of(ins)}'
    if m in MULDIV:
        if size_of(ins) != 'W':
            return None   # the 68020's long forms: not a 68000 instruction
        f, dn = MULDIV[m], (w >> 9) & 7
        word = 'i16' if m == 'divs' else 'u16'
        imm = 'cy::ea_cycles(cy::Ea::Imm, cy::Size::W)'
        if src == 'Dn':
            v = f'r.d({w & 7}) as {word}'
        elif src == '#imm' and len(raw) >= 8:
            v = f'0x{raw[4:8]}u16' + (' as i16' if m == 'divs' else '')
        else:
            return uncounted(ins, f'{m}.w with a {src} source')
        if m.startswith('mul'):
            if src == '#imm':
                return [tick(f'cy::{f}({v}) + {imm}')]
            return [tick_rt(f'cy::{f}({v})')]
        ea = f' + const {{ {imm} }}' if src == '#imm' else ''
        dividend = f'r.d({dn}) as i32' if m == 'divs' else f'r.d({dn})'
        return [tick_rt(f'cy::{f}({dividend}, {v}){ea}')]
    if m in SHIFTS:
        return [tick_rt(f'cy::shift_cycles({size}, r.d({(w >> 9) & 7}))')]
    if m in SCC:
        return [tick_rt(f'cy::scc_reg(cy::cc_holds({(w >> 8) & 15}, r))')]
    if m in ('bset', 'bchg', 'bclr'):
        op = 'Bclr' if m == 'bclr' else 'Bset'
        return [tick_rt(f'cy::of(cy::Op::{op}(r.d({(w >> 9) & 7}) & 31), cy::Size::L, cy::Ea::Dn, cy::Ea::Dn)')]
    if m.startswith('db') and w & 0xf0f8 == 0x50c8:
        return [tick_rt(f'cy::dbcc(cy::cc_holds({(w >> 8) & 15}, r), r.d({w & 7}) as u16)')]
    return None


def cycles(ins):
    if base_mnemonic(ins) in BCC:
        return []
    lines = dynamic(ins)
    if lines is not None:
        return lines
    k = kind(ins)
    if k is None:
        return []
    op, size, src, dst = k
    return [tick(f'cy::of(cy::Op::{op}, cy::Size::{size}, cy::Ea::{src}, cy::Ea::{dst})')]


def branch_cycles(ins):
    if base_mnemonic(ins) not in BCC:
        return None
    size = size_of(ins)
    return tick(f'cy::bcc(true, cy::Size::{size})'), tick(f'cy::bcc(false, cy::Size::{size})')
