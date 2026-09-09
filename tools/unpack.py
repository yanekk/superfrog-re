#!/usr/bin/env python3
"""Carve + decompress + classify every ATN! chunk on all four disks.

Writes decompressed chunks to build/decomp/ and prints an inventory table.
Run tools/extract_disks.py first.
"""
import glob, os, struct, sys
from atn import carve_chunks, ATNDecompressor

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ADF = os.path.join(ROOT, 'build', 'adf')
OUT = os.path.join(ROOT, 'build', 'decomp')

def classify(out):
    tag = out[:4]
    if tag == b'T7MP':
        # a level: read XBLK/YBLK if present
        try:
            x = struct.unpack_from('>I', out, 0x14)[0]
            y = struct.unpack_from('>I', out, 0x20)[0]
            return f"level {x}x{y}"
        except Exception:
            return "level"
    if out[:4] == b'FORM':
        return f"IFF:{out[8:12].decode(errors='.')}"
    if struct.unpack_from('>I', out, 0)[0] == 0x3f3:
        return "hunk-exe"
    if tag.isalpha() and tag.isupper():
        return f"tag:{tag.decode()}"
    return "binary/gfx"

def main():
    os.makedirs(OUT, exist_ok=True)
    d1 = sorted(glob.glob(os.path.join(ADF, '*Disk 1*.adf')))
    if not d1:
        raise SystemExit("run tools/extract_disks.py first")
    dec = ATNDecompressor(open(d1[0], 'rb').read())
    ok_n = lvl_n = tot = 0
    print(f"{'disk':5}{'#':>3}{'decomp':>9}{'comp':>8}  {'ok':3} {'class':14} name")
    for adf in sorted(glob.glob(os.path.join(ADF, '*.adf'))):
        data = open(adf, 'rb').read()
        dn = os.path.basename(adf).split('Disk ')[1][0]
        for i, (o, dsz, csz, chunk) in enumerate(carve_chunks(data), 1):
            tot += 1
            out, ok = dec.decompress(chunk, dsz)
            cl = classify(out)
            slug = cl.split()[0].replace(':', '_').replace('/', '-')
            name = f"d{dn}_c{i:02d}_{slug}_{dsz}.bin"
            open(os.path.join(OUT, name), 'wb').write(out)
            ok_n += ok
            if cl.startswith('level'):
                lvl_n += 1
            print(f"d{dn:<4}{i:3d}{dsz:9d}{csz:8d}  {'ok' if ok else 'X':3} {cl:14} {name}")
    print(f"\n{ok_n}/{tot} decompressed OK; {lvl_n} T7MP levels -> {os.path.relpath(OUT, ROOT)}/")

if __name__ == '__main__':
    main()
