#!/usr/bin/env python3
"""The routines in game_frame's call tree, sized for translation: writes or checks re/translate.toml.

    venv/bin/python games/forest-dumb/tools/call_tree.py [--exe FILE] [--toml FILE] [--check] [--summary]

Reads the main program `fd-tools extract` writes to build/ (never committed), decodes it from each
address the flow reaches, with re/symbols.toml for names, and follows every instruction reachable from
`game_frame`: fall-through, both ways of each conditional branch, `bra`/`jmp`
to a known address, and `bsr`/`jsr` as calls. Each call target is a routine of its own. A call or jump
whose target the code does not give (`jsr (a0)`, `jmp 0(pc,d0.w)`, `jsr -$x(a6)`) is an `[[unfollowed]]`
entry with its reason, so no edge of the tree is silent (plan forest-dumb-level-1, T34; DESIGN §4.3).

Sizes per routine, in instructions:
  own           what its own flow reaches, without callees
  instructions  with every callee, each instruction counted once
  unit          with its callees less those in `split_at` and what only they reach: what one translation
                call sees (T33 answers `too-big` over 180)
  split_at      the callees split off, chosen greedily (largest drop first) until unit <= 180
  by_hand       true when the unit is still over 180 after every split: its own flow is too big, so
                T33 answers too-big and the porting task's builder translates it

The classification (`subsystem`, `gen_task`, `port_task`, `note`) is a person's reading and is kept from
the existing toml by address on every rewrite; a routine without one is written `unassigned`, which
--check refuses. Addresses and sizes only: no listing text is written (DESIGN §3.5).
"""
import argparse, os, re, sys, tomllib

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(ROOT, '..', '..', 'toolkit', 'tools'))
import hunk_listing

EXE = os.path.join(ROOT, 'build', 'files', 'disk1', 'FOREST_DUMB_FOREVER')
EXE_ID = 'forest_dumb_forever'
TOML = os.path.join(ROOT, 're', 'translate.toml')
ROOT_NAME = 'game_frame'
LIMIT = 180  # DESIGN §4.3: Muse passed 24 of 24 runs up to 183 instructions, 2 of 6 at 298 to 461
TASK_LIMIT_INS, TASK_LIMIT_ROUTINES = 1500, 15  # DESIGN §4.3, a porting or generator task's share

SUBSYSTEMS = ('character', 'camera', 'drawing', 'objects', 'rest', 'sound')
GEN_TASK = {'character': 'T35', 'camera': 'T36', 'drawing': 'T37', 'objects': 'T38', 'rest': 'T39'}

CC = 'hi|ls|cc|cs|ne|eq|vc|vs|pl|mi|ge|lt|gt|le|hs|lo'
BRANCH_RE = re.compile(rf'^(bra|bsr|b({CC})|db(ra|t|f|{CC}))(\.[bsw])?$')


class Ins:
    __slots__ = ('op', 'operand', 'size', 'target')

    def __init__(self, op, operand, size, target):
        self.op, self.operand, self.size, self.target = op, operand, size, target


class Program:
    """The program's code, decoded on demand from each address the flow reaches, never by a linear
    sweep: the listing's sweep loses step after data in the code, and 7 of game_frame's call targets
    fell inside an instruction there (T34). Decoding is toolkit/tools/hunk_listing.py's capstone setup."""

    def __init__(self, exe):
        from capstone import Cs, CS_ARCH_M68K, CS_MODE_BIG_ENDIAN, CS_MODE_M68K_000
        self.exe = exe
        self.md = Cs(CS_ARCH_M68K, CS_MODE_BIG_ENDIAN | CS_MODE_M68K_000)
        self.md.detail = True
        self.cache = {}

    def hunk_of(self, a):
        loc = self.exe.locate(a)
        if loc is None:
            raise ValueError(f'address ${a:x} is in no hunk')
        return loc

    def name_of(self, a):
        h, off = self.hunk_of(a)
        return f'h{h}+${off:x}'

    def at(self, a):
        """The instruction at a, or None when a is not decodable code (data, outside a code hunk)."""
        if a in self.cache:
            return self.cache[a]
        loc = self.exe.locate(a)
        ins = None
        if loc is not None:
            h = self.exe.hunks[loc[0]]
            if h.kind == 'CODE' or (h.kind == 'DATA' and h.index == CODE_IN_DATA_HUNK):
                c = next(self.md.disasm(h.data[loc[1]:loc[1] + 10], a, 1), None)
                if c is not None and c.id != 0:
                    ins = Ins(c.mnemonic, c.op_str, c.size, self._target(c, h))
        self.cache[a] = ins
        return ins

    def _target(self, c, h):
        """Where a branch, jump or call goes: a displacement, a relocated absolute, or pc-relative."""
        if not (c.mnemonic in ('jsr', 'jmp') or BRANCH_RE.match(c.mnemonic)):
            return None
        from capstone import m68k as M
        # jmp 2(pc,d0.w) names its table's base, not where it goes: a computed jump, never followed
        if any(o.type == M.M68K_OP_MEM and o.address_mode == M.M68K_AM_PCI_INDEX_8_BIT_DISP
               for o in c.operands):
            return None
        refs = hunk_listing._refs(c, self.exe, h)
        return refs[0][0] if refs else None


