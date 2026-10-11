#!/usr/bin/env python3
"""Labelled m68k listing of a Superfrog executable, driven by re/symbols.toml.

    tools/re_listing.py <chunk-id> [--out DIR] [--xrefs] [--symbols FILE]

<chunk-id> is a decompressed ATN! chunk (`d1_c03` = disk 1, chunk 3, 1-based as in
docs/05) holding an Amiga hunk executable, or a raw code region of a disk
(`d1_boot`, `d1_stage15`; see RAW_REGIONS). The chunk is read straight from the
player's disks (SUPERFROG_DISKS, else the main checkout root) and decompressed in
memory with tools/atn.py; nothing is written to build/.

Hunks are parsed and relocated with amitools. Hunks are laid out back to back from
address 0, as the game's own hunk loader does (stage 1.5, +0x43A), so a listing
address is an offset from the load base. Every relocated reference is printed as a
symbol name, or as `hN+$off` when nothing in symbols.toml names it.

The listing is derived from copyrighted code, so it is written outside the repo:
~/.cache/superfrog-re/re/<chunk-id>.lst by default (DESIGN §2.3, §3.5).
See docs/RE-WORKBENCH.md.
"""
import argparse, bisect, glob, io, os, re, subprocess, sys, tomllib, zipfile
from dataclasses import dataclass, field

from amitools.binfmt.hunk import Hunk as H
from amitools.binfmt.hunk.HunkReader import HunkReader
from amitools.binfmt.hunk.HunkRelocate import HunkRelocate
from capstone import Cs, CS_ARCH_M68K, CS_MODE_BIG_ENDIAN, CS_MODE_M68K_000
from capstone import m68k as M

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from atn import carve_chunks, ATNDecompressor

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SYMBOLS = os.path.join(ROOT, 're', 'symbols.toml')
DEFAULT_OUT = os.path.join('~', '.cache', 'superfrog-re', 're')
KINDS = ('func', 'data', 'table', 'var')
# raw (non-hunk) code regions: id -> (disk, start, end) in the disk image
RAW_REGIONS = {
    'd1_boot': (1, 0x000, 0x400),       # bootblock (docs/02)
    'd1_stage15': (1, 0x400, 0x1600),   # stage-1.5 loader: ATN!, hunk loader (docs/02)
}


# ---------------------------------------------------------------------------
# Disks and chunks
# ---------------------------------------------------------------------------

def disks_dir():
    env = os.environ.get('SUPERFROG_DISKS')
    if env:
        return env
    common = subprocess.run(['git', 'rev-parse', '--path-format=absolute', '--git-common-dir'],
                            cwd=ROOT, capture_output=True, text=True).stdout.strip()
    return os.path.dirname(common) if common else ROOT


def load_disks(where=None):
    """Return {disk number: adf bytes} from the *.zip or *.adf files in `where`."""
    where = where or disks_dir()
    disks = {}
    for path in sorted(glob.glob(os.path.join(where, '*.zip')) + glob.glob(os.path.join(where, '*.adf'))):
        m = re.search(r'Disk (\d)', os.path.basename(path))
        if not m or int(m.group(1)) in disks:
            continue
        if path.endswith('.zip'):
            with zipfile.ZipFile(path) as zf:
                names = [n for n in zf.namelist() if n.lower().endswith('.adf')]
                if names:
                    disks[int(m.group(1))] = zf.read(names[0])
        else:
            with open(path, 'rb') as f:
                disks[int(m.group(1))] = f.read()
    if 1 not in disks:
        raise SystemExit(f"Superfrog disk images not found in {where}; "
                         "set SUPERFROG_DISKS to the folder holding the four disk zips")
    return disks


