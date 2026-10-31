# Level index and tile addressing

Name any spot as **`<level-id>:<col>,<row>`**: 0-indexed, top-left origin, 16-pixel cells, so
`W1-1:10,20` is the cell at pixel (160, 320) of `W1-1.png`. One level a world, each named after its
files' `-1`. `python games/forest-dumb/tools/render_levels.py --spot W1-1:10,20` prints what the three
maps hold there (-1 past a map's last row). Pictures are in `games/forest-dumb/build/renders/`.

| level id | tile map | tiles | collision | shapes | objects | tiles count | size (cols x rows) | collision rows | render |
|---|---|---|---|---|---|---:|---|---:|---|
| W1-1 | `_Mapa1-1` | `_Gen1-1` | `_Mapa1-1.Mapa` | `_Gen1-1.Mapa` | `_MapaBobow1` | 328 | 96x280 | 240 | `W1-1.png` |
| W2-1 | `_Mapa2-1` | `_Gen2-1` | `_Mapa2-1.Mapa` | `_Gen2-1.Mapa` | `_MapaBobow2` | 335 | 96x280 | 240 | `W2-1.png` |
| W3-1 | `_Mapa3-1` | `_Gen3-1` | `_Mapa3-1.Mapa` | `_Gen3-1.Mapa` | `_MapaBobow3` | 306 | 96x280 | 240 | `W3-1.png` |
| W4-1 | `_Mapa4-1` | `_Gen4-1` | `_Mapa4-1.Mapa` | `_Gen4-1.Mapa` | `_MapaBobow4` | 270 | 96x280 | 240 | `W4-1.png` |
| W5-1 | `_Mapa5-1` | `_Gen5-1` | `_Mapa5-1.Mapa` | `_Gen5-1.Mapa` | `_MapaBobow5` | 224 | 96x280 | 240 | `W5-1.png` |
| W6-1 | `_Mapa6-1` | `_Gen6-1` | `_Mapa6-1.Mapa` | `_Gen6-1.Mapa` | `_MapaBobow6` | 227 | 96x320 | 320 | `W6-1.png` |

Regenerate the table with `render_levels.py --index`; a test checks it. Formats:
[05-file-formats.md](05-file-formats.md), [06-graphics-and-rendering.md](06-graphics-and-rendering.md).
