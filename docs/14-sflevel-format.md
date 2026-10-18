# 14 — `.sflevel`: one level in a file, the map editor's target

The engine plays a level from a `.sflevel` file in place of the disk level it names (DESIGN
§2.10). The map editor reads and writes this format; the engine writes it with **File → Export
Level ▸ W1-1…** and plays it with **File → Play Level File…**. An unchanged export plays frame for
frame as the disk level does (`engine/crates/sf-assets/tests/sflevel.rs`), because the file carries
the level's own `T7MP` and the engine loads it through the same loader as a disk level.

An exported file holds the disk's level data, so it is the owner's and is never committed:
`*.sflevel` is git-ignored, and `tools/check_no_disk_data.py` decodes a `.sflevel`'s sections
before it scans them.

Code: `engine/crates/sf-assets/src/sflevel.rs` (`sflevel_encode`, `sflevel_decode`),
`Game::new_with_override` in `sf-core`, the menu in `sf-app/src/levelfile.rs`.

## Version 1 (fixed)

UTF-8 JSON, one object:

```json
{
  "format": "sflevel",
  "version": 1,
  "replaces": "W1-1",
  "chunk": {"disk": 1, "index": 21, "sector": 1012},
  "sections": {
    "t7mp": "VDdNUAAAAAQAANFUWEJMSw…"
  }
}
```

(The `sector` and the cut-off base64 are an example's.)

| Field | Required | Meaning |
|---|---|---|
| `format` | yes | the string `"sflevel"` |
| `version` | yes | the integer `1`. Any other integer is refused with its number; a later version is a new document section here |
| `replaces` | yes | the level played in place of: a level id as `docs/LEVEL-INDEX.md` writes it. `W1-1`…`W6-4`, `W6-1alt`…`W6-4alt` (W6's second set of maps, what a one-drive A500 loads), `WB-1` (bonus level), `WW-1` (final level) |
| `chunk` | no | where the level came from: `disk` 1–4, `index` its 1-based chunk number on that disk (`d1_c21`), `sector` the sector it starts at. Informational: the engine does not read it. Missing, it reads as `disk 1, index 0, sector 0` |
| `sections` | yes, with `t7mp` | the level's files in their original formats, each standard base64 (RFC 4648, with `=` padding, no line breaks), by name |

Sections:

- `t7mp` — the level's `T7MP` file exactly as `load_file` would read it from the disk (docs/04):
  every tag in file order, `BODY` the `300 × 84` grid of `tile (low 10 bits) | marker (high 6
  bits)` cells. The map **must be 300 × 84**: the game unpacks that many cells whatever `XBLK` and
  `YBLK` say (`map_unpack`, h0+$1647a). The size tag (`T7MP`) gives the whole file's size. Object
  placement is the marker field of `BODY` (docs/09); there is no other per-level file.
- Any other name is kept and written back unchanged (`LevelData::extra`) and not read by the game.
  None exist today.

Key order and whitespace are free on reading; the engine writes the keys in the order above, two
spaces of indent, a newline at the end. Unknown top-level keys are ignored.

## Refusals

A file that cannot be played is refused with its reason, in this order, and the game carries on.
When the file can be read as far as a `replaces` naming a level on the disks, the message offers
**Play W1-1 from the Disks**; the disk level starts only if the player chooses it (DESIGN §2.11).

| `SflevelError` | When | The message says |
|---|---|---|
| `NotJson` | not JSON (or not UTF-8) | it is not a level file (not JSON) |
| `WrongFormat` | not an object; `format` not `"sflevel"`; `version` missing or not a non-negative integer; `replaces` missing or not a string; `chunk` or `sections` of the wrong shape; a section not a string; a section other than `t7mp` not base64 | it is not a Superfrog level file |
| `Version(n)` | `version` is an integer other than 1 | it is a version n level file; this Superfrog reads version 1 |
| `UnknownLevel(id)` | `replaces` is no level id, or one the disks do not hold | it replaces "id", which is not a level |
| `MissingSection("t7mp")` | no `t7mp` section | its "t7mp" section is missing |
| `BadT7mp(why)` | `t7mp` not base64, not a well-formed T7MP (docs/04), or not 300 × 84 | its map is damaged: why |

## For the editor

- Start from an export: it carries every tag the level has, which the game reads (`PALA` for the
  palette, `BODY` for the map and its objects).
- Edit cells in `BODY`; keep the other tags as they are unless their meaning is known (docs/04,
  docs/07 for the collision flags, docs/09 for the object markers).
- Write `T7MP`'s size, `XBLK`, `YBLK` and `BODY`'s length to match (`t7mp_encode` does).
