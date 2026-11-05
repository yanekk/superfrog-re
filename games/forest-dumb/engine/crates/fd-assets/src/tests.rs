//! The readers on made-up files of the right shape: no disk is needed. The real files are checked
//! against the original in `fd-conform/tests/level_files.rs`.

use crate::*;

/// A world-1 level whose every file is well formed: `tiles` tile pictures, `shapes` shapes, every
/// cell tile `(col + row) % tiles` and shape `col % shapes`, object 7 at (3, 4).
fn files(tiles: usize, shapes: usize) -> impl Fn(&str) -> Result<Vec<u8>, AssetError> {
    move |name: &str| {
        Ok(match name {
            "_Mapa1-1" => (0..280usize)
                .flat_map(|c| (0..96usize).map(move |r| ((c + r) % tiles) as u16))
                .flat_map(u16::to_be_bytes)
                .collect(),
            "_Gen1-1" => (0..tiles * TILE_BYTES)
                .map(|i| (i / TILE_BYTES) as u8)
                .collect(),
            "_Mapa1-1.Mapa" => (0..240usize)
                .flat_map(|c| (0..96).map(move |_| (c % shapes) as u8))
                .collect(),
            "_Gen1-1.Mapa" => (0..shapes * SHAPE_BYTES).map(|i| i as u8).collect(),
            "_MapaBobow1" => {
                let mut v = vec![0; 400 * 96];
                v[3 * 96 + 4] = 7;
                v
            }
            "StworkiNew.rawm" | "StworkiNew.mask" => vec![0; 44800],
            "_ELEMENTY.RAWM" | "_ELEMENTY.MASK" => vec![0; 6400],
            "_Plomien1" => vec![0; 1280],
            "_ZNIKAJKA1" => vec![0; 1920],
            "_WYBIJACZKA" => vec![0; 640],
            _ => return Err(AssetError::missing(name, "not in the test's files")),
        })
    }
}

/// `files` with `name` replaced by `with`.
fn with(name: &'static str, with: Vec<u8>) -> impl Fn(&str) -> Result<Vec<u8>, AssetError> {
    let base = files(10, 4);
    move |n: &str| if n == name { Ok(with.clone()) } else { base(n) }
}

#[test]
fn a_well_formed_level_reads_column_by_column() {
    let level = read_level(1, &files(10, 4)).unwrap();
    assert_eq!((level.tiles.columns, level.collision.columns), (280, 240));
    assert_eq!(level.objects.columns, 400);
    assert_eq!((level.gens.len(), level.shapes.len()), (10, 4));
    // (col + row) % 10: column order puts row 1 of column 0 next to row 0, not column 1.
    assert_eq!(level.tile(0, 1), 1);
    assert_eq!(level.tile(5, 2), 7);
    assert_eq!(level.tile(279, 95), (279 + 95) % 10);
    assert_eq!(level.shape(6, 50), 2);
    assert_eq!(level.object(3, 4), 7);
    assert_eq!(level.object(4, 3), 0);
    // Shape 1's first word is bytes 32 and 33.
    assert_eq!(level.shapes[1][0], 0x2021);
    assert_eq!(level.gens[3][0], 3);
}

#[test]
fn a_tile_pixel_takes_plane_0_as_the_low_bit_and_bit_7_as_the_left() {
    let mut level = read_level(1, &files(10, 4)).unwrap();
    let mut t = [0u8; TILE_BYTES];
    // Line 2: plane 0 byte 0 = $80 (pixel 0), plane 4 byte 1 = $01 (pixel 15).
    t[2 * 10] = 0x80;
    t[2 * 10 + 4 * 2 + 1] = 0x01;
    level.gens[0] = t;
    assert_eq!(level.tile_pixel(0, 0, 2), 1);
    assert_eq!(level.tile_pixel(0, 15, 2), 16);
    assert_eq!(level.tile_pixel(0, 1, 2), 0);
    assert_eq!(level.tile_pixel(0, 0, 1), 0);
}

#[test]
fn a_short_map_is_an_error_naming_its_size() {
    let e = read_level(1, &with("_Mapa1-1", vec![0; 53760 - 192])).unwrap_err();
    assert_eq!(e.file, "_Mapa1-1");
    assert_eq!(
        e.why,
        Why::Size {
            expected: 53760,
            got: 53760 - 192
        }
    );
}

