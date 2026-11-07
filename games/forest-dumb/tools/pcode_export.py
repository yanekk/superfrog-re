#!/usr/bin/env python3
"""Export Ghidra's P-code for every routine of re/translate.toml, hunk-relative (translator DESIGN §3.4).

    python3 -I games/forest-dumb/tools/pcode_export.py [--out games/forest-dumb/build/translator] [--reimport]

Imports the main program into a Ghidra project under <out>/ghidra through
toolkit/tools/ghidra_headless.py with re/symbols.toml, seeds the entries of re/translate.toml and runs
tools/ghidra/ExportPcode.java, which writes every instruction reached by direct flow with its P-code.
Writes <out>/pcode.json and prints one JSON summary line. Exit 2, the path on stderr, when Ghidra, its
JDK, the program or a symbols file is missing; exit 1 when Ghidra fails.

Ghidra's Amiga loader records no relocations, so the program file's HUNK_RELOC32 entries are read here
(stdlib: amitools is not on `python3 -I`'s path) and handed to the script. Everything written derives
from the game's code and stays under build/ (git-ignored).
"""
import argparse, json, os, struct, sys, tempfile, tomllib
from dataclasses import dataclass

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(ROOT, '..', '..', 'toolkit', 'tools'))
import ghidra_headless  # noqa: E402

EXE = os.path.join(ROOT, 'build', 'files', 'disk1', 'FOREST_DUMB_FOREVER')
EXE_ID = 'forest_dumb_forever'
SYMBOLS = os.path.join(ROOT, 're', 'symbols.toml')
TRANSLATE = os.path.join(ROOT, 're', 'translate.toml')
OUT = os.path.join(ROOT, 'build', 'translator')
PROJECT = 'translator'

HUNK_HEADER, HUNK_CODE, HUNK_DATA, HUNK_BSS = 0x3F3, 0x3E9, 0x3EA, 0x3EB
HUNK_RELOC32, HUNK_SYMBOL, HUNK_DEBUG, HUNK_END = 0x3EC, 0x3F0, 0x3F1, 0x3F2
KIND = {HUNK_CODE: 'CODE', HUNK_DATA: 'DATA', HUNK_BSS: 'BSS'}


class Usage(Exception):
    """A missing input: exit 2 with the path."""


@dataclass
class Hunks:
    sizes: list    # allocated size per hunk, from the header
    kinds: list    # CODE | DATA | BSS, or ABSENT when the file lacks the hunk
    relocs: list   # (hunk, offset, target hunk, value the file holds there)


def read_hunks(data):
    """Walk a hunk executable: sizes, kinds and every HUNK_RELOC32 entry with the long it relocates."""
    def long(p):
        if p + 4 > len(data):
            raise ValueError(f"truncated at file offset {p}")
        return struct.unpack_from('>I', data, p)[0]

    if long(0) != HUNK_HEADER:
        raise ValueError("not a hunk executable (no HUNK_HEADER)")
    p = 4
    while (n := long(p)) != 0:   # resident library names, none expected
        p += 4 + 4 * n
    p += 4
    count, first, last = long(p), long(p + 4), long(p + 8)
    p += 12
    sizes = [(long(p + 4 * i) & 0x3FFFFFFF) * 4 for i in range(last - first + 1)]
    p += 4 * len(sizes)
    kinds = ['ABSENT'] * len(sizes)
    contents = [b''] * len(sizes)
    relocs = []
    h = 0
    while p < len(data):
        t = long(p) & 0x3FFFFFFF
        p += 4
        if t in (HUNK_CODE, HUNK_DATA):
            n = long(p) * 4
            contents[h] = data[p + 4:p + 4 + n]
            kinds[h] = KIND[t]
            p += 4 + n
        elif t == HUNK_BSS:
            kinds[h] = 'BSS'
            p += 4
        elif t == HUNK_RELOC32:
            while (n := long(p)) != 0:
                target = long(p + 4)
                for i in range(n):
                    off = long(p + 8 + 4 * i)
                    relocs.append((h, off, target, struct.unpack_from('>I', contents[h], off)[0]))
                p += 8 + 4 * n
            p += 4
        elif t in (HUNK_SYMBOL,):
            while (n := long(p)) != 0:
                p += 4 + 4 * n + 4
            p += 4
        elif t == HUNK_DEBUG:
            p += 4 + long(p) * 4
        elif t == HUNK_END:
            h += 1
        else:
            raise ValueError(f"unknown hunk block ${t:x} at file offset {p - 4}")
    if count != len(sizes):
        raise ValueError(f"header declares {count} hunks, table holds {len(sizes)}")
    return Hunks(sizes, kinds, relocs)


@dataclass
class Sym:
    """What ghidra_headless.run_import reads of a symbol (hunk_listing's Sym needs amitools)."""
    hunk: int
    offset: int
    name: str
    kind: str
    size: int
    note: str


def load_symbols(path):
    with open(path, 'rb') as f:
        doc = tomllib.load(f)
    return [Sym(s['hunk'], s['offset'], s['name'], s['kind'], s.get('size', 0), s.get('note', ''))
            for s in doc.get('sym', []) if s.get('exe') == EXE_ID]


def load_entries(path):
    """(hunk, offset, instructions with callees) of every routine, in file order."""
    with open(path, 'rb') as f:
        doc = tomllib.load(f)
    return [(r['hunk'], r['offset'], r.get('instructions')) for r in doc.get('routine', [])]


