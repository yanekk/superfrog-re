//! Gathers every `re/objects-*.toml` into the crate at build time (DESIGN §2.1): W1-1's table,
//! `objects-more.toml` and any per-world table a later plan adds, so a new table is read without a
//! code change. The list is sorted by file name, the order T01's test merges them in.

use std::path::Path;

fn main() {
    let re = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../re");
    let re = re.canonicalize().unwrap_or(re);
    // A directory: cargo reruns this when any file in it changes, a new table included.
    println!("cargo::rerun-if-changed={}", re.display());
    let mut names: Vec<String> = std::fs::read_dir(&re)
        .unwrap_or_else(|e| panic!("{}: {e}", re.display()))
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("objects-") && n.ends_with(".toml"))
        .collect();
    names.sort();
    let rows: String = names
        .iter()
        .map(|n| {
            format!(
                "    (\"re/{n}\", include_str!({:?})),\n",
                re.join(n).display().to_string()
            )
        })
        .collect();
    let out = Path::new(&std::env::var("OUT_DIR").unwrap()).join("tables.rs");
    std::fs::write(out, format!("&[\n{rows}]\n")).unwrap();
}
