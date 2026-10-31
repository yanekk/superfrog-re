"""The formats on synthetic bytes, then every picture rendered from the real files. Non-blank would also
pass a wrong format, so the real-file tests check what a wrong offset or layout would break."""
import sys
import tempfile
import unittest
from pathlib import Path

import numpy as np
from PIL import Image

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import render_levels as R
import unpack
from test_unpack import extracted


class Formats(unittest.TestCase):
    def test_a_tile_is_interleaved_line_by_line(self):
        data = bytearray(R.TILE_BYTES)
        data[0:2] = b'\x80\x00'    # line 0, plane 0, pixel 0
        data[4:6] = b'\x80\x00'    # line 0, plane 2, pixel 0
        data[10:12] = b'\x00\x01'  # line 1, plane 0, pixel 15
        tile = R.parse_tiles(bytes(data))[0]
        self.assertEqual(tile[0, 0], 0b00101)
        self.assertEqual(tile[1, 15], 1)
        self.assertEqual(int(tile.sum()), 6)

    def test_maps_are_big_endian_and_stored_a_column_at_a_time(self):
        data = bytearray(R.LEVEL_HEIGHT * 2 * 2)
        data[2 * (R.LEVEL_HEIGHT + 3):2 * (R.LEVEL_HEIGHT + 3) + 2] = b'\x01\x02'  # column 1, row 3
        cells = R.parse_tile_map(bytes(data), 0x103)
        self.assertEqual(cells.shape, (96, 2))
        self.assertEqual(cells[3, 1], 0x102)
        collision = bytearray(R.LEVEL_HEIGHT * 2)
        collision[R.LEVEL_HEIGHT + 3] = 7
        self.assertEqual(R.parse_collision_map(bytes(collision), 8)[3, 1], 7)
        objects = bytearray(R.OBJECT_MAP_BYTES)
        objects[R.LEVEL_HEIGHT + 3] = 9
        self.assertEqual(R.parse_object_map(bytes(objects))[3, 1], 9)

    def test_a_map_short_by_a_whole_column_or_too_long_is_reported(self):
        with self.assertRaisesRegex(R.FormatError, 'truncated at offset 192: expected 384'):
            R.parse_tile_map(bytes(192), 1, columns=2)
        with self.assertRaisesRegex(R.FormatError, 'truncated at offset 96: expected 192'):
            R.parse_collision_map(bytes(96), 1, columns=2)
        with self.assertRaisesRegex(R.FormatError, f'corrupt at offset {R.OBJECT_MAP_BYTES}'):
            R.parse_object_map(bytes(R.OBJECT_MAP_BYTES + 1))

    def test_truncation_names_the_offset(self):
        cases = [(R.parse_tiles, (b'\0' * 161,), '161'), (R.parse_tile_map, (b'\0' * 193, 1), '193'),
                 (R.parse_shapes, (b'\0' * 33,), '33'), (R.parse_collision_map, (b'\0' * 97, 1), '97'),
                 (R.parse_object_map, (b'\0' * 100,), '100'), (R.parse_palette, (b'\0' * 50, 0), '50')]
        for parse, args, offset in cases:
            with self.assertRaisesRegex(R.FormatError, f'truncated at offset {offset}'):
                parse(*args)

    def test_corruption_names_the_offset(self):
        bad_map = bytearray(R.LEVEL_HEIGHT * 2)
        bad_map[6:8] = b'\x00\x09'
        with self.assertRaisesRegex(R.FormatError, 'corrupt at offset 6: tile 9'):
            R.parse_tile_map(bytes(bad_map), 9)
        bad_collision = bytearray(R.LEVEL_HEIGHT)
        bad_collision[5] = 4
        with self.assertRaisesRegex(R.FormatError, 'corrupt at offset 5: shape 4'):
            R.parse_collision_map(bytes(bad_collision), 4)
        with self.assertRaisesRegex(R.FormatError, 'corrupt at offset 0'):
            R.parse_palette(bytes(128), 0)

    def test_the_palette_reads_register_value_pairs(self):
        data = b''.join((0x180 + 2 * i).to_bytes(2, 'big') + (0x123 if i == 1 else 0).to_bytes(2, 'big')
                        for i in range(32))
        self.assertEqual(R.parse_palette(data, 0)[1].tolist(), [0x11, 0x22, 0x33])

    def test_object_classes_follow_the_games_scan(self):
        for value, name in ((1, 'creature'), (6, 'creature'), (7, 'ignored'), (0x1f, 'ignored'),
                            (0x20, 'element'), (0x3f, 'element'), (0x40, 'high'), (0xff, 'end')):
            self.assertEqual(R.object_class(value), name)

    def test_a_clear_mask_pixel_draws_magenta_and_a_set_one_the_picture(self):
        frames = np.zeros((1, 1, 2), 'u1')
        masks = np.array([[[0, 1]]], 'u1')
        colours = np.array([(1, 2, 3)], 'u1')
        drawn = np.asarray(R.sheet(frames, colours, 1, masks))
        self.assertEqual(drawn[0, 0].tolist(), list(R.MAGENTA))
        self.assertEqual(drawn[0, 1].tolist(), [1, 2, 3])

    def test_a_bad_spot_is_refused(self):
        for spot in ('W1-1', 'W7-1:0,0', 'W1-1:a,b'):
            with self.assertRaises(ValueError):
                R.resolve_spot(spot)


