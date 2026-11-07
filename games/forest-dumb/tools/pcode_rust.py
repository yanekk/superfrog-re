#!/usr/bin/env python3
"""Translate a P-code export into one Rust module that runs every exported routine instruction for
instruction on `pcode-rt` (the translator trial's T03; its DESIGN §2.3, §2.5, §2.8, §3.3).

    python3 -I pcode_rust.py <pcode.json> --out <tree.rs> [--hints <toml>]

Prints one JSON line, {"functions":N,"blocks":N,"unsupported":N,"computed_unresolved":N}; exits 2 on
a bad input, naming it on stderr.

No model and no knowledge of any program: everything arrives in the export (T01's format) and the
hints file. Addresses are `hN+$off` throughout, and the generated code resolves them through the
hunk bases it is given at run time (`Image::h`), so it runs wherever the program is loaded.

Export varnodes are `[space, offset, size]`. An offset written `"hN+$off"` (a `ram` address inside
a hunk, or a relocated `const`) is hunk-relative; an instruction's `relocs` entry
`{"in_op":i,"slot":s,"target":"hN+$off"}` makes input `s` of op `i` the same. A `const` offset
inside a `BRANCH`/`CBRANCH` is P-code's relative branch within the instruction.

The generated code, per call target, one function: its basic blocks are the arms of a `match` on
the block address inside a loop, which counts blocks against `STEP_CAP`. An address that is no
block of the function falls to the last arm, which enters the translated function there (a jump
into another routine's entry runs it; its return is checked by our caller) or stops with
`Untranslated`. A `RETURN` writes the popped address into `PC` and returns; the caller compares it
with the address after its call and, on a mismatch, jumps there the same way, since 68000 code may
return to an address it pushed. An op the translator does not carry out makes its whole
instruction `Stop::Unsupported`, and only that instruction.

Python's standard library only, so it runs under `python3 -I`.
"""
import argparse, json, os, re, sys, tomllib

U32 = (1 << 32) - 1
ADDR = re.compile(r'^h(\d+)\+\$([0-9a-fA-F]+)$')

# Binary ops whose helper takes (a, b, size of the inputs).
BINARY = {
    'INT_ADD': 'int_add', 'INT_SUB': 'int_sub', 'INT_CARRY': 'int_carry', 'INT_SCARRY': 'int_scarry',
    'INT_SBORROW': 'int_sborrow', 'INT_LESS': 'int_less', 'INT_LESSEQUAL': 'int_lessequal',
    'INT_SLESS': 'int_sless', 'INT_SLESSEQUAL': 'int_slessequal', 'INT_EQUAL': 'int_equal',
    'INT_NOTEQUAL': 'int_notequal', 'INT_MULT': 'int_mult', 'INT_DIV': 'int_div',
    'INT_SDIV': 'int_sdiv', 'INT_REM': 'int_rem', 'INT_SREM': 'int_srem', 'INT_AND': 'int_and',
    'INT_OR': 'int_or', 'INT_XOR': 'int_xor',
}
# Shifts take (a, amount, size of the shifted value); the amount is its own varnode's plain value.
SHIFT = {'INT_LEFT': 'int_left', 'INT_RIGHT': 'int_right', 'INT_SRIGHT': 'int_sright'}
# Unary ops taking (a, size of the input).
UNARY = {'INT_2COMP': 'int_2comp', 'INT_NEGATE': 'int_negate', 'POPCOUNT': 'popcount'}
BOOL = {'BOOL_AND': 'bool_and', 'BOOL_OR': 'bool_or', 'BOOL_XOR': 'bool_xor'}
JUMPS = {'BRANCH', 'CBRANCH', 'BRANCHIND', 'RETURN'}
# Ops after which control never reaches the next op of the instruction.
NO_FALLTHROUGH = {'BRANCH', 'BRANCHIND', 'RETURN'}


class BadInput(Exception):
    pass


class Unsupported(Exception):
    """An op (or a varnode) this translator does not carry out; the instruction becomes a stop."""


def parse_addr(s, what):
    m = ADDR.match(s) if isinstance(s, str) else None
    if not m:
        raise BadInput(f'{what}: {s!r} is not an address of the form hN+$off')
    return int(m.group(1)), int(m.group(2), 16)


def fmt_addr(a):
    return f'h{a[0]}+${a[1]:x}'


