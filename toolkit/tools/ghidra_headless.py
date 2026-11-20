#!/usr/bin/env python3
"""Import an Amiga executable into Ghidra headless, apply symbols, export the decompilation.

The generic half of a game's Ghidra tool: the game's driver finds the bytes and the symbols
(hunk_listing.py's Sym) and calls run_import. Needs Ghidra with the ghidra-amiga loader
(toolkit/tools/setup/install-ghidra.sh). Writes under `out`:

    bin/<name>.exe                  the executable Ghidra imports
    ghidra/                         the Ghidra project (open it in the GUI to keep naming)
    <name>.c                        decompilation of every function
    <name>.ghidra-names.toml        names given in Ghidra but missing from the symbols

The first run imports and analyses; later runs re-apply the symbols to the existing program and
re-export, keeping what was named in the Ghidra GUI. `reimport` starts over.

A game's own post-scripts run after ApplySymbols: `script_paths` adds folders to Ghidra's script path,
`post_scripts` is a list of (script file, [args]). `complete=False` skips complete_hunks, for a caller
that has checked the file declares no absent hunk and runs without amitools (`python3 -I`).

An executable that declares more hunks than it carries is refused by the Amiga loader, so
complete_hunks appends the absent ones as empty HUNK_BSS. A raw region is imported as a plain 68000
binary at 0.
"""
import glob, os, shutil, struct, subprocess, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

HUNK_BSS, HUNK_END = 0x3EB, 0x3F2


def complete_hunks(data):
    """Append an empty HUNK_BSS for every hunk the header declares but the file lacks."""
    # Imported here: hunk_listing needs amitools, which a stdlib-only caller does not have.
    import hunk_listing as L
    exe = L.parse_exe(data)
    extra = b''.join(struct.pack('>III', HUNK_BSS, h.size // 4, HUNK_END)
                     for h in exe.hunks if h.kind == 'ABSENT')
    return data + extra


def ghidra_home():
    if os.environ.get('GHIDRA_HOME'):
        return os.environ['GHIDRA_HOME']
    run = shutil.which('ghidraRun')
    if run:
        return os.path.join(os.path.dirname(os.path.dirname(os.path.realpath(run))), 'libexec')
    raise SystemExit("Ghidra not found: run toolkit/tools/setup/install-ghidra.sh or set GHIDRA_HOME")


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
    raise SystemExit("no JDK for Ghidra: run toolkit/tools/setup/install-ghidra.sh or set JAVA_HOME")


def run_import(name, data, syms, out, project, raw=False, reimport=False,
               script_paths=(), post_scripts=(), complete=True):
    """Import `data` as `name` into `project` under `out`; returns the path of the decompilation."""
    if not raw and complete:
        data = complete_hunks(data)
    for d in ('bin', 'ghidra'):
        os.makedirs(os.path.join(out, d), exist_ok=True)
    exe_path = os.path.join(out, 'bin', f'{name}.exe')
    with open(exe_path, 'wb') as f:
        f.write(data)
    with tempfile.NamedTemporaryFile('w', suffix='.tsv', delete=False) as f:
        for s in syms:
            note = s.note.replace('\t', ' ').replace('\n', ' ')
            f.write(f"{s.hunk}\t{s.offset}\t{s.name}\t{s.kind}\t{s.size}\t{note}\n")
        tsv = f.name
    marker = os.path.join(out, 'ghidra', f'{name}.imported')
    cmd = [os.path.join(ghidra_home(), 'support', 'analyzeHeadless'), os.path.join(out, 'ghidra'), project]
    if os.path.exists(marker) and not reimport:
        cmd += ['-process', os.path.basename(exe_path), '-noanalysis']
    else:
        cmd += ['-import', exe_path, '-overwrite']
        if raw:
            cmd += ['-loader', 'BinaryLoader', '-processor', '68000:BE:32:default']
    cmd += ['-scriptPath', ';'.join([os.path.join(HERE, 'ghidra'), *script_paths]),
            '-postScript', 'ApplySymbols.java', tsv, os.path.join(out, name), name]
    for script, args in post_scripts:
        cmd += ['-postScript', script, *args]
    try:
        r = subprocess.run(cmd, env=dict(os.environ, JAVA_HOME=java_home()),
                           capture_output=True, text=True, timeout=1800)
    finally:
        os.unlink(tsv)
    scripts = ['ApplySymbols.java'] + [s for s, _ in post_scripts]
    report = [l for l in r.stdout.splitlines()
              if any(f'{s}>' in l for s in scripts) or 'ERROR' in l]
    print('\n'.join(report))
    decompiled = os.path.join(out, f'{name}.c')
    if r.returncode != 0 or not os.path.exists(decompiled):
        sys.stderr.write(r.stdout[-3000:] + r.stderr[-3000:])
        raise SystemExit(f"ghidra_headless: analyzeHeadless failed ({r.returncode})")
    open(marker, 'w').close()
    return decompiled
