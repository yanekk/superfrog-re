//! The map page's numbers against the original (plan forest-dumb-map-page T02, DESIGN §2.2, §4): for
//! each world, `fd-map`'s items needed equals `items_left` and its exit equals the `$ff` cells of the
//! RAM object map (`h0+$cb38`), both as `level_init` leaves them, read when `main_loop` first runs.
//!
//! The way in is the `world-N-start` cases' (translator T33): `world-2-start`'s input from power-on,
//! and for worlds 2 to 6 the case's `world_index` poked as `load_world` (`h0+$578`) is entered;
//! world 1 is the same run unpoked. About 35 s a world on the original: full.

use amiga_oracle::{Amiga, OracleConfig, kickstart_path};
use fd_assets::{AssetError, read_level};
use fd_conform::{Case, HUNK_BASES, Inputs, conform_dir, disks};
use fd_core::layout_var;

const LOAD_WORLD: u32 = 0x578;
const LOAD_WORLD_RETURNS: u32 = 0x232;
const MAIN_LOOP: u32 = 0x252;
const OBJECT_MAP: u32 = 0xcb38;
const OBJECT_MAP_BYTES: usize = 400 * 96;
const FRAMES: u32 = 7000;

fn h(hunk: usize, off: u32) -> u32 {
    HUNK_BASES[hunk] + off
}

fn bcd(w: u16) -> u16 {
    (0..4).rev().fold(0, |n, i| n * 10 + (w >> (4 * i) & 0xf))
}

fn disk(name: &str) -> Result<Vec<u8>, String> {
    fd_tools::game_file(&fd_tools::disk_dir()?, name)
}

/// (`items_left` decoded, the object map) on the original when `main_loop` first runs in world
/// `world`'s level.
fn original_at_level_start(world: u8) -> (u16, Vec<u8>) {
    let case_file = format!("cases/world-{}-start.toml", world.max(2));
    let text = std::fs::read_to_string(conform_dir().join(&case_file)).unwrap();
    let case = Case::parse(&case_file, &text).unwrap();
    let inputs = Inputs::load(&case).unwrap();
    let wi = layout_var("world_index").unwrap();
    let il = layout_var("items_left").unwrap();
    let poke = (world > 1).then(|| {
        let set = case
            .set
            .iter()
            .find(|s| s.var == "world_index")
            .expect("world_index set");
        assert_eq!(set.value, world as u32 - 1, "{case_file}");
        (set.value).to_be_bytes()[4 - wi.width.bytes()..].to_vec()
    });
    let cfg = OracleConfig::new(kickstart_path().unwrap(), disks::load().unwrap(), FRAMES);
    let mut am = Amiga::power_on(&cfg).unwrap();
    am.watch_pcs(&[h(0, LOAD_WORLD_RETURNS), h(0, MAIN_LOOP)]);
    let mut loaded = false;
    for frame in 0..FRAMES {
        if let Some(s) = case.swap.filter(|s| s.frame == frame) {
            am.insert_disk(0, s.disk as usize - 1);
        }
        // Armed at the idle title: the boot runs load_world's address before the program is there.
        if let Some(p) = &poke
            && frame == case.sync_frame
        {
            am.poke_at(h(0, LOAD_WORLD), h(wi.hunk as usize, wi.offset), p);
        }
        am.step(inputs.at(frame)).unwrap();
        let hit = |pc: u32| am.pc_hits().iter().any(|&(p, n)| p == h(0, pc) && n > 0);
        // The boot passes load_world's return address too; the level's load is after the title.
        if frame > case.sync_frame && hit(LOAD_WORLD_RETURNS) {
            loaded = true;
        }
        if loaded && hit(MAIN_LOOP) {
            let w = am.peek(h(wi.hunk as usize, wi.offset), wi.width.bytes());
            assert_eq!(
                *w.last().unwrap() as u32,
                world as u32 - 1,
                "W{world}-1 world_index"
            );
            let items = am.peek(h(il.hunk as usize, il.offset), 2);
            let map = am.peek(h(0, OBJECT_MAP), OBJECT_MAP_BYTES).to_vec();
            return (bcd(u16::from_be_bytes([items[0], items[1]])), map);
        }
    }
    panic!("W{world}-1: main_loop not reached in {FRAMES} frames");
}

#[test]
fn items_needed_and_exit_equal_the_original_at_level_start() {
    let program = disk("disk1/FOREST_DUMB_FOREVER").unwrap_or_else(|e| panic!("{e}"));
    let files =
        |name: &str| disk(&format!("disk2/{name}")).map_err(|e| AssetError::missing(name, e));
    let mut wrong = Vec::new();
    for world in fd_map::WORLDS {
        let level = read_level(world, &files).unwrap_or_else(|e| panic!("{e}"));
        let m = fd_map::map_level(world, &level, &program).unwrap_or_else(|e| panic!("{e}"));
        let (items_left, ram) = original_at_level_start(world);
        let exit: Vec<(u16, u16)> = (0..m.cols)
            .flat_map(|c| (0..96u16).map(move |r| (c, r)))
            .filter(|&(c, r)| ram[c as usize * 96 + r as usize] == 0xff)
            .collect();
        println!(
            "W{world}-1: items_left {items_left}, fd-map {}; exit {exit:?}",
            m.items_needed
        );
        if items_left != m.items_needed {
            wrong.push(format!(
                "W{world}-1 items: original {items_left}, fd-map {}",
                m.items_needed
            ));
        }
        if exit != m.exit {
            wrong.push(format!(
                "W{world}-1 exit: original {exit:?}, fd-map {:?}",
                m.exit
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
