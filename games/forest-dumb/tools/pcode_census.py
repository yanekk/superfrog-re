#!/usr/bin/env python3
"""Count every instruction kind a P-code export holds: stage 1's work list (translator DESIGN §2.6).

    python3 -I pcode_census.py <pcode.json> --out <census.json>

A kind is the mnemonic as Ghidra gives it (size suffix included) with the addressing mode of each
operand, in Ghidra's operand order: `add.w Dn,(An)+`. census.json is a list, commonest kind first:

    {"kind", "mnemonic", "size", "src", "dst", "count", "encodings", "sites"}

`size` is 1, 2 or 4 from the suffix, null without one. `src` is the first operand's mode and `dst` the
second's, null when absent (a lone operand is `src`). `encodings` are the distinct opcode words, sorted;
`sites` the first five addresses. Prints one JSON line, {"kinds":N,"instructions":N}; exits 2 on a bad
input or an operand whose mode it cannot tell, naming it on stderr.

Modes come from Ghidra's operand text (`operands`, ExportPcode.java) where that text names one, and
from the opcode word's effective-address field where it does not: Ghidra writes movem's `-(An)` and
`(An)+` as a bare `An`, an absolute `jsr` as a bare number and `jsr (d16,An)` as two operands, a
number and the register, so `jsr`, `jmp` and `pea` take their one mode from the opcode word alone.

Python's standard library only, so it runs under `python3 -I`.
"""
import argparse, json, os, re, sys

SIZES = {'b': 1, 'w': 2, 'l': 4}
NUM = r'-?(?:0x[0-9a-fA-F]+|\d+)'
AREG = r'(?:A[0-7]|SP)'
XREG = r'(?:[AD][0-7][wl]?|SP)'
TEXT = [  # (pattern, mode), first match wins
    (rf'D[0-7][bwl]?', 'Dn'),
    (AREG, 'An'),
    (rf'\({AREG}\)', '(An)'),
    (rf'\({AREG}\)\+', '(An)+'),
    (rf'-\({AREG}\)', '-(An)'),
    (rf'\({NUM},{AREG}\)', '(d16,An)'),
    (rf'\({NUM},{AREG},{XREG}(?:\*(?:0x)?\d)?\)', '(d8,An,Xn)'),
    (rf'\({NUM}\)\.w', 'abs.w'),
    (rf'\({NUM}\)\.l', 'abs.l'),
    (rf'\({NUM},PC\)', '(d16,PC)'),
    (rf'\({NUM},PC,{XREG}(?:\*(?:0x)?\d)?\)', '(d8,PC,Xn)'),
    (rf'#{NUM}', '#imm'),
    (r'\{[^}]*\}', 'list'),
    (r'SR', 'SR'), (r'CCR', 'CCR'), (r'USP', 'USP'),
]
TEXT = [(re.compile(p + '$'), m) for p, m in TEXT]
# The 6-bit effective-address field (mode, register) of an opcode word.
EA = {0: 'Dn', 1: 'An', 2: '(An)', 3: '(An)+', 4: '-(An)', 5: '(d16,An)', 6: '(d8,An,Xn)'}
EA7 = {0: 'abs.w', 1: 'abs.l', 2: '(d16,PC)', 3: '(d8,PC,Xn)', 4: '#imm'}
# A bare number is a branch's target, or the quick immediate of these.
QUICK = {'moveq', 'addq', 'subq'}
# One effective address and nothing else, whatever Ghidra's text splits it into.
EA_ONLY = {'jsr', 'jmp', 'pea'}


class BadInput(Exception):
    pass


def ea_mode(word):
    mode, reg = (word >> 3) & 7, word & 7
    if mode < 7:
        return EA[mode]
    if reg in EA7:
        return EA7[reg]
    raise ValueError(f'mode 7 register {reg}')


