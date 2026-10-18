#!/usr/bin/env python3
"""Fail if any file in the repository carries a run of disk data (DESIGN §2.3).

The repository and the built program must be shareable without any Team 17 bytes. This check
decompresses every ATN! chunk on the owner's four disks (with tools/atn.py, into memory; nothing
is written), indexes them, and scans every file git would commit (tracked, plus untracked files
that are not ignored) for a run of 128 or more bytes found in a chunk or in a raw disk image.

    tools/check_no_disk_data.py [--disks DIR] [FILE ...]

Exit 0 when clean, 1 listing `file:offset (N bytes, same as <source>+offset)` for every hit,
2 when the disks cannot be found. FILEs default to the repository's files.

How: every chunk is indexed at a 64-byte stride, keeping every place a block occurs; each file is
scanned at a 1-byte stride, so any copied run of 127 bytes or more contains one indexed block. A
hit is extended byte by byte against each place and reported if the longest shared run is 128
bytes or longer. Blocks with fewer than 4 distinct byte values (zero fill, solid colour) are not
indexed: they are not data anyone could recover.

A `.sflevel` (an exported level, DESIGN §2.10: JSON with the level's files in base64) is also
scanned with its sections decoded, since base64 hides every run from the byte scan.

A flagged table must be read from the disks instead; this check is never relaxed for it.
"""
import argparse, base64, binascii, glob, io, json, os, subprocess, sys, zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from atn import carve_chunks, ATNDecompressor  # noqa: E402

BLOCK = 64
MIN_RUN = 128
MIN_DISTINCT = 4


def repo_root():
    return subprocess.run(['git', 'rev-parse', '--show-toplevel'], cwd=os.path.dirname(os.path.abspath(__file__)),
                          capture_output=True, text=True, check=True).stdout.strip()


def default_disk_dir():
    """SUPERFROG_DISKS, else the main checkout (the parent of the git common dir)."""
    env = os.environ.get('SUPERFROG_DISKS')
    looked = []
    if env:
        if find_adfs(env, quiet=True):
            return env
        looked.append(f'SUPERFROG_DISKS={env}')
    else:
        looked.append('SUPERFROG_DISKS: not set')
    common = subprocess.run(['git', 'rev-parse', '--path-format=absolute', '--git-common-dir'],
                            cwd=os.path.dirname(os.path.abspath(__file__)),
                            capture_output=True, text=True).stdout.strip()
    if common:
        main = os.path.dirname(common.rstrip('/'))
        if find_adfs(main, quiet=True):
            return main
        looked.append(f'main checkout {main}')
    raise SystemExit('disk images not found; looked in:\n  - ' + '\n  - '.join(looked))


def find_adfs(d, quiet=False):
    """{disk number: ADF bytes} for the four disks in `d`, zipped or raw."""
    adfs = {}
    for path in sorted(glob.glob(os.path.join(d, '*'))):
        name = os.path.basename(path).lower()
        n = next((k for k in '1234' if f'disk {k} of 4' in name), None)
        if n is None:
            continue
        if name.endswith('.adf'):
            adfs[n] = open(path, 'rb').read()
        elif name.endswith('.zip'):
            with zipfile.ZipFile(path) as zf:
                for member in zf.namelist():
                    if member.lower().endswith('.adf'):
                        adfs[n] = zf.read(member)
    if len(adfs) != 4:
        if quiet:
            return None
        raise SystemExit(f'{d}: found disks {sorted(adfs)} of 4')
    return adfs


def disk_sources(adfs):
    """[(label, bytes)]: every decompressed chunk and every raw disk image."""
    dec = ATNDecompressor(adfs['1'])
    sources = []
    for n, adf in sorted(adfs.items()):
        sources.append((f'd{n}.adf', adf))
        for i, (_o, dsz, _csz, chunk) in enumerate(carve_chunks(adf), 1):
            out, _ok = dec.decompress(chunk, dsz)
            sources.append((f'd{n}_c{i:02d}', out))
    return sources


def build_index(sources):
    index = {}
    for si, (_label, data) in enumerate(sources):
        for off in range(0, len(data) - BLOCK + 1, BLOCK):
            block = data[off:off + BLOCK]
            if len(set(block)) >= MIN_DISTINCT:
                index.setdefault(block, []).append((si, off))
    return index


def scan(data, index, sources):
    """[(file offset, run length, source label, source offset)] for every shared run >= MIN_RUN."""
    hits = []
    pos = 0
    n = len(data)
    while pos + BLOCK <= n:
        refs = index.get(data[pos:pos + BLOCK])
        if refs is None:
            pos += 1
            continue
        # The block may occur at several places on the disks; measure the run against each.
        run, back, fwd, si, soff = 0, 0, 0, 0, 0
        for rsi, rsoff in refs:
            src = sources[rsi][1]
            b = 0
            while pos - b > 0 and rsoff - b > 0 and data[pos - b - 1] == src[rsoff - b - 1]:
                b += 1
            f = BLOCK
            while pos + f < n and rsoff + f < len(src) and data[pos + f] == src[rsoff + f]:
                f += 1
            if b + f > run:
                run, back, fwd, si, soff = b + f, b, f, rsi, rsoff
        if run >= MIN_RUN:
            hits.append((pos - back, run, sources[si][0], soff - back))
            pos += fwd
        else:
            pos += 1
    return hits


def sflevel_sections(data):
    """[(name, bytes)]: the decoded sections of a `.sflevel`; [] for any other file."""
    try:
        doc = json.loads(data)
    except (ValueError, UnicodeDecodeError):
        return []
    if not isinstance(doc, dict) or doc.get('format') != 'sflevel' or not isinstance(doc.get('sections'), dict):
        return []
    out = []
    for name, text in doc['sections'].items():
        try:
            out.append((name, base64.b64decode(text, validate=True)))
        except (TypeError, ValueError, binascii.Error):
            pass
    return out


def repo_files(root):
    out = subprocess.run(['git', 'ls-files', '-z', '--cached', '--others', '--exclude-standard'],
                         cwd=root, capture_output=True, check=True).stdout
    return [os.path.join(root, p) for p in out.decode().split('\0') if p]


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split('\n\n')[0])
    ap.add_argument('--disks', help='folder holding the four disk images (default: SUPERFROG_DISKS, else the main checkout)')
    ap.add_argument('files', nargs='*', help='files to check (default: every file git would commit)')
    args = ap.parse_args(argv)

    adfs = find_adfs(args.disks) if args.disks else find_adfs(default_disk_dir())
    sources = disk_sources(adfs)
    index = build_index(sources)
    root = repo_root()
    files = args.files or repo_files(root)

    found = 0
    for path in files:
        if not os.path.isfile(path):
            continue
        data = open(path, 'rb').read()
        rel = os.path.relpath(path, root) if not args.files else path
        for off, run, label, soff in scan(data, index, sources):
            print(f'{rel}:{off} ({run} bytes, same as {label}+{soff})')
            found += 1
        for name, section in sflevel_sections(data):
            for off, run, label, soff in scan(section, index, sources):
                print(f'{rel}:section {name}:{off} ({run} bytes, same as {label}+{soff})')
                found += 1
    if found:
        print(f'\n{found} run(s) of disk data found. Read that data from the disks instead; '
              'do not relax this check (DESIGN §2.3).', file=sys.stderr)
        return 1
    print(f'no disk data in {len(files)} file(s)')
    return 0


if __name__ == '__main__':
    sys.exit(main())
