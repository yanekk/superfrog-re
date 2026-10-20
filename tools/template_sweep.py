#!/usr/bin/env python3
"""Every object template a level can spawn, against those the Rust engine ports (T204).

    tools/template_sweep.py [--registered FILE]

Walks the game's code (d1_c03's hunk 0, disassembled with capstone as tools/re_listing.py does)
from each world's marker spawners (`obj_flag_jumptab`, h0+$178ec: seven tables of 64 `bra.w`
stubs) for the markers each level's map holds, and from every template so reached (its init,
update and touch routines, and every routine it sets into a slot's `+$0`, `+$4` or `+$10`),
following branches and calls. Each `obj_templates` reference met is a template that spawner or
object can spawn. References no walk reaches are listed apart, with the routine they sit in.

The engine's side comes from `cargo run -q --release --example registered_templates` (run from
engine/), or from FILE holding its output. Prints a table per template; exits 1 when a spawnable
template has no kind in the engine. Reads the player's disks as re_listing.py does; writes
nothing (DESIGN §2.3).
"""
import argparse, os, subprocess, sys
from collections import defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import re_listing as R
import t7mp
from capstone import Cs, CS_ARCH_M68K, CS_MODE_BIG_ENDIAN, CS_MODE_M68K_000
from capstone import m68k as M

TEMPLATES, SIZE, COUNT = 0x1c6e2, 0xae, 160
JUMPTAB, WORLDS, MARKERS = 0x178ec, 7, 64
# The levels' chunks (docs/LEVEL-INDEX.md) and the table of obj_flag_jumptab each uses
# (`obj_world_offset / $100`: W1-W6 0..5, the final level 5, the bonus level 6).
LEVELS = [
    ('W1-1', 'd1_c21', 0), ('W1-2', 'd1_c22', 0), ('W1-3', 'd1_c23', 0), ('W1-4', 'd1_c24', 0),
    ('W2-1', 'd1_c31', 1), ('W2-2', 'd1_c32', 1), ('W2-3', 'd1_c33', 1), ('W2-4', 'd1_c34', 1),
    ('W3-1', 'd2_c12', 2), ('W3-2', 'd2_c13', 2), ('W3-3', 'd2_c14', 2), ('W3-4', 'd2_c15', 2),
    ('W4-1', 'd2_c22', 3), ('W4-2', 'd2_c23', 3), ('W4-3', 'd2_c24', 3), ('W4-4', 'd2_c25', 3),
    ('W5-1', 'd2_c32', 4), ('W5-2', 'd2_c33', 4), ('W5-3', 'd2_c34', 4), ('W5-4', 'd2_c35', 4),
    ('WB-1', 'd2_c42', 6),
    ('W6-1', 'd4_c02', 5), ('W6-2', 'd4_c03', 5), ('W6-3', 'd4_c04', 5), ('W6-4', 'd4_c05', 5),
    ('WW-1', 'd4_c13', 5),
    ('W6-2-alt', 'd4_c14', 5), ('W6-3-alt', 'd4_c15', 5), ('W6-4-alt', 'd4_c17', 5),
]
# Routines every object shares, not walked into: the spawners and freeing (they name no template
# but the coin's init), and what a touch does to the frog (the frog's code, not the object's).
SHARED = {0x1b3d8, 0x1b4e6, 0x1b5d0, 0x1b706, 0x1b822, 0x1b90e}
# Slot fields holding a routine: init, update, touch.
ROUTINE_FIELDS = (0, 4, 0x10)


def template_of(addr):
    """The template an address falls in, or None."""
    if TEMPLATES <= addr < TEMPLATES + SIZE * COUNT:
        return (addr - TEMPLATES) // SIZE
    return None