CODE_IN_DATA_HUNK = 4  # hunk 4's header says DATA but the program runs code in it (re_listing.py)


def unfollowed_kind(op, operand):
    operand = operand.strip()
    if re.fullmatch(r'-?\$?[0-9a-f]*\(a6\)', operand):
        return 'kickstart'  # a library vector off a6; which library is a reading, not checked here
    if re.fullmatch(r'\$[0-9a-f]+(\.[wl])?', operand):
        return 'unrelocated-absolute'  # a fixed address the loader does not move: ROM or a table
    return 'indirect-call' if op == 'jsr' or op.startswith('bsr') else 'indirect-jump'


def flow(prog, entry):
    """One routine's own flow: (instruction addresses, [callee addrs in order], [(at, kind)])."""
    seen, calls, unf, todo = set(), [], [], [entry]
    while todo:
        pc = todo.pop()
        while pc is not None and pc not in seen:
            ins = prog.at(pc)
            if ins is None:
                unf.append((pc, 'undecodable'))
                break
            seen.add(pc)
            op, t = ins.op, ins.target
            if op in ('rts', 'rte', 'rtr', 'stop', 'illegal'):
                break
            if op == 'jmp' or op.startswith('bra'):
                if t is None:
                    unf.append((pc, unfollowed_kind(op, ins.operand)))
                    break
                pc = t
                continue
            if op == 'jsr' or op.startswith('bsr'):
                if t is None:
                    unf.append((pc, unfollowed_kind(op, ins.operand)))
                elif t not in calls:
                    calls.append(t)
            elif BRANCH_RE.match(op):
                if t is None:
                    unf.append((pc, 'indirect-jump'))
                else:
                    todo.append(t)
            pc += ins.size
    return seen, calls, unf


class Tree:
    def __init__(self, lst, root):
        self.lst = lst
        self.own, self.calls, self.unf = {}, {}, {}
        todo = [root]
        while todo:
            r = todo.pop()
            if r in self.own:
                continue
            self.own[r], self.calls[r], self.unf[r] = flow(lst, r)
            todo += self.calls[r]

    def reach(self, r, cut=()):
        """Instructions r reaches with callees, not entering a routine in cut (other than r)."""
        out, seen, todo = set(), set(), [r]
        while todo:
            x = todo.pop()
            if x in seen or (x != r and x in cut):
                continue
            seen.add(x)
            out |= self.own[x]
            todo += self.calls[x]
        return out

    def routines_under(self, r, cut=()):
        seen, todo = set(), [r]
        while todo:
            x = todo.pop()
            if x in seen or (x != r and x in cut):
                continue
            seen.add(x)
            todo += self.calls[x]
        return seen - {r}

    def split(self, r):
        """Greedy: split off the callee whose removal drops the unit most, until it fits or none left."""
        cut = []
        unit = len(self.reach(r))
        while unit > LIMIT:
            best = None
            for c in self.routines_under(r, cut):
                n = len(self.reach(r, cut + [c]))
                if best is None or n < best[0] or (n == best[0] and c < best[1]):
                    best = (n, c)
            if best is None or best[0] == unit:
                break
            unit = best[0]
            cut.append(best[1])
        return sorted(cut), unit


def symbol_names(path=os.path.join(ROOT, 're', 'symbols.toml')):
    syms = tomllib.load(open(path, 'rb'))['sym']
    return {(s['hunk'], s['offset']): s['name'] for s in syms if s.get('exe', EXE_ID) == EXE_ID}


def load(exe_path, symbols=os.path.join(ROOT, 're', 'symbols.toml')):
    """(Program, root address, names) for the real program and game_frame."""
    if not os.path.exists(exe_path):
        raise SystemExit(f"call_tree: {exe_path} not found; run `cargo run -p fd-tools -- extract` "
                         "in games/forest-dumb/engine/ first")
    exe = hunk_listing.load_exe(open(exe_path, 'rb').read())
    names = symbol_names(symbols)
    root = next(((h, o) for (h, o), n in names.items() if n == ROOT_NAME), None)
    if root is None:
        raise SystemExit(f'call_tree: no {ROOT_NAME} in {symbols}')
    return Program(exe), exe.hunks[root[0]].base + root[1], names