def operand_mode(text, i, mnem, word, where):
    """The addressing mode of operand `i` (Ghidra's text `text`) of an instruction `mnem`."""
    base = mnem.split('.')[0]
    ea = lambda: ea_mode(word)
    try:
        if base == 'movem' and re.fullmatch(AREG, text):
            return ea()           # -(An) or (An)+, written as a bare register
        if re.fullmatch(NUM, text):
            if base in QUICK:
                return '#imm'
            if base == 'lea':
                return ea()       # an absolute source, written as a number
            if base.startswith('b') or base.startswith('db'):
                return 'label'
        for pat, mode in TEXT:
            if pat.match(text):
                return mode
    except ValueError as e:
        raise BadInput(f'{where}: operand {i} {text!r}: {e}')
    raise BadInput(f'{where}: operand {i} {text!r} of {mnem}: no addressing mode')


def modes_of(mnem, ops, word, where):
    if mnem.split('.')[0] in EA_ONLY:
        try:
            return [ea_mode(word)]
        except ValueError as e:
            raise BadInput(f'{where}: {mnem}: {e}')
    if len(ops) > 2:
        raise BadInput(f'{where}: {len(ops)} operands')
    return [operand_mode(o, i, mnem, word, where) for i, o in enumerate(ops)]


def census(doc, path='pcode.json'):
    if not isinstance(doc, dict) or not isinstance(doc.get('insns'), dict):
        raise BadInput(f'{path}: no "insns" object')
    kinds = {}
    def order(a):
        h, off = a.split('+$')
        return int(h[1:]), int(off, 16)
    try:
        addrs = sorted(doc['insns'], key=order)
    except (ValueError, IndexError):
        raise BadInput(f'{path}: an instruction address is not hN+$off')
    for at in addrs:
        ins = doc['insns'][at]
        where = f'{path}: {at}'
        mnem = ins.get('mnem') if isinstance(ins, dict) else None
        ops, raw = (ins.get('operands'), ins.get('bytes')) if isinstance(ins, dict) else (None, None)
        if not isinstance(mnem, str) or not mnem:
            raise BadInput(f'{where}: no "mnem"')
        if not isinstance(ops, list) or not all(isinstance(o, str) for o in ops):
            raise BadInput(f'{where}: no "operands" (an export from before translator T04?)')
        if not isinstance(raw, str) or len(raw) < 4 or not re.fullmatch(r'[0-9a-fA-F]+', raw):
            raise BadInput(f'{where}: no "bytes"')
        modes = modes_of(mnem, ops, int(raw[:4], 16), where)
        src = modes[0] if modes else None
        dst = modes[1] if len(modes) > 1 else None
        kind = mnem + (' ' + ','.join(modes) if modes else '')
        k = kinds.get(kind)
        if k is None:
            suffix = mnem.rsplit('.', 1)[1] if '.' in mnem else ''
            k = kinds[kind] = {'kind': kind, 'mnemonic': mnem.split('.')[0], 'size': SIZES.get(suffix),
                               'src': src, 'dst': dst, 'count': 0, 'encodings': set(), 'sites': []}
        k['count'] += 1
        k['encodings'].add(raw[:4].lower())
        if len(k['sites']) < 5:
            k['sites'].append(at)
    out = sorted(kinds.values(), key=lambda k: (-k['count'], k['kind']))
    for k in out:
        k['encodings'] = sorted(k['encodings'])
    return out


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.split('\n\n')[0])
    p.add_argument('pcode')
    p.add_argument('--out', required=True)
    args = p.parse_args(argv)
    try:
        try:
            with open(args.pcode, encoding='utf-8') as f:
                doc = json.load(f)
        except OSError as e:
            raise BadInput(f'{args.pcode}: cannot read: {e.strerror}')
        except json.JSONDecodeError as e:
            raise BadInput(f'{args.pcode}: not JSON: {e}')
        rows = census(doc, args.pcode)
    except BadInput as e:
        print(f'pcode_census: {e}', file=sys.stderr)
        return 2
    tmp = args.out + '.tmp'
    try:
        with open(tmp, 'w', encoding='utf-8') as f:
            json.dump(rows, f, indent=1)
            f.write('\n')
        os.replace(tmp, args.out)
    except OSError as e:
        print(f'pcode_census: {args.out}: cannot write: {e.strerror}', file=sys.stderr)
        return 2
    print(json.dumps({'kinds': len(rows), 'instructions': sum(r['count'] for r in rows)}, separators=(',', ':')))
    return 0


if __name__ == '__main__':
    sys.exit(main())
