"""Register every downloaded model output of a world onto its source grid (cached).

python tools/hd/register_all.py <world>   ->  build/hd2/w<N>/reg/<name>.png + registration.json
"""
import json
import os
import sys
import time

import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from hd import register as rg, source as s  # noqa: E402
from hd.prepare import out_dir  # noqa: E402


def main(world):
    d = out_dir(world)
    pal = s.palette(world)
    os.makedirs(os.path.join(d, 'reg'), exist_ok=True)
    info_path = os.path.join(d, 'registration.json')
    info = json.load(open(info_path)) if os.path.exists(info_path) else {}
    with open(os.path.join(d, 'manifest.json')) as fh:
        man = json.load(fh)
    for src in man['sources']:
        n = src['name']
        o = os.path.join(d, 'out', n + '.png')
        dst = os.path.join(d, 'reg', n + '.png')
        if not os.path.exists(o) or (os.path.exists(dst) and n in info):
            continue
        t = time.time()
        idx = np.load(os.path.join(d, 'src', n + '_idx.npy'))
        warped, inf = rg.register(pal[idx], np.asarray(Image.open(o).convert('RGB')))
        Image.fromarray(warped.clip(0, 255).round().astype(np.uint8)).save(dst)
        info[n] = {k: float(v) for k, v in inf.items()}
        with open(info_path, 'w') as fh:
            json.dump(info, fh, indent=1)
        print(n, info[n], f'{time.time() - t:.0f}s', flush=True)


if __name__ == '__main__':
    main(int(sys.argv[1]))
