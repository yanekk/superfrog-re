//! Save states (DESIGN §2.8): the whole [`Game`] as bytes and back, tagged with the engine build
//! and the asset cache hash and refused when either differs, because the state's layout changes as
//! mechanics are added and a half-compatible load would silently corrupt a game.
//!
//! Layout:
//!
//! ```text
//! header   bincode StateHeader: magic "SFST", engine_build, cache_hash, frame
//! hash     32 bytes   BLAKE3 of the body
//! body     bincode Game, field by field in declaration order
//! ```
//!
//! What the disks decide ([`crate::DisplayData`]) is not in it: [`Game::load`] builds it again from
//! the `GameData` it is given, which the cache hash ties to the one the state was saved with.

use std::collections::BTreeSet;
use std::fmt;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use sf_assets::GameData;

use crate::game::Game;
use crate::render::DisplayData;

pub const MAGIC: [u8; 4] = *b"SFST";

/// What a state file says about itself, ahead of the game it holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateHeader {
    pub magic: [u8; 4],
    /// The engine build that wrote it.
    pub engine_build: String,
    /// [`sf_assets::cache_hash`] of the game data it was played on.
    pub cache_hash: [u8; 32],
    /// [`Game::frame`] when it was saved.
    pub frame: u32,
}

/// Why a state cannot be loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateError {
    /// Not a save state at all.
    NotAState,
    /// Written by another build of the engine, named.
    OtherBuild { theirs: String },
    /// Written on other game data (the disks imported again differently, or a changed cache).
    OtherCache,
    /// A save state, of this build, that is cut short or damaged.
    Corrupt,
}

impl fmt::Display for StateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StateError::NotAState => write!(f, "the file is not a save state"),
            StateError::OtherBuild { theirs } => {
                write!(f, "the save state was made by another version of this game ({theirs})")
            }
            StateError::OtherCache => {
                write!(f, "the save state was made with other game data (the disks imported differently)")
            }
            StateError::Corrupt => write!(f, "the save state is damaged"),
        }
    }
}

impl std::error::Error for StateError {}

/// bincode with its fixed-width integers and no limit beyond the input's length.
fn opts() -> impl bincode::Options {
    use bincode::Options;
    bincode::DefaultOptions::new()
        .with_fixint_encoding()
        .with_little_endian()
        .reject_trailing_bytes()
}

impl Game {
    /// The game as a save state, tagged with `engine_build` and `cache_hash` (the
    /// [`sf_assets::cache_hash`] of the game data it plays). The same game always gives the same
    /// bytes.
    pub fn save(&self, engine_build: &str, cache_hash: &[u8; 32]) -> Vec<u8> {
        use bincode::Options;
        let header = StateHeader {
            magic: MAGIC,
            engine_build: engine_build.to_string(),
            cache_hash: *cache_hash,
            frame: self.frame(),
        };
        let body = opts().serialize(self).expect("a game always encodes");
        let mut out = opts().serialize(&header).expect("a header always encodes");
        out.extend_from_slice(&sf_assets::blake3(&body));
        out.extend_from_slice(&body);
        out
    }

    /// A save state's header, if `bytes` begin with one.
    pub fn state_header(bytes: &[u8]) -> Result<StateHeader, StateError> {
        use bincode::Options;
        if bytes.len() < MAGIC.len() {
            // A prefix of the magic is a state cut short; anything else is not a state.
            return Err(if !bytes.is_empty() && MAGIC.starts_with(bytes) {
                StateError::Corrupt
            } else {
                StateError::NotAState
            });
        }
        if bytes[..MAGIC.len()] != MAGIC {
            return Err(StateError::NotAState);
        }
        let mut r = bytes;
        bincode::Options::allow_trailing_bytes(opts())
            .deserialize_from(&mut r)
            .map_err(|_| StateError::Corrupt)
    }