def load_chunk(chunk_id, disks):
    """Bytes of a chunk id: a decompressed ATN! chunk, or a raw region."""
    if chunk_id in RAW_REGIONS:
        d, lo, hi = RAW_REGIONS[chunk_id]
        return disks[d][lo:hi]
    m = re.fullmatch(r'd(\d)_c(\d+)', chunk_id)
    if not m or int(m.group(1)) not in disks:
        raise SystemExit(f"unknown chunk id {chunk_id!r} (want dN_cNN or one of {', '.join(RAW_REGIONS)})")
    chunks = list(carve_chunks(disks[int(m.group(1))]))
    n = int(m.group(2))
    if not 1 <= n <= len(chunks):
        raise SystemExit(f"{chunk_id}: disk {m.group(1)} has {len(chunks)} chunks")
    _, dsz, _, data = chunks[n - 1]
    out, ok = ATNDecompressor(disks[1]).decompress(data, dsz)
    if not ok:
        raise SystemExit(f"{chunk_id}: ATN! decompression failed (docs/03)")
    return out


# ---------------------------------------------------------------------------
# Hunk executable
# ---------------------------------------------------------------------------

@dataclass
class Hunk:
    index: int
    kind: str          # CODE | DATA | BSS | ABSENT (declared in the header, not in the file)
    memf: str
    size: int          # allocated size, from the header table
    base: int          # load address (hunks back to back from 0)
    data: bytes        # relocated contents (zeros for BSS/ABSENT)


@dataclass
class Exe:
    raw: bool
    hunks: list
    blocks: list = field(default_factory=list)   # (type name, file offset, file length)
    relocs: dict = field(default_factory=dict)   # (hunk, offset) -> (target hunk, delta)

    def locate(self, addr):
        """(hunk, offset) of a load address, or None."""
        for h in self.hunks:
            if h.base <= addr < h.base + max(h.size, 1):
                return h.index, addr - h.base
        return None


def parse_exe(data):
    """Parse a hunk executable (amitools) and relocate it to the game's layout."""
    hr = HunkReader()
    res = hr.read_file_obj('exe', io.BytesIO(data))
    if res != H.RESULT_OK:
        raise ValueError(f"amitools: {H.result_names[res]}: {hr.error_string}")
    if not hr.build_segments():
        raise ValueError(f"amitools: {hr.error_string}")
    offs = [h['hunk_file_offset'] for h in hr.hunks] + [len(data)]
    blocks = [(h['type_name'], offs[i], offs[i + 1] - offs[i]) for i, h in enumerate(hr.hunks)]
    table = hr.header['hunks']
    # d1_c03 and d3_c04 declare four hunks but carry only two; the game's loader
    # still reserves room for all four. amitools' relocator indexes a missing
    # hunk and fails, so the missing ones are added as empty BSS (FINDINGS).
    present = {seg[0]['hunk_no'] for seg in hr.segments}
    for n, t in enumerate(table):
        if n not in present:
            hr.segments.append([{'type': H.HUNK_BSS, 'type_name': 'ABSENT', 'hunk_no': n,
                                 'size': t['size'], 'alloc_size': t['size'], 'memf': t['memf']}])
    hr.segments.sort(key=lambda s: s[0]['hunk_no'])
    rel = HunkRelocate(hr)
    bases = rel.get_seq_addrs(0)
    datas = rel.relocate(bases)
    exe = Exe(raw=False, hunks=[], blocks=blocks)
    for seg, base, d in zip(hr.segments, bases, datas):
        main = seg[0]
        kind = {'HUNK_CODE': 'CODE', 'HUNK_DATA': 'DATA', 'HUNK_BSS': 'BSS'}.get(main['type_name'], main['type_name'])
        exe.hunks.append(Hunk(main['hunk_no'], kind, table[main['hunk_no']]['memf'] or 'any',
                              main['alloc_size'], base, bytes(d)))
        for blk in seg[1:]:
            for target, offsets in blk.get('reloc', {}).items():
                for o in offsets:
                    delta = int.from_bytes(main['data'][o:o + 4], 'big')
                    exe.relocs[(main['hunk_no'], o)] = (target, delta)
    return exe