def scan(lst, root, names=None):
    """Every entry of the inventory, structure only: a list of dicts in address order, and unfollowed."""
    names = names or {}
    tree = Tree(lst, root)

    def nm(a):
        h, off = lst.hunk_of(a)
        return names.get((h, off)) or f'h{h}_{off:x}'

    entries, unfollowed = [], []
    for r in sorted(tree.own):
        h, off = lst.hunk_of(r)
        cut, unit = tree.split(r)
        entries.append({
            'name': nm(r), 'hunk': h, 'offset': off,
            'own': len(tree.own[r]), 'instructions': len(tree.reach(r)), 'unit': unit,
            'calls': [lst.name_of(c) for c in tree.calls[r]],
            'split_at': [lst.name_of(c) for c in cut],
            # own flow alone is over the limit: no callee split brings it under, so T33 answers
            # too-big and its porting task's builder translates it (DESIGN §4.3)
            'by_hand': unit > LIMIT,
        })
        for at, kind in tree.unf[r]:
            unfollowed.append({'routine': nm(r), 'at': lst.name_of(at), 'kind': kind})
    return entries, unfollowed


def merge(entries, unfollowed, old):
    """Carry each routine's classification and each unfollowed edge's reason over from the old toml."""
    by_addr = {(e['hunk'], e['offset']): e for e in old.get('routine', [])}
    for e in entries:
        o = by_addr.get((e['hunk'], e['offset']), {})
        for k in ('subsystem', 'gen_task', 'port_task', 'note'):
            if k in o:
                e[k] = o[k]
        e.setdefault('subsystem', 'unassigned')
    reasons = {u['at']: u.get('reason') for u in old.get('unfollowed', [])}
    for u in unfollowed:
        if reasons.get(u['at']):
            u['reason'] = reasons[u['at']]


def q(s):
    return '"' + s.replace('\\', '\\\\').replace('"', '\\"') + '"'


def render(entries, unfollowed):
    out = [
        "# game_frame's call tree, sized for translation (plan forest-dumb-level-1, T34; DESIGN §4.3).",
        "# Sizes and calls are written by games/forest-dumb/tools/call_tree.py from the listing; subsystem,",
        "# gen_task, port_task, note and reason are a person's reading, kept by address on a rewrite.",
        "# own: own flow; instructions: with all callees; unit: less split_at and what only they reach.",
        "# Over 180 a unit is split_at callees; still over, it is by_hand (T33 answers too-big).",
        "# sound is out of scope (the plan is silent): no gen_task, no port_task.",
        "# Totals per task: docs/11-translation.md. Addresses and sizes only, no listing text.",
        "",
    ]
    for e in entries:
        out.append('[[routine]]')
        out.append(f"name = {q(e['name'])}")
        out.append(f"hunk = {e['hunk']}")
        out.append(f"offset = 0x{e['offset']:x}")
        for k in ('own', 'instructions', 'unit'):
            out.append(f'{k} = {e[k]}')
        out.append(f"subsystem = {q(e['subsystem'])}")
        for k in ('gen_task', 'port_task'):
            if e.get(k):
                out.append(f'{k} = {q(e[k])}')
        out.append('calls = [' + ', '.join(q(c) for c in e['calls']) + ']')
        if e['split_at']:
            out.append('split_at = [' + ', '.join(q(c) for c in e['split_at']) + ']')
        if e['by_hand']:
            out.append('by_hand = true')
        if e.get('note'):
            out.append(f"note = {q(e['note'])}")
        out.append('')
    for u in unfollowed:
        out.append('[[unfollowed]]')
        out.append(f"routine = {q(u['routine'])}")
        out.append(f"at = {q(u['at'])}")
        out.append(f"kind = {q(u['kind'])}")
        if u.get('reason'):
            out.append(f"reason = {q(u['reason'])}")
        out.append('')
    return '\n'.join(out)


