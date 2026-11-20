//! The level's hidden passages, listed from its files (`docs/17-secrets.md`, plan
//! forest-dumb-map-page T07).
//!
//! What the character stands on or walks through is `collision_map`, built only from the cells'
//! shapes (`_GenN-1.Mapa`) and rewritten under vanishing blocks; the tile picture goes to the
//! screen alone. So a passage is a cell whose picture is a wall's and whose shape is empty. A
//! picture is a wall's when the same tile is fully solid in the level often enough: in at least
//! [`WALL_MIN`] cells and a tenth of its cells. Per-tile majority is the wrong test: some worlds
//! build their hidden tunnels from stone tiles used mostly there (W1-1's tiles 301 to 310 are
//! empty in 136 cells and solid in 60), and those must still read as wall.
//!
//! Such cells also make the trim of every visible opening: door frames, alcove edges, the exit's
//! frame, one cell thick beside open background. A group of them (4-connected) counts only when
//! one of its cells has a wall picture on all four sides, so it lies inside a wall, not on its rim.
//! Shapes neither fully solid nor empty (slopes, ledges) never count. Kinds the original was not
//! seen to have are not listed; the doc says what was looked for.

use crate::level::{LEVEL_ROWS, Level};

/// A wall picture is fully solid in at least this many cells of the level: one solid use of a
/// background tile (a stray cell under a ceiling) does not make it a wall.
pub const WALL_MIN: u32 = 2;

