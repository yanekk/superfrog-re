# Collision and secret passages

Moved from Claude's memory 2026-10-30; checked against docs/07, docs/LEVEL-INDEX.md, SECRETS.md and
find_secrets.py. Still true; paths updated.

The T7MP BODY cell = tile index (low 10 bits) | flags (high 6 bits, = cell>>10).
The 6-bit flag is per-cell collision:
- solid (frog collides) iff `flag & 0b11000 == 0b11000` (bits 3 AND 4). 61=0b111101
  = solid ground, 63 = solid surface-top (bit1 = standable top / green outline).
- 0 = open air.
- **38 (0b100110) = fake wall / SECRET PASSAGE**: drawn with normal solid terrain
  tiles but passable; the frog walks through what looks like wall into hidden rooms.

Secret-passage detector = games/superfrog/tools/find_secrets.py: ANY cell with flag==38 (do NOT
also require the tile to be "usually solid": vertical drop-shafts use passage-only
tiles that are never solid elsewhere, so that filter hides them; this was a real
miss the user caught in World 1-1's col-94 drop-shaft). Clusters (8-conn, >=3 cells)
= passages. Outputs build/secrets/*.png (magenta) + SECRETS.md + montage. 144
passages across 29 levels. Verified: World 1-1 = horizontal cave tunnel + vertical
drop-shaft; World 4 (d2_c22) = maze of flag-38 corridors linking collectible rooms.

Wall-tiles with flag 0 (carved-open) also exist but are ambiguous (cave mouths/shafts)
so NOT counted. Rarer flags (6,8,9,37,55,...) = hazards/one-way/water, not yet pinned.
See [t7mp-level-format.md](t7mp-level-format.md) and [07-secret-passages.md](07-secret-passages.md).

TILE ADDRESSING (user references spots this way): games/superfrog/tools/export_annotated.py exports each
level to build/levels_annotated/<id>_<chunk>.png with a col/row grid. Level id =
W<world>-<n> in disk order: W1-1..W1-4=d1_c21-24, W2-1..4=d1_c31-34, W3-1..4=d2_c12-15,
W4-1..4=d2_c22-25, W5-1..4=d2_c32-35, WB-1=d2_c42(bonus), W6-1..4=d4_c02-05,
WW-1=d4_c13(worldmap), W6-5=d4_c14, W6-6=d4_c15, W6-7=d4_c17. Tile ref = `<id>:<col>,<row>`
(0-indexed, origin top-left, 16px blocks), e.g. W1-1:94,51 = the World 1-1 drop-shaft.
Full map + per-passage ids in [LEVEL-INDEX.md](LEVEL-INDEX.md). When user says "missing secret at
{id}", parse id->chunk, run find_secrets/inspect that tile's tile+flag.
