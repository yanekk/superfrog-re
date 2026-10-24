"""Download finished model outputs: python tools/hd/fetch.py <out dir> name=url [name=url ...]"""
import os
import subprocess
import sys

out = sys.argv[1]
os.makedirs(out, exist_ok=True)
for arg in sys.argv[2:]:
    name, url = arg.split('=', 1)
    dst = os.path.join(out, name + '.png')
    r = subprocess.run(['curl', '-s', '-f', '-o', dst, url])
    print(name, 'ok' if r.returncode == 0 else f'FAILED ({r.returncode})')