def load_exe(data):
    if data[:4] == b'\x00\x00\x03\xf3':
        return parse_exe(data)
    return Exe(raw=True, hunks=[Hunk(0, 'CODE', 'raw', len(data), 0, data)])


# ---------------------------------------------------------------------------
# Symbols
# ---------------------------------------------------------------------------

@dataclass
class Sym:
    exe: str
    hunk: int
    offset: int
    name: str
    kind: str
    size: int
    note: str
    line: int


class SymbolError(ValueError):
    pass


def load_symbols(path=SYMBOLS, text=None):
    """Parse and check symbols.toml; errors name the line of the offending [[sym]]."""
    if text is None:
        with open(path, encoding='utf-8') as f:
            text = f.read()
    try:
        doc = tomllib.loads(text)
    except tomllib.TOMLDecodeError as e:
        raise SymbolError(f"{path}: {e}")
    lines = [i + 1 for i, l in enumerate(text.splitlines()) if re.match(r'\s*\[\[\s*sym\s*\]\]', l)]
    syms = []
    for entry, line in zip(doc.get('sym', []), lines):
        where = f"{path}:{line}"
        for key in ('exe', 'hunk', 'offset', 'name', 'kind'):
            if key not in entry:
                raise SymbolError(f"{where}: missing {key}")
        if entry['kind'] not in KINDS:
            raise SymbolError(f"{where}: kind {entry['kind']!r} is not one of {', '.join(KINDS)}")
        if not re.fullmatch(r'[A-Za-z_][A-Za-z0-9_]*', entry['name']):
            raise SymbolError(f"{where}: name {entry['name']!r} is not an identifier")
        unknown = set(entry) - {'exe', 'hunk', 'offset', 'name', 'kind', 'size', 'note'}
        if unknown:
            raise SymbolError(f"{where}: unknown key {', '.join(sorted(unknown))}")
        size = entry.get('size', 0)
        if entry['kind'] != 'func' and size <= 0:
            raise SymbolError(f"{where}: {entry['kind']} symbol {entry['name']} needs a size")
        syms.append(Sym(entry['exe'], entry['hunk'], entry['offset'], entry['name'], entry['kind'],
                        size, entry.get('note', ''), line))
    seen, spans = {}, {}
    for s in syms:
        key = (s.exe, s.name)
        if key in seen:
            raise SymbolError(f"{path}:{s.line}: duplicate name {s.name} in {s.exe} (first at line {seen[key]})")
        seen[key] = s.line
        at = (s.exe, s.hunk, s.offset)
        if at in spans:
            raise SymbolError(f"{path}:{s.line}: {s.name} is at the same place as {spans[at].name} "
                              f"(line {spans[at].line})")
        spans[at] = s
    for (exe, hunk), group in _group(syms).items():
        data = sorted((s for s in group if s.kind != 'func'), key=lambda s: s.offset)
        for a, b in zip(data, data[1:]):
            if b.offset < a.offset + a.size:
                raise SymbolError(f"{path}:{b.line}: {b.name} overlaps {a.name} (line {a.line})")
    return syms


def _group(syms):
    out = {}
    for s in syms:
        out.setdefault((s.exe, s.hunk), []).append(s)
    return out


# ---------------------------------------------------------------------------
# Listing
# ---------------------------------------------------------------------------

class Namer:
    """Names load addresses: a symbol, `sym+$n` inside a sized one, else `hN+$off`."""

    def __init__(self, exe, syms):
        self.exe = exe
        self.at = {}
        self.sized = []
        for s in syms:
            if s.hunk >= len(exe.hunks):
                raise SymbolError(f"line {s.line}: {s.name}: hunk {s.hunk} does not exist")
            h = exe.hunks[s.hunk]
            if s.offset >= max(h.size, 1):
                raise SymbolError(f"line {s.line}: {s.name}: offset ${s.offset:x} is past hunk {s.hunk} "
                                  f"(${h.size:x} bytes)")
            self.at[h.base + s.offset] = s
            if s.size:
                self.sized.append((h.base + s.offset, s))
        self.sized.sort(key=lambda t: t[0])
        self.keys = [a for a, _ in self.sized]

    def name(self, addr):
        if addr in self.at:
            return self.at[addr].name
        i = bisect.bisect_right(self.keys, addr) - 1
        if i >= 0:
            a, s = self.sized[i]
            if addr < a + s.size:
                return f"{s.name}+${addr - a:x}"
        loc = self.exe.locate(addr)
        if loc is None:
            return None
        return f"h{loc[0]}+${loc[1]:x}"


