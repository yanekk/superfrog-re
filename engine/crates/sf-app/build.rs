//! The engine build identity save states are tagged with (DESIGN §2.8): the version and a hash of
//! the game's sources, the pure crates and the lock file. Any change to the game changes it, so a
//! state is refused by a build whose layout or mechanics may differ, committed or not; a build of
//! the same game, on any checkout, accepts it.

use std::path::{Path, PathBuf};

/// The crates whose code is the game a save state holds.
const GAME: [&str; 4] = ["sf-core", "sf-assets", "sf-paula", "sf-disk"];

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// FNV-1a, 64 bits: stable across compilers, unlike the standard library's hasher.
fn fnv(h: &mut u64, bytes: &[u8]) {
    for &b in bytes {
        *h ^= b as u64;
        *h = h.wrapping_mul(0x100_0000_01b3);
    }
}

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let crates = manifest.parent().unwrap();
    let workspace = crates.parent().unwrap();
    let mut inputs = vec![workspace.join("Cargo.lock")];
    for c in GAME {
        let dir = crates.join(c);
        println!("cargo:rerun-if-changed={}", dir.join("src").display());
        println!("cargo:rerun-if-changed={}", dir.join("Cargo.toml").display());
        inputs.push(dir.join("Cargo.toml"));
        files(&dir.join("src"), &mut inputs);
    }
    println!("cargo:rerun-if-changed={}", workspace.join("Cargo.lock").display());
    inputs.sort();
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for p in &inputs {
        fnv(&mut h, p.strip_prefix(workspace).unwrap_or(p).to_string_lossy().as_bytes());
        fnv(&mut h, &[0]);
        fnv(&mut h, &std::fs::read(p).unwrap_or_default());
        fnv(&mut h, &[0]);
    }
    let version = std::env::var("CARGO_PKG_VERSION").unwrap();
    println!("cargo:rustc-env=SF_ENGINE_BUILD=superfrog {version} ({h:016x})");
}