def fn_name(a):
    return f'h{a[0]}_{a[1]:x}'


def key(a):
    return f'({a[0]}, 0x{a[1]:x})'


def hex_lit(v):
    return f'0x{v:x}'


# ---------------------------------------------------------------------------------------------
# Reading the export and the hints

def check_varnode(v, where):
    if not (isinstance(v, list) and len(v) == 3 and isinstance(v[0], str)
            and isinstance(v[2], int) and v[2] > 0):
        raise BadInput(f'{where}: {v!r} is not a varnode [space, offset, size]')
    if isinstance(v[1], str):
        parse_addr(v[1], where)
    elif not isinstance(v[1], int) or v[1] < 0:
        raise BadInput(f'{where}: {v!r} has a bad offset')


def load_export(path):
    try:
        with open(path, encoding='utf-8') as f:
            doc = json.load(f)
    except OSError as e:
        raise BadInput(f'{path}: cannot read: {e.strerror}')
    except json.JSONDecodeError as e:
        raise BadInput(f'{path}: not JSON: {e}')
    if not isinstance(doc, dict):
        raise BadInput(f'{path}: not a P-code export (expected an object)')
    for k in ('hunks', 'entries', 'insns'):
        if k not in doc:
            raise BadInput(f'{path}: no "{k}"')
    hunks = doc['hunks']
    if not isinstance(hunks, list) or not all(isinstance(h, dict) and isinstance(h.get('size'), int)
                                              for h in hunks):
        raise BadInput(f'{path}: "hunks" must be a list of objects with a "size"')
    for i, h in enumerate(hunks):
        if h.get('index', i) != i:
            raise BadInput(f'{path}: hunk {i} has index {h.get("index")}')
    entries = [parse_addr(e, f'{path}: entries') for e in doc['entries']]
    insns = {}
    if not isinstance(doc['insns'], dict):
        raise BadInput(f'{path}: "insns" must be an object keyed by address')
    for at, ins in doc['insns'].items():
        a = parse_addr(at, f'{path}: insns')
        where = f'{path}: {at}'
        if a[0] >= len(hunks):
            raise BadInput(f'{where}: hunk {a[0]} is not in "hunks"')
        if not isinstance(ins, dict) or not isinstance(ins.get('len'), int) or ins['len'] <= 0:
            raise BadInput(f'{where}: no "len"')
        ops = ins.get('ops')
        if not isinstance(ops, list):
            raise BadInput(f'{where}: no "ops"')
        for j, op in enumerate(ops):
            if not isinstance(op, dict) or not isinstance(op.get('op'), str):
                raise BadInput(f'{where}: op {j} has no "op"')
            if op.get('out') is not None:
                check_varnode(op['out'], f'{where} op {j}')
            if not isinstance(op.get('in', []), list):
                raise BadInput(f'{where}: op {j} "in" is not a list')
            for v in op.get('in', []):
                check_varnode(v, f'{where} op {j}')
        ops = [dict(op, **{'in': [list(v) for v in op.get('in', [])]}) for op in ops]
        for rel in ins.get('relocs', []):
            try:
                i, s, t = rel['in_op'], rel['slot'], rel['target']
                ops[i]['in'][s][1] = t
            except (KeyError, IndexError, TypeError):
                raise BadInput(f'{where}: reloc {rel!r} names no operand')
            parse_addr(t, f'{where} reloc')
        flow = ins.get('flow') or {}
        targets = [parse_addr(t, f'{where} flow') for t in flow.get('targets', [])]
        insns[a] = {'len': ins['len'], 'mnem': str(ins.get('mnem', '')), 'ops': ops,
                    'targets': targets}
    return {'sizes': [h['size'] for h in hunks], 'entries': entries, 'insns': insns}


