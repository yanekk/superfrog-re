//! Compiles the shim (`shim/shim.cpp`) against the vAmiga core that `tools/setup/oracle.sh` built
//! into `~/.cache/superfrog-re/vamiga/<commit>/<patch-set hash>/`, and links the core.
//!
//! The patch-set hash is the SHA-1 of `patches/*.patch` concatenated in name order, first 12 hex
//! digits, exactly as the script computes it; a changed patch therefore asks for a fresh build.

use std::path::{Path, PathBuf};

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let commit = std::fs::read_to_string(manifest.join("VAMIGA_COMMIT")).expect("VAMIGA_COMMIT");
    let commit = commit.trim();
    let mut patches: Vec<PathBuf> = std::fs::read_dir(manifest.join("patches"))
        .map(|d| d.map(|e| e.unwrap().path()).collect())
        .unwrap_or_default();
    patches.retain(|p| p.extension().is_some_and(|x| x == "patch"));
    patches.sort();
    let mut all = Vec::new();
    for p in &patches {
        all.extend(std::fs::read(p).unwrap());
        println!("cargo:rerun-if-changed={}", p.display());
    }
    let phash = &sf_disk::sha1_hex(&all)[..12];

    let cache = std::env::var_os("SUPERFROG_CACHE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var("HOME").expect("HOME")).join(".cache/superfrog-re")
        });
    let dest = cache.join("vamiga").join(commit).join(phash);
    println!("cargo:rerun-if-env-changed=SUPERFROG_CACHE");
    println!("cargo:rerun-if-changed=VAMIGA_COMMIT");
    println!("cargo:rerun-if-changed=patches");
    println!("cargo:rerun-if-changed=shim/shim.cpp");
    println!("cargo:rerun-if-changed=shim/shim.h");
    println!("cargo:rerun-if-changed={}", dest.join(".done").display());
    if !dest.join(".done").exists() {
        panic!(
            "sf-oracle: no vAmiga build at {} (commit {commit}, patches {phash}). Run tools/setup/oracle.sh \
             from the repository root (the test block's setup does).",
            dest.display()
        );
    }
    println!("cargo:rustc-env=SF_ORACLE_BUILD={commit}-{phash}");

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .file("shim/shim.cpp")
        .include("shim")
        .opt_level(2)
        .warnings(false);
    for flag in read_lines(&dest.join("shim-flags.txt")) {
        if let Some(dir) = flag.strip_prefix("-I") {
            build.include(dir);
        } else if let Some(def) = flag.strip_prefix("-D") {
            match def.split_once('=') {
                Some((k, v)) => build.define(k, v),
                None => build.define(def, None),
            };
        } else {
            build.flag(&flag);
        }
    }
    build.compile("sfshim");

    for lib in read_lines(&dest.join("libs.txt")) {
        let lib = Path::new(&lib);
        let name = lib.file_stem().unwrap().to_string_lossy();
        println!(
            "cargo:rustc-link-search=native={}",
            lib.parent().unwrap().display()
        );
        println!(
            "cargo:rustc-link-lib=static={}",
            name.strip_prefix("lib").unwrap_or(&name)
        );
    }
    println!("cargo:rustc-link-lib=dylib=z");
    println!("cargo:rustc-link-lib=dylib=c++");
    println!("cargo:rustc-link-lib=framework=CoreMIDI");
    println!("cargo:rustc-link-lib=framework=CoreFoundation");
}

fn read_lines(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{}: {e}; rerun tools/setup/oracle.sh", path.display()))
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect()
}
