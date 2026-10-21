//! The map cache (DESIGN §2.8, §2.9): a level's movement map saved once, so everyday planning pays
//! the build once. Shell side: it reads the environment and touches files.
//!
//! A file is trusted only whole and only for the key it was written under: magic, format version,
//! the full key and a checksum of the payload are checked before it is used, and anything else is
//! rebuilt (and overwritten). Writes go to a temp file named for the writer, then `rename`, so two
//! planners writing the same key leave one whole file and a reader sees a whole map or none.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use sf_core::LevelId;

use crate::core::MoveMap;

/// File magic: the planner's map, format below.
const MAGIC: &[u8; 8] = b"SFPLMAP\0";
/// Bumped when the file layout changes.
const FORMAT: u32 = 1;

/// The planner's core sources. The engine build covers the game, not the map builder: a map built
/// by an older `core::map` is stale too (§2.9), so the key carries a hash of these as well.
/// `tests/cache.rs` checks the list is every file under `src/core/`.
pub const CORE_FILES: [(&str, &[u8]); 7] = [
    ("coins.rs", include_bytes!("core/coins.rs")),
    ("danger.rs", include_bytes!("core/danger.rs")),
    ("facts.rs", include_bytes!("core/facts.rs")),
    ("graph.rs", include_bytes!("core/graph.rs")),
    ("map.rs", include_bytes!("core/map.rs")),
    ("mod.rs", include_bytes!("core/mod.rs")),
    ("moves.rs", include_bytes!("core/moves.rs")),
];

/// The start a map is built from (DESIGN §2.1). The map is measured from the start state, so two
/// starts of one level are two maps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StartKey {
    /// The level's sync point (set A).
    AtLevel,
    /// A `whole_game` leg snapshot, by a hash of its bytes (set B): see [`StartKey::snap`].
    Snap(u64),
    /// `Game::start_level` under `Rng::seeded(seed, seed)` (set C).
    App { seed: u32 },
}

impl StartKey {
    /// The key of a leg snapshot's bytes.
    pub fn snap(bytes: &[u8]) -> StartKey {
        let mut h = FNV_START;
        fnv(&mut h, bytes);
        StartKey::Snap(h)
    }
}

/// FNV-1a, 64 bits: stable across compilers and runs, unlike the standard library's hasher.
const FNV_START: u64 = 0xcbf2_9ce4_8422_2325;
fn fnv(h: &mut u64, bytes: &[u8]) {
    for &b in bytes {
        *h ^= b as u64;
        *h = h.wrapping_mul(0x100_0000_01b3);
    }
}

/// `$SF_MAP_CACHE`, else `~/.cache/superfrog-re/maps` (beside the oracle build, shared by all
/// worktrees).
pub fn cache_dir() -> PathBuf {
    if let Some(d) = std::env::var_os("SF_MAP_CACHE").filter(|d| !d.is_empty()) {
        return PathBuf::from(d);
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    home.join(".cache/superfrog-re/maps")
}

/// The cache key, also the file's name: the level, the start, the wings, and a hash of the engine
/// build (`SF_ENGINE_BUILD`), the planner's core and the file format.
pub fn key(level: LevelId, start: StartKey, wings: bool) -> String {
    let start = match start {
        StartKey::AtLevel => "at".to_string(),
        StartKey::Snap(h) => format!("snap{h:016x}"),
        StartKey::App { seed } => format!("app{seed}"),
    };
    let wings = if wings { "wings" } else { "nowings" };
    format!("{level}.{start}.{wings}.{:016x}", build_hash())
}

/// The hash of everything that makes a map: the engine build, the core's sources, the format.
fn build_hash() -> u64 {
    let mut h = FNV_START;
    fnv(&mut h, env!("SF_ENGINE_BUILD").as_bytes());
    fnv(&mut h, &[0]);
    for (name, src) in CORE_FILES {
        fnv(&mut h, name.as_bytes());
        fnv(&mut h, &[0]);
        fnv(&mut h, src);
        fnv(&mut h, &[0]);
    }
    fnv(&mut h, &FORMAT.to_le_bytes());
    h
}

/// The map saved under `key`, else `build()`'s, saved for next time. With `no_cache` it always
/// builds and leaves the file alone (the benchmark's `--no-cache`, so M3 sees the build's cost).
/// A file that is truncated, corrupt, of another format or another key is rebuilt, never trusted.
/// A failed write costs only the next build: it is said on stderr and the map returned.
pub fn load_or_build(key: &str, no_cache: bool, build: impl FnOnce() -> MoveMap) -> MoveMap {
    if no_cache {
        return build();
    }
    let dir = cache_dir();
    let path = dir.join(format!("{key}.map"));
    if let Some(map) = fs::read(&path).ok().and_then(|b| decode(key, &b)) {
        return map;
    }
    let map = build();
    if let Err(e) = save(&dir, &path, &encode(key, &map)) {
        eprintln!("map cache: not saved to {}: {e}", path.display());
    }
    map
}

/// The file: magic, format (u32 LE), key length (u32 LE) and key, payload length (u64 LE),
/// payload checksum (u64 LE, FNV-1a), payload (bincode `MoveMap`).
pub fn encode(key: &str, map: &MoveMap) -> Vec<u8> {
    let payload = bincode::serialize(map).expect("a MoveMap serializes");
    let mut sum = FNV_START;
    fnv(&mut sum, &payload);
    let mut b = Vec::with_capacity(8 + 4 + 4 + key.len() + 16 + payload.len());
    b.extend_from_slice(MAGIC);
    b.extend_from_slice(&FORMAT.to_le_bytes());
    b.extend_from_slice(&(key.len() as u32).to_le_bytes());
    b.extend_from_slice(key.as_bytes());
    b.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    b.extend_from_slice(&sum.to_le_bytes());
    b.extend_from_slice(&payload);
    b
}

/// The map in `b` if it is a whole file of this format written under `key`.
pub fn decode(key: &str, b: &[u8]) -> Option<MoveMap> {
    fn take<'a>(b: &mut &'a [u8], n: usize) -> Option<&'a [u8]> {
        let (head, rest) = b.split_at_checked(n)?;
        *b = rest;
        Some(head)
    }
    let mut b = b;
    if take(&mut b, 8)? != MAGIC {
        return None;
    }
    if u32::from_le_bytes(take(&mut b, 4)?.try_into().ok()?) != FORMAT {
        return None;
    }
    let klen = u32::from_le_bytes(take(&mut b, 4)?.try_into().ok()?) as usize;
    if take(&mut b, klen)? != key.as_bytes() {
        return None;
    }
    let plen = u64::from_le_bytes(take(&mut b, 8)?.try_into().ok()?);
    let sum = u64::from_le_bytes(take(&mut b, 8)?.try_into().ok()?);
    if b.len() as u64 != plen {
        return None;
    }
    let mut h = FNV_START;
    fnv(&mut h, b);
    if h != sum {
        return None;
    }
    bincode::deserialize(b).ok()
}

/// Writes `bytes` to a temp file of this writer's own, then renames it over `path`: the rename is
/// atomic within a directory, so a reader or a second writer never sees a part-written file.
fn save(dir: &Path, path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    static WRITES: AtomicU64 = AtomicU64::new(0);
    fs::create_dir_all(dir)?;
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let tmp = dir.join(format!(
        "{name}.{}.{}.tmp",
        std::process::id(),
        WRITES.fetch_add(1, Ordering::Relaxed)
    ));
    let written = (|| {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        fs::rename(&tmp, path)
    })();
    if written.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    written
}