/// Object values `$30` to `$33`: vanishing blocks, which write `collision_map` themselves
/// (`docs/10-object-system.md`), so their cell's shape says nothing.
const VANISHING: std::ops::RangeInclusive<u8> = 0x30..=0x33;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SecretKind {
    /// Drawn as wall, passable: confirmed on the original in W1-1, W3-1 and W4-1
    /// (`fd-conform/tests/secret_claims.rs`).
    HiddenPassage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Secret {
    pub c: u16,
    pub r: u16,
    pub kind: SecretKind,
}

/// Every secret cell of `level`, column by column, over the collision map's columns.
pub fn secrets(level: &Level) -> Vec<Secret> {
    let cols = level.collision.columns;
    let rows = LEVEL_ROWS;
    let full = |c, r| level.shapes[level.shape(c, r) as usize] == [0xffff; 16];
    let empty = |c, r| level.shapes[level.shape(c, r) as usize] == [0; 16];

    let tiles = level.gens.len();
    let (mut solid, mut used) = (vec![0u32; tiles], vec![0u32; tiles]);
    for c in 0..cols {
        for r in 0..rows {
            let t = level.tile(c, r) as usize;
            used[t] += 1;
            solid[t] += u32::from(full(c, r));
        }
    }
    let wall_tile: Vec<bool> = (0..tiles)
        .map(|t| solid[t] >= WALL_MIN && solid[t] * 10 >= used[t])
        .collect();
    // Past the map's edge counts as wall: the level is closed there.
    let wall = |c: i32, r: i32| {
        c < 0
            || r < 0
            || c >= cols as i32
            || r >= rows as i32
            || wall_tile[level.tile(c as u16, r as u16) as usize]
    };
    let at = |c: u16, r: u16| c as usize * rows as usize + r as usize;
    let mut candidate = vec![false; cols as usize * rows as usize];
    for c in 0..cols {
        for r in 0..rows {
            candidate[at(c, r)] = wall(c as i32, r as i32)
                && empty(c, r)
                && !VANISHING.contains(&level.object(c, r));
        }
    }

    let mut seen = vec![false; candidate.len()];
    let mut out = Vec::new();
    for c in 0..cols {
        for r in 0..rows {
            if !candidate[at(c, r)] || seen[at(c, r)] {
                continue;
            }
            let mut group = Vec::new();
            let mut stack = vec![(c, r)];
            seen[at(c, r)] = true;
            while let Some((gc, gr)) = stack.pop() {
                group.push((gc, gr));
                for (dc, dr) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let (nc, nr) = (gc as i32 + dc, gr as i32 + dr);
                    if nc < 0 || nr < 0 || nc >= cols as i32 || nr >= rows as i32 {
                        continue;
                    }
                    let n = at(nc as u16, nr as u16);
                    if candidate[n] && !seen[n] {
                        seen[n] = true;
                        stack.push((nc as u16, nr as u16));
                    }
                }
            }
            let inside = group.iter().any(|&(gc, gr)| {
                let (gc, gr) = (gc as i32, gr as i32);
                wall(gc + 1, gr) && wall(gc - 1, gr) && wall(gc, gr + 1) && wall(gc, gr - 1)
            });
            if inside {
                out.extend(group.into_iter().map(|(c, r)| Secret {
                    c,
                    r,
                    kind: SecretKind::HiddenPassage,
                }));
            }
        }
    }
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::{CollisionMap, ObjectMap, TileMap};
    use crate::{OBJECT_COLUMNS, TILE_BYTES};

    const COLS: u16 = 12;
    /// Shapes: 0 solid, 1 empty, 2 a slope.
    const SOLID: u8 = 0;
    const EMPTY: u8 = 1;
    const SLOPE: u8 = 2;
    /// Tiles: 0 open background, 1 a wall picture, 2 a picture solid in one cell only.
    const BACK: u16 = 0;
    const WALL: u16 = 1;
    const RARE: u16 = 2;

    /// A level of `COLS` columns: open background (tile 0, empty) everywhere, and a block of wall
    /// (tile 1, solid) at columns 4 to 9, rows 10 to 19.
    fn level() -> Level {
        let n = COLS as usize * LEVEL_ROWS as usize;
        let mut l = Level {
            tiles: TileMap {
                columns: COLS,
                cells: vec![BACK; n],
            },
            gens: vec![[0; TILE_BYTES]; 3],
            collision: CollisionMap {
                columns: COLS,
                cells: vec![EMPTY; n],
            },
            shapes: vec![[0xffff; 16], [0; 16], std::array::from_fn(|i| 0xffff >> i)],
            objects: ObjectMap {
                columns: OBJECT_COLUMNS,
                cells: vec![0; OBJECT_COLUMNS as usize * LEVEL_ROWS as usize],
            },
        };
        for c in 4..10 {
            for r in 10..20 {
                put(&mut l, c, r, WALL, SOLID);
            }
        }
        l
    }

    fn put(l: &mut Level, c: u16, r: u16, tile: u16, shape: u8) {
        let i = c as usize * LEVEL_ROWS as usize + r as usize;
        l.tiles.cells[i] = tile;
        l.collision.cells[i] = shape;
    }

    fn cells(l: &Level) -> Vec<(u16, u16)> {
        secrets(l).iter().map(|s| (s.c, s.r)).collect()
    }

    #[test]
    fn a_wall_tile_met_empty_inside_the_wall_is_a_passage() {
        let mut l = level();
        put(&mut l, 6, 14, WALL, EMPTY);
        assert_eq!(cells(&l), [(6, 14)]);
        assert_eq!(secrets(&l)[0].kind, SecretKind::HiddenPassage);
    }

    #[test]
    fn a_tunnel_reaching_the_open_is_listed_whole_with_its_mouth() {
        let mut l = level();
        for c in 4..8 {
            put(&mut l, c, 15, WALL, EMPTY);
        }
        // Column 4's cell borders open background on its left: on its own it would be trim, but
        // it belongs to a group with cells inside the wall.
        assert_eq!(cells(&l), [(4, 15), (5, 15), (6, 15), (7, 15)]);
    }

    #[test]
    fn a_wall_tile_met_empty_on_the_rim_is_trim_not_a_passage() {
        let mut l = level();
        // A frame one cell thick: open background on its left.
        put(&mut l, 4, 12, WALL, EMPTY);
        put(&mut l, 4, 13, WALL, EMPTY);
        assert_eq!(cells(&l), []);
    }

    #[test]
    fn a_slope_under_a_wall_tile_does_not_count() {
        let mut l = level();
        put(&mut l, 6, 14, WALL, SLOPE);
        assert_eq!(cells(&l), []);
    }

    #[test]
    fn a_tile_solid_in_one_cell_only_is_not_a_wall_picture() {
        let mut l = level();
        put(&mut l, 1, 1, RARE, SOLID);
        put(&mut l, 6, 14, RARE, EMPTY);
        assert_eq!(cells(&l), []);
        // Solid in two cells, and in more than a tenth of its cells, it is.
        put(&mut l, 1, 2, RARE, SOLID);
        assert_eq!(cells(&l), [(6, 14)]);
    }

    #[test]
    fn a_tile_solid_in_fewer_than_a_tenth_of_its_cells_is_not_a_wall_picture() {
        let mut l = level();
        // Tile 2 solid in 2 cells and empty in 19 outside the block: 2 of 21, under a tenth.
        put(&mut l, 1, 1, RARE, SOLID);
        put(&mut l, 1, 2, RARE, SOLID);
        for r in 30..49 {
            put(&mut l, 1, r, RARE, EMPTY);
        }
        put(&mut l, 6, 14, RARE, EMPTY);
        assert_eq!(cells(&l), []);
    }

    #[test]
    fn a_vanishing_block_cell_is_not_a_passage() {
        let mut l = level();
        put(&mut l, 6, 14, WALL, EMPTY);
        l.objects.cells[6 * LEVEL_ROWS as usize + 14] = 0x31;
        assert_eq!(cells(&l), []);
    }

    #[test]
    fn a_background_tile_met_solid_is_not_listed() {
        // Solid cells drawn as background were looked for (docs/17) and are no kind.
        let mut l = level();
        put(&mut l, 1, 40, BACK, SOLID);
        assert_eq!(cells(&l), []);
    }

    #[test]
    fn only_kinds_confirmed_on_the_original_exist() {
        // docs/17: an invisible floor was looked for and is not a kind; a new variant needs its
        // oracle claim in fd-conform/tests/secret_claims.rs first.
        match SecretKind::HiddenPassage {
            SecretKind::HiddenPassage => {}
        }
    }
}
