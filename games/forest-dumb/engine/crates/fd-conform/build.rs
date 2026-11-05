// The translation judge's port (src/judge/mod.rs) comes from FD_PORT, a file under build/ that
// tools/translate.py writes: never committed, since a translation of the game's code derives from
// it. Without it a stub that changes nothing builds, which fails every routine that changes
// anything: the judge's own negative check. As the spike's judge/build.rs.
use std::{env, fs, path::Path};

fn main() {
    println!("cargo:rerun-if-env-changed=FD_PORT");
    let out = Path::new(&env::var("OUT_DIR").unwrap()).join("port.rs");
    let src = match env::var("FD_PORT") {
        Ok(p) if !p.is_empty() => {
            println!("cargo:rerun-if-changed={p}");
            fs::read_to_string(&p).unwrap_or_else(|e| panic!("FD_PORT={p}: {e}"))
        }
        _ => "pub fn port(_cpu: &mut Cpu) {}\n".to_string(),
    };
    // Written only when it changed, so a rebuild with the same port recompiles nothing.
    if fs::read_to_string(&out).ok().as_deref() != Some(src.as_str()) {
        fs::write(out, src).unwrap();
    }
}
