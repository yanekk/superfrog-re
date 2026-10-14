//! The cheapest route from a level's start (marker `$35`) to its exit (54) through its map, a step
//! through an open cell costing 1 and through a solid one (`marker & $18 == $18`) 60, and every
//! solid cell it crosses: where a switch's gate or the like must open; and each enemy marker's
//! cheapest instance. For steering scripts.
//!
//! `cargo run --release --example map_route -- <level>`
fn main() {
    let id: sf_core::LevelId = std::env::args().nth(1).unwrap().parse().unwrap();
    let data = sf_conform::game_data();
    let t = &data.levels.iter().find(|l| *l.0 == id).unwrap().1.t7mp;
    let (w, h) = (t.width as usize, t.height as usize);
    let m = |x: usize, y: usize| t.body[y * w + x] >> 10;
    let solid = |x: usize, y: usize| m(x, y) & 0x18 == 0x18;
    let start = (0..w * h).find(|&k| t.body[k] >> 10 == 0x35).unwrap();
    let mut d = vec![u32::MAX; w * h];
    let mut from = vec![usize::MAX; w * h];
    let mut q = std::collections::BinaryHeap::new();
    d[start] = 0;
    q.push(std::cmp::Reverse((0u32, start)));
    while let Some(std::cmp::Reverse((here, k))) = q.pop() {
        if here > d[k] {
            continue;
        }
        let (x, y) = (k % w, k / w);
        for (nx, ny) in [(x.wrapping_sub(1), y), (x + 1, y), (x, y.wrapping_sub(1)), (x, y + 1)] {
            if nx < w && ny < h {
                let nd = here + if solid(nx, ny) { 60 } else { 1 };
                if nd < d[ny * w + nx] {
                    d[ny * w + nx] = nd;
                    from[ny * w + nx] = k;
                    q.push(std::cmp::Reverse((nd, ny * w + nx)));
                }
            }
        }
    }
    let exit = (0..w * h).filter(|&k| t.body[k] >> 10 == 54).min_by_key(|&k| d[k]).unwrap();
    let mut path = vec![exit];
    while *path.last().unwrap() != start {
        path.push(from[*path.last().unwrap()]);
    }
    path.reverse();
    println!("cost {}; {} cells", d[exit], path.len());
    let mut run: Vec<String> = Vec::new();
    for &k in &path {
        let (x, y) = (k % w, k / w);
        if solid(x, y) {
            run.push(format!("{x},{y}(m{})", m(x, y)));
        }
    }
    println!("solid crossed: {}", run.join(" "));
    // Each enemy marker's instance cheapest to reach, and the open cell next to it it is reached by.
    let d = &d;
    for mk in 12..=20u16 {
        let best = (0..w * h)
            .filter(|&k| t.body[k] >> 10 == mk)
            .flat_map(|k| {
                let (x, y) = (k % w, k / w);
                [(x, y + 1), (x, y.wrapping_sub(1)), (x + 1, y), (x.wrapping_sub(1), y), (x, y + 2)]
                    .into_iter()
                    .filter(|&(a, b)| a < w && b < h && !solid(a, b))
                    .map(move |(a, b)| (d[b * w + a], x, y, a, b))
            })
            .min();
        if let Some((c, x, y, a, b)) = best {
            println!("marker {mk}: {x},{y} cost {c}, by {a},{b}");
        }
    }
    let pts: Vec<String> = path.iter().step_by(8).map(|&k| format!("{},{}", k % w, k / w)).collect();
    println!("route: {}", pts.join(" "));
}
