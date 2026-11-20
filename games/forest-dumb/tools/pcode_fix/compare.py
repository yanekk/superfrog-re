"""Compares: cmp, cmpa, cmpi, cmpm.

The sinc's `subflags` macro, shared by sub and the compares, ends `XF = CF`; on the 68000 cmp, cmpa,
cmpi and cmpm leave X alone, and abcd, sbcd, addx and subx after a compare read the X from before it.

`cmpm.b (Ay)+,(Ax)+` steps each register by 1 in the sinc; the 68000 keeps A7 even, so a byte
postincrement of A7 steps 2, as the sinc's own `(A7)+` addressing mode does elsewhere.
"""
from . import base_mnemonic

COMPARES = {'cmp', 'cmpa', 'cmpi', 'cmpm'}
XF_OFF = 67
CF_OFF = 71
A7 = ['register', 60, 4]


def correct(ins):
    """`ins` with a compare's copy of C into X removed and `cmpm.b`'s A7 stepping 2; any other
    instruction unchanged."""
    if base_mnemonic(ins) not in COMPARES:
        return ins
    ops = [op for op in ins['ops'] if not (op['op'] == 'COPY' and op['out'] == ['register', XF_OFF, 1]
                                           and op['in'] == [['register', CF_OFF, 1]])]
    if ins['mnem'].lower().startswith('cmpm.b'):
        ops = [dict(op, **{'in': [A7, ['const', 2, 4]]}) if op['op'] == 'INT_ADD' and op['out'] == A7
               and op['in'] == [A7, ['const', 1, 4]] else op for op in ops]
    return dict(ins, ops=ops) if ops != ins['ops'] else ins