class RealFiles(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        extracted()
        if not (R.UNPACKED / '_Gen1-1').exists():
            unpack.unpack_all()
        cls.scratch = tempfile.TemporaryDirectory()
        cls.pictures = R.render_all(cls.scratch.name)
        cls.levels = {world: R.load_level(world) for world in R.WORLDS}

    @classmethod
    def tearDownClass(cls):
        cls.scratch.cleanup()

    def test_every_picture_exists_and_is_not_blank(self):
        self.assertEqual(len(self.pictures), 6 * 5 + len(R.SPRITE_SETS))
        for path in self.pictures:
            variance = np.asarray(Image.open(path).convert('L'), float).var()
            self.assertGreater(variance, 100, path.name)

    def test_the_palette_is_the_level_block_both_start_routines_copy(self):
        # Every run of copper colour pairs parses, so the register check alone would take a wrong block.
        program = R.MAIN_PROGRAM.read_bytes()
        chosen = R.palette()
        self.assertEqual(R.parse_palette(program, 0x5e564 + 0x384).tolist(), chosen.tolist())
        self.assertEqual(R.parse_palette(program, 0x5e564 + 0x404).tolist(), chosen.tolist())
        self.assertGreater(len(np.unique(chosen, axis=0)), 16)

    def test_the_tile_counts_are_what_the_maps_use(self):
        # Each map uses every tile of its set exactly: a wrong tile size or map offset breaks this.
        for world, level in self.levels.items():
            self.assertEqual(len(np.unique(level['map'])), len(level['tiles']), world)

    def test_the_tile_map_and_the_collision_map_agree(self):
        # The tiles filling open cells never occur in solid ones. A wrong width or row offset in
        # either map mixes them; this is what ties the two files together.
        for world, level in self.levels.items():
            tiles = level['map'][:, :level['collision'].shape[1]]
            flat = level['shapes'].reshape(len(level['shapes']), -1)
            solid, empty = flat.all(axis=1)[level['collision']], ~flat.any(axis=1)[level['collision']]
            numbers, counts = np.unique(tiles[empty], return_counts=True)
            open_tiles = numbers[counts > 200]
            self.assertGreater(np.isin(tiles[empty], open_tiles).mean(), 0.7, world)
            self.assertEqual(np.isin(tiles[solid], open_tiles).sum(), 0, world)

    def test_neighbouring_tiles_join_up_only_the_right_way_round(self):
        # Drawn with rows and columns swapped, a level still looks like terrain, and passes every
        # other test here. Its pictures no longer meet at the tile edges, which this measures.
        colours = R.palette()

        def seams(cells, tiles):
            rgb = colours[R.draw_map(cells, tiles)].astype(int)
            return (np.abs(rgb[:, 15::16][:, :-1] - rgb[:, 16::16]).mean()
                    + np.abs(rgb[15::16][:-1] - rgb[16::16]).mean())

        for world, level in self.levels.items():
            self.assertLess(seams(level['map'], level['tiles']), seams(level['map'].T, level['tiles']), world)

    def test_the_object_map_stays_inside_the_collision_maps_columns(self):
        for world, level in self.levels.items():
            used = np.flatnonzero(level['objects'].any(axis=0))
            self.assertLess(used.max(), level['collision'].shape[1], world)

    def test_every_sprite_set_cuts_into_the_frames_its_code_addresses(self):
        # The counts come from the game's addressing (docs/06). The worker-judged pass over the
        # pictures is what shows these layouts draw sprites and not noise; a metric could not tell.
        expected = {'StworkiNew': 70, 'ELEMENTY': 40, 'Plomien': 2, 'ZNIKAJKA': 6, 'WYBIJACZKA': 1}
        for name, raw, _, line_bytes, width, height in R.SPRITE_SETS:
            frames = R.cut_frames(R.planes_to_pixels(R.read(raw), line_bytes, raw), width, height)
            self.assertEqual(frames.shape[1:], (height, width), name)
            self.assertEqual(len(frames), expected[name.rstrip('123')], name)

    def test_wybijaczka_is_one_tall_frame(self):
        # Rendered as two 32 x 16 frames at first, which cut the pillar in half side by side.
        for name, raw, _, line_bytes, width, height in R.SPRITE_SETS:
            if name.startswith('WYBIJACZKA'):
                self.assertEqual(R.planes_to_pixels(R.read(raw), line_bytes, raw).shape, (height, width), name)
                self.assertEqual((width, height), (32, 32))

    def test_a_mask_repeats_one_plane_and_covers_its_picture(self):
        for name, raw, mask, line_bytes, _, _ in R.SPRITE_SETS:
            if mask:
                planes = np.frombuffer(R.read(mask), 'u1').reshape(-1, 5, line_bytes)
                self.assertTrue((planes == planes[:, :1]).all(), name)
                drawn = np.frombuffer(R.read(raw), 'u1').reshape(-1, 5, line_bytes)
                uncovered = np.bitwise_and(np.bitwise_or.reduce(drawn, axis=1), ~planes[:, 0])
                # 13 bytes of the game's own _ELEMENTY art draw outside its mask.
                self.assertEqual(int(np.count_nonzero(uncovered)), {'StworkiNew': 0, 'ELEMENTY': 13}[name], name)

    def test_the_index_doc_carries_the_current_table(self):
        self.assertIn(R.index_table(), R.INDEX_DOC.read_text())

    def test_a_spot_resolves_through_the_index(self):
        doc = R.INDEX_DOC.read_text()
        for world in R.WORLDS:
            identifier = R.level_id(world)
            self.assertIn(f'| {identifier} |', doc)
            spot = R.resolve_spot(f'{identifier}:10,20')
            level = self.levels[world]
            self.assertEqual((spot['x'], spot['y']), (160, 320))
            self.assertEqual(spot['tile'], level['map'][20, 10])
            self.assertEqual(spot['shape'], level['collision'][20, 10])
            self.assertEqual(spot['object'], level['objects'][20, 10])
        self.assertEqual(R.resolve_spot('W1-1:270,0')['shape'], -1)
        for outside in ('W1-1:280,0', 'W1-1:0,96'):
            with self.assertRaises(ValueError):
                R.resolve_spot(outside)


if __name__ == '__main__':
    unittest.main()