def load_hints(path):
    try:
        with open(path, 'rb') as f:
            doc = tomllib.load(f)
    except OSError as e:
        raise BadInput(f'{path}: cannot read: {e.strerror}')
    except tomllib.TOMLDecodeError as e:
        raise BadInput(f'{path}: not TOML: {e}')
    targets, data = {}, []
    for i, h in enumerate(doc.get('hint', [])):
        where = f'{path}: hint {i + 1}'
        at = parse_addr(h.get('at'), where)
        kind = h.get('kind')
        if kind == 'targets':
            ts = h.get('targets')
            if not isinstance(ts, list) or not ts:
                raise BadInput(f'{where}: kind "targets" needs a non-empty "targets"')
            targets.setdefault(at, []).extend(parse_addr(t, where) for t in ts)
        elif kind == 'data':
            n = h.get('len')
            if not isinstance(n, int) or n <= 0:
                raise BadInput(f'{where}: kind "data" needs a positive "len"')
            data.append((at[0], at[1], at[1] + n))
        else:
            raise BadInput(f'{where}: kind must be "targets" or "data", not {kind!r}')
    return targets, data


def apply_hints(export, targets, data):
    """Bytes marked data hold no instruction; a computed jump or call gains the targets given."""
    insns = {}
    for a, ins in export['insns'].items():
        if any(h == a[0] and lo < a[1] + ins['len'] and a[1] < hi for h, lo, hi in data):
            continue
        extra = targets.get(a, [])
        insns[a] = dict(ins, targets=sorted(set(ins['targets']) | set(extra))) if extra else ins
    return dict(export, insns=insns)


# ---------------------------------------------------------------------------------------------
# Control flow: which instructions make up each function, and its basic blocks

def op_names(ins):
    return [op['op'] for op in ins['ops']]


def is_relative(op):
    return op['op'] in ('BRANCH', 'CBRANCH') and op['in'] and op['in'][0][0] == 'const'


def ram_target(op):
    """The hunk-relative target of a direct BRANCH/CBRANCH/CALL, or None (absolute or relative)."""
    v = op['in'][0] if op['in'] else None
    if v and v[0] == 'ram' and isinstance(v[1], str):
        return parse_addr(v[1], 'target')
    return None


def falls_through(ins):
    ops = ins['ops']
    return not ops or ops[-1]['op'] not in NO_FALLTHROUGH or is_relative(ops[-1])


def succ(a, ins):
    """(in-function successors, call targets) of the instruction at `a`."""
    local, calls = [], []
    has_ind_branch = False
    for op in ins['ops']:
        o = op['op']
        if o in ('BRANCH', 'CBRANCH') and not is_relative(op):
            t = ram_target(op)
            if t:
                local.append(t)
        elif o == 'BRANCHIND':
            has_ind_branch = True
        elif o == 'CALL':
            t = ram_target(op)
            if t:
                calls.append(t)
        elif o == 'CALLIND':
            calls.extend(ins['targets'])
    if has_ind_branch:
        local.extend(ins['targets'])
    if falls_through(ins):
        local.append((a[0], a[1] + ins['len']))
    return local, calls


def ends_block(ins):
    return any(op['op'] in JUMPS and not is_relative(op) for op in ins['ops'])


def build_function(entry, insns):
    """The instructions reachable from `entry` without a call, its blocks, and its call targets."""
    seen, work, calls, labels = set(), [entry], set(), {entry}
    while work:
        a = work.pop()
        if a in seen or a not in insns:
            continue
        seen.add(a)
        local, c = succ(a, insns[a])
        calls.update(c)
        ins = insns[a]
        nxt = (a[0], a[1] + ins['len'])
        for t in local:
            if t != nxt or ends_block(ins):
                labels.add(t)
            work.append(t)
    labels = {l for l in labels if l in seen}
    # A block runs from a label along fall-through until the next label or a jump. An instruction
    # left over (reached only by falling through from one that ends a block) starts its own.
    while True:
        blocks, covered = {}, set()
        for l in sorted(labels):
            run, a = [], l
            while True:
                run.append(a)
                covered.add(a)
                ins = insns[a]
                nxt = (a[0], a[1] + ins['len'])
                if ends_block(ins) or not falls_through(ins) or nxt not in seen or nxt in labels:
                    break
                a = nxt
            blocks[l] = run
        left = seen - covered
        if not left:
            return blocks, calls
        labels.add(min(left))


def translate(export):
    insns = export['insns']
    funcs, work = {}, list(export['entries'])
    while work:
        e = work.pop()
        if e in funcs or e not in insns:
            continue
        blocks, calls = build_function(e, insns)
        funcs[e] = blocks
        work.extend(sorted(calls))
    return funcs


# ---------------------------------------------------------------------------------------------
# Rust