def rel(h, off):
    return f"h{h}+${off:x}"


def reach_counts(export):
    """Instructions each entry reaches with its callees, following every listed flow target."""
    insns = export['insns']

    def succ(a):
        i = insns[a]
        h, off = a[1:].split('+$')
        nxt = []
        kind = i['flow']['kind']
        if kind not in ('branch', 'computed', 'return'):
            nxt.append(rel(int(h), int(off, 16) + i['len']))
        nxt += i['flow']['targets']
        return [n for n in nxt if n in insns]

    out = {}
    for e in export['entries']:
        seen, todo = set(), [e]
        while todo:
            a = todo.pop()
            if a in seen or a not in insns:
                continue
            seen.add(a)
            todo += succ(a)
        out[e] = len(seen)
    return out


def check_inputs(exe, symbols, translate):
    """Every input present, before anything starts Java: Usage names the first one missing."""
    for p in (exe, symbols, translate):
        if not os.path.isfile(p):
            raise Usage(f"missing: {p}")
    try:
        home = ghidra_headless.ghidra_home()
    except SystemExit as e:
        raise Usage(str(e))
    headless = os.path.join(home, 'support', 'analyzeHeadless')
    if not os.path.isfile(headless):
        raise Usage(f"Ghidra not found: no {headless} (GHIDRA_HOME or ghidraRun on the PATH)")
    try:
        java = ghidra_headless.java_home()
    except SystemExit as e:
        raise Usage(str(e))
    if not os.path.isfile(os.path.join(java, 'bin', 'java')):
        raise Usage(f"no JDK for Ghidra: no {os.path.join(java, 'bin', 'java')} (JAVA_HOME)")


def export(exe_path, symbols, entries, out, reimport=False, name=EXE_ID):
    """Run the import and the export script; returns the pcode.json document (also written to <out>)."""
    with open(exe_path, 'rb') as f:
        data = f.read()
    hunks = read_hunks(data)
    if 'ABSENT' in hunks.kinds:
        # complete_hunks needs amitools; this program carries every hunk it declares
        raise Usage(f"{exe_path} declares hunks it does not carry; run under the venv's python")
    os.makedirs(out, exist_ok=True)
    with tempfile.TemporaryDirectory() as tmp:
        etsv, rtsv, raw = (os.path.join(tmp, n) for n in ('entries.tsv', 'relocs.tsv', 'export.json'))
        with open(etsv, 'w') as f:
            f.writelines(f"{h}\t{off}\n" for h, off, _ in entries)
        with open(rtsv, 'w') as f:
            f.writelines(f"{h}\t{off}\t{t}\t{v}\n" for h, off, t, v in hunks.relocs)
        ghidra_headless.run_import(name, data, symbols, out, PROJECT, reimport=reimport, complete=False,
                                   script_paths=[os.path.join(HERE, 'ghidra')],
                                   post_scripts=[('ExportPcode.java', [etsv, rtsv, raw])])
        if not os.path.exists(raw):
            raise SystemExit("pcode_export: ExportPcode.java wrote nothing (see the Ghidra output above)")
        with open(raw) as f:
            doc = json.load(f)
    doc['hunks'] = [{'index': i, 'size': s, 'kind': k}
                    for i, (s, k) in enumerate(zip(hunks.sizes, hunks.kinds))]
    doc['entries'] = [rel(h, off) for h, off, _ in entries]
    doc['check']['relocs_in_file'] = len(hunks.relocs)
    ordered = {k: doc[k] for k in ('hunks', 'entries', 'insns', 'check', 'ghidra_bases')}
    path = os.path.join(out, 'pcode.json')
    with open(path + '.tmp', 'w') as f:
        json.dump(ordered, f, separators=(',', ':'))
    os.replace(path + '.tmp', path)
    return ordered


def summary(doc):
    c = doc['check']
    return {'routines': len(doc['entries']), 'instructions': c['instructions'], 'functions': c['functions'],
            'unresolved': c['unresolved'], 'relocs': c['relocs'], 'relocs_matched': c['relocs_matched'],
            'unmatched': len(c['unmatched']), 'mismatched': len(c['mismatched']),
            'unexplained': len(c['unexplained']), 'undecoded': len(c['undecoded'])}


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--out', default=OUT)
    ap.add_argument('--exe', default=EXE)
    ap.add_argument('--symbols', default=SYMBOLS)
    ap.add_argument('--translate', default=TRANSLATE)
    ap.add_argument('--reimport', action='store_true', help='discard the Ghidra program and import afresh')
    a = ap.parse_args(argv)
    try:
        check_inputs(a.exe, a.symbols, a.translate)
        doc = export(a.exe, load_symbols(a.symbols), load_entries(a.translate), a.out, a.reimport)
    except Usage as e:
        sys.stderr.write(f"pcode_export: {e}\n")
        return 2
    # per entry, the reach with callees against call_tree.py's count; over 5% is printed for FINDINGS
    want = {rel(h, off): n for h, off, n in load_entries(a.translate)}
    for e, n in reach_counts(doc).items():
        if want.get(e) and abs(n - want[e]) > 0.05 * want[e]:
            sys.stderr.write(f"pcode_export: {e} reaches {n} instructions, translate.toml says {want[e]}\n")
    print(json.dumps(summary(doc), separators=(',', ':')))
    return 0


if __name__ == '__main__':
    sys.exit(main())
