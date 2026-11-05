//! One world's level: five files (`docs/05-file-formats.md`).
//!
//! All three maps are stored a column at a time, 96 cells to a column: the game's drawing loop at
//! hunk 0 `$708` reads index `col * $60 + row`. Read as 96 across, the terrain does not join at the
//! tile edges, which is how the renders were once drawn transposed; `fd-conform/tests/level_files.rs`
//! holds the seam check that tells the two readings apart.

use crate::error::{AssetError, Why};
use crate::{Files, PLANES};

/// Cells in one column of every map: the level's height.
pub const LEVEL_ROWS: u16 = 96;
/// Bytes a tile picture: 16 lines of five planes of two bytes, line-interleaved.
pub const TILE_BYTES: usize = 16 * PLANES * 2;
/// Bytes a collision shape: 16 big-endian words, top line first, most significant bit leftmost.
pub const SHAPE_BYTES: usize = 32;
/// The object map always fills the game's whole buffer, whatever the level's width.
pub const OBJECT_COLUMNS: u16 = 400;

/// Tile-map and collision-map columns. World 6 is wider; the other five tile maps run 40 columns of
/// filler rock past their collision map (chapter 05).
fn columns(world: u8) -> (u16, u16) {
    if world == 6 { (320, 320) } else { (280, 240) }
}

/// The five files of world `world`, in the order tile map, tile pictures, collision map, shapes,
/// object map.
pub fn level_file_names(world: u8) -> [String; 5] {
    [
        format!("_Mapa{world}-1"),
        format!("_Gen{world}-1"),
        format!("_Mapa{world}-1.Mapa"),
        format!("_Gen{world}-1.Mapa"),
        format!("_MapaBobow{world}"),
    ]
}

/// The tile map: a tile number a cell, column by column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileMap {
    pub columns: u16,
    /// `cells[col * 96 + row]`, as the file and the game's memory hold them.
    pub cells: Vec<u16>,
}

/// The collision map: a shape number a cell, column by column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollisionMap {
    pub columns: u16,
    pub cells: Vec<u8>,
}

/// The object map: an object byte a cell, column by column, 0 for nothing. The game also writes to
/// it at run time (hunk 0 `$2e54`), so the level holds its own copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectMap {
    pub columns: u16,
    pub cells: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Level {
    pub tiles: TileMap,
    /// `_GenN-1`: the tile pictures, indexed by tile number, each as the file holds it.
    pub gens: Vec<[u8; TILE_BYTES]>,
    pub collision: CollisionMap,
    /// `_GenN-1.Mapa`: the collision shapes, indexed by shape number; a set bit is solid.
    pub shapes: Vec<[u16; 16]>,
    pub objects: ObjectMap,
}

impl Level {
    /// The tile number at a cell. Panics past the map, as a slice index does: the camera's clamps
    /// keep every read the game makes inside it.
    pub fn tile(&self, col: u16, row: u16) -> u16 {
        self.tiles.cells[index(col, row, self.tiles.columns)]
    }

    /// The collision shape number at a cell. Panics past the map.
    pub fn shape(&self, col: u16, row: u16) -> u8 {
        self.collision.cells[index(col, row, self.collision.columns)]
    }

    /// The object byte at a cell. Panics past the map.
    pub fn object(&self, col: u16, row: u16) -> u8 {
        self.objects.cells[index(col, row, self.objects.columns)]
    }

    /// The colour number (0 to 31) of pixel (`x`, `y`) of tile `tile`, both 0 to 15.
    pub fn tile_pixel(&self, tile: u16, x: u8, y: u8) -> u8 {
        let t = &self.gens[tile as usize];
        let (x, y) = (x as usize & 15, y as usize & 15);
        let mut colour = 0;
        for plane in 0..PLANES {
            let at = (y * PLANES + plane) * 2 + x / 8;
            colour |= (t[at] >> (7 - x % 8) & 1) << plane;
        }
        colour
    }
}

fn index(col: u16, row: u16, columns: u16) -> usize {
    assert!(
        col < columns && row < LEVEL_ROWS,
        "cell ({col}, {row}) is outside a map of {columns} x {LEVEL_ROWS}"
    );
    col as usize * LEVEL_ROWS as usize + row as usize
}

/// World `world`'s level (1 to 6), its files from `files`.
pub fn read_level(world: u8, files: &Files) -> Result<Level, AssetError> {
    if !(1..=6).contains(&world) {
        return Err(AssetError::missing(
            &format!("world {world}"),
            "there are worlds 1 to 6",
        ));
    }
    let [map, gen_, coll, shp, obj] = level_file_names(world);
    let (tile_cols, coll_cols) = columns(world);

    let gens = units::<TILE_BYTES>(&gen_, &files(&gen_)?)?;
    let shapes = units::<SHAPE_BYTES>(&shp, &files(&shp)?)?
        .into_iter()
        .map(|s| std::array::from_fn(|i| u16::from_be_bytes([s[2 * i], s[2 * i + 1]])))
        .collect::<Vec<[u16; 16]>>();

    let raw = files(&map)?;
    sized(&map, &raw, cells(tile_cols) * 2)?;
    let tile_cells: Vec<u16> = raw
        .chunks_exact(2)
        .map(|w| u16::from_be_bytes([w[0], w[1]]))
        .collect();
    below(
        &map,
        tile_cells.iter().map(|&t| t as usize),
        gens.len(),
        2,
        "tile",
    )?;

    let coll_cells = files(&coll)?;
    sized(&coll, &coll_cells, cells(coll_cols))?;
    below(
        &coll,
        coll_cells.iter().map(|&s| s as usize),
        shapes.len(),
        1,
        "shape",
    )?;

    let obj_cells = files(&obj)?;
    sized(&obj, &obj_cells, cells(OBJECT_COLUMNS))?;

    Ok(Level {
        tiles: TileMap {
            columns: tile_cols,
            cells: tile_cells,
        },
        gens,
        collision: CollisionMap {
            columns: coll_cols,
            cells: coll_cells,
        },
        shapes,
        objects: ObjectMap {
            columns: OBJECT_COLUMNS,
            cells: obj_cells,
        },
    })
}

fn cells(columns: u16) -> usize {
    columns as usize * LEVEL_ROWS as usize
}

pub(crate) fn sized(file: &str, data: &[u8], expected: usize) -> Result<(), AssetError> {
    if data.len() != expected {
        return Err(AssetError::new(
            file,
            Why::Size {
                expected,
                got: data.len(),
            },
        ));
    }
    Ok(())
}

/// `data` cut into `N`-byte units: a whole, non-zero number of them.
fn units<const N: usize>(file: &str, data: &[u8]) -> Result<Vec<[u8; N]>, AssetError> {
    if data.is_empty() || !data.len().is_multiple_of(N) {
        return Err(AssetError::new(
            file,
            Why::NotWhole {
                unit: N,
                got: data.len(),
            },
        ));
    }
    Ok(data
        .chunks_exact(N)
        .map(|c| c.try_into().expect("chunks_exact gives N bytes"))
        .collect())
}

/// Every value an index into a set of `count`, or the first that is not, by its byte offset.
fn below(
    file: &str,
    values: impl Iterator<Item = usize>,
    count: usize,
    width: usize,
    what: &str,
) -> Result<(), AssetError> {
    for (i, v) in values.enumerate() {
        if v >= count {
            return Err(AssetError::new(
                file,
                Why::Malformed {
                    at: i * width,
                    what: format!("{what} {v}, and there are only {count}"),
                },
            ));
        }
    }
    Ok(())
}