def _refs(ins, exe, hunk):
    """[(target address, how)] for the references one instruction makes."""
    refs = []
    off = ins.address - hunk.base
    for o in range(off, off + ins.size - 3):
        if (hunk.index, o) in exe.relocs:
            t, delta = exe.relocs[(hunk.index, o)]
            refs.append((exe.hunks[t].base + delta, 'abs'))
    for op in ins.operands:
        if op.type == M.M68K_OP_BR_DISP:
            refs.append((ins.address + 2 + op.br_disp.disp, 'branch'))
        elif op.type == M.M68K_OP_MEM and op.address_mode in (M.M68K_AM_PCI_DISP,
                                                                M.M68K_AM_PCI_INDEX_8_BIT_DISP):
            # pc-relative is source-only, so its extension word follows the opcode
            refs.append((ins.address + 2 + op.mem.disp, 'pcrel'))
    return refs


def _decode(exe, namer):
    """Linear sweep: [(hunk, addr, size, text, refs)], data symbols kept as dc."""
    md = Cs(CS_ARCH_M68K, CS_MODE_BIG_ENDIAN | CS_MODE_M68K_000)
    md.detail = True
    rows = []
    for h in exe.hunks:
        if h.kind not in ('CODE', 'DATA'):
            continue
        data_syms = {h.base + s.offset: s for a, s in namer.sized
                     if s.kind != 'func' and s.hunk == h.index}
        pos = 0
        while pos < h.size:
            addr = h.base + pos
            if addr in data_syms:
                s = data_syms[addr]
                n = min(s.size, h.size - pos)
                for k in range(0, n, 16):
                    chunk = h.data[pos + k:pos + min(k + 16, n)]
                    rows.append((h, addr + k, len(chunk), 'dc.b ' + ','.join(f'${b:02x}' for b in chunk), []))
                pos += n
                continue
            # stop the sweep at the next data symbol so it is not swallowed
            nxt = min([a for a in data_syms if a > addr] + [h.base + h.size])
            ins = next(md.disasm(h.data[pos:nxt - h.base], addr, 1), None)
            if ins is not None and ins.id == 0:     # capstone's own "invalid" word
                ins = None
            if ins is None or h.kind == 'DATA':
                w = h.data[pos:pos + 2]
                rows.append((h, addr, len(w), f"dc.w ${int.from_bytes(w, 'big'):0{len(w) * 2}x}", []))
                pos += len(w)
                continue
            refs = _refs(ins, exe, h)
            text = f"{ins.mnemonic:<8} {ins.op_str}".rstrip()
            for target, how in refs:
                label = namer.name(target)
                if label is None:
                    continue
                # capstone prints relocated, branch and pc-relative targets as $hex
                new = re.sub(rf'\${target:x}(\.l)?\b', label, text)
                text = new if new != text else f"{text}   ; {label}"
            rows.append((h, addr, ins.size, text, refs))
            pos += ins.size
    return rows