class Emit:
    def __init__(self, funcs, insns):
        self.funcs = funcs
        self.insns = insns

    # Varnodes -------------------------------------------------------------------------------

    @staticmethod
    def check_size(v):
        if v[2] > 8:
            raise Unsupported(f'{v[2]}-byte {v[0]}')

    def addr_expr(self, off):
        """The address `off` names, as a u32 expression."""
        if isinstance(off, str):
            h, o = parse_addr(off, 'varnode')
            return f'img.h({h}, {hex_lit(o)})'
        return hex_lit(off & U32)

    def rd(self, v):
        space, off, size = v
        self.check_size(v)
        if space == 'const':
            if isinstance(off, str):
                return f'({self.addr_expr(off)} as u64)'
            return hex_lit(off & ((1 << (8 * size)) - 1)) + 'u64'
        if space == 'register':
            return f'r.get({hex_lit(off)}, {size})'
        if space == 'unique':
            return f'u_{off:x}'
        if space == 'ram':
            return f'load(bus, {self.addr_expr(off)}, {size})'
        raise Unsupported(f'{space} space')

    def wr(self, v, val):
        space, off, size = v
        self.check_size(v)
        if space == 'register':
            return f'r.set({hex_lit(off)}, {size}, {val});'
        if space == 'unique':
            m = '' if size >= 8 else f' & {hex_lit((1 << (8 * size)) - 1)}'
            return f'u_{off:x} = {val}{m};'
        if space == 'ram':
            return f'store(bus, img, {self.addr_expr(off)}, {size}, {val})?;'
        raise Unsupported(f'write to {space} space')

    # Ops ------------------------------------------------------------------------------------

    def value(self, op):
        """The expression an op with an output computes."""
        o, ins = op['op'], op['in']
        n = lambda k: self.rd(ins[k])
        s0 = ins[0][2] if ins else 0
        out = op['out'][2]
        if o == 'COPY':
            return n(0)
        if o == 'LOAD':
            return f'load(bus, {n(1)} as u32, {out})'
        if o in BINARY:
            return f'{BINARY[o]}({n(0)}, {n(1)}, {s0})'
        if o in SHIFT:
            return f'{SHIFT[o]}({n(0)}, {n(1)}, {s0})'
        if o in UNARY:
            return f'{UNARY[o]}({n(0)}, {s0})'
        if o in BOOL:
            return f'{BOOL[o]}({n(0)}, {n(1)})'
        if o == 'BOOL_NEGATE':
            return f'bool_negate({n(0)})'
        if o == 'INT_ZEXT':
            return f'int_zext({n(0)}, {s0})'
        if o == 'INT_SEXT':
            return f'int_sext({n(0)}, {s0}, {out})'
        if o == 'SUBPIECE':
            if ins[1][0] != 'const' or not isinstance(ins[1][1], int):
                raise Unsupported('SUBPIECE with a non-constant offset')
            return f'subpiece({n(0)}, {s0}, {ins[1][1]}, {out})'
        if o == 'PIECE':
            return f'piece({n(0)}, {n(1)}, {ins[1][2]}, {out})'
        raise Unsupported(o)

    def goto(self, off):
        """Control to the address `off` names (a ram varnode's offset)."""
        if isinstance(off, str):
            return f'pc = {key(parse_addr(off, "target"))}; continue \'b;'
        return f'return Err(Stop::Untranslated({json.dumps(f"${off & U32:06x}")}.into()));'

    @staticmethod
    def goto_abs(expr):
        """Control to the absolute address `expr` computes, through this function's blocks."""
        return (f'let a = {expr} as u32;\nmatch img.hunk_of(a) {{\n    Some(k) => {{ pc = k; continue \'b; }}\n'
                f'    None => return Err(Stop::Untranslated(img.describe(a))),\n}}')

    def ret_check(self, nxt):
        return (f'let ret = r.get(PC, 4) as u32;\n'
                f'if ret != img.h({nxt[0]}, {hex_lit(nxt[1])}) {{\n'
                + '\n'.join('    ' + l for l in self.goto_abs('ret').split('\n')) + '\n}')

    def op_lines(self, op, j, a, ins):
        o = op['op']
        nxt = (a[0], a[1] + ins['len'])
        if o == 'STORE':
            v = op['in']
            self.check_size(v[2])
            return [f'let v = {self.rd(v[2])};',
                    f'store(bus, img, {self.rd(v[1])} as u32, {v[2][2]}, v)?;']
        if o in ('BRANCH', 'CBRANCH'):
            t = op['in'][0]
            if t[0] == 'const':
                rel = t[1] if t[1] < (1 << (8 * t[2] - 1)) else t[1] - (1 << (8 * t[2]))
                jump = f'i = {j + rel}; continue \'i;'
            elif t[0] == 'ram':
                jump = self.goto(t[1])
            else:
                raise Unsupported(f'{o} to {t[0]} space')
            if o == 'BRANCH':
                return [jump]
            return [f'if {self.rd(op["in"][1])} != 0 {{ {jump} }}']
        if o == 'BRANCHIND':
            return [self.goto_abs(self.rd(op['in'][0]))]
        if o == 'CALL':
            t = op['in'][0]
            if t[0] != 'ram':
                raise Unsupported(f'CALL to {t[0]} space')
            if not isinstance(t[1], str):
                return [self.goto(t[1])]
            c = parse_addr(t[1], 'call')
            if c not in self.funcs:
                return [f'return Err(Stop::Untranslated({json.dumps(fmt_addr(c))}.into()));']
            return [f'{fn_name(c)}(r, bus, img)?;', self.ret_check(nxt)]
        if o == 'CALLIND':
            return [f'let a = ({self.rd(op["in"][0])}) as u32;',
                    'match img.hunk_of(a).and_then(by_addr) {\n    Some(f) => f(r, bus, img)?,\n'
                    '    None => return Err(Stop::Untranslated(img.describe(a))),\n}',
                    self.ret_check(nxt)]
        if o == 'RETURN':
            return [f'let v = {self.rd(op["in"][0])};', 'r.set(PC, 4, v);', 'return Ok(());']
        if o == 'CALLOTHER':
            name = op.get('name')
            raise Unsupported(f'CALLOTHER {name}' if name else 'CALLOTHER')
        if op.get('out') is None:
            raise Unsupported(o)
        return [f'let v = {self.value(op)};', self.wr(op['out'], 'v')]

    def instruction(self, a, ins):
        """The Rust for one instruction, and whether it is an unsupported stop."""
        head = f'// {fmt_addr(a)} {ins["mnem"]}'.rstrip()
        try:
            bodies = [[x for l in self.op_lines(op, j, a, ins) for x in l.split('\n')]
                      for j, op in enumerate(ins['ops'])]
        except Unsupported as e:
            stop = (f'return Err(Stop::Unsupported {{ op: {json.dumps(str(e))}.into(), '
                    f'at: {json.dumps(fmt_addr(a))}.into() }});')
            return [head, stop], True
        uniques = sorted({v[1] for op in ins['ops'] for v in (op.get('in', []) + [op.get('out')])
                          if v and v[0] == 'unique'})
        lines = [head, '{']
        lines += [f'    let mut u_{u:x}: u64 = 0;' for u in uniques]
        if any(is_relative(op) for op in ins['ops']):
            # P-code branches within the instruction: its ops become arms of a loop of their own.
            lines += ['    let mut i: usize = 0;', '    \'i: loop {', '        match i {']
            for j, body in enumerate(bodies):
                lines.append(f'            {j} => {{')
                lines += ['                ' + l for l in body]
                lines.append('            }')
            lines += ['            _ => break \'i,', '        }', '        i += 1;', '    }']
        else:
            for body in bodies:
                lines += ['    ' + l for l in body]
        lines.append('}')
        return lines, False

    def function(self, e, blocks):
        lines = [ALLOW, f'pub fn {fn_name(e)}(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {{',
                 f'    let mut pc: (u32, u32) = {key(e)};',
                 '    let mut steps: u64 = 0;',
                 '    \'b: loop {',
                 '        steps += 1;',
                 '        if steps > STEP_CAP {',
                 '            return Err(Stop::Hang);',
                 '        }',
                 '        match pc {']
        unsupported = 0
        for l in sorted(blocks):
            lines.append(f'            {key(l)} => {{')
            run = blocks[l]
            for a in run:
                body, stop = self.instruction(a, self.insns[a])
                unsupported += stop
                lines += ['                ' + x for x in body]
            last = run[-1]
            lines.append(f'                pc = {key((last[0], last[1] + self.insns[last]["len"]))};')
            lines.append('            }')
        lines += ['            k => return enter(k, r, bus, img),', '        }', '    }', '}']
        return lines, unsupported


