#!/usr/bin/env python3
"""Extract the .adf disk images from the provided .zip files into games/superfrog/build/adf/."""
import glob, os, sys, zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from disk_lookup import disks_dir

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, 'build', 'adf')

def main():
    os.makedirs(OUT, exist_ok=True)
    zips = sorted(glob.glob(os.path.join(disks_dir(), '*.zip')))
    if not zips:
        raise SystemExit("no .zip disk images found; set SUPERFROG_DISKS or put them in games/superfrog/")
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
