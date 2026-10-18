//! T210: the display windows the engine draws over the cases given, each case's distinct
//! `(scene, left, top, width, height)` with the first frame it is seen on, then the largest width
//! and height over them all: what `sf-app`'s window is sized for (DESIGN §2.7).
//!
//! `cargo run --release --example windows -- <case>...` (a name in `conform/cases`, or a path)
//!
//! Each frame's picture is the engine's for 4:3 (`render(View::default())`), read before the next
//! step. A divergence is printed and the case's windows up to it still count.

use std::collections::BTreeMap;
use std::path::PathBuf;

use sf_conform::Case;
use sf_core::View;

fn main() {
    let mut most = (0u16, 0u16);
    for a in std::env::args().skip(1) {
        let path = if a.ends_with(".toml") {
            PathBuf::from(&a)
        } else {
            sf_conform::cases_dir().join(format!("{a}.toml"))
        };
        let case = Case::load(&path).unwrap_or_else(|e| panic!("{a}: {e}"));
        let mut seen: BTreeMap<(String, i32, u16, u16, u16), u32> = BTreeMap::new();
        let r = sf_conform::run_with(&case, |f, g| {
            let p = g.render(View::default());
            let scene = format!("{:?}", g.scene());
            seen.entry((scene, p.left, p.top, p.width, p.height)).or_insert(f);
        });
        if let Err(d) = r {
            println!("{a}: {d}");
        }
        for ((scene, left, top, w, h), f) in &seen {
            println!("{a}: {w}×{h} at ({left}, {top}) {scene}, first frame {f}");
            most = (most.0.max(*w), most.1.max(*h));
        }
    }
    println!("largest: {}×{}", most.0, most.1);
}