class Code:
    """Hunk 0, its relocations, and a recursive-descent walk over it."""

    def __init__(self, exe):
        self.h = exe.hunks[0]
        self.exe = exe
        self.md = Cs(CS_ARCH_M68K, CS_MODE_BIG_ENDIAN | CS_MODE_M68K_000)
        self.md.detail = True
        self.memo = {}

    def long(self, addr):
        return int.from_bytes(self.h.data[addr:addr + 4], 'big')

    def reloc(self, at):
        """The hunk-0 address a relocated long at `at` points to, or None."""
        r = self.exe.relocs.get((0, at))
        return self.exe.hunks[r[0]].base + r[1] if r and r[0] == 0 else None

    def insn(self, addr):
        if addr not in self.memo:
            ins = next(self.md.disasm(self.h.data[addr:addr + 10], addr, 1), None)
            self.memo[addr] = None if ins is None or ins.id == 0 else ins
        return self.memo[addr]

    def walk(self, entries):
        """Walk from `entries`: ({template: first site}, {routine entry set into a slot})."""
        seen, todo = set(), list(entries)
        templates, routines = {}, set()
        while todo:
            a = todo.pop()
            while a not in seen and 0 <= a < self.h.size:
                seen.add(a)
                ins = self.insn(a)
                if ins is None:
                    break
                refs = [self.reloc(o) for o in range(a + 2, a + ins.size - 3, 2)]
                refs = [r for r in refs if r is not None]
                for r in refs:
                    t = template_of(r)
                    if t is not None:
                        templates.setdefault(t, a)
                ops = ins.operands
                # `move.l #routine, $0/$4/$10(an)`: a routine the slot turns to.
                if (ins.mnemonic == 'move.l' and len(ops) == 2 and ops[0].type == M.M68K_OP_IMM
                        and refs and ops[1].type == M.M68K_OP_MEM
                        and ops[1].address_mode in (M.M68K_AM_REGI_ADDR_DISP, M.M68K_AM_REGI_ADDR)
                        and ops[1].mem.disp in ROUTINE_FIELDS and template_of(refs[-1]) is None):
                    routines.add(refs[-1])
                m = ins.mnemonic
                target = None
                for op in ops:
                    if op.type == M.M68K_OP_BR_DISP:
                        target = a + 2 + op.br_disp.disp
                if m in ('jsr', 'jmp'):
                    if refs:
                        target = refs[-1]
                    elif ops[0].type == M.M68K_OP_MEM and ops[0].address_mode == M.M68K_AM_PCI_DISP:
                        target = a + 2 + ops[0].mem.disp
                if target is not None and target not in SHARED:
                    todo.append(target)
                if m in ('rts', 'rte', 'rtr', 'jmp', 'bra', 'bra.b', 'bra.w', 'bra.s'):
                    break
                a += ins.size
        return templates, routines

    def object_walk(self, t):
        """What template `t`'s routines reach, the routines they set followed too."""
        base = TEMPLATES + t * SIZE
        entries = {self.long(base + f) for f in ROUTINE_FIELDS} - {0}
        done, templates = set(), {}
        while entries - done:
            batch = entries - done
            done |= batch
            got, routines = self.walk(sorted(batch))
            for k, v in got.items():
                templates.setdefault(k, v)
            entries |= routines
        return templates, done

    def marker_stub(self, world, marker):
        """The stub's target, or None (`rts`)."""
        at = JUMPTAB + 4 * (world * MARKERS + marker)
        w = int.from_bytes(self.h.data[at:at + 2], 'big')
        if w != 0x6000:
            return None
        d = int.from_bytes(self.h.data[at + 2:at + 4], 'big', signed=True)
        return at + 2 + d

    def references(self):
        """Every relocated long in hunk 0 pointing into `obj_templates`, outside the templates."""
        out = {}
        for (hunk, o), (tgt, delta) in self.exe.relocs.items():
            if hunk != 0 or tgt != 0 or template_of(o) is not None:
                continue
            t = template_of(self.exe.hunks[0].base + delta)
            if t is not None:
                out.setdefault(t, []).append(o)
        return out


def level_markers(disks):
    """{level: (table, set of markers 1..$39 in its map)}."""
    out = {}
    for name, chunk, table in LEVELS:
        data = R.load_chunk(chunk, disks)
        _, _, _, flags = t7mp.level_grid(data)
        out[name] = (table, {f for row in flags for f in row if 0 < f < 0x3a})
    return out


def registered(path=None):
    if path:
        text = open(path).read()
    else:
        engine = os.path.join(R.ROOT, 'engine')
        text = subprocess.run(['cargo', 'run', '-q', '--release', '--example', 'registered_templates'],
                              cwd=engine, capture_output=True, text=True, check=True).stdout
    lines = dict(l.split(' ', 1) for l in text.splitlines() if ' ' in l)
    return {int(t) for t in lines['templates'].split()}


def sweep(code, levels):
    """{template: [source, ...]}: each a 'W3-1 marker 27' or 'template 65' that spawns it."""
    sources = defaultdict(list)
    by_stub = defaultdict(list)
    for name, (table, markers) in levels.items():
        for m in sorted(markers):
            by_stub[(table, m)].append(name)
    todo = []
    for (table, m), names in sorted(by_stub.items()):
        stub = code.marker_stub(table, m)
        if stub is None:
            continue
        got, _ = code.walk([stub])
        for t in sorted(got):
            sources[t].append(f"marker {m} (table {table}: {', '.join(names)})")
            todo.append(t)
    done = set()
    while todo:
        t = todo.pop()
        if t in done:
            continue
        done.add(t)
        got, _ = code.object_walk(t)
        for c in sorted(got):
            if c != t:
                sources[c].append(f"template {t}")
                todo.append(c)
    return sources


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split('\n\n')[0])
    ap.add_argument('--registered', help="registered_templates' output, instead of running it")
    args = ap.parse_args(argv)
    disks = R.load_disks()
    code = Code(R.load_exe(R.load_chunk('d1_c03', disks)))
    sources = sweep(code, level_markers(disks))
    ported = registered(args.registered)
    missing = 0
    print("template  ported  spawned by")
    for t in sorted(sources):
        ok = t in ported
        missing += not ok
        print(f"{t:8}  {'yes' if ok else 'NO ':6}  {'; '.join(dict.fromkeys(sources[t]))}")
    print("\nreferenced but reached from no map spawner or object (by the walk):")
    for t, sites in sorted(code.references().items()):
        if t not in sources:
            where = ', '.join(f"h0+${s:x}" for s in sorted(sites))
            print(f"{t:8}  {'yes' if t in ported else 'NO ':6}  at {where}")
    print(f"\n{len(sources)} templates spawnable from the maps, {missing} with no kind in the engine")
    return 1 if missing else 0


if __name__ == '__main__':
    sys.exit(main())