def problems(doc, entries=None, unfollowed=None):
    """What is wrong with a translate.toml; against a fresh scan when one is given."""
    errs = []
    rs = doc.get('routine', [])
    names = {r['name'] for r in rs}
    by_hoff = {f"h{r['hunk']}+${r['offset']:x}": r for r in rs}
    for r in rs:
        n = r['name']
        if r.get('subsystem') not in SUBSYSTEMS:
            errs.append(f"{n}: subsystem {r.get('subsystem')!r} is not one of {', '.join(SUBSYSTEMS)}")
        elif r['subsystem'] == 'sound':
            if r.get('gen_task') or r.get('port_task'):
                errs.append(f'{n}: sound is out of scope and takes no gen_task or port_task')
        else:
            if r.get('gen_task') != GEN_TASK[r['subsystem']]:
                errs.append(f"{n}: {r['subsystem']} goes to {GEN_TASK[r['subsystem']]}, not {r.get('gen_task')}")
            if not re.fullmatch(r'T\d\d', r.get('port_task', '')):
                errs.append(f'{n}: no port_task')
        if r['unit'] > LIMIT and not r.get('by_hand'):
            errs.append(f"{n}: unit of {r['unit']} instructions over {LIMIT}, not split and not by_hand")
        if r.get('by_hand') and r['unit'] <= LIMIT:
            errs.append(f"{n}: by_hand but its unit of {r['unit']} fits")
        for c in r.get('calls', []) + r.get('split_at', []):
            if c not in by_hoff:
                errs.append(f'{n}: calls {c}, which has no entry')
    for u in doc.get('unfollowed', []):
        if u['routine'] not in names:
            errs.append(f"unfollowed at {u['at']}: routine {u['routine']} has no entry")
        if not u.get('reason'):
            errs.append(f"unfollowed {u['kind']} at {u['at']} in {u['routine']}: no reason")
    if entries is not None:
        fresh = {f"h{e['hunk']}+${e['offset']:x}": e for e in entries}
        for k, e in fresh.items():
            r = by_hoff.get(k)
            if r is None:
                errs.append(f"{k} ({e['name']}) is in game_frame's tree and has no entry")
                continue
            for f in ('own', 'instructions', 'unit', 'calls', 'split_at', 'by_hand'):
                if r.get(f, False if f == 'by_hand' else []) != e[f]:
                    errs.append(f"{r['name']}: {f} is {r.get(f)} in the toml, {e[f]} in the program")
        for k in by_hoff:
            if k not in fresh:
                errs.append(f'{k} has an entry but is not in the tree')
        have = {u['at'] for u in doc.get('unfollowed', [])}
        for u in unfollowed:
            if u['at'] not in have:
                errs.append(f"unfollowed {u['kind']} at {u['at']} in {u['routine']} is not listed")
    return errs


def shares(doc):
    """Per task: [routines, own instructions, by_hand names]. Sound is left out."""
    port, gen = {}, {}
    for r in doc.get('routine', []):
        if r['subsystem'] == 'sound':
            continue
        for table, key in ((port, 'port_task'), (gen, 'gen_task')):
            t = table.setdefault(r.get(key, '?'), [0, 0, []])
            t[0] += 1
            t[1] += r['own']
            if r.get('by_hand'):
                t[2].append(r['name'])
    return port, gen


def summary_text(doc):
    port, gen = shares(doc)
    lines = []
    for title, table in (('Porting task', port), ('Generator task', gen)):
        lines.append(f'| {title} | Routines | Own instructions | By hand | Over §4.3 |')
        lines.append('|---|---|---|---|---|')
        for t in sorted(table):
            n, ins, hand = table[t]
            over = 'yes' if n > TASK_LIMIT_ROUTINES or ins > TASK_LIMIT_INS else ''
            lines.append(f"| {t} | {n} | {ins} | {', '.join(hand)} | {over} |")
        lines.append('')
    return '\n'.join(lines)


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--exe', default=EXE, help="the program (default: fd-tools extract's copy in build/)")
    ap.add_argument('--toml', default=TOML)
    ap.add_argument('--check', action='store_true', help='check the toml against the program; write nothing')
    ap.add_argument('--summary', action='store_true', help="print each task's share from the toml")
    a = ap.parse_args(argv)
    old = tomllib.load(open(a.toml, 'rb')) if os.path.exists(a.toml) else {}
    if a.summary:
        print(summary_text(old))
        return 0
    prog, root, names = load(a.exe)
    entries, unfollowed = scan(prog, root, names)
    if a.check:
        errs = problems(old, entries, unfollowed)
        for e in errs:
            print(e)
        print(f"call_tree: {len(entries)} routines, {len(errs)} problems")
        return 1 if errs else 0
    merge(entries, unfollowed, old)
    text = render(entries, unfollowed)
    tmp = a.toml + '.tmp'
    with open(tmp, 'w', encoding='utf-8') as f:
        f.write(text)
    os.replace(tmp, a.toml)
    print(f"{a.toml}: {len(entries)} routines, {len(unfollowed)} unfollowed")
    return 0


if __name__ == '__main__':
    sys.exit(main())
