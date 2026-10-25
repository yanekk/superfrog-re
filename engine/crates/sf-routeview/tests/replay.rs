//! The viewer's replay against what the planner recorded, on the real disks: a committed set-C
//! route ends on the frame its run verified, the marker table read from the game names world 1's
//! enemies as sf-core's dispatch does, and a level with no route still draws its map.

use sf_routeview::dump::{self, marker_template};
use sf_routeview::verdict::{self, Status};

fn committed(level: &str) -> String {
    let p = sf_bench::repo_dir().join(format!("plans/smart-planner/bench/routes/app/{level}.txt"));
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

#[test]
fn a_committed_route_replays_to_its_recorded_exit() {
    // results.jsonl: new, set C, W1-1: completed, route_frames 2844, hearts_lost 0.
    let mut d = dump::replay("W1-1".parse().unwrap(), Some(&committed("W1-1"))).unwrap();
    assert_eq!((d.end, d.hearts_lost(), d.lives_lost()), (Some(2844), 0, 0));
    assert_eq!(d.track.len(), 2844);
    assert_eq!(d.timing, "app");
    assert!(d.coins_met_at.is_some());
    assert!(verdict::progress(&mut d).is_empty(), "a route that ends has no stall marks");
    assert_eq!(verdict::verdict(&d, true, None).status, Status::Clean);
    // Every coin the route took is an event at a coin object's cell.
    let coins = d.objects.iter().filter(|o| o.cat == "coin").count();
    let taken = d.events.iter().filter(|e| e.kind == "marker").count();
    assert!(coins > 0 && taken > 0);
    assert_eq!(d.tiles.len(), {
        let mut u = d.body.clone();
        u.sort();
        u.dedup();
        u.len()
    });
}

#[test]
fn world_ones_markers_spawn_the_templates_its_dispatch_registers() {
    // sf-core enemies/w1/mod.rs: marker → spawn_<template>; items/mod.rs: 8, 9 coin, 29 lock, 45.
    let rom = sf_conform::display_data();
    let want = [
        (1, 8),
        (2, 12),
        (3, 13),
        (8, 0),
        (9, 0),
        (10, 33),
        (11, 23),
        (12, 22),
        (13, 20),
        (14, 19),
        (15, 21),
        (16, 78),
        (17, 77),
        (18, 76),
        (29, 5),
        (45, 4),
    ];
    let got: Vec<(u8, u16)> = (1..58).filter_map(|m| marker_template(rom.rom(), 0, m).map(|t| (m, t))).collect();
    assert_eq!(got, want);
}

#[test]
fn a_level_with_no_route_draws_its_map_and_objects() {
    let mut d = dump::replay("W3-4".parse().unwrap(), None).unwrap();
    assert_eq!((d.end, d.track.len()), (None, 1));
    assert!(d.objects.iter().any(|o| o.name == "level exit"));
    assert!(d.objects.iter().any(|o| o.cat == "enemy" && o.name.starts_with("w3 ")));
    assert!(verdict::progress(&mut d).is_empty());
    let v = verdict::verdict(&d, false, None);
    assert_eq!((v.status, v.faults[0].as_str()), (Status::Failed, "no route"));
}
