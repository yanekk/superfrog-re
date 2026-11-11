"""Corrections to Ghidra's P-code, one module per instruction family (translator DESIGN §2.2, §7).

Each family is its own file so the tasks fixing them (T07 to T12) never edit the same one:

    bcd           abcd, sbcd (nbcd: T07)
    compare       cmp, cmpa, cmpi, cmpm
    arith         add .. chk (T08)
    shift_bit     shifts, rotates, bit ops, scc, tas, swap (T09)
    move_control  moves, movem, lea, pea, link, jumps, branches, returns, trap (T10)
    cycles        the cycles an instruction adds (T11, T12)

A module may define any of:

    correct(ins) -> ins          the instruction with its P-code corrected (unchanged when not its)
    emit(ins) -> [str] | None    Rust replacing the instruction's P-code whole, or None
    cycles(ins) -> [str]         Rust lines run after the instruction (T11)

`ins` is the translator's form ({'at', 'len', 'mnem', 'ops', 'targets', 'bytes'}), `at` its
(hunk, offset). A line `emit` returns may be a P-code op (a dict, no branch within the instruction),
which the translator writes as it writes any op, so a correction can keep the P-code it does not
replace. A correction is chosen by
mnemonic and P-code pattern, never by address: an address special case is a hand edit (§2.2).
`correct` runs every module in MODULES order; `emit` takes the first module answering.
Standard library only, so it runs under `python3 -I`.
"""
import importlib

def base_mnemonic(ins):
    """`cmpi.b #1,d3b` -> `cmpi`, lower case."""
    return ins['mnem'].split(' ')[0].split('.')[0].lower()


# After base_mnemonic, which the modules import from here.
NAMES = ('bcd', 'compare', 'arith', 'shift_bit', 'move_control', 'cycles')
MODULES = [importlib.import_module(f'{__name__}.{n}') for n in NAMES]


def correct(ins):
    for m in MODULES:
        f = getattr(m, 'correct', None)
        if f:
            ins = f(ins)
    return ins


def emit(ins):
    for m in MODULES:
        f = getattr(m, 'emit', None)
        lines = f(ins) if f else None
        if lines is not None:
            return lines, m.__name__.rsplit('.', 1)[1]
    return None, None


def cycles(ins):
    return [l for m in MODULES if getattr(m, 'cycles', None) for l in m.cycles(ins)]
