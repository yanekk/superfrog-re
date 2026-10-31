#!/usr/bin/env python3
"""Render every level and sprite set of Forest Dumb Forever (docs/05, docs/06).

    render_levels.py [--out build/renders]   everything, one printed line a picture
    render_levels.py --index                 the table docs/LEVEL-INDEX.md carries
    render_levels.py --spot W1-1:10,20       what the three maps hold at that spot

Reads build/unpacked (tools/unpack.py --all) and the main program in build/files, which holds the
palette. Nothing derived from the disks is committed, so the palette is read, never written down here.
"""
import argparse
import re
import struct
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

GAME = Path(__file__).resolve().parent.parent
BUILD = GAME / 'build'
UNPACKED = BUILD / 'unpacked' / 'disk2'
MAIN_PROGRAM = BUILD / 'files' / 'disk1' / 'FOREST_DUMB_FOREVER'
INDEX_DOC = GAME / 'docs' / 'LEVEL-INDEX.md'

CELL = 16
MAP_WIDTH = 96
PLANES = 5
TILE_BYTES = 160
SHAPE_BYTES = 32
OBJECT_MAP_BYTES = 38400
OBJECT_END = 0xff
WORLDS = range(1, 7)

# Hunk 1's level copper list: 32 (register, colour) pairs. The routines at hunk 1 $484 and $49e copy
# the block at $384 or $404 over $304 when a level starts; the two blocks are identical, so all six
# worlds share one palette. Hunk 1's data starts at file offset $5e564.
PALETTE_OFFSET = 0x5e564 + 0x404
COLOUR00 = 0x180

# (sheet name, file, mask file or None, bytes a line of one plane, frame width, frame height).
# The widths come from the code that addresses them: 4 bytes between neighbouring frames (hunk 0
# $3818 table, $63ec table), 2 for ELEMENTY (hunk 0 $66f2), and each file's size fixes the lines. WYBIJACZKA is one 32 x 32 frame:
# its blit (hunk 0 $23dc) steps 42 bytes on a 46-byte screen line, so 4 bytes a line, and the game
# keeps two copies, at hunk 4 $9618 and $9898.
SPRITE_SETS = [
    ('StworkiNew', 'StworkiNew.rawm', 'StworkiNew.mask', 40, 32, 32),
    ('ELEMENTY', '_ELEMENTY.RAWM', '_ELEMENTY.MASK', 40, 16, 16),
    *[(f'Plomien{n}', f'_Plomien{n}', None, 8, 32, 32) for n in (1, 2, 3)],
    *[(f'ZNIKAJKA{n}', f'_ZNIKAJKA{n}', None, 24, 32, 16) for n in (1, 2, 3)],
    *[(f'WYBIJACZKA{s}', f'_WYBIJACZKA{s}', None, 4, 32, 32) for s in ('', '2', '3')],
]

MAGENTA = (255, 0, 255)
GAP = 40


class FormatError(ValueError):
    """A file does not match its documented format; the message names the byte offset."""


def planes_to_pixels(data, line_bytes, name='data', planes=PLANES):
    """Line-interleaved planes (each line holds plane 0, 1, ...) to a lines x pixels array of indices."""
    stride = line_bytes * planes
    if len(data) % stride:
        raise FormatError(f'{name}: truncated at offset {len(data)}: not a whole number of {stride}-byte lines')
    raw = np.frombuffer(data, 'u1').reshape(-1, planes, line_bytes)
    bits = np.unpackbits(raw, axis=2)
    out = np.zeros((raw.shape[0], line_bytes * 8), 'u1')
    for plane in range(planes):
        out |= bits[:, plane, :] << plane
    return out


def parse_tiles(data, name='tiles'):
    """A `_GenN-1` file: tile count x 16 x 16 array. A tile is 16 lines, interleaved like a screen."""
    if not data or len(data) % TILE_BYTES:
        raise FormatError(f'{name}: truncated at offset {len(data)}: not a whole number of {TILE_BYTES}-byte tiles')
    return planes_to_pixels(data, 2, name).reshape(-1, CELL, CELL)


def parse_tile_map(data, tile_count, name='tile map'):
    """A `_MapaN-1` file: rows x 96 array of big-endian tile numbers."""
    if not data or len(data) % (MAP_WIDTH * 2):
        raise FormatError(f'{name}: truncated at offset {len(data)}: not a whole number of {MAP_WIDTH * 2}-byte rows')
    cells = np.frombuffer(data, '>u2').reshape(-1, MAP_WIDTH)
    bad = np.flatnonzero(cells.ravel() >= tile_count)
    if bad.size:
        at = int(bad[0])
        raise FormatError(f'{name}: corrupt at offset {at * 2}: tile {int(cells.ravel()[at])}, only {tile_count} tiles')
    return cells


