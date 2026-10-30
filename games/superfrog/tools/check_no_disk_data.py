#!/usr/bin/env python3
"""Fail if any file in the repository carries a run of disk data (DESIGN §2.3).

The repository and the built program must be shareable without any Team 17 bytes. This check
decompresses every ATN! chunk on the owner's four disks (with games/superfrog/tools/atn.py, into memory; nothing
is written), indexes them, and scans every file git would commit (tracked, plus untracked files
that are not ignored) for a run of 128 or more bytes found in a chunk or in a raw disk image.

    games/superfrog/tools/check_no_disk_data.py [--disks DIR] [FILE ...]

Exit 0 when clean, 1 listing `file:offset (N bytes, same as <source>+offset)` for every hit,
2 when the disks cannot be found. FILEs default to the repository's files.

This is the game's driver (disk lookup, ATN! chunk sources, .sflevel decode); the scan itself is
toolkit/tools/disk_data_scan.py.

A `.sflevel` (an exported level, DESIGN §2.10: JSON with the level's files in base64) is also
scanned with its sections decoded, since base64 hides every run from the byte scan.

A flagged table must be read from the disks instead; this check is never relaxed for it.
"""
import argparse, base64, binascii, glob, json, os, sys, zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', '..', 'toolkit', 'tools'))
from atn import carve_chunks, ATNDecompressor  # noqa: E402
from disk_data_scan import BLOCK, MIN_DISTINCT, MIN_RUN, build_index, repo_files, repo_root, scan  # noqa: E402,F401
from disk_lookup import checkout_dirs, main_checkout  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))


def default_disk_dir():
    """SUPERFROG_DISKS, else games/superfrog/ in the main checkout, else its root."""
    env = os.environ.get('SUPERFROG_DISKS')
    looked = []
    if env:
        if find_adfs(env, quiet=True):
            return env
        looked.append(f'SUPERFROG_DISKS={env}')
    else:
        looked.append('SUPERFROG_DISKS: not set')
    main = main_checkout()
    if main:
        for d in checkout_dirs(main):
            if find_adfs(d, quiet=True):
                return d
            looked.append(f'main checkout {d}')
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


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split('\n\n')[0])
    ap.add_argument('--disks', help='folder holding the four disk images (default: SUPERFROG_DISKS, else games/superfrog/ in the main checkout, else its root)')
    ap.add_argument('files', nargs='*', help='files to check (default: every file git would commit)')
    args = ap.parse_args(argv)

    adfs = find_adfs(args.disks) if args.disks else find_adfs(default_disk_dir())
    sources = disk_sources(adfs)
    index = build_index(sources)
    root = repo_root(HERE)
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
