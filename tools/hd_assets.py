"""HD assets for a world, redrawn by an image model on Higgsfield. See docs/15-hd-assets.md.

    python tools/hd_assets.py prepare  --world 1   # model inputs -> build/hd2/w1/src/
    (a session runs each src image through the model via the Higgsfield MCP tools and saves
     the result as build/hd2/w1/out/<same name>.png; tools/hd/upload.py, tools/hd/fetch.py)
    python tools/hd_assets.py register --world 1   # out/ -> reg/ (aligned to the original grid)
    python tools/hd_assets.py tiles    --world 1   # reg/ -> final/ tiles, levels, report
    python tools/hd_assets.py sprites  --world 1   # sprite sheets (see hd/sprites.py)

The engine does not read any of this yet; outputs are derived from the disks and stay in build/.
"""
import argparse
import json
import os
import sys

import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from hd import analysis as an, prepare as pr, source as s, tiles as tz  # noqa: E402


def cmd_tiles(world):
    d = pr.out_dir(world)
    fin = os.path.join(d, 'final')
    os.makedirs(fin, exist_ok=True)
    tl = s.tiles(world); lv = s.levels(world); pal = s.palette(world)
    canon = an.canonical(tl)
    hd, info = tz.build(world)
    # tile sheet: 40 per row, index = row * 40 + col, every id (duplicates share their twin's art)
    n = len(tl)
    sheet = np.zeros((-(-n // 40) * tz.TS, 40 * tz.TS, 3), np.uint8)
    for t in range(n):
        p = hd.get(canon[t])
        if p is None:
            p = tz.up(pal[tl[t]])
        r, c = divmod(t, 40)
        sheet[r * tz.TS:(r + 1) * tz.TS, c * tz.TS:(c + 1) * tz.TS] = np.clip(p, 0, 255).round()
    Image.fromarray(sheet).save(os.path.join(fin, f'tiles_W{world}_hd.png'))
    report = dict(info, levels={})
    for name, g in lv.items():
        img = tz.compose(hd, g, canon)
        Image.fromarray(img).save(os.path.join(fin, f'level_{name}_hd.png'))
        Image.fromarray(img).resize((img.shape[1] // 8, img.shape[0] // 8), Image.LANCZOS).save(
            os.path.join(fin, f'level_{name}_preview.png'))
        orig = tz.up(pal[s.render_grid(g, tl)])
        report['levels'][name] = dict(seam_hd=round(tz.seam_score(img), 3),
                                      seam_original_4x=round(tz.seam_score(orig), 3))
        print(name, report['levels'][name], flush=True)
        del img, orig
    with open(os.path.join(fin, 'report.json'), 'w') as fh:
        json.dump(report, fh, indent=1)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('cmd', choices=['prepare', 'register', 'tiles', 'sprites'])
    ap.add_argument('--world', type=int, default=1)
    a = ap.parse_args()
    if a.cmd == 'prepare':
        pr.prepare(a.world)
    elif a.cmd == 'register':
        from hd import register_all
        register_all.main(a.world)
    elif a.cmd == 'tiles':
        cmd_tiles(a.world)
    else:
        from hd import sprites
        sprites.main(a.world)


if __name__ == '__main__':
    main()
