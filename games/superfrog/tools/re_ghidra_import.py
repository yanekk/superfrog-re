#!/usr/bin/env python3
"""Import a Superfrog executable into Ghidra headless, apply games/superfrog/re/symbols.toml, decompile.

    games/superfrog/tools/re_ghidra_import.py <chunk-id> [--out DIR] [--reimport]

The game's driver for toolkit/tools/ghidra_headless.py, which says what it writes: reads the chunk
from the player's disks like games/superfrog/tools/re_listing.py, outside the repo by default
(~/.cache/superfrog-re/re/). The two big executables declare four hunks but carry two (FINDINGS).
Raw regions (d1_stage15, d1_boot) are imported as plain 68000 binaries at 0.
See games/superfrog/docs/RE-WORKBENCH.md.
"""
import argparse, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import re_listing as R
import ghidra_headless
from ghidra_headless import complete_hunks  # noqa: F401


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('chunk')
    ap.add_argument('--out', default=R.DEFAULT_OUT)
    ap.add_argument('--symbols', default=R.SYMBOLS)
    ap.add_argument('--reimport', action='store_true', help='discard the Ghidra program and import afresh')
    a = ap.parse_args(argv)
    try:
        syms = [s for s in R.load_symbols(a.symbols) if s.exe == a.chunk]
    except R.SymbolError as e:
        raise SystemExit(f"re_ghidra_import: {e}")
    data = R.load_chunk(a.chunk, R.load_disks())
    print(ghidra_headless.run_import(a.chunk, data, syms, os.path.expanduser(a.out), 'superfrog',
                                     raw=a.chunk in R.RAW_REGIONS, reimport=a.reimport))


if __name__ == '__main__':
    main()
