"""Compares: cmp, cmpa, cmpi, cmpm (T07 proves them on the instruction check).

The sinc's `subflags` macro, shared by sub and the compares, ends `XF = CF`; on the 68000 cmp, cmpa,
cmpi and cmpm leave X alone, and abcd, sbcd, addx and subx after a compare read the X from before it.
"""
from . import base_mnemonic

COMPARES = {'cmp', 'cmpa', 'cmpi', 'cmpm'}
XF_OFF = 67
CF_OFF = 71


def correct(ins):
    """`ins` with a compare's copy of C into X removed; any other instruction unchanged."""
    if base_mnemonic(ins) not in COMPARES:
        return ins
    ops = [op for op in ins['ops'] if not (op['op'] == 'COPY' and op['out'] == ['register', XF_OFF, 1]
                                           and op['in'] == [['register', CF_OFF, 1]])]
    return dict(ins, ops=ops) if len(ops) != len(ins['ops']) else ins
