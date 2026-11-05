// The translation judge's port (src/judge/mod.rs) comes from FD_PORT, a file under build/ that
// tools/translate.py writes: never committed, since a translation of the game's code derives from
// it. Without it a stub that changes nothing builds, which fails every routine that changes
// anything: the judge's own negative check. As the spike's judge/build.rs.
//
// The generator service's candidate (src/gen/mod.rs, `candidate`) comes from FD_GEN the same way
// (tools/generate.py, T61): a generator module as the committed ones in src/gen/ are written, kept
// under build/ until the builder has read it and commits it. Without it the module lists nothing.
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
}

/// `out` holds the file `var` names, or `stub` without one.
fn built_in(var: &str, out: &Path, stub: &str) {
    println!("cargo:rerun-if-env-changed={var}");
    let src = match env::var(var) {
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