def parse_shapes(data, name='shapes'):
    """A `_GenN-1.Mapa` file: shape count x 16 x 16 array of 0 or 1, one plane to a shape."""
    if not data or len(data) % SHAPE_BYTES:
        raise FormatError(f'{name}: truncated at offset {len(data)}: not a whole number of {SHAPE_BYTES}-byte shapes')
    return planes_to_pixels(data, 2, name, planes=1).reshape(-1, CELL, CELL)


def parse_collision_map(data, shape_count, name='collision map'):
    """A `_MapaN-1.Mapa` file: rows x 96 array of shape numbers, one byte a cell."""
    if not data or len(data) % MAP_WIDTH:
        raise FormatError(f'{name}: truncated at offset {len(data)}: not a whole number of {MAP_WIDTH}-byte rows')
    cells = np.frombuffer(data, 'u1').reshape(-1, MAP_WIDTH)
    bad = np.flatnonzero(cells.ravel() >= shape_count)
    if bad.size:
        at = int(bad[0])
        raise FormatError(f'{name}: corrupt at offset {at}: shape {int(cells.ravel()[at])}, only {shape_count} shapes')
    return cells


def parse_object_map(data, name='object map'):
    """A `_MapaBobowN` file: 400 rows x 96 of object bytes. Every file fills the game's whole buffer."""
    if len(data) != OBJECT_MAP_BYTES:
        raise FormatError(f'{name}: truncated at offset {len(data)}: expected {OBJECT_MAP_BYTES} bytes')
    return np.frombuffer(data, 'u1').reshape(-1, MAP_WIDTH)


def parse_palette(program, offset=PALETTE_OFFSET, name='main program'):
    """The 32 colours as RGB, from the copper list's register/value pairs, checking each register."""
    if len(program) < offset + 128:
        raise FormatError(f'{name}: truncated at offset {len(program)}: the palette needs {offset + 128} bytes')
    words = struct.unpack_from('>64H', program, offset)
    colours = []
    for i in range(32):
        register, colour = words[2 * i], words[2 * i + 1]
        if register != COLOUR00 + 2 * i or colour > 0xfff:
            raise FormatError(f'{name}: corrupt at offset {offset + 4 * i}: {register:#x}={colour:#x} is not colour {i}')
        colours.append(((colour >> 8) * 17, ((colour >> 4) & 15) * 17, (colour & 15) * 17))
    return np.array(colours, 'u1')


def read(name):
    path = UNPACKED / name
    if not path.exists():
        raise SystemExit(f'{path} is missing: run `cargo run -p fd-tools -- extract` in engine/, '
                         'then tools/unpack.py --all')
    return path.read_bytes()


def palette():
    if not MAIN_PROGRAM.exists():
        raise SystemExit(f'{MAIN_PROGRAM} is missing: run `cargo run -p fd-tools -- extract` in engine/')
    return parse_palette(MAIN_PROGRAM.read_bytes())


def level_files(world):
    return {'tile map': f'_Mapa{world}-1', 'tiles': f'_Gen{world}-1', 'collision': f'_Mapa{world}-1.Mapa',
            'shapes': f'_Gen{world}-1.Mapa', 'objects': f'_MapaBobow{world}'}


def load_level(world):
    files = level_files(world)
    tiles = parse_tiles(read(files['tiles']), files['tiles'])
    shapes = parse_shapes(read(files['shapes']), files['shapes'])
    return {
        'tiles': tiles,
        'shapes': shapes,
        'map': parse_tile_map(read(files['tile map']), len(tiles), files['tile map']),
        'collision': parse_collision_map(read(files['collision']), len(shapes), files['collision']),
        'objects': parse_object_map(read(files['objects']), files['objects']),
    }


def draw_map(cells, images):
    """rows x cols of image numbers to one big array of pixels."""
    rows, cols = cells.shape
    return images[cells].transpose(0, 2, 1, 3).reshape(rows * CELL, cols * CELL)


def object_class(value):
    """The groups the game's own row scan tells apart (hunk 0 $9a8). What each value does is not known yet."""
    if value == OBJECT_END:
        return 'end'
    if value < 7:
        return 'creature'
    if value < 0x20:
        return 'ignored'
    return 'element' if value < 0x40 else 'high'


CLASS_COLOURS = {'creature': (255, 60, 60), 'ignored': (160, 160, 160), 'element': (60, 255, 60),
                 'high': (80, 180, 255), 'end': (255, 255, 255)}


def level_picture(level, colours):
    return Image.fromarray(colours[draw_map(level['map'], level['tiles'])])


def collision_picture(level):
    return Image.fromarray(draw_map(level['collision'], level['shapes']) * 255)


def objects_picture(level, colours):
    """The level with every object cell boxed, coloured by the class `object_class` gives."""
    picture = level_picture(level, colours)
    draw = ImageDraw.Draw(picture)
    for row, col in zip(*np.nonzero(level['objects'])):
        colour = CLASS_COLOURS[object_class(int(level['objects'][row, col]))]
        x, y = int(col) * CELL, int(row) * CELL
        draw.rectangle((x, y, x + CELL - 1, y + CELL - 1), outline=colour)
    return picture


