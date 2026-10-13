//! Pure: chunks -> GameData and its cache codec (DESIGN §3.2).
//!
//! [`unpack`] turns the identified disks into their decompressed chunks (with the game's own rule
//! for the three that need a sector past their carve); [`decode`] turns those chunks into
//! [`GameData`], every kind of content the game reads from its disks, typed; [`cache_encode`] and
//! [`cache_decode`] keep it between launches and [`decide`] says on launch what to do about it. Each format is in
//! docs/04, docs/06 and docs/10. Sound is T11's and is carried undecoded until then.

mod anim;
mod blake3;
mod cache;
mod error;
mod exe;
mod font;
mod gamedata;
mod launch;
mod palette;
mod picture;
pub mod sound;
mod sprites;
mod t7mp;
mod tables;
mod tiles;

pub use anim::{Anim, AnimFrame, AnimId, decode_anim};
pub use cache::{CACHE_FORMAT, CacheError, cache_decode, cache_encode};
pub use error::{AssetError, Why};
pub use exe::{ExeId, ExeImage, Hunk, HunkKind, Reloc, parse_exe};
pub use font::{Font, FontId};
pub use gamedata::{
    ChunkId, GameData, INTRO_SPAN_INDEX, LevelData, LevelId, World, catalogue, decode, unpack,
    unpack_chunk,
};
pub use launch::{LaunchAction, LaunchFacts, decide};
pub use palette::Palette;
pub use picture::{Bitmap, Picture, PictureId};
pub use sprites::{SpriteBank, SpriteId};
pub use t7mp::{T7mp, Tag, t7mp_decode, t7mp_encode};
pub use tables::{
    FileEntry, HiscoreEntry, LampRect, LampRow, PasswordEntry, ReelFrame, SLOT_LAMPS, Tables,
};
pub use tiles::{TILE_BYTES, TileMasks, Tileset, WorldGfx};
