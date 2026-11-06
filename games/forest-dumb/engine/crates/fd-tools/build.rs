//! The engine build a bug recording is stamped with and replay checks (DESIGN §2.8, §2.12): the
//! version and a hash of the game's sources, the pure crates' and the lock file. Any change to the
//! game changes it, committed or not; a build of the same sources, on any checkout, has the same
//! one. Here, in a shell crate both `fd-app` and `fd-conform` depend on, so the two always agree;
//! not in `fd-core`, whose every file the purity test holds to no file access.

use std::path::{Path, PathBuf};

/// The crates whose code decides what a recording's inputs do: the engine's, and the toolkit's it
/// is built on.
const GAME: [&str; 2] = ["fd-core", "fd-assets"];
const TOOLKIT: [&str; 6] = [
    "amiga-chips",
    "amiga-clock",
    "amiga-input",
    "amiga-paula",
    "amiga-disk",
    "amiga-record",
];

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
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
    let repo = workspace.ancestors().nth(3).unwrap();
    let toolkit = repo.join("toolkit/crates");
    let mut inputs = vec![workspace.join("Cargo.lock")];
    let dirs = GAME
        .iter()
        .map(|c| crates.join(c))
        .chain(TOOLKIT.iter().map(|c| toolkit.join(c)));
    for dir in dirs {
        // A crate that moved without this list following would hash as nothing.
        assert!(
            dir.join("src").is_dir(),
            "build tag: no sources at {}",
            dir.display()
        );
        println!("cargo:rerun-if-changed={}", dir.join("src").display());
        println!(
            "cargo:rerun-if-changed={}",
            dir.join("Cargo.toml").display()
        );
        inputs.push(dir.join("Cargo.toml"));
        files(&dir.join("src"), &mut inputs);
    }
    println!(
        "cargo:rerun-if-changed={}",
        workspace.join("Cargo.lock").display()
    );
    inputs.sort();
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for p in &inputs {
        // Relative to the repository, so the tag is the same on every checkout.
        fnv(
            &mut h,
            p.strip_prefix(repo)
                .unwrap_or(p)
                .to_string_lossy()
                .as_bytes(),
        );
        fnv(&mut h, &[0]);
        fnv(&mut h, &std::fs::read(p).unwrap_or_default());
        fnv(&mut h, &[0]);
    }
    let version = std::env::var("CARGO_PKG_VERSION").unwrap();
    println!("cargo:rustc-env=FD_ENGINE_BUILD=forest-dumb {version} ({h:016x})");
}
