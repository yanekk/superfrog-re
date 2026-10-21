//! Shell side of the map build: the core's layer parts run on `std::thread::scope` (DESIGN §3.1;
//! no rayon, §5). Results go back in part order, so the map is the one `core::map::build` makes.

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use sf_core::Game;

use crate::core::map::{BuildStats, Builder, Probe};
use crate::core::{Facts, MoveMap, MoveSet};

/// Parts per thread a layer is cut into: a move's cost varies with how long it flies, so small
/// parts taken in turn keep the threads busy to the layer's end.
const PARTS_PER_THREAD: usize = 8;

/// The movement map built on `threads` threads.
pub fn build_threaded(start: &Game, facts: &Facts, moves: &MoveSet, threads: usize) -> (MoveMap, BuildStats) {
    let threads = threads.max(1);
    let mut b = Builder::new(start, facts, moves);
    loop {
        let parts = b.parts(threads * PARTS_PER_THREAD);
        if parts.is_empty() {
            break;
        }
        let results: Vec<Mutex<Option<Vec<Probe>>>> = parts.iter().map(|_| Mutex::new(None)).collect();
        let next = AtomicUsize::new(0);
        let builder = &b;
        std::thread::scope(|s| {
            for _ in 0..threads {
                s.spawn(|| {
                    loop {
                        let k = next.fetch_add(1, Ordering::Relaxed);
                        let Some(part) = parts.get(k) else { break };
                        let r = builder.build_part(part);
                        *results[k].lock().unwrap() = Some(r);
                    }
                });
            }
        });
        b.absorb(results.into_iter().map(|m| m.into_inner().unwrap().unwrap()).collect());
    }
    let stats = b.stats().clone();
    (b.finish(), stats)
}