#[test]
fn a_long_collision_map_and_an_object_map_of_another_size_are_errors() {
    let e = read_level(1, &with("_Mapa1-1.Mapa", vec![0; 23041])).unwrap_err();
    assert_eq!(
        e.why,
        Why::Size {
            expected: 23040,
            got: 23041
        }
    );
    let e = read_level(1, &with("_MapaBobow1", vec![0; 280 * 96])).unwrap_err();
    assert_eq!(e.file, "_MapaBobow1");
}

#[test]
fn tile_pictures_and_shapes_must_be_whole_and_present() {
    for (file, bytes, unit) in [
        ("_Gen1-1", vec![0; 159], 160),
        ("_Gen1-1", vec![], 160),
        ("_Gen1-1.Mapa", vec![0; 33], 32),
    ] {
        let got = bytes.len();
        let e = read_level(1, &with(file, bytes)).unwrap_err();
        assert_eq!(
            (e.file.as_str(), e.why),
            (file, Why::NotWhole { unit, got })
        );
    }
}

#[test]
fn a_tile_or_shape_number_past_the_set_is_named_by_its_offset() {
    // 3 tiles but cells up to 9.
    let e = read_level(1, &with("_Gen1-1", vec![0; 3 * 160])).unwrap_err();
    assert_eq!(e.file, "_Mapa1-1");
    assert!(matches!(e.why, Why::Malformed { at: 6, .. }), "{e}");
    let e = read_level(1, &with("_Gen1-1.Mapa", vec![0; 2 * 32])).unwrap_err();
    assert_eq!(e.file, "_Mapa1-1.Mapa");
    // Column 2 holds shape 2 first.
    assert!(matches!(e.why, Why::Malformed { at: 192, .. }), "{e}");
}

#[test]
fn a_missing_file_is_the_closures_error() {
    let e = read_level(2, &files(10, 4)).unwrap_err();
    assert_eq!(e.file, "_Gen2-1");
    assert!(matches!(e.why, Why::Missing(_)));
    assert!(read_level(0, &files(10, 4)).is_err());
    assert!(read_level(7, &files(10, 4)).is_err());
}

#[test]
fn world_6_is_320_columns_wide() {
    let names = level_file_names(6);
    assert_eq!(names[0], "_Mapa6-1");
    let f = move |n: &str| match n {
        "_Mapa6-1" => Ok(vec![0; 320 * 96 * 2]),
        "_Mapa6-1.Mapa" => Ok(vec![0; 320 * 96]),
        "_Gen6-1" => Ok(vec![0; 160]),
        "_Gen6-1.Mapa" => Ok(vec![0; 32]),
        "_MapaBobow6" => Ok(vec![0; 400 * 96]),
        _ => Err(AssetError::missing(n, "no")),
    };
    let level = read_level(6, &f).unwrap();
    assert_eq!((level.tiles.columns, level.collision.columns), (320, 320));
}

#[test]
#[should_panic(expected = "outside a map")]
fn a_cell_past_the_map_panics_like_a_slice() {
    read_level(1, &files(10, 4)).unwrap().shape(240, 0);
}

#[test]
fn sprites_read_with_their_masks_and_check_every_size() {
    let s = read_sprites(1, &files(10, 4)).unwrap();
    assert_eq!(s.creatures.lines(), 224);
    assert!(s.creatures.mask.is_some() && s.elements.mask.is_some());
    assert!(s.flame.mask.is_none());
    assert_eq!(s.knocker.file, "_WYBIJACZKA");
    for (file, len) in [
        ("StworkiNew.mask", 44799),
        ("_ELEMENTY.RAWM", 6401),
        ("_Plomien1", 0),
        ("_ZNIKAJKA1", 1280),
        ("_WYBIJACZKA", 1280),
    ] {
        let e = read_sprites(1, &with(file, vec![0; len])).unwrap_err();
        assert_eq!(e.file, file);
        assert!(matches!(e.why, Why::Size { got, .. } if got == len), "{e}");
    }
}

#[test]
fn the_world_sets_follow_chapter_06() {
    assert_eq!(sprite_file_names(2)[6], "_WYBIJACZKA");
    assert_eq!(sprite_file_names(3)[4], "_Plomien2");
    assert_eq!(sprite_file_names(4)[6], "_WYBIJACZKA2");
    assert_eq!(sprite_file_names(6)[5], "_ZNIKAJKA3");
}

#[test]
fn a_sheet_pixel_reads_across_frames() {
    let mut s = read_sprites(1, &files(10, 4)).unwrap().knocker;
    // Line 1, plane 1, byte 3: the last 8 pixels of the 32.
    s.picture[(5 + 1) * 4 + 3] = 0x01;
    assert_eq!(s.pixel(31, 1), 2);
    assert_eq!(s.pixel(30, 1), 0);
}
