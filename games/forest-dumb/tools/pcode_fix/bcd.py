"""abcd, sbcd and nbcd, carried out with Moira's arithmetic (pcode-rt's `bcd` module).

Ghidra 12.0's 68000.sinc writes their decimal adjust as a CALLOTHER to a user op no emulator
defines, and around it gets the instruction wrong (pcode-rt's `bcd` module lists how). abcd and sbcd
have two forms only, so one whose P-code has the shape the sinc produces is emitted from its operands
(`emit`). nbcd takes any alterable data address, so its P-code keeps Ghidra's addressing, load and
store, and only the arithmetic and flags are replaced by one NBCD op (`correct`, `op_lines`). Any
other shape under these mnemonics stays an unsupported stop.
"""
from . import base_mnemonic

BCD = {'abcd': ('INT_ADD', 'bcd_add'), 'sbcd': ('INT_SUB', 'bcd_sub')}
DATA_BYTES = {4 * n + 3 for n in range(8)}     # Dn's low byte in Ghidra's register space
ADDR_REGS = {32 + 4 * n: n for n in range(8)}   # An
XF_OFF = 67


def hex_lit(v):
    return f'0x{v:x}'


def operands(ins):
    """('reg', dst, src) register offsets, ('mem', ax, ay) address-register offsets, or None."""
    mnem = base_mnemonic(ins)
    if mnem not in BCD:
        return None
    ops = ins['ops']
    if sum(op['op'] == 'CALLOTHER' for op in ops) != 1:
        return None
    arith = next((op for op in ops if op['op'] == BCD[mnem][0] and len(op['in']) == 2
                  and all(v[2] == 1 for v in op['in']) and op['in'][1][:2] != ['register', XF_OFF]), None)
    if arith is None:
        return None
    dst, src = arith['in']
    if dst[0] == src[0] == 'register' and {dst[1], src[1]} <= DATA_BYTES:
        return ('reg', dst[1], src[1])
    if dst[0] == src[0] == 'unique':
        # Memory form: each operand is a byte loaded through an address register that the
        # instruction predecrements, and the result is stored through the destination's.
        loaded = {}
        for op in ops:
            if op is arith:
                break
            if op['op'] == 'LOAD' and op['in'][1][0] == 'register' and op['in'][1][2] == 4:
                loaded[op['out'][1]] = op['in'][1][1]
        ax, ay = loaded.get(dst[1]), loaded.get(src[1])
        dec = {op['in'][0][1] for op in ops if op['op'] == 'INT_SUB' and op['out'] == op['in'][0]
               and op['in'][0][0] == 'register' and op['in'][1][:2] == ['const', 1]}
        stores = {op['in'][1][1] for op in ops if op['op'] == 'STORE' and op['in'][1][0] == 'register'}
        if ax in ADDR_REGS and ay in ADDR_REGS and {ax, ay} <= dec and stores == {ax}:
            return ('mem', ax, ay)
    return None


def emit(ins):
    form = operands(ins)
    if form is None:
        return None
    fn = 'pcode_rt::bcd::' + BCD[base_mnemonic(ins)][1]
    kind, x, y = form
    if kind == 'reg':
        return [f'let (v, x, o) = {fn}(r.get({hex_lit(y)}, 1), r.get({hex_lit(x)}, 1), r.get({hex_lit(XF_OFF)}, 1));',
                f'r.set({hex_lit(x)}, 1, v);',
                'pcode_rt::bcd::bcd_flags(r, v, x, o);']

    def step(a):
        # The 68000 keeps A7 even: a byte predecrement of it steps 2 (Ghidra's sinc steps 1).
        return 2 if ADDR_REGS[a] == 7 else 1
    # The source is decremented and read first, then the destination, as the 68000 does; Ghidra
    # does the destination first, which differs when both are the same register.
    return [f'let ay = (r.get({hex_lit(y)}, 4) as u32).wrapping_sub({step(y)});',
            f'r.set({hex_lit(y)}, 4, ay as u64);',
            'let s = load(bus, ay, 1);',
            f'let ax = (r.get({hex_lit(x)}, 4) as u32).wrapping_sub({step(x)});',
            f'r.set({hex_lit(x)}, 4, ax as u64);',
            'let d = load(bus, ax, 1);',
            f'let (v, x, o) = {fn}(s, d, r.get({hex_lit(XF_OFF)}, 1));',
            'store(bus, img, ax, 1, v)?;',
            'pcode_rt::bcd::bcd_flags(r, v, x, o);']


# nbcd: `tmp = <ea>; CF = ..; tmp = 0 - tmp - XF; <ea> = bcdAdjust(tmp); XF = CF; ZF = ..` in the sinc.
NBCD = 'NBCD'


def correct(ins):
    """nbcd with the sinc's arithmetic and flags replaced by one NBCD op: (out = the destination the
    CALLOTHER wrote, in = [the byte read]). Any other instruction, or another shape, unchanged."""
    if base_mnemonic(ins) != 'nbcd':
        return ins
    ops = ins['ops']
    calls = [k for k, op in enumerate(ops) if op['op'] == 'CALLOTHER']
    if len(calls) != 1:
        return ins
    c = calls[0]
    adj = ops[c]
    if len(adj['in']) != 2 or not adj.get('out') or adj['out'][2] != 1:
        return ins
    tmp = adj['in'][1]
    # The read: the first COPY into the CALLOTHER's input, a byte.
    read = next((k for k, op in enumerate(ops[:c]) if op['op'] == 'COPY' and op['out'] == tmp), None)
    if read is None or tmp[:1] != ['unique'] or tmp[2] != 1:
        return ins
    # Between the read and the CALLOTHER only the sinc's carry and `0 - tmp - XF`; after it only its
    # store of the result and the flags.
    middle = {op['op'] for op in ops[read + 1:c]}
    if not middle <= {'INT_NOTEQUAL', 'INT_EQUAL', 'BOOL_OR', 'INT_SUB'}:
        return ins
    tail = ops[c + 1:]
    stores = [op for op in tail if op['op'] == 'STORE']
    if any(op['op'] not in ('STORE', 'COPY', 'INT_EQUAL', 'INT_MULT', 'INT_NOTEQUAL', 'INT_ADD') for op in tail) \
            or any(op['in'][2] != adj['out'] for op in stores) \
            or any(op['op'] != 'STORE' and op['out'][0] == 'ram' for op in tail):
        return ins
    nbcd = {'op': NBCD, 'out': adj['out'], 'in': [tmp]}
    return dict(ins, ops=ops[:read + 1] + [nbcd] + stores)


def op_lines(op, rd, wr):
    """`0 - byte - X`, decimal, as Moira's nbcd does it (its `bcd<SBCD>(byte, 0)`), and its flags."""
    if op['op'] != NBCD:
        return None
    return [f'let (v, x, o) = pcode_rt::bcd::bcd_sub({rd(op["in"][0])}, 0, r.get({hex_lit(XF_OFF)}, 1));',
            wr(op['out'], 'v'),
            'pcode_rt::bcd::bcd_flags(r, v, x, o);']
