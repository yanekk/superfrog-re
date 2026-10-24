"""Print Higgsfield generate_image_batch request lists for a world's uploaded inputs.

    python tools/hd/requests.py <world> <names,comma,separated> [anchor job id] [start index]

A session pastes the output into the batch tool (at most 8 at a time: the backend rate-limits
more). Style references: the owner's chosen sample (STYLE) and, for a world's later windows, the
world's own first redraw (anchor), so every window of a world is drawn in one hand.
"""
import json
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
STYLE = '5cdea430-093a-43dc-a1ce-c17f718a4bd5'          # style D, chosen 2026-10-24
SPRITE_REF = '9438c781-8969-42c1-916d-9bd9a3a7d8d2'     # W1 bob sheet redraw, for sprite style

WINDOW = ("Re-render the FIRST image (a screen from a 1993 2D platformer) as modern high-resolution "
          "pixel art in the art style of the {refs} (their rendering: rich colour ramps, highlights "
          "and shading, crisp fine pixels about 4x finer than the original, no blur). The other images "
          "are STYLE references only: do not copy their content, and keep this screen's own materials "
          "and colour scheme. This is a texture-preserving upscale: keep EVERY brick, stone, block, "
          "plank, pipe, crack, mortar line, pattern, edge, ledge, slope, object and pickup of the first "
          "image in exactly the same place, the same shape and the same size; do not invent new "
          "patterns, do not merge or enlarge shapes, do not move, add or remove anything. Same "
          "framing; keep flat background colours flat. No text, no border.")
PACK = ("The FIRST image is a sheet of square panels cut from a 1993 2D platformer, on a flat "
        "background. Re-render it as modern high-resolution pixel art in the art style of the {refs} "
        "(rich colour ramps, highlights and shading, crisp fine pixels about 4x finer, no blur); they "
        "are STYLE references only, keep this sheet's own materials and colours. Texture-preserving: "
        "each panel stays a separate square in the same place; keep every brick, stone, block, edge, "
        "pattern and object in exactly the same place, shape and size; do not merge panels, do not "
        "move, add or remove anything; keep empty background empty. No text, no border.")
SPRITE = ("The FIRST image is a sprite sheet from a 1993 2D platformer on a flat magenta (#FF00FF) "
          "background: animation frames of enemies, pickups and effects. Redraw EVERY frame as modern "
          "HIGH-RESOLUTION PIXEL ART in the art style of the SECOND and THIRD images (crisp fine "
          "pixels, richer shading, clean dark outlines, no blur); they are STYLE references only. Keep "
          "EXACTLY the layout of the first image: every frame in the same place, same size, same pose "
          "and same silhouette; frames of one animation must look identical except for the pose; keep "
          "digits and letters the same characters. Keep the background pure flat magenta #FF00FF "
          "outside the sprites (keep any coloured panel behind a sprite), no shadows, no glow, no "
          "gradient. Do not add, remove or merge frames. No border.")


def main(world, names, anchor=None, start=0):
    ids = json.load(open(os.path.join(ROOT, 'build', 'hd2', f'w{world}', 'media_ids.json')))
    out = []
    for i, n in enumerate(names):
        if n.startswith('bobs'):
            medias, prompt = [ids[n], STYLE, SPRITE_REF], SPRITE
        else:
            medias = [ids[n], STYLE] + ([anchor] if anchor else [])
            refs = 'SECOND and THIRD images' if anchor else 'SECOND image'
            prompt = (PACK if n.startswith('pack') else WINDOW).format(refs=refs)
        out.append(dict(index=start + i, params=dict(
            model='gpt_image_2_5', resolution='2k', quality='high', aspect_ratio='27:16',
            medias=[dict(value=m, role='image_references') for m in medias], prompt=prompt)))
    print(json.dumps(out))


if __name__ == '__main__':
    main(int(sys.argv[1]), sys.argv[2].split(','), sys.argv[3] if len(sys.argv) > 3 and sys.argv[3] != '-' else None,
         int(sys.argv[4]) if len(sys.argv) > 4 else 0)
