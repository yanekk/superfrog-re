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

Nothing for the data-dependent kinds (`DYNAMIC`, shifts and bit ops numbered by a register, `scc` on
a register), nor for an instruction without `operands` and `bytes` (a hand-written fixture from
before translator T04): T12 counts them.
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
       'andi': 'Andi', 'cmpi': 'Cmpi', 'addq': 'Quick', 'subq': 'Quick', 'clr': 'Clr', 'neg': 'Clr',
       'negx': 'Clr', 'not': 'Clr', 'tst': 'Tst', 'addx': 'Addx', 'subx': 'Addx', 'abcd': 'Bcd',
       'sbcd': 'Bcd', 'nbcd': 'Nbcd', 'ext': 'Ext', 'swap': 'Swap', 'exg': 'Exg', 'nop': 'Nop',
       'lea': 'Lea', 'pea': 'Pea', 'jsr': 'Jsr', 'jmp': 'Jmp', 'bra': 'Bra', 'bsr': 'Bsr', 'rts': 'Rts',
       'rte': 'Rte', 'rtr': 'Rtr', 'link': 'Link', 'unlk': 'Unlk'}
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
    return op, size, EA[src], EA[dst]


def tick(expr):
    return f'r.tick(const {{ {expr} }});'


def cycles(ins):
    k = None if base_mnemonic(ins) in BCC else kind(ins)
    if k is None:
        return []
    op, size, src, dst = k
    return [tick(f'cy::of(cy::Op::{op}, cy::Size::{size}, cy::Ea::{src}, cy::Ea::{dst})')]


def branch_cycles(ins):
    if base_mnemonic(ins) not in BCC:
        return None
    size = size_of(ins)
    return tick(f'cy::bcc(true, cy::Size::{size})'), tick(f'cy::bcc(false, cy::Size::{size})')
