#!/usr/bin/env python3
"""Labelled m68k listing of Forest Dumb Forever's main program, driven by games/forest-dumb/re/symbols.toml.

    venv/bin/python games/forest-dumb/tools/re_listing.py [--exe FILE] [--out DIR] [--xrefs] [--symbols FILE]

The program is read from `fd-tools extract`'s output under build/files/disk1/. The hunk parsing, symbols
and listing are toolkit/tools/hunk_listing.py; this is the game's driver. Hunks are laid out back to back
from address 0, so a listing address is an offset from the load base, not the h0+$x a note uses.

The listing derives from copyrighted code, so it goes to build/re/ (git-ignored) and is never committed
(DESIGN §3.5).
"""
import argparse, os, sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(ROOT, '..', '..', 'toolkit', 'tools'))
import hunk_listing
from hunk_listing import SymbolError
import call_tree

EXE_ID = 'forest_dumb_forever'
EXE = os.path.join(ROOT, 'build', 'files', 'disk1', 'FOREST_DUMB_FOREVER')
SYMBOLS = os.path.join(ROOT, 're', 'symbols.toml')
OUT = os.path.join(ROOT, 'build', 're')
CODE_IN_DATA_HUNK = 4


def flow_entries(exe, syms):
    """Every instruction call_tree.py's flow decoding reaches from its ROOTS. The sweep resyncs at each,
    so game_frame's tree lists in step: without them data in the code put h0+$2c7e, $3698, $5820,
    $62b6 and $94ba inside other instructions, and translate.py's flow_text found no label (T40)."""
    at = {s.name: exe.hunks[s.hunk].base + s.offset for s in syms}
    roots = [at[r] for r in call_tree.ROOTS if r in at]
    return set().union(*call_tree.Tree(call_tree.Program(exe), roots).own.values()) if roots else set()


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--exe', default=EXE, help="the program (default: fd-tools extract's copy in build/)")
    ap.add_argument('--out', default=OUT, help='output folder (default build/re/)')
    ap.add_argument('--xrefs', action='store_true', help='list the references to every label')
    ap.add_argument('--symbols', default=SYMBOLS, help='symbols file (default re/symbols.toml)')
    a = ap.parse_args(argv)
    if not os.path.exists(a.exe):
        raise SystemExit(f"re_listing: {a.exe} not found; run `cargo run -p fd-tools -- extract` "
                         "in games/forest-dumb/engine/ first")
    with open(a.exe, 'rb') as f:
        data = f.read()
    try:
        syms = [s for s in hunk_listing.load_symbols(a.symbols) if s.exe == EXE_ID]
        exe = hunk_listing.load_exe(data)
        # Hunk 4's header says DATA, but the program calls code in it (the load lists, the music
        # player), and the shared lister prints a DATA hunk as dc.w without decoding it.
        exe.hunks[CODE_IN_DATA_HUNK].kind = 'CODE'
        text = hunk_listing.listing(EXE_ID, exe, syms, a.xrefs,
                                    tool='games/forest-dumb/tools/re_listing.py', entries=flow_entries(exe, syms))
    except (SymbolError, ValueError) as e:
        raise SystemExit(f"re_listing: {e}")
    os.makedirs(a.out, exist_ok=True)
    path = os.path.join(a.out, f"{EXE_ID}.lst")
    tmp = path + '.tmp'
    with open(tmp, 'w', encoding='utf-8') as f:
        f.write(text)
    os.replace(tmp, path)
    print(path)


if __name__ == '__main__':
    main()
