//! Compiles the shim (`shim/shim.cpp`) against the vAmiga core that `toolkit/tools/setup/oracle.sh`
//! built into `~/.cache/amiga-toolkit/vamiga/<commit>/<patch-set hash>/` (`AMIGA_TOOLKIT_CACHE`
//! replaces `~/.cache/amiga-toolkit`), and links the core.
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
    let phash = &amiga_disk::sha1::sha1_hex(&all)[..12];

    let cache = std::env::var_os("AMIGA_TOOLKIT_CACHE")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var("HOME").expect("HOME")).join(".cache/amiga-toolkit")
        });
    let dest = cache.join("vamiga").join(commit).join(phash);
    println!("cargo:rerun-if-env-changed=AMIGA_TOOLKIT_CACHE");
    println!("cargo:rerun-if-changed=VAMIGA_COMMIT");
    println!("cargo:rerun-if-changed=patches");
    println!("cargo:rerun-if-changed=shim/shim.cpp");
    println!("cargo:rerun-if-changed=shim/shim.h");
    println!("cargo:rerun-if-changed=shim/moira_shim.cpp");
    println!("cargo:rerun-if-changed=shim/moira_shim.h");
    println!("cargo:rerun-if-changed=shim/moira-stub");
    println!("cargo:rerun-if-changed={}", dest.join(".done").display());
    if !dest.join(".done").exists() {
        panic!(
            "amiga-oracle: no vAmiga build at {} (commit {commit}, patches {phash}). Run \
             toolkit/tools/setup/oracle.sh.",
            dest.display()
        );
    }
    println!("cargo:rustc-env=AMIGA_ORACLE_BUILD={commit}-{phash}");

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
    build.compile("amigashim");
    build_moira(&dest);

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

/// The routine runner: Moira compiled alone from the fetched vAmiga tree, with
/// `shim/moira-stub` standing in for the two vAmiga headers it includes, and its `vamiga` namespace
/// renamed so it cannot collide with the Moira inside libVACore. None of vAmiga's other headers are
/// on the include path.
fn build_moira(dest: &Path) {
    let moira = dest.join("src/Core/Components/CPU/Moira");
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++20")
        .include("shim/moira-stub")
        .include(&moira)
        .include("shim")
        .define("vamiga", "amiga_moira")
        .define("NDEBUG", None)
        .opt_level(2)
        .warnings(false)
        .file("shim/moira_shim.cpp");
    for f in [
        "Moira.cpp",
        "MoiraCore68000.cpp",
        "MoiraCore68010.cpp",
        "MoiraCore68020.cpp",
        "MoiraDebugger.cpp",
    ] {
        build.file(moira.join(f));
    }
    build.compile("amigamoira");
}

fn read_lines(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{}: {e}; rerun toolkit/tools/setup/oracle.sh", path.display()))
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect()
}
