"""The shift_bit family: asl, asr, lsl, lsr, rol, ror, roxl, roxr, btst, bset, bclr, bchg, scc, tas, swap. Owned by
T09.

Shifts and rotates of a data register are emitted whole, from their opcode word, through
`pcode_rt::ops::shift68k`, which steps them as Moira does. Ghidra 12.0's sinc gets them wrong in
three ways (measured by `tests/insn_shift_bit.rs` on Moira):

- V is the sign before against the sign after: a logical shift must clear V, and `asl` must set it
  when the sign changes at any step, which before against after misses once the count passes 1;
- a rotate by a register count past the operand's size leaves the wrong value;
- `asr` by a register count past the size leaves the wrong X.

The memory forms (one step, a word) and the bit ops, scc, tas and swap match Moira as Ghidra writes
them, and are left alone. An instruction is taken only when its opcode word is a register-form
shift and its mnemonic and size say the same; anything else keeps its P-code.
"""
from . import base_mnemonic

TYPES = ('as', 'ls', 'rox', 'ro')
SIZES = {0: 1, 1: 2, 2: 4}
SUFFIX = {1: 'b', 2: 'w', 4: 'l'}
XF_OFF, NF_OFF, ZF_OFF, VF_OFF, CF_OFF = 0x43, 0x44, 0x45, 0x46, 0x47


def hex_lit(v):
    return f'0x{v:x}'


def register_shift(ins):
    """(kind, left, size, count register or None, immediate count, data register), or None."""
    raw = ins.get('bytes')
    if not raw or len(raw) < 4 or ins.get('len') != 2:
        return None
    w = int(raw[:4], 16)
    if w & 0xf000 != 0xe000 or (w >> 6) & 3 == 3:
        return None
    kind, left, size = (w >> 3) & 3, (w >> 8) & 1, SIZES[(w >> 6) & 3]
    parts = ins['mnem'].lower().split(' ')[0].split('.')
    if base_mnemonic(ins) != TYPES[kind] + 'lr'[1 - left] or parts[1:] != [SUFFIX[size]]:
        return None
    field = (w >> 9) & 7
    if (w >> 5) & 1:
        return kind, left, size, field, None, w & 7
    return kind, left, size, None, field or 8, w & 7


def emit(ins):
    form = register_shift(ins)
    if form is None:
        return None
    kind, left, size, creg, imm, dreg = form
    # Dn's low byte, word and long in Ghidra's register space (big-endian, 4 bytes a register).
    dst = hex_lit(4 * dreg + 4 - size)
    cnt = f'r.get({hex_lit(4 * creg)}, 4) & 0x3f' if creg is not None else f'{imm}u64'
    return [f'let cnt = {cnt};',
            f'let [v, x, n, z, o, c] = pcode_rt::ops::shift68k({kind}, {"true" if left else "false"}, {size}, '
            f'cnt, r.get({dst}, {size}), r.get({hex_lit(XF_OFF)}, 1));',
            f'r.set({dst}, {size}, v);',
            f'r.set({hex_lit(XF_OFF)}, 1, x);',
            f'r.set({hex_lit(NF_OFF)}, 1, n);',
            f'r.set({hex_lit(ZF_OFF)}, 1, z);',
            f'r.set({hex_lit(VF_OFF)}, 1, o);',
            f'r.set({hex_lit(CF_OFF)}, 1, c);']
