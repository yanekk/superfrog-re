"""abcd and sbcd (nbcd: T07), emitted from their operands with Moira's arithmetic.

Ghidra 12.0's 68000.sinc writes abcd/sbcd's decimal adjust as a CALLOTHER to a user op no emulator
defines, and around it gets the instruction wrong (pcode-rt's `bcd` module lists how). An
instruction of that mnemonic whose P-code has the shape the sinc produces is emitted from its
operands instead; any other shape stays an unsupported stop.
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
