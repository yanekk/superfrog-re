//! Forest Dumb Forever's level and sprite files, read from their unpacked bytes
//! (`docs/05-file-formats.md`, `docs/06-graphics-and-rendering.md`).
//!
//! Pure (plans/forest-dumb-level-1 DESIGN §3.1): files arrive through a closure the caller supplies,
//! so this crate touches no disk. None of the files has a header, so a file's size is its whole
//! check; a file of the wrong size, or holding a number its format cannot hold, is an
//! [`AssetError`], never a panic.

mod error;
mod level;
mod program;
mod sprites;

pub use error::{AssetError, Why};
pub use level::{
    CollisionMap, LEVEL_ROWS, Level, OBJECT_COLUMNS, ObjectMap, SHAPE_BYTES, TILE_BYTES, TileMap,
    level_file_names, read_level,
};
pub use program::loaded_hunk;
pub use sprites::{Sheet, Sprites, read_sprites, sprite_file_names};

/// A closure handing over one of the game's files, unpacked, by its name: a level file of disk 2,
/// or the main program `FOREST_DUMB_FOREVER` of disk 1 (T08).
pub type Files<'a> = dyn Fn(&str) -> Result<Vec<u8>, AssetError> + 'a;

/// Interleaved bitmaps hold five planes, one line of each plane after the other (chapter 06).
pub const PLANES: usize = 5;

#[cfg(test)]
mod tests;
