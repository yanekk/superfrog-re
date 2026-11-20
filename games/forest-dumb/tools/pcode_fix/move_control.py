"""The move_control family: move, movea, moveq, movem, lea, pea, link, unlk, exg, move to and from SR, CCR and USP, jmp,
jsr, bsr, rts, rte, rtr, dbcc, bcc, trap. Owned by T10.

What Ghidra 12.0's 68000.sinc gets wrong here, each corrected by mnemonic or opcode word and P-code
shape, never by address:

- The status register. The sinc keeps T, S and the mask in bytes of their own and its `packflags`
  ORs them into a one-byte temporary, which drops them: `move sr,<ea>` and `andi/ori/eori #,sr`
  lose the system byte. pcode-rt keeps it in `SR`'s high byte instead (`ops::sr_get`, `sr_set`,
  `ccr_set`), so in every instruction that touches SR or CCR the sinc's `packflags` becomes one
  SR_PACK op, its `unpackflags` one SR_UNPACK (a whole SR) or CCR_UNPACK (the low byte only), and
  its own uses of the `SR` register a temporary: only SR_UNPACK writes SR, and it sees the S bit from
  before, to swap A7 with USP when the mode changes.
- `andi/ori/eori #,ccr`, `move <ea>,ccr` and `rtr` write the whole SR in the sinc; the 68000 changes
  only X N Z V C, so their unpack is CCR_UNPACK.
- `rte` is `return [0]` in the sinc: it pops SR and then PC, and the new SR may leave supervisor mode.
- `movem.w <list>,-(An)` steps An by 4 a register in the sinc; a word is 2.

Ghidra already keeps a7 even for byte `-(a7)` and `(a7)+` in moves; `link`, `unlk`, `pea`, `exg`,
`dbcc`, `jsr`, `jmp`, `bsr` and `rts` are right as exported (fd-conform's tests/insn_moves.rs).
"""
from . import base_mnemonic

SR = ['register', 0x200, 2]
SP = ['register', 0x3c, 4]
PC = ['register', 0x50, 4]
MOVEMPTR = ['register', 0x304, 4]
FLAGS = list(range(0x40, 0x48))          # TF SVF IPL XF NF ZF VF CF
# Stands in for the sinc's own uses of `SR` in a corrected instruction; far above the uniques
# Ghidra numbers (a few hundred thousand at most).
SR_TEMP = ['unique', 0xfff000, 2]
RAM_SPACE = ['const', 417, 8]          # the LOAD/STORE space id Ghidra exports for ram

PACK_OPS = ['INT_LEFT', 'INT_LEFT', 'INT_OR'] + ['INT_LEFT', 'INT_OR'] * 5 + ['INT_OR', 'INT_ZEXT']
UNPACK_OPS = ['INT_AND', 'INT_NOTEQUAL'] * 2 + ['SUBPIECE', 'INT_AND'] + ['INT_AND', 'INT_NOTEQUAL'] * 5


def word(ins):
    """The opcode word, or None in an export without bytes."""
    raw = ins.get('bytes')
    return int(raw[:4], 16) if raw and len(raw) >= 4 else None


def touches(ins):
    """'sr' or 'ccr' for an instruction writing that register, by its opcode word; else None."""
    w = word(ins)
    if w is None:
        return None
    if w in (0x027c, 0x007c, 0x0a7c) or (w & 0xffc0) == 0x46c0:      # andi/ori/eori #,sr; move <ea>,sr
        return 'sr'
    if w in (0x023c, 0x003c, 0x0a3c, 0x4e77) or (w & 0xffc0) == 0x44c0:  # .. #,ccr; rtr; move <ea>,ccr
        return 'ccr'
    if (w & 0xffc0) == 0x40c0:                                        # move sr,<ea>: reads only
        return 'read'
    return None


def find_pack(ops):
    """(start, end exclusive) of the sinc's `packflags`, or None."""
    for i in range(len(ops) - len(PACK_OPS) + 1):
        run = ops[i:i + len(PACK_OPS)]
        if [op['op'] for op in run] != PACK_OPS:
            continue
        read = [v[1] for op in run for v in op['in'] if v[0] == 'register']
        if read == FLAGS and run[-1]['in'][0] == run[-2]['out']:
            return i, i + len(PACK_OPS)
    return None