    /// The game a save state holds, on `data`, if it was written by `engine_build` on game data
    /// whose hash is `cache_hash`, and is whole.
    pub fn load(
        data: &GameData,
        bytes: &[u8],
        engine_build: &str,
        cache_hash: &[u8; 32],
    ) -> Result<Game, StateError> {
        Game::load_with(Arc::new(DisplayData::new(data)), bytes, engine_build, cache_hash)
    }

    /// [`Game::load`] with the disks' display content already built (as [`Game::with_display`]).
    pub fn load_with(
        display: Arc<DisplayData>,
        bytes: &[u8],
        engine_build: &str,
        cache_hash: &[u8; 32],
    ) -> Result<Game, StateError> {
        use bincode::Options;
        let header = Game::state_header(bytes)?;
        if header.engine_build != engine_build {
            return Err(StateError::OtherBuild { theirs: header.engine_build });
        }
        if header.cache_hash != *cache_hash {
            return Err(StateError::OtherCache);
        }
        let at = opts().serialized_size(&header).map_err(|_| StateError::Corrupt)? as usize;
        let rest = &bytes[at..];
        if rest.len() < 32 {
            return Err(StateError::Corrupt);
        }
        let (hash, body) = rest.split_at(32);
        if sf_assets::blake3(body)[..] != *hash {
            return Err(StateError::Corrupt);
        }
        let mut game: Game = opts().deserialize(body).map_err(|_| StateError::Corrupt)?;
        game.restore_display(display);
        Ok(game)
    }
}

/// `&'static str` fields: written as text, read back as the one copy of that text kept for the
/// whole run (the strings are a few short names: unported paths, instruction patterns).
pub(crate) mod static_str {
    use super::*;

    static INTERNED: Mutex<BTreeSet<&'static str>> = Mutex::new(BTreeSet::new());

    pub fn intern(s: &str) -> &'static str {
        let mut set = INTERNED.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(&k) = set.get(s) {
            return k;
        }
        let k: &'static str = Box::leak(s.to_owned().into_boxed_str());
        set.insert(k);
        k
    }

    pub fn serialize<S: serde::Serializer>(s: &&'static str, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(s)
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(de: D) -> Result<&'static str, D::Error> {
        let s = String::deserialize(de)?;
        Ok(intern(&s))
    }

    /// `Vec<&'static str>`.
    pub mod vec {
        use super::*;

        pub fn serialize<S: serde::Serializer>(v: &[&'static str], ser: S) -> Result<S::Ok, S::Error> {
            ser.collect_seq(v.iter())
        }

        pub fn deserialize<'de, D: serde::Deserializer<'de>>(
            de: D,
        ) -> Result<Vec<&'static str>, D::Error> {
            let v = Vec::<String>::deserialize(de)?;
            Ok(v.iter().map(|s| intern(s)).collect())
        }
    }
}

/// Arrays longer than serde's 32: written as their elements in order.
pub(crate) mod big_array {
    use serde::de::{Error, SeqAccess, Visitor};
    use serde::ser::SerializeTuple;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer, T: Serialize, const N: usize>(
        a: &[T; N],
        ser: S,
    ) -> Result<S::Ok, S::Error> {
        let mut t = ser.serialize_tuple(N)?;
        for x in a {
            t.serialize_element(x)?;
        }
        t.end()
    }

    pub fn deserialize<'de, D: Deserializer<'de>, T: Deserialize<'de>, const N: usize>(
        de: D,
    ) -> Result<[T; N], D::Error> {
        struct V<T, const N: usize>(std::marker::PhantomData<T>);
        impl<'de, T: Deserialize<'de>, const N: usize> Visitor<'de> for V<T, N> {
            type Value = [T; N];
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "an array of {N}")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<[T; N], A::Error> {
                let mut v = Vec::with_capacity(N);
                for i in 0..N {
                    v.push(seq.next_element()?.ok_or_else(|| A::Error::invalid_length(i, &self))?);
                }
                v.try_into().map_err(|_| A::Error::custom("array length"))
            }
        }
        de.deserialize_tuple(N, V::<T, N>(std::marker::PhantomData))
    }
}