def listing(chunk_id, exe, syms, xrefs=False):
    namer = Namer(exe, syms)
    rows = _decode(exe, namer)
    xref = {}
    for h, addr, size, text, refs in rows:
        for target, how in refs:
            xref.setdefault(target, []).append((addr, how))
    out = [f"; {chunk_id} — generated by tools/re_listing.py from the player's disks.",
           "; Derived from copyrighted code: never commit this file (DESIGN §2.3).",
           "; Addresses are load offsets, hunks back to back from 0 as the game's loader places them.", ";"]
    if exe.raw:
        out.append(f"; raw region, {exe.hunks[0].size} bytes")
    else:
        out.append(f"; {len(exe.hunks)} hunks: " + ', '.join(
            f"h{h.index} {h.kind} {h.memf} ${h.size:x} @ ${h.base:x}" for h in exe.hunks))
        out.append(f"; {len(exe.relocs)} relocations")
    out.append("")
    starts = {addr for _, addr, _, _, _ in rows}
    for h in exe.hunks:
        out.append(f"\n;;; hunk {h.index}  {h.kind}  {h.memf}  ${h.size:x} bytes  @ ${h.base:x}")
        if h.kind in ('BSS', 'ABSENT'):
            labels = sorted(a for a in set(namer.at) | set(xref) if h.base <= a < h.base + h.size)
            for a in labels:
                out.extend(_label(a, namer, xref, xrefs))
                out.append(f"{'':10}{'':8}  ; h{h.index}+${a - h.base:x}")
            continue
        for row in rows:
            if row[0] is not h:
                continue
            _, addr, size, text, _ = row
            if addr in namer.at or addr in xref:
                out.extend(_label(addr, namer, xref, xrefs))
            # labels that fall inside this instruction (a misaligned sweep or data)
            for a in range(addr + 1, addr + size):
                if (a in namer.at or a in xref) and a not in starts:
                    out.append(f"{'':10}; {namer.name(a)} = ${a:x} falls inside this instruction")
            raw = h.data[addr - h.base:addr - h.base + size].hex()
            out.append(f"{addr:08x}  {raw:<20}  {text}")
    return '\n'.join(out) + '\n'


def _label(addr, namer, xref, xrefs):
    lines = []
    s = namer.at.get(addr)
    if s is not None:
        lines.append("")
        head = f"; {s.kind}"
        if s.size:
            head += f", {s.size} bytes"
        if s.note:
            head += f" — {s.note}"
        lines.append(head)
    lines.append(f"{namer.name(addr)}:")
    if xrefs and addr in xref:
        refs = sorted(set(xref[addr]))
        names = ', '.join(f"{_where(namer, a)} ({how})" for a, how in refs)
        lines.append(f"          ; xref: {names}")
    return lines


def _where(namer, addr):
    """Name a referencing instruction as `func+$n` or `hN+$off`."""
    funcs = [(a, s) for a, s in namer.at.items() if s.kind == 'func' and a <= addr]
    loc = namer.exe.locate(addr)
    if funcs:
        a, s = max(funcs, key=lambda t: t[0])
        floc = namer.exe.locate(a)
        if floc and loc and floc[0] == loc[0]:
            return s.name if a == addr else f"{s.name}+${addr - a:x}"
    return f"h{loc[0]}+${loc[1]:x}"


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('chunk', help='chunk id, e.g. d1_c03, or a raw region: ' + ', '.join(RAW_REGIONS))
    ap.add_argument('--out', default=DEFAULT_OUT, help=f'output folder (default {DEFAULT_OUT})')
    ap.add_argument('--xrefs', action='store_true', help='list the references to every label')
    ap.add_argument('--symbols', default=SYMBOLS, help='symbols file (default re/symbols.toml)')
    a = ap.parse_args(argv)
    try:
        syms = [s for s in load_symbols(a.symbols) if s.exe == a.chunk]
        exe = load_exe(load_chunk(a.chunk, load_disks()))
        text = listing(a.chunk, exe, syms, a.xrefs)
    except (SymbolError, ValueError) as e:
        raise SystemExit(f"re_listing: {e}")
    out_dir = os.path.expanduser(a.out)
    os.makedirs(out_dir, exist_ok=True)
    path = os.path.join(out_dir, f"{a.chunk}.lst")
    tmp = path + '.tmp'
    with open(tmp, 'w', encoding='utf-8') as f:
        f.write(text)
    os.replace(tmp, path)
    print(path)


if __name__ == '__main__':
    main()
