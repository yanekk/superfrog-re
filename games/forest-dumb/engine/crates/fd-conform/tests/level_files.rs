//! `fd-assets`' readers against the original (plans/forest-dumb-level-1 T05): every level 1 file
//! read from the disks equals the bytes the game loaded at its target, and the map's cells are the
//! tiles the original draws there. Needs the disks and the Kickstart: absent, it fails naming where
//! it looked. The level sync point comes from the cache (`fd_conform::sync`).

use amiga_conform::script;
use amiga_oracle::{Amiga, Rgb12Frame};
use fd_assets::{AssetError, Level, level_file_names, read_level, read_sprites, sprite_file_names};
use fd_conform::{HUNK_BASES, sync};

/// A file from disk 2, unpacked, the way the engine's shell will hand it over.
fn files(name: &str) -> Result<Vec<u8>, AssetError> {
    let dir = fd_tools::disk_dir().map_err(|e| AssetError::missing(name, e))?;
    fd_tools::game_file(&dir, &format!("disk2/{name}")).map_err(|e| AssetError::missing(name, e))
}

fn word(am: &Amiga, addr: u32) -> u16 {
    let b = am.peek(addr, 2);
    u16::from_be_bytes([b[0], b[1]])
}

/// Where world 1's load list puts each file (`docs/07-executable-map.md`, "Memory layout"), in the
/// order of `level_file_names` then `sprite_file_names`: (hunk, offset).
const TARGETS: [(usize, u32); 12] = [
    (0, 0x16318),
    (3, 0x13540),
    (0, 0x28f18),
    (0, 0xbda0),
    (0, 0xcb38),
    (3, 0x20c18),
    (3, 0x2bb18),
    (4, 0x0),
    (4, 0x1900),
    (4, 0xa298),
    (4, 0x9b18),
    (4, 0x9618),
];
/// The game's second copy of `_WYBIJACZKA`, which `h0+$23dc` picks by a flag. World 1 never loads it.
const KNOCKER_COPY: u32 = 0x9898;

#[test]
fn every_level_1_file_equals_the_bytes_at_its_target_at_the_sync_point() {
    let am = sync::reach_level_1(0).unwrap_or_else(|e| panic!("{e}"));
    let mut names = level_file_names(1).to_vec();
    names.extend(sprite_file_names(1));
    let mut checked = Vec::new();
    for (name, (hunk, offset)) in names.iter().zip(TARGETS) {
        let bytes = files(name).unwrap_or_else(|e| panic!("{e}"));
        let addr = HUNK_BASES[hunk] + offset;
        let held = am.peek(addr, bytes.len());
        let first = (0..bytes.len()).find(|&i| bytes[i] != held[i]);
        assert_eq!(
            first,
            None,
            "{name} ({} bytes) differs from h{hunk}+${offset:x} at its byte {first:?}",
            bytes.len()
        );
        checked.push((name.clone(), bytes));
    }
    // World 1's load list (the last section from h4+$19fc4) writes `_WYBIJACZKA` to h4+$9618 twice
    // and never to the second copy at h4+$9898, which the other worlds' lists load. So in world 1 the
    // second copy is the main program's own bytes, as loaded, not the file (measured 2026-11-05).
    let knocker = &checked[11].1;
    let copy = am.peek(HUNK_BASES[4] + KNOCKER_COPY, knocker.len());
    assert!(
        copy != *knocker,
        "world 1's second knocker copy now holds the file"
    );
    let program = fd_tools::game_file(&fd_tools::disk_dir().unwrap(), "disk1/FOREST_DUMB_FOREVER")
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(
        program.windows(copy.len()).any(|w| w == copy),
        "world 1's second knocker copy is neither the file nor the program's own bytes"
    );

    // And the readers take every one of them, the sizes they check being the buffers' sizes.
    let level = read_level(1, &files).unwrap_or_else(|e| panic!("{e}"));
    let sprites = read_sprites(1, &files).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(level.gens.len(), 328, "docs/LEVEL-INDEX.md");
    assert_eq!(level.shapes.len(), 1568 / 32);
    assert_eq!(sprites.creatures.picture.len(), 0x2bb18 - 0x20c18);
    assert_eq!(sprites.elements.picture.len(), 0x1900);
    assert_eq!(
        sprites.knocker.picture.len(),
        (KNOCKER_COPY - 0x9618) as usize
    );
    assert_eq!(sprites.vanishing.picture.len(), 0xa298 - 0x9b18);
}

/// The play area's window (docs/09-play-measured.md).
const PLAY_STRT: u16 = 0x2F91;
const PLAY_STOP: u16 = 0xE2A1;
const CAMERA_X: u32 = 0x1e62;
const CAMERA_Y: u32 = 0x1e64;
/// Measured 2026-11-05: the play window's top-left pixel shows level pixel
/// (`camera_x + 17`, `camera_y + 1`), at camera x 0 and at 522 (a fine scroll of 10).
const VIEW_DX: i32 = 17;
const VIEW_DY: i32 = 1;

/// The level palette: hunk 1's copper list at `$304` once a level has started, 32 pairs of
/// `COLORnn` and its value (docs/06).
fn palette(am: &Amiga) -> Vec<u16> {
    let at = HUNK_BASES[1] + 0x304;
    (0..32u16)
        .map(|i| {
            let reg = word(am, at + 4 * i as u32);
            assert_eq!(reg, 0x180 + 2 * i, "the copper list's colour {i}");
            word(am, at + 4 * i as u32 + 2)
        })
        .collect()
}

