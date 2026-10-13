//! Scratch (T36): reach levels' first frames with the enemies frozen (warms the sync cache).
use sf_conform::{SyncPoint, sync, InputScript};
use sf_conform::freeze::Freeze;
fn main() {
    for id in std::env::args().skip(1) {
        let t = std::time::Instant::now();
        let s = sync::reach_frozen(&SyncPoint::AtLevel(id.parse().unwrap()), &InputScript::idle(), 1, Freeze::Enemies);
        println!("{id}: {} in {:?}", if s.is_ok() { "ok" } else { "FAILED" }, t.elapsed());
    }
}