def sheet(frames, colours, per_row, masks=None):
    """Frames laid out with a gap. Where a mask is given, its clear pixels draw magenta."""
    count, height, width = frames.shape
    rows = (count + per_row - 1) // per_row
    out = np.full((rows * (height + 1), per_row * (width + 1), 3), GAP, 'u1')
    for i in range(count):
        row, col = divmod(i, per_row)
        picture = colours[frames[i]]
        if masks is not None:
            picture = picture.copy()
            picture[masks[i] == 0] = MAGENTA
        out[row * (height + 1):row * (height + 1) + height, col * (width + 1):col * (width + 1) + width] = picture
    return Image.fromarray(out)


def cut_frames(pixels, width, height):
    """A bitmap of side-by-side frames, row after row, to count x height x width."""
    lines, across = pixels.shape
    return np.array([pixels[y:y + height, x:x + width]
                     for y in range(0, lines, height) for x in range(0, across, width)])


def sprite_sheet(entry, colours):
    _, raw_name, mask_name, line_bytes, width, height = entry
    frames = cut_frames(planes_to_pixels(read(raw_name), line_bytes, raw_name), width, height)
    masks = None
    if mask_name:
        # The mask repeats its one plane five times, because the blitter's mask channel steps with
        # the interleaved source.
        masks = cut_frames(planes_to_pixels(read(mask_name), line_bytes, mask_name), width, height)
    return sheet(frames, colours, line_bytes // (width // 8), masks)


def level_id(world):
    return f'W{world}-1'


def world_of(identifier):
    match = re.fullmatch(r'W([1-6])-1', identifier)
    if not match:
        raise ValueError(f'no level {identifier!r}: the ids are W1-1 to W6-1')
    return int(match.group(1))


def resolve_spot(spot):
    """`W1-1:10,20` to what is there: the tile, shape and object numbers, -1 past a map's last row."""
    match = re.fullmatch(r'([^:]+):(\d+),(\d+)', spot)
    if not match:
        raise ValueError(f'{spot!r} is not <level-id>:<col>,<row>')
    world, col, row = world_of(match.group(1)), int(match.group(2)), int(match.group(3))
    level = load_level(world)
    if col >= MAP_WIDTH or row >= level['map'].shape[0]:
        raise ValueError(f'{spot}: outside the level, {MAP_WIDTH} columns by {level["map"].shape[0]} rows')
    in_map = lambda cells: int(cells[row, col]) if row < cells.shape[0] else -1
    return {'level': match.group(1), 'col': col, 'row': row, 'x': col * CELL, 'y': row * CELL,
            'tile': in_map(level['map']), 'shape': in_map(level['collision']), 'object': in_map(level['objects'])}


def index_table():
    rows = ['| level id | tile map | tiles | collision | shapes | objects | tiles count | size (cols x rows) '
            '| collision rows | render |',
            '|---|---|---|---|---|---|---:|---|---:|---|']
    for world in WORLDS:
        level, files = load_level(world), level_files(world)
        rows.append(f'| {level_id(world)} | `{files["tile map"]}` | `{files["tiles"]}` | `{files["collision"]}` '
                    f'| `{files["shapes"]}` | `{files["objects"]}` | {len(level["tiles"])} '
                    f'| {MAP_WIDTH}x{level["map"].shape[0]} | {level["collision"].shape[0]} '
                    f'| `{level_id(world)}.png` |')
    return '\n'.join(rows) + '\n'


def render_all(out):
    """Write every picture under `out`; return their paths in order."""
    out = Path(out)
    out.mkdir(parents=True, exist_ok=True)
    colours = palette()
    pictures = []

    def save(name, picture):
        path = out / name
        picture.save(path)
        pictures.append(path)

    for world in WORLDS:
        level = load_level(world)
        identifier = level_id(world)
        save(f'{identifier}.png', level_picture(level, colours))
        save(f'{identifier}-collision.png', collision_picture(level))
        save(f'{identifier}-objects.png', objects_picture(level, colours))
        save(f'{identifier}-tiles.png', sheet(level['tiles'], colours, 32))
        save(f'{identifier}-shapes.png', sheet(level['shapes'], np.array([(0, 0, 0), (255, 255, 255)], 'u1'), 16))
    for entry in SPRITE_SETS:
        save(f'sprites-{entry[0]}.png', sprite_sheet(entry, colours))
    return pictures


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--out', default=BUILD / 'renders', type=Path)
    parser.add_argument('--index', action='store_true')
    parser.add_argument('--spot')
    args = parser.parse_args()
    try:
        if args.index:
            sys.stdout.write(index_table())
        elif args.spot:
            print(resolve_spot(args.spot))
        else:
            for path in render_all(args.out):
                print(path)
    except ValueError as error:
        raise SystemExit(str(error))


if __name__ == '__main__':
    main()
