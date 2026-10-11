//! Pure: chunks -> GameData and its cache codec (DESIGN §3.2).
//!
//! [`unpack`] turns the identified disks into their decompressed chunks (with the game's own rule
//! for the three that need a sector past their carve); [`decode`] turns those chunks into
//! [`GameData`], every kind of content the game reads from its disks, typed. Each format is in
//! docs/04, docs/06 and docs/10. Sound is T11's and is carried undecoded until then.

mod anim;
mod error;
mod exe;
mod font;
mod gamedata;
mod palette;
mod picture;
mod sprites;
mod t7mp;
mod tables;
mod tiles;

pub use anim::{Anim, AnimFrame, AnimId, decode_anim};
pub use error::{AssetError, Why};
pub use exe::{ExeId, ExeImage, Hunk, HunkKind, Reloc, parse_exe};
pub use font::{Font, FontId};
pub use gamedata::{
    ChunkId, GameData, LevelData, LevelId, World, catalogue, decode, unpack, unpack_chunk,
};
pub use palette::Palette;
pub use picture::{Bitmap, Picture, PictureId};
pub use sprites::{SpriteBank, SpriteId};
pub use t7mp::{T7mp, Tag, t7mp_decode, t7mp_encode};
pub use tables::{FileEntry, HiscoreEntry, PasswordEntry, Tables};
pub use tiles::{TILE_BYTES, TileMasks, Tileset, WorldGfx};