def find_unpack(ops):
    """(start, end exclusive, the value unpacked) of the sinc's `unpackflags`, or None."""
    for i in range(len(ops) - len(UNPACK_OPS) + 1):
        run = ops[i:i + len(UNPACK_OPS)]
        if [op['op'] for op in run] != UNPACK_OPS:
            continue
        written = [op['out'][1] for op in run if op['out'][0] == 'register']
        v = run[0]['in'][0]
        if written == FLAGS and run[4]['in'][0] == v:
            return i, i + len(UNPACK_OPS), v
    return None


def rename_sr(op):
    def r(v):
        return SR_TEMP if v == SR else v
    out = dict(op, **{'in': [r(v) for v in op['in']]})
    if op.get('out'):
        out['out'] = r(op['out'])
    return out


def correct_sr(ins, kind):
    ops = ins['ops']
    pack = find_pack(ops)
    unpack = find_unpack(ops)
    if kind == 'read':
        if pack is None or unpack is not None:
            return ins
    elif unpack is None:
        return ins
    new = []
    k = 0
    while k < len(ops):
        if pack and k == pack[0]:
            new.append({'op': 'SR_PACK', 'out': ops[pack[1] - 1]['out'], 'in': []})
            k = pack[1]
        elif unpack and k == unpack[0]:
            new.append({'op': 'SR_UNPACK' if kind == 'sr' else 'CCR_UNPACK', 'out': None, 'in': [unpack[2]]})
            k = unpack[1]
        else:
            new.append(ops[k])
            k += 1
    return dict(ins, ops=[rename_sr(op) for op in new])


def correct_rte(ins):
    if ins['ops'] != [{'op': 'RETURN', 'out': None, 'in': [['const', 0, 4]]}]:
        return ins
    # The 68000 pops SR, then PC, then writes SR: a change to user mode keeps the popped SSP.
    return dict(ins, ops=[
        {'op': 'LOAD', 'out': SR_TEMP, 'in': [RAM_SPACE, SP]},
        {'op': 'INT_ADD', 'out': SP, 'in': [SP, ['const', 2, 4]]},
        {'op': 'LOAD', 'out': PC, 'in': [RAM_SPACE, SP]},
        {'op': 'INT_ADD', 'out': SP, 'in': [SP, ['const', 4, 4]]},
        {'op': 'SR_UNPACK', 'out': None, 'in': [SR_TEMP]},
        {'op': 'RETURN', 'out': None, 'in': [PC]}])


def correct_movem_w_predec(ins):
    ops = ins['ops']
    if not all(op['in'][2][2] == 2 for op in ops if op['op'] == 'STORE'):
        return ins
    fixed = [dict(op, **{'in': [MOVEMPTR, ['const', 2, 4]]})
             if op['op'] == 'INT_SUB' and op['out'] == MOVEMPTR and op['in'] == [MOVEMPTR, ['const', 4, 4]] else op
             for op in ops]
    return dict(ins, ops=fixed) if fixed != ops else ins


def correct(ins):
    """`ins` with its P-code corrected as the module's docstring lists; any other instruction unchanged."""
    m = base_mnemonic(ins)
    if m == 'rte':
        return correct_rte(ins)
    kind = touches(ins)
    if kind:
        return correct_sr(ins, kind)
    w = word(ins)
    if m == 'movem' and w is not None and (w & 0xfff8) == 0x48a0:
        return correct_movem_w_predec(ins)
    return ins


def op_lines(op, rd, wr):
    """The three ops `correct` puts in place of the sinc's `packflags` and `unpackflags`."""
    if op['op'] == 'SR_PACK':
        return [wr(op['out'], 'pcode_rt::ops::sr_get(r)')]
    if op['op'] == 'SR_UNPACK':
        return [f'pcode_rt::ops::sr_set(r, {rd(op["in"][0])});']
    if op['op'] == 'CCR_UNPACK':
        return [f'pcode_rt::ops::ccr_set(r, {rd(op["in"][0])});']
    return None
