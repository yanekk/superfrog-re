// The port under judgement comes from SPIKE_PORT (a file under build/, never committed: a translation
// of the game's code is derived from it). Without it the stub below builds, which leaves memory and
// registers alone and so fails every routine: the judge's own negative check.
use std::{env, fs, path::Path};

fn main() {
    println!("cargo:rerun-if-env-changed=SPIKE_PORT");
    let out = Path::new(&env::var("OUT_DIR").unwrap()).join("port.rs");
    let src = match env::var("SPIKE_PORT") {
        Ok(p) if !p.is_empty() => {
            println!("cargo:rerun-if-changed={p}");
            fs::read_to_string(&p).unwrap_or_else(|e| panic!("SPIKE_PORT={p}: {e}"))
        }
        _ => "pub fn port(_cpu: &mut Cpu) {}\n".to_string(),
    };
    fs::write(out, src).unwrap();
}