/// The whole cells in view and the ones whose 256 pixels are not the tile the map names.
fn compare_view(am: &Amiga, level: &Level) -> (usize, Vec<(i32, i32)>) {
    let pal = palette(am);
    let f: Rgb12Frame = am.picture_in(PLAY_STRT, PLAY_STOP);
    let (w, h) = (f.width as i32, f.height as i32);
    let ox = word(am, HUNK_BASES[0] + CAMERA_X) as i32 + VIEW_DX;
    let oy = word(am, HUNK_BASES[0] + CAMERA_Y) as i32 + VIEW_DY;
    let (mut whole, mut wrong) = (0, Vec::new());
    for col in (ox + 15) / 16..(ox + w) / 16 {
        for row in (oy + 15) / 16..(oy + h) / 16 {
            whole += 1;
            let t = level.tile(col as u16, row as u16);
            let same = (0..16u8).all(|y| {
                (0..16u8).all(|x| {
                    let (px, py) = (col * 16 + x as i32 - ox, row * 16 + y as i32 - oy);
                    pal[level.tile_pixel(t, x, y) as usize] == f.pixel(px as u16, py as u16)
                })
            });
            if !same {
                wrong.push((col, row));
            }
        }
    }
    (whole, wrong)
}

/// Every whole cell in view shows its tile, pixel for pixel, except under the creatures and items
/// cut over the tiles: none at frame 6340, 8 at 6640 (measured 2026-11-05; 11 before T59 turned the
/// oracle's collision detection on, which changes what is cut there). So the map's cells, the
/// tile pictures, their bit order and the palette are what the original draws. The first frames of play fade the
/// palette in (frames 6301 to 6331), so the first still view checked is frame 6340.
#[test]
fn cells_in_view_show_the_tile_the_map_names_still_and_scrolled() {
    let level = read_level(1, &files).unwrap_or_else(|e| panic!("{e}"));
    let sc = script::parse("6345: R\n6400: R F\n6410: R\n6600: -\n").unwrap();
    let mut am = sync::reach_level_1(340).unwrap_or_else(|e| panic!("{e}"));
    for (frame, camera, covered) in [(6340, (0, 51), 0), (6640, (522, 464), 8)] {
        while am.frame() < frame {
            let at = am.frame();
            am.step(script::input_at(&sc.lines, at))
                .unwrap_or_else(|e| panic!("{e}"));
        }
        let seen = (
            word(&am, HUNK_BASES[0] + CAMERA_X),
            word(&am, HUNK_BASES[0] + CAMERA_Y),
        );
        assert_eq!(seen, camera, "camera at frame {frame}, measured 2026-11-05");
        let (whole, wrong) = compare_view(&am, &level);
        assert_eq!(
            whole, 160,
            "whole cells in the 272 x 179 view at frame {frame}"
        );
        // The oracle is deterministic, so the count is exact; a reader that gets a few tiles wrong
        // must not hide inside a slack allowance.
        assert!(
            wrong.len() == covered,
            "frame {frame}: {} of {whole} cells are not their tile: {wrong:?}",
            wrong.len()
        );
    }
}

/// The share of neighbouring cells holding different tiles whose touching edges agree in at least
/// half their 16 pixels, across and down, with the cell at (c, r) given by `cell`.
fn seam_score(level: &Level, cols: u16, rows: u16, cell: impl Fn(u16, u16) -> u16) -> f64 {
    let (mut agree, mut pairs) = (0u32, 0u32);
    let mut pair = |a: u16, b: u16, edge: &dyn Fn(u16, u16, u8) -> (u8, u8)| {
        if a != b {
            pairs += 1;
            let same = (0..16).filter(|&i| {
                let (x, y) = edge(a, b, i);
                x == y
            });
            agree += (same.count() >= 8) as u32;
        }
    };
    for c in 0..cols {
        for r in 0..rows {
            if c + 1 < cols {
                pair(cell(c, r), cell(c + 1, r), &|a, b, y| {
                    (level.tile_pixel(a, 15, y), level.tile_pixel(b, 0, y))
                });
            }
            if r + 1 < rows {
                pair(cell(c, r), cell(c, r + 1), &|a, b, x| {
                    (level.tile_pixel(a, x, 15), level.tile_pixel(b, x, 0))
                });
            }
        }
    }
    agree as f64 / pairs as f64
}

/// Read a column at a time, neighbouring tiles join at their edges; read as 96 across (the renders'
/// old, transposed reading) they do not. Measured 2026-11-05 on W1-1: 0.46 against 0.32.
#[test]
fn the_tile_map_joins_at_its_seams_read_by_columns_and_not_by_rows() {
    let level = read_level(1, &files).unwrap_or_else(|e| panic!("{e}"));
    let cells = &level.tiles.cells;
    let by_columns = seam_score(&level, 280, 96, |c, r| level.tile(c, r));
    let by_rows = seam_score(&level, 96, 280, |c, r| cells[r as usize * 96 + c as usize]);
    assert!(
        by_columns > by_rows + 0.1,
        "seams agree {by_columns:.3} by columns, {by_rows:.3} by rows"
    );
}
