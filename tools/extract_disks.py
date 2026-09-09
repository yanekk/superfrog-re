#!/usr/bin/env python3
"""Extract the .adf disk images from the provided .zip files into build/adf/."""
import glob, os, zipfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, 'build', 'adf')

def main():
    os.makedirs(OUT, exist_ok=True)
    zips = sorted(glob.glob(os.path.join(ROOT, '*.zip')))
    if not zips:
        raise SystemExit("no .zip disk images found in repo root")
    for z in zips:
        with zipfile.ZipFile(z) as zf:
            for name in zf.namelist():
                if name.lower().endswith('.adf'):
                    dest = os.path.join(OUT, os.path.basename(name))
                    with zf.open(name) as src, open(dest, 'wb') as f:
                        f.write(src.read())
                    print("extracted", os.path.relpath(dest, ROOT))

if __name__ == '__main__':
    main()
