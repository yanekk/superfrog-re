//! real-widescreen T03 (DESIGN §2.4, D4, D11): widescreen has `WIDE_SLOTS` more object slots, bob
//! records, free-list entries, draw-list places and hitbox entries, at an address the original
//! never maps; the 4:3 game never touches them.

use sf_core::input::{FrameInput, Joy};
use sf_core::layout::LAYOUT;
use sf_core::{Game, GameData, LevelId, Start};

fn data() -> &'static GameData {
    sf_conform::game_data()
}

/// Frame `f`'s input: a walk right with jumps and fire, so the camera scrolls and objects spawn.
fn input(f: u32) -> FrameInput {
    let mut joy = Joy::NONE;
    if f % 90 < 60 {
        joy = joy | Joy::RIGHT;
    } else if f % 90 < 75 {
        joy = joy | Joy::LEFT;
    }
    if f % 37 < 4 {
        joy = joy | Joy::UP;
    }
    if f % 53 < 2 {
        joy = joy | Joy::FIRE;
    }
    FrameInput::joy(joy)
}

/// FNV-1a, so a stored hash means the same on every build.
fn fnv(h: &mut u64, bytes: &[u8]) {
    for &b in bytes {
        *h ^= b as u64;
        *h = h.wrapping_mul(0x0100_0000_01b3);
    }
}

/// Every variable of the state but the widescreen ones (named `wide_…`), each frame's picture and
/// Paula writes, over `frames` 4:3 frames of `level`.
fn run_hash(level: &str, frames: u32) -> u64 {
    let mut g = Game::new(data(), Start::Level(level.parse::<LevelId>().unwrap()));
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for f in 0..frames {
        let out = g.step(input(f));
        fnv(&mut h, &out.picture.pixels);
        for p in &out.picture.palettes {
            for c in p {
                fnv(&mut h, &c.to_be_bytes());
            }
        }
        fnv(&mut h, format!("{:?}", out.paula).as_bytes());
    }
    for d in LAYOUT.iter().filter(|d| !d.name.starts_with("wide_")) {
        fnv(&mut h, &g.state().bytes_of(d.id).1);
    }
    h
}

/// The 4:3 game is byte for byte what it was before the extra slots existed: hashes taken on
/// `pir/real-widescreen` at a541fd8, before T03.
#[test]
fn the_4_3_game_is_unchanged() {
    for (level, want) in [
        ("W1-1", 0x6ec8_0e17_29d4_66fe),
        ("W3-2", 0xd828_925a_82f9_071c),
        ("WB-1", 0x27b5_9227_da42_dda9),
    ] {
        assert_eq!(run_hash(level, 2000), want, "{level}: the 4:3 game changed");
    }
}
