// The translation judge's port (src/judge/mod.rs) comes from FD_PORT, a file under build/ that
// tools/translate.py writes: never committed, since a translation of the game's code derives from
// it. Without it a stub that changes nothing builds, which fails every routine that changes
// anything: the judge's own negative check. As the spike's judge/build.rs.
//
// The generator service's candidate (src/gen/mod.rs, `candidate`) comes from FD_GEN the same way
// (tools/generate.py, T61): a generator module as the committed ones in src/gen/ are written, kept
// under build/ until the builder has read it and commits it. Without it the module lists nothing.
//
// The translated tree (src/judge/translated.rs, `tree`) comes from FD_TRANSLATED the same way
// (translator-trial T04): `build/translator/tree.rs`, which `tools/pcode_rust.py` writes from the
// game's code, so never committed. Without it the stub has no entries, so every routine judged
// `--translated` stops as untranslated: that mode's negative check. FD_TRANSLATED may also name a
// directory, `build/translator/tree/` (translator T04): its `mod.rs` and parts are compiled where they
// are, through a `#[path]`, so each part is a module of its own and rustc's errors name the part.
//
// The instruction check's module (src/judge/insn.rs, `tree`) comes from FD_INSN the same way
// (translator T06): `build/translator/insn.rs`, written by `pcode_rust.py --insn`. Without it the
// stub has no sites, so `fd-judge --insn-all` checks nothing.
use std::{env, fs, path::Path};

fn main() {
    let out = Path::new(&env::var("OUT_DIR").unwrap()).to_path_buf();
    built_in(
        "FD_PORT",
        &out.join("port.rs"),
        "pub fn port(_cpu: &mut Cpu) {}\n",
    );
    built_in(
        "FD_GEN",
        &out.join("candidate.rs"),
        "use super::Generator;\npub const ALL: &[(&str, Generator)] = &[];\n",
    );
    built_in(
        "FD_TRANSLATED",
        &out.join("tree.rs"),
        "use pcode_rt::{Bus, Image, Regs, Stop};\n\
         pub const CODE: &[(u32, u32, u32)] = &[];\n\
         pub fn entry(_addr: &str) -> Option<fn(&mut Regs, &mut dyn Bus, &Image) -> Result<(), Stop>> {\n    None\n}\n",
    );
    built_in(
        "FD_INSN",
        &out.join("insn.rs"),
        "use pcode_rt::{Bus, Image, Regs, Stop};\n\
         pub const CODE: &[(u32, u32, u32)] = &[];\n\
         pub const HUNK_SIZES: &[u32] = &[];\n\
         pub type Routine = fn(&mut Regs, &mut dyn Bus, &Image) -> Result<(), Stop>;\n\
         pub const SITES: &[(&str, &str, u32, u32, u32, &[(u32, u32)], Routine)] = &[];\n",
    );
}

/// `out` holds the file `var` names, or `stub` without one.
fn built_in(var: &str, out: &Path, stub: &str) {
    println!("cargo:rerun-if-env-changed={var}");
    let src = match env::var(var) {
        Ok(p) if !p.is_empty() && Path::new(&p).is_dir() => {
            // Cargo watches a directory's files, so a regenerated part rebuilds.
            println!("cargo:rerun-if-changed={p}");
            let root = Path::new(&p).join("mod.rs");
            let root = fs::canonicalize(&root).unwrap_or_else(|e| panic!("{var}={p}: {}: {e}", root.display()));
            format!("#[path = {:?}]\nmod generated;\npub use generated::*;\n", root.display().to_string())
        }
        Ok(p) if !p.is_empty() => {
            println!("cargo:rerun-if-changed={p}");
            let s = fs::read_to_string(&p).unwrap_or_else(|e| panic!("{var}={p}: {e}"));
            // `include!` takes items only: a module's `//!` doc lines become plain comments, so the
            // same file builds here and, committed, as a module of its own.
            s.lines()
                .map(|l| match l.trim_start().strip_prefix("//!") {
                    Some(rest) => format!("//{rest}\n"),
                    None => format!("{l}\n"),
                })
                .collect()
        }
        _ => stub.to_string(),
    };
    // Written only when it changed, so a rebuild with the same file recompiles nothing.
    if fs::read_to_string(out).ok().as_deref() != Some(src.as_str()) {
        fs::write(out, src).unwrap();
    }
}
