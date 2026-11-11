"""The arith family: add, addx, addq, addi, adda, sub, subx, subq, subi, suba, neg, negx, clr, mulu, muls, divu,
divs, and, or, eor, not and their immediates, ext, tst, chk. Owned by T08.

Three corrections, each by mnemonic and P-code shape (Ghidra 12.0's 68000.sinc), checked against Moira by
`insn_check.py --kinds arith`:

    addx      the sinc's `addxflags` sets C but never copies it to X; the 68000 sets X = C.
    divu/divs the sinc divides with no overflow test and no divide trap: a quotient over a word lands
              in the destination, and P-code's division by zero gives 0. Emitted as a call to
              `pcode_rt::ops::divu_w`/`divs_w`, which follow Moira; a zero divisor is `Stop::Trap`
              vector 5.
    clr       the 68000's clr reads its operand before writing 0, and a word read of a write-only custom
              register writes the data bus's last word into it (the reference's `peekCustomFaulty16`):
              that word is the prefetch, the word after the instruction. The sinc has no read, so a
              word or long clr of memory writes that word first when its address is such a register
              (`clr.w AUD0VOL` writes the next opcode as a volume, then 0). A clr.l writes its low word
              first, as the 68000 does.

Not in the census, so not corrected: the sinc's subx V ignores X, and negx's C and V are computed from the
wrapped `src + X` (FINDINGS). Their Z (only ever cleared) is already right.
"""
from . import base_mnemonic

XF, NF, ZF, VF, CF = 0x43, 0x44, 0x45, 0x46, 0x47
EXTENDED = {'addx', 'subx', 'negx'}


def hex_lit(v):
    return f'0x{v:x}'


def size_of(ins):
    """The instruction's size in bytes from its mnemonic (`clr.w` -> 2), or None."""
    parts = ins['mnem'].split(' ')[0].split('.')
    return {'b': 1, 'w': 2, 'l': 4}.get(parts[1].lower()) if len(parts) > 1 else None


def fmt_at(ins):
    h, off = ins['at']
    return f'h{h}+${off:x}'


# --- addx, subx, negx: X is C -----------------------------------------------------------------------

def correct(ins):
    """`ins` with X = C appended when an extended op sets C and never X; any other instruction unchanged."""
    if base_mnemonic(ins) not in EXTENDED:
        return ins
    outs = [op.get('out') for op in ins['ops']]
    if ['register', XF, 1] in outs or ['register', CF, 1] not in outs:
        return ins
    copy = {'op': 'COPY', 'out': ['register', XF, 1], 'in': [['register', CF, 1]]}
    return dict(ins, ops=ins['ops'] + [copy])


# --- divu, divs ---------------------------------------------------------------------------------------

DIV = {'divu': ('INT_DIV', 'divu_w'), 'divs': ('INT_SDIV', 'divs_w')}


def divide(ins):
    """Rust for a divu.w/divs.w in the sinc's shape: its operand fetch kept as P-code, the rest a helper
    call; None for any other instruction or shape."""
    mnem = base_mnemonic(ins)
    if mnem not in DIV or size_of(ins) != 2:
        return None
    ops = ins['ops']
    k = next((i for i, op in enumerate(ops) if op['op'] == DIV[mnem][0]), None)
    if k is None or sum(op['op'] == DIV[mnem][0] for op in ops) != 1:
        return None
    dividend, divisor = ops[k]['in']
    last = ops[-1]
    if (dividend[0] != 'unique' or divisor[0] != 'unique' or last['op'] != 'INT_OR'
            or last['out'][0] != 'register' or last['out'][2] != 4 or last['out'][1] >= 0x20):
        return None
    if any(op['op'] in ('BRANCH', 'CBRANCH', 'BRANCHIND', 'CALL', 'CALLIND', 'RETURN') for op in ops[:k]):
        return None
    dst = last['out'][1]
    return ops[:k] + [
        f'let Some((q, v, n, z)) = pcode_rt::ops::{DIV[mnem][1]}(u_{dividend[1]:x} as u32, u_{divisor[1]:x}) else {{',
        f'    return Err(Stop::Trap {{ vector: 5, at: "{fmt_at(ins)}".into() }});',
        '};',
        f'r.set({hex_lit(dst)}, 4, q as u64);',
        f'r.set({hex_lit(CF)}, 1, 0);',
        f'r.set({hex_lit(VF)}, 1, v);',
        f'r.set({hex_lit(NF)}, 1, n);',
        f'r.set({hex_lit(ZF)}, 1, z);']


# --- clr of memory: the read before the write ------------------------------------------------------------

def clear(ins):
    """Rust for a clr.w/clr.l of memory: its P-code, with the faulty read's write before the store of 0;
    None for any other instruction or shape."""
    size = size_of(ins)
    if base_mnemonic(ins) != 'clr' or size not in (2, 4):
        return None
    ops = ins['ops']
    stores = [i for i, op in enumerate(ops)
              if op['op'] == 'STORE' or (op.get('out') or [None])[0] == 'ram']
    if len(stores) != 1:
        return None
    if any(op['op'] in ('BRANCH', 'CBRANCH', 'BRANCHIND', 'CALL', 'CALLIND', 'RETURN') for op in ops):
        return None
    k = stores[0]
    op = ops[k]
    if op['op'] == 'STORE':
        addr, val = op['in'][1], op['in'][2]
    else:
        addr, val = ['const', op['out'][1], 4], op['in'][0]
    if val[0] == 'unique':
        # A store through an address register stores a unique the sinc set to 0 before it.
        val = next((o['in'][0] for o in reversed(ops[:k]) if o['op'] == 'COPY' and o['out'] == val), val)
    if val[0] != 'const' or val[1] != 0 or val[2] != size:
        return None
    # The store's address into a unique of our own, so the lines below can name it.
    tmp = max([v[1] for o in ops for v in o.get('in', []) + [o.get('out')]
               if v and v[0] == 'unique' and isinstance(v[1], int)], default=0) + 0x100
    h, off = ins['at']
    words = [f'a.wrapping_add({2 * i})' if i else 'a' for i in range(size // 2)]
    # The prefetched word is read only for such a register: a clr of RAM costs no extra bus access.
    lines = [f'let a = u_{tmp:x} as u32;']
    for wa in words:
        lines += [f'if pcode_rt::ops::custom_write_only({wa}) {{',
                  f'    let w = load(bus, img.h({h}, {hex_lit(off + ins["len"])}), 2);',
                  f'    store(bus, img, {wa}, 2, w)?;', '}']
    if size == 4:
        # The 68000's clr.l writes its low word first (Moira's bus order), which a pair of custom
        # registers sees; the sinc's one long store writes the high word first.
        lines += ['store(bus, img, a.wrapping_add(2), 2, 0)?;', 'store(bus, img, a, 2, 0)?;']
        return ops[:k] + [{'op': 'COPY', 'out': ['unique', tmp, 4], 'in': [addr]}] + lines + ops[k + 1:]
    return ops[:k] + [{'op': 'COPY', 'out': ['unique', tmp, 4], 'in': [addr]}] + lines + ops[k:]


def emit(ins):
    return divide(ins) or clear(ins)
