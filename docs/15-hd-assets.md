# 15 — HD assets (Higgsfield, World 1)

A 4x HD version of World 1's art, redrawn by image models on Higgsfield (MCP) and rebuilt into
the game's own units: 64 x 64 tiles, the four levels composed from them, and RGBA sprite sheets
with the game's frame table. The engine is not changed; nothing here is read by it yet.

This replaces the first pilot (faithful 4x upscale, branch history up to 3872ee0). The owner
asked on 2026-10-24 for a clearly redrawn look, no visible seams and a simpler method.

```
python tools/hd_assets.py prepare  --world 1   # level windows + rare-tile sheets -> build/hd2/w1/src/
python tools/hd_assets.py sprites  --world 1   # (first run) sprite sheet halves -> src/
# a session runs each src image through the model (Higgsfield MCP; prompts below) and saves the
# result as build/hd2/w1/out/<same name>.png  (tools/hd/upload.py, tools/hd/fetch.py move bytes)
python tools/hd_assets.py register --world 1   # out/ -> reg/, aligned to the original grid
python tools/hd/refit.py prepare 1             # optional: worst-fitting tiles in context -> src/refit00*
# model: inpaint refit00.png with refit00_mask.png -> out/refit00.png
python tools/hd_assets.py tiles    --world 1   # reg/ (+ refit) -> final/ tiles, levels, report
python tools/hd_assets.py sprites  --world 1   # (second run) out/ -> final/ sprite sheets, frames
```

Outputs are derived from the disks, so they stay in `build/` (git-ignored).

| File in `build/hd2/w1/final/` | What |
|---|---|
| `tiles_W1_hd.png` | all 840 tiles, 40 per row, 64 x 64 each (index = row*40 + col) |
| `level_W1-{1..4}_hd.png` | each level composed from the HD tiles, 19 200 x 5 376 |
| `level_*_preview.png` | the same at 1/8 |
| `hero_hd.png`, `bobs1_hd.png` | frog sheet (1280 x 1536) and W1 bob sheet (1280 x 1280), RGBA |
| `frames.json` | frame records from the exe (hero h0+$1a6f4, shared h0+$2c9b4, W1 h0+$2ed2c), original pixels; multiply by `scale` |
| `frames/{hero,shared,world}/NNN.png` | one PNG per non-empty record |
| `report.json` | seam score per level, rigid joints, families, sky colours |

## Style and model

Style D, "hi-res pixel art, GPT look", chosen by the owner on 2026-10-24 from six samples (painted,
hi-res pixel, vector cartoon, each on Nano Banana 2.1 and GPT Image 2.5). Model `gpt_image_2_5`,
2k, quality high, 27:16 (2592 x 1536 = a 648 x 384 original window at 4x): 2.75 credits an image.
Every request carries the chosen sample (job 5cdea430) as style reference.

Stability test (same input, two runs each, 2026-10-24): Nano Banana stays aligned only when it
barely redraws, and with `aspect_ratio: auto` once returned a square, recomposed image; GPT Image
redraws fully but drifts 1.5-3 original px locally and run to run. So nothing relies on exact
positions: every output is registered (below).

**Prompt: texture-preserving.** The first run asked for a redraw keeping the layout; the model
painted its own large boulders over the dirt. Interchangeable dirt tiles cut from that cannot join
(a 64 px wallpaper grid). The prompt now says to keep every stone, crack and mortar line in place,
with a dirt redraw (job c84e2e4d) as a second style reference. That is what makes the dirt join.

## How it works

- **Windows.** Greedy set cover over the four levels picks 648 x 384 windows; windows adding fewer
  than 8 new tiles are dropped and their tiles go on rare-tile sheets (5 x 5-tile context patches,
  32 per sheet). W1: 16 windows + 2 sheets for 732 used tiles (after merging 74 exact duplicates).
- **Registration** (`hd/register.py`, numpy only). One x/y scale + shift, then 40 px blocks of edge
  features matched within +-4 original px, outliers dropped, smoothed into a displacement field,
  warp. ~12 s a window.
- **Harmonise.** A smooth per-painting correction pulls every flat palette area to the median
  colour of that area over all paintings.
- **Joints, by kind** (W1 shares of joint occurrences): sky|sky 36% (flat sky pinned to one colour
  per palette index: exact); dirt/cave/leaf families 52% (interchangeable fill tiles,
  `analysis.families`; join because the redraw keeps the original's stones); rigid sides (a side
  that only ever meets one tile: trees, signs) are cut as a stamp from one place in one painting
  (exact); the rest (~14%, 5 496 distinct pairs).
- **Choosing copies.** A tile occurs in several windows. `choose_jointly` picks one copy per tile
  so that touching edges agree over every joint the levels use, weighted by frequency, with a
  penalty for copies far from the tile's typical look (ICM over 8 candidates per tile).
- **Refit** (`hd/refit.py`). Tiles with one or two copies cannot be chosen to fit. The 45 worst
  (edge mismatch against their neighbours) are shown in their most common 3 x 3 context of finished
  HD tiles with their original pixels in the middle, and Nano Banana inpaints only the middle. A refit
  replaces a tile only if its edges fit the neighbours better (`tiles.fit_cost`): W1 kept 7 of 45;
  the model left most middles near the original and returned the 16:9 sheet stretched to a square.
- **Tried and dropped.** A shared torus-periodic border ring for families (made a visible grid);
  least-squares levelling of border colours per pixel (fought real edges such as a mortar line on a
  tile border, smeared them) and per tile (made the cave passage murky).

## Sprites

Sheets go in halves at 8x on magenta (324 x 192 canvas = 27:16), registered like windows (drift
0.5 px), reduced to 4x. Alpha is the original silhouette with a smoothed outline, so the shape the
game collides against cannot change. Magenta inside it is filled from neighbours, large areas from
the original (the blue panel behind the spring), and the magenta tint is removed along edges.
Frames that are the same picture up to a one-to-one colour swap share the copy that best matches
its original, recoloured per frame. Text the model misreads is drawn from the original with smooth
outlines instead (`sprites.TEXT`: the 100/200/50 score popups; it wrote 201).

## Seam score

`tiles.seam_score`: mean colour step across tile borders / mean step between neighbouring columns
inside tiles. It counts real edges in the art too, so judge by eye; the original at 1x scores
1.03-1.16, and the report's `seam_original_4x` is not comparable (pixel doubling).

## Known issues

- Rare edge tiles (one copy) can still show a square outline, e.g. W1-4's right cliffs; a faint
  row line runs through the dark cave dirt in W1-4's tunnel.
- Dirt keeps the original's two-row stone rhythm (visible from afar, as in the original).
- Registration resamples, so HD pixels are slightly soft rather than hard-edged.
- Colour cycling (COLOR06, docs/10) is baked in at its base colour.
- World 1 only (`WORLDS` in `hd/source.py`); other worlds need their chunk ids and bob table.

## Cost (2026-10-24)

About 142 credits: style samples 14.25, stability test 9.5, boulder run 47 (discarded), sprites 11,
dirt tests 4.75, texture-preserving run 49.5, refit 2, spike 2.75.