ALLOW = ('#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, '
         'unused_labels, non_snake_case, clippy::all)]')


def code_ranges(funcs, insns):
    """(hunk, start, end) for every run of translated instruction bytes, merged and sorted."""
    spans = sorted({(a[0], a[1], a[1] + insns[a]['len'])
                    for blocks in funcs.values() for run in blocks.values() for a in run})
    out = []
    for h, s, e in spans:
        if out and out[-1][0] == h and s <= out[-1][2]:
            out[-1][2] = max(out[-1][2], e)
        else:
            out.append([h, s, e])
    return out


def render(export, source_name):
    insns = export['insns']
    funcs = translate(export)
    em = Emit(funcs, insns)
    stats = {'functions': len(funcs), 'blocks': sum(len(b) for b in funcs.values()),
             'unsupported': 0, 'computed_unresolved': 0}
    for blocks in funcs.values():
        for run in blocks.values():
            for a in run:
                ops = op_names(insns[a])
                if ('BRANCHIND' in ops or 'CALLIND' in ops) and not insns[a]['targets']:
                    stats['computed_unresolved'] += 1
    out = [f'// Generated from {source_name}; never committed when made from a game.',
           '// By pcode_rust.py. Do not edit: regenerate.',
           '#[allow(unused_imports)]',
           'use pcode_rt::{Bus, Image, Regs, Stop, ops::*, load, store, PC, STEP_CAP};',
           '',
           '/// The bytes of every translated instruction, (hunk, start, end exclusive), sorted.',
           'pub const CODE: &[(u32, u32, u32)] = &[']
    out += [f'    ({h}, 0x{s:x}, 0x{e:x}),' for h, s, e in code_ranges(funcs, insns)]
    out += ['];', '', '/// Each hunk\'s length, as the export gives it.',
            'pub const HUNK_SIZES: &[u32] = &[' + ', '.join(hex_lit(s) for s in export['sizes']) + '];',
            '', '#[allow(dead_code)]', 'pub type Routine = fn(&mut Regs, &mut dyn Bus, &Image) -> Result<(), Stop>;', '',
            '/// The translated function entered at `addr` (`hN+$off`).',
            'pub fn entry(addr: &str) -> Option<Routine> {', '    match addr {']
    out += [f'        {json.dumps(fmt_addr(e))} => Some({fn_name(e)}),' for e in sorted(funcs)]
    out += ['        _ => None,', '    }', '}', '',
            '#[allow(dead_code)]', 'fn by_addr(k: (u32, u32)) -> Option<Routine> {', '    match k {']
    out += [f'        {key(e)} => Some({fn_name(e)}),' for e in sorted(funcs)]
    out += ['        _ => None,', '    }', '}', '',
            '/// Control reached `k`, which is no block of the function it was in: enter the translated',
            '/// function there, whose return our caller checks, or stop.',
            '#[allow(dead_code)]', 'fn enter(k: (u32, u32), r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {',
            '    match by_addr(k) {',
            '        Some(f) => f(r, bus, img),',
            '        None => Err(Stop::Untranslated(format!("h{}+${:x}", k.0, k.1))),',
            '    }', '}']
    for e in sorted(funcs):
        lines, n = em.function(e, funcs[e])
        stats['unsupported'] += n
        out += [''] + lines
    return '\n'.join(out) + '\n', stats


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.split('\n\n')[0])
    p.add_argument('pcode')
    p.add_argument('--out', required=True)
    p.add_argument('--hints')
    args = p.parse_args(argv)
    try:
        export = load_export(args.pcode)
        if args.hints:
            export = apply_hints(export, *load_hints(args.hints))
        text, stats = render(export, os.path.basename(args.pcode))
    except BadInput as e:
        print(f'pcode_rust: {e}', file=sys.stderr)
        return 2
    tmp = args.out + '.tmp'
    with open(tmp, 'w', encoding='utf-8') as f:
        f.write(text)
    os.replace(tmp, args.out)
    print(json.dumps(stats, separators=(',', ':')))
    return 0


if __name__ == '__main__':
    sys.exit(main())
