#!/usr/bin/env python3
"""Labelled m68k listing of a Superfrog executable, driven by games/superfrog/re/symbols.toml.

    games/superfrog/tools/re_listing.py <chunk-id> [--out DIR] [--xrefs] [--symbols FILE]

<chunk-id> is a decompressed ATN! chunk (`d1_c03` = disk 1, chunk 3, 1-based as in
games/superfrog/docs/05) holding an Amiga hunk executable, a raw code region of a disk
(`d1_boot`, `d1_stage15`; see RAW_REGIONS), or an ATN! block embedded in a chunk's
first hunk, `<chunk-id>@<hex offset>` (`d1_c03@36a04`, the Accessing-disk overlay). The chunk is read straight from the
player's disks (SUPERFROG_DISKS, else games/superfrog/ in the main checkout, else its root) and decompressed in
memory with games/superfrog/tools/atn.py; nothing is written to games/superfrog/build/.

The hunk parsing, symbols and listing are toolkit/tools/hunk_listing.py; this is the game's
driver. Hunks are laid out back to back from address 0, as the game's own hunk loader does
(stage 1.5, +0x43A), so a listing address is an offset from the load base.

The listing is derived from copyrighted code, so it is written outside the repo:
~/.cache/superfrog-re/re/<chunk-id>.lst by default (DESIGN §2.3, §3.5).
See games/superfrog/docs/RE-WORKBENCH.md.
"""
import argparse, glob, os, re, sys, zipfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(ROOT, '..', '..', 'toolkit', 'tools'))
import hunk_listing
from hunk_listing import Exe, Hunk, Namer, Sym, SymbolError, _decode, load_exe, parse_exe  # noqa: F401
from atn import carve_chunks, ATNDecompressor
from disk_lookup import disks_dir
SYMBOLS = os.path.join(ROOT, 're', 'symbols.toml')
DEFAULT_OUT = os.path.join('~', '.cache', 'superfrog-re', 're')
# raw (non-hunk) code regions: id -> (disk, start, end) in the disk image
RAW_REGIONS = {
    'd1_boot': (1, 0x000, 0x400),       # bootblock (games/superfrog/docs/02)
    'd1_stage15': (1, 0x400, 0x1600),   # stage-1.5 loader: ATN!, hunk loader (games/superfrog/docs/02)
}


# ---------------------------------------------------------------------------
# Disks and chunks
# ---------------------------------------------------------------------------

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
    if '@' in chunk_id:
        outer, off = chunk_id.split('@', 1)
        h0 = load_exe(load_chunk(outer, disks)).hunks[0].data
        at = int(off, 16)
        if h0[at:at + 4] != b'ATN!':
            raise SystemExit(f"{chunk_id}: no ATN! block at h0+${at:x}")
        dsz, csz = int.from_bytes(h0[at + 4:at + 8], 'big'), int.from_bytes(h0[at + 8:at + 12], 'big')
        out, ok = ATNDecompressor(disks[1]).decompress(h0[at:at + 12 + csz + 64], dsz)
        if not ok:
            raise SystemExit(f"{chunk_id}: ATN! decompression failed (games/superfrog/docs/03)")
        return out
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
        raise SystemExit(f"{chunk_id}: ATN! decompression failed (games/superfrog/docs/03)")
    return out


def load_symbols(path=SYMBOLS, text=None):
    return hunk_listing.load_symbols(path, text)


def listing(chunk_id, exe, syms, xrefs=False):
    return hunk_listing.listing(chunk_id, exe, syms, xrefs, tool='games/superfrog/tools/re_listing.py')


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('chunk', help='chunk id, e.g. d1_c03, or a raw region: ' + ', '.join(RAW_REGIONS))
    ap.add_argument('--out', default=DEFAULT_OUT, help=f'output folder (default {DEFAULT_OUT})')
    ap.add_argument('--xrefs', action='store_true', help='list the references to every label')
    ap.add_argument('--symbols', default=SYMBOLS, help='symbols file (default games/superfrog/re/symbols.toml)')
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
