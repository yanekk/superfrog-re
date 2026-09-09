#!/usr/bin/env python3
"""Export each level as a standalone annotated PNG with a coordinate grid and a
stable level id, so any tile can be addressed as  <level-id>:<col>,<row>.

Level ids: W<world>-<n> in disk order (e.g. W1-1 = disk1 chunk21). Bonus = WB-*,
world map = WW-*. Detected secret passages are boxed and labelled S1, S2, ...

Origin is top-left; col = X (0..width-1), row = Y (0..height-1), in 16px blocks.
Outputs build/levels_annotated/<id>_<chunk>.png + INDEX.md. Run unpack.py first.
"""
import os, sys
import numpy as np
from PIL import Image, ImageDraw, ImageFont
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import t7mp
import render_levels as RL
import find_secrets as FS

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, 'build', 'levels_annotated')
S = RL.TW                 # 16 px per block
ML, MT = 52, 34           # left / top margin for rulers
STEP = 10                 # label + heavy gridline every STEP blocks
MINOR = 5                 # light gridline every MINOR blocks

def font(sz):
    for name in ('DejaVuSans-Bold.ttf', 'DejaVuSans.ttf', 'Arial.ttf'):
        try:
            return ImageFont.truetype(name, sz)
        except Exception:
            pass
    return ImageFont.load_default()

F_LBL, F_ID, F_S = font(15), font(24), font(15)

def level_ids(assocs):
    """Assign W<world>-<n> ids in order."""
    counters = {}
    ids = []
    for f, world, tpath, pal in assocs:
        wcode = {'B': 'B', 'W': 'W'}.get(world, world)
        counters[wcode] = counters.get(wcode, 0) + 1
        ids.append(f"W{wcode}-{counters[wcode]}")
    return ids

def annotate(level_bytes, tiles, pal, level_id, chunk):
    w, h, _, _ = t7mp.level_grid(level_bytes)
    _, _, mask, comps, _ = FS.find(level_bytes)
    lvl = RL.render_level(level_bytes, tiles, pal).convert('RGB')
    arr = np.array(lvl)
    for cells in comps:                     # tint fake-wall cells magenta
        for y, x in cells:
            blk = arr[y*S:(y+1)*S, x*S:(x+1)*S].astype(np.uint16)
            arr[y*S:(y+1)*S, x*S:(x+1)*S] = (blk*0.35 + np.array([255, 0, 255])*0.65).astype(np.uint8)
    lvl = Image.fromarray(arr)

    canvas = Image.new('RGB', (ML + w*S, MT + h*S), (18, 18, 22))
    canvas.paste(lvl, (ML, MT))
    d = ImageDraw.Draw(canvas)
    grid = Image.new('RGBA', canvas.size, (0, 0, 0, 0))
    gd = ImageDraw.Draw(grid)
    for c in range(0, w+1, MINOR):
        x = ML + c*S
        heavy = (c % STEP == 0)
        gd.line([(x, MT), (x, MT + h*S)], fill=(255, 255, 255, 60 if heavy else 22))
        if heavy:
            d.text((x+2, 2), str(c), font=F_LBL, fill=(210, 210, 220))
    for r in range(0, h+1, MINOR):
        y = MT + r*S
        heavy = (r % STEP == 0)
        gd.line([(ML, y), (ML + w*S, y)], fill=(255, 255, 255, 60 if heavy else 22))
        if heavy:
            d.text((3, y+1), str(r), font=F_LBL, fill=(210, 210, 220))
    canvas = Image.alpha_composite(canvas.convert('RGBA'), grid).convert('RGB')
    d = ImageDraw.Draw(canvas)
    # passage boxes + labels
    passages = []
    for i, cells in enumerate(comps, 1):
        xs = [x for _, x in cells]; ys = [y for y, _ in cells]
        x0, x1, y0, y1 = min(xs), max(xs), min(ys), max(ys)
        d.rectangle([ML+x0*S-1, MT+y0*S-1, ML+(x1+1)*S, MT+(y1+1)*S],
                    outline=(255, 240, 0), width=2)
        lab = f"S{i}"
        d.text((ML+x0*S+1, MT+y0*S-16), lab, font=F_S, fill=(255, 240, 0))
        passages.append((lab, x0, y0, x1, y1, len(cells)))
    d.text((ML+4, 6), f"{level_id}   {chunk}   {w}x{h} blocks",
           font=F_ID, fill=(120, 230, 140))
    return canvas, passages

def main():
    os.makedirs(OUT, exist_ok=True)
    assocs = RL.associations()
    ids = level_ids(assocs)
    tiles_cache = {}
    index = ["# Level index & tile addressing\n",
             "Address any tile as **`<level-id>:<col>,<row>`** (0-indexed, "
             "origin top-left, 16px blocks). E.g. `W1-1:94,53`.\n",
             "Secret passages are boxed and labelled S1.. on each image.\n",
             "| level id | chunk | world | size | passages |",
             "|----------|-------|------:|------|---------:|"]
    for (f, world, tpath, pal), lid in zip(assocs, ids):
        chunk = os.path.basename(f).replace('_53588.bin', '')
        if tpath not in tiles_cache:
            tiles_cache[tpath] = RL.decode_tiles(open(tpath, 'rb').read())
        d = open(f, 'rb').read()
        img, passages = annotate(d, tiles_cache[tpath], pal, lid, chunk)
        out = os.path.join(OUT, f"{lid}_{chunk}.png")
        img.save(out)
        w, h, _, _ = t7mp.level_grid(d)
        index.append(f"| {lid} | {chunk} | {world} | {w}x{h} | {len(passages)} |")
        print(f"{lid:6} {chunk:12} {len(passages):2d} passages -> {os.path.relpath(out, ROOT)}")
    # per-passage appendix
    index.append("\n## Passages per level\n")
    for (f, world, tpath, pal), lid in zip(assocs, ids):
        d = open(f, 'rb').read()
        _, _, _, comps, _ = FS.find(d)
        if not comps:
            continue
        index.append(f"\n### {lid} ({os.path.basename(f).replace('_53588.bin','')})")
        for i, cells in enumerate(comps, 1):
            xs = [x for _, x in cells]; ys = [y for y, _ in cells]
            index.append(f"- S{i}: cols {min(xs)}-{max(xs)}, rows {min(ys)}-{max(ys)} "
                         f"({len(cells)} cells) — id `{lid}:{min(xs)},{min(ys)}`")
    open(os.path.join(OUT, 'INDEX.md'), 'w').write("\n".join(index))
    print(f"\n{len(assocs)} annotated levels -> {os.path.relpath(OUT, ROOT)}/ (see INDEX.md)")

if __name__ == '__main__':
    main()
