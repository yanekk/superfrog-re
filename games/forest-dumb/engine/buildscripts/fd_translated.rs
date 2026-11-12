// fd-translated's build script (translator T28): the generated tree goes into OUT_DIR/tree.rs, as
// fd-conform's build.rs does for the judge. FD_TRANSLATED names a tree (a directory with mod.rs, or
// one .rs file); without it games/forest-dumb/build/translator/tree/ is used when it exists; else a
// stub with no entries, so every run stops `untranslated` at once (DESIGN §2.14).
//
// Outside the crate's folder because the purity test scans every .rs under a pure crate.
//
// What is watched: the tree when there is one. With none, the nearest existing of build/translator/
// and build/ (the export creates the first, `fd-tools extract` the second), never a missing path:
// Cargo counts a missing watched path as changed on every build, which would rebuild this crate and
// fd-conform each time. In a checkout with no build/ at all, a first tree is not noticed until
// `cargo clean -p fd-translated` (README).
use std::{env, fs, path::Path, path::PathBuf};

const STUB: &str = "use pcode_rt::{Bus, Image, Regs, Stop};\n\
pub const CODE: &[(u32, u32, u32)] = &[];\n\
pub const HUNK_SIZES: &[u32] = &[];\n\
pub type Routine = fn(&mut Regs, &mut dyn Bus, &Image) -> Result<(), Stop>;\n\
pub type Resume = fn((u32, u32), &mut Regs, &mut dyn Bus, &Image) -> Result<(), Stop>;\n\
pub fn entry(_addr: &str) -> Option<Routine> {\n    None\n}\n\
pub fn resume(_k: (u32, u32)) -> Option<Resume> {\n    None\n}\n";

fn main() {
    let out = Path::new(&env::var("OUT_DIR").unwrap()).join("tree.rs");
    println!("cargo:rerun-if-env-changed=FD_TRANSLATED");
    let game = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("../../..");
    let translator = game.join("build/translator");
    let tree: Option<PathBuf> = match env::var("FD_TRANSLATED") {
        Ok(p) if !p.is_empty() => Some(PathBuf::from(p)),
        _ if translator.join("tree/mod.rs").is_file() => Some(translator.join("tree")),
        _ => None,
    };
    let src = match tree {
        Some(p) if p.is_dir() => {
            // Cargo watches a directory's files, so a regenerated part rebuilds.
            println!("cargo:rerun-if-changed={}", p.display());
            let root = p.join("mod.rs");
            let root = fs::canonicalize(&root)
                .unwrap_or_else(|e| panic!("FD_TRANSLATED={}: {}: {e}", p.display(), root.display()));
            format!(
                "#[path = {:?}]\nmod generated;\npub use generated::*;\n",
                root.display().to_string()
            )
        }
        Some(p) => {
            println!("cargo:rerun-if-changed={}", p.display());
            let s = fs::read_to_string(&p)
                .unwrap_or_else(|e| panic!("FD_TRANSLATED={}: {e}", p.display()));
            // `include!` takes items only: a module's `//!` lines become plain comments.
            s.lines()
                .map(|l| match l.trim_start().strip_prefix("//!") {
                    Some(rest) => format!("//{rest}\n"),
                    None => format!("{l}\n"),
                })
                .collect()
        }
        None => {
            for watch in [translator.clone(), game.join("build")] {
                if watch.is_dir() {
                    println!("cargo:rerun-if-changed={}", watch.display());
                    break;
                }
            }
            STUB.to_string()
        }
    };
    // Written only when it changed, so a rebuild with the same tree recompiles nothing.
    if fs::read_to_string(&out).ok().as_deref() != Some(src.as_str()) {
        fs::write(&out, src).unwrap();
    }
}
