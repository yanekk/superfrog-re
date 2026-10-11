#!/usr/bin/env python3
"""Import a Superfrog executable into Ghidra headless, apply re/symbols.toml, decompile.

    tools/re_ghidra_import.py <chunk-id> [--out DIR] [--reimport]

Needs Ghidra with the ghidra-amiga loader (tools/setup/install-ghidra.sh). Reads the
chunk from the player's disks like tools/re_listing.py and writes, outside the repo
(default ~/.cache/superfrog-re/re/):

    bin/<chunk>.exe                 the decompressed executable Ghidra imports
    ghidra/                         the Ghidra project (open it in the GUI to keep naming)
    <chunk>.c                       decompilation of every function
    <chunk>.ghidra-names.toml       names given in Ghidra but missing from symbols.toml

The first run imports and analyses; later runs re-apply symbols.toml to the existing
program and re-export, keeping what was named in the Ghidra GUI. --reimport starts over.

The two big executables declare four hunks but carry two; the Amiga loader refuses
them, so the absent hunks are appended as empty HUNK_BSS before import (FINDINGS).
Raw regions (d1_stage15, d1_boot) are imported as plain 68000 binaries at 0.
See docs/RE-WORKBENCH.md.
"""
import argparse, glob, os, shutil, struct, subprocess, sys, tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import re_listing as R

HUNK_BSS, HUNK_END = 0x3EB, 0x3F2


def complete_hunks(data):
    """Append an empty HUNK_BSS for every hunk the header declares but the file lacks."""
    exe = R.parse_exe(data)
    extra = b''.join(struct.pack('>III', HUNK_BSS, h.size // 4, HUNK_END)
                     for h in exe.hunks if h.kind == 'ABSENT')
    return data + extra


def ghidra_home():
    if os.environ.get('GHIDRA_HOME'):
        return os.environ['GHIDRA_HOME']
    run = shutil.which('ghidraRun')
    if run:
        return os.path.join(os.path.dirname(os.path.dirname(os.path.realpath(run))), 'libexec')
    raise SystemExit("Ghidra not found: run tools/setup/install-ghidra.sh or set GHIDRA_HOME")


def java_home():
    if os.environ.get('JAVA_HOME'):
        return os.environ['JAVA_HOME']
    for save in sorted(glob.glob(os.path.expanduser('~/Library/ghidra/ghidra_*/java_home.save'))):
        with open(save) as f:
            home = f.read().strip()
        if os.path.isdir(home):
            return home
    home = '/opt/homebrew/opt/openjdk@21/libexec/openjdk.jdk/Contents/Home'
    if os.path.isdir(home):
        return home
    raise SystemExit("no JDK for Ghidra: run tools/setup/install-ghidra.sh or set JAVA_HOME")


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('chunk')
    ap.add_argument('--out', default=R.DEFAULT_OUT)
    ap.add_argument('--symbols', default=R.SYMBOLS)
    ap.add_argument('--reimport', action='store_true', help='discard the Ghidra program and import afresh')
    a = ap.parse_args(argv)
    out = os.path.expanduser(a.out)
    try:
        syms = [s for s in R.load_symbols(a.symbols) if s.exe == a.chunk]
    except R.SymbolError as e:
        raise SystemExit(f"re_ghidra_import: {e}")
    data = R.load_chunk(a.chunk, R.load_disks())
    raw = a.chunk in R.RAW_REGIONS
    if not raw:
        data = complete_hunks(data)
    for d in ('bin', 'ghidra'):
        os.makedirs(os.path.join(out, d), exist_ok=True)
    exe_path = os.path.join(out, 'bin', f'{a.chunk}.exe')
    with open(exe_path, 'wb') as f:
        f.write(data)
    with tempfile.NamedTemporaryFile('w', suffix='.tsv', delete=False) as f:
        for s in syms:
            note = s.note.replace('\t', ' ').replace('\n', ' ')
            f.write(f"{s.hunk}\t{s.offset}\t{s.name}\t{s.kind}\t{s.size}\t{note}\n")
        tsv = f.name
    marker = os.path.join(out, 'ghidra', f'{a.chunk}.imported')
    cmd = [os.path.join(ghidra_home(), 'support', 'analyzeHeadless'), os.path.join(out, 'ghidra'), 'superfrog']
    if os.path.exists(marker) and not a.reimport:
        cmd += ['-process', os.path.basename(exe_path), '-noanalysis']
    else:
        cmd += ['-import', exe_path, '-overwrite']
        if raw:
            cmd += ['-loader', 'BinaryLoader', '-processor', '68000:BE:32:default']
    cmd += ['-scriptPath', os.path.join(R.ROOT, 'tools', 'ghidra'),
            '-postScript', 'SuperfrogSymbols.java', tsv, os.path.join(out, a.chunk), a.chunk]
    try:
        r = subprocess.run(cmd, env=dict(os.environ, JAVA_HOME=java_home()),
                           capture_output=True, text=True, timeout=1800)
    finally:
        os.unlink(tsv)
    report = [l for l in r.stdout.splitlines() if 'SuperfrogSymbols.java>' in l or 'ERROR' in l]
    print('\n'.join(report))
    if r.returncode != 0 or not os.path.exists(os.path.join(out, f'{a.chunk}.c')):
        sys.stderr.write(r.stdout[-3000:] + r.stderr[-3000:])
        raise SystemExit(f"re_ghidra_import: analyzeHeadless failed ({r.returncode})")
    open(marker, 'w').close()
    print(os.path.join(out, f'{a.chunk}.c'))


if __name__ == '__main__':
    main()
