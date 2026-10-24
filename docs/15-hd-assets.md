# 15 — HD assets (Higgsfield pilot, World 1)

A 4x HD version of the game's art, redrawn by an image model on Higgsfield (MCP) and rebuilt
into the game's own units: 64 x 64 tiles, sprite sheets with alpha, and the game's frame table.
The engine is not changed; nothing here is read by it yet. Tool: `tools/hd_assets.py`.

```
python tools/hd_assets.py prepare  --world 1   # build/hd/w1/src/*.png + manifest.json
# run each src/*.png through the model, save the result as build/hd/w1/out/<same name>.png
python tools/hd_assets.py assemble --world 1   # build/hd/w1/final/
```

Outputs are derived from the disks, so they stay in `build/` (git-ignored).

| File in `build/hd/w1/final/` | What |
|---|---|
| `tiles_W1_hd.png` | 840 tiles, 40 per row, 64 x 64 each (index = row*40 + col) |
| `level_W1-{1..4}_hd.png` | each level composed from HD tiles, 19 200 x 5 376 |
| `level_*_preview.png` | the same at 1/8 |
| `hero_hd.png`, `bobs1_hd.png` | frog sheet (1280 x 1536) and W1 bob sheet (1280 x 1280), RGBA |
| `frames.json` | frame records from the exe (`hero_rects` h0+$1a6f4, shared h0+$2c9b4, W1 h0+$2ed2c), original pixels; multiply by `scale` |
| `frames/{hero,shared,world}/NNN.png` | one PNG per non-empty record |
| `report.json` | coverage, error against the original, seam score |

## Style and model

Style C, "faithful upscale", chosen by the owner on 2026-10-24 from three samples (hi-res pixel
art, painted cartoon, faithful upscale). Painted redesigned the frog (two eyes, new body) and
moved small objects. Model `nano_banana_pro` at 2k, 16:9: 2 credits an image; outputs are
2752 x 1536. The pilot cost 48 credits (12 for style samples, 36 for 18 inputs incl. a retry).

## How it works

- **Tiles are redrawn in context.** A tileset is fragments (a tree spans dozens of tiles); alone they
  mean nothing to the model. `prepare` picks level windows of 43 x 24 tiles (exactly 2752 x 1536 at
  4x) by greedy set cover over all four W1 levels: 11 windows cover 718 of the 794 tiles used. The
  rest go in packed sheets of 5 x 5-tile context patches (3 sheets).
- **Registration.** The model keeps layout but not framing: outputs come back ~0.5-1.2% scaled and
  up to 8 px shifted. `register` fits x/y scale and shift per image (cached in `registration.json`);
  `take` then nudges each tile up to 6 px.
- **Choosing a version.** A tile occurs in many windows. Cost = typicality (medoid distance) +
  error against the original - centrality bonus. Lowest error alone picks copies the model left
  blocky; medoid alone fails on tiny letters. `choose_jointly` then picks per tile so that
  neighbouring tiles' touching edges agree (ICM over all adjacent pairs in the levels).
- **Colour pinning.** The model repaints flat areas a slightly different shade per image, which
  showed as darker boxes around coins and apples. `normalise` sets every pixel whose original 3x3
  neighbourhood is one palette index to that exact colour (2 px feather), then corrects remaining
  drift at low frequency (original minus the redraw shrunk to 16 px, blurred, enlarged, added).
- **Seams.** `seam_fix` evens each edge toward the mean of the touching lines over all neighbour
  pairs, smoothed 4 px along the edge, 8 px falloff. A 20 px per-pixel version wiped the mushroom
  cap's top outline. `seam_score` (edge step / in-tile step) 1.36 -> 0.85.
- **Overrides.** `tools/hd_overrides/w<N>.json` names the model image to take a tile from when no
  rule is right. W1: tile 660, an R bonus coin the model drew as R, A, n, R, B.
- **Sprites.** Sheets go in halves at 8x on magenta. Alpha = model's key-colour cut x the original
  silhouette (grown 2 px, blurred), so the outline the game collides against cannot change.
  Frog pixel v is palette 16+v (attached sprites); bobs use all 32; index 0 is transparent.

## Known issues (pilot)

- A few faint flecks at canopy-outline and bone tile borders; flat background art (clouds,
  pine silhouettes) stays slightly blocky because the model mostly leaves it as is.
- Faint magenta-fringe hairlines at some sprite frame edges.
- Tiny text (bonus letters) is unreliable; check letter tiles by eye and add overrides.
- Colour cycling (COLOR06, docs/10) is baked in at its base colour.
- Only World 1 (`WORLDS` in the tool); other worlds need their chunk ids and bob table added.
