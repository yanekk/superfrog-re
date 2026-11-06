//! What the tests share: reading a run's capture, and the headless game it is compared with.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use amiga_input::{FrameInput, Joy, KeySet};
use fd_core::{Game, GameData, Start};
use fd_e2e::{Captured, Image};

pub const UP: u8 = 1;
pub const DOWN: u8 = 2;
pub const LEFT: u8 = 4;
pub const RIGHT: u8 = 8;
pub const FIRE: u8 = 16;

/// What the game was fed at each step that had any input (`inputs.txt`).
pub fn inputs(c: &Captured) -> BTreeMap<u64, FrameInput> {
    c.inputs
        .lines()
        .map(|l| {
            let mut w = l.split(' ');
            let n = w.next().unwrap().parse().unwrap();
            let joy = u8::from_str_radix(w.next().unwrap(), 16).unwrap();
            let mut keys = KeySet::NONE;
            for k in w.next().unwrap_or("").split(',').filter(|k| !k.is_empty()) {
                keys.0 |= 1u128 << u8::from_str_radix(k, 16).unwrap();
            }
            (n, FrameInput { joy: Joy(joy), keys })
        })
        .collect()
}

/// The joystick at each step from `from` to `to`, inclusive (0 where nothing was fed).
pub fn joy(c: &Captured, from: u64, to: u64) -> Vec<u8> {
    let i = inputs(c);
    (from..=to).map(|n| i.get(&n).map_or(0, |e| e.joy.0)).collect()
}

/// `hashes.txt`: the state hash of each frame, frame 0 first.
pub fn hashes(c: &Captured) -> Vec<String> {
    let text = std::fs::read_to_string(c.dir.join(fd_app::HASHES)).expect("hashes.txt");
    text.lines()
        .enumerate()
        .map(|(i, l)| {
            let (n, h) = l.split_once(' ').expect("`N hash`");
            assert_eq!(n.parse::<usize>().unwrap(), i, "hashes.txt in order");
            h.to_string()
        })
        .collect()
}

/// The game's data from the owner's disks, read once per test process.
pub fn data() -> Arc<GameData> {
    static D: OnceLock<Arc<GameData>> = OnceLock::new();
    D.get_or_init(|| fd_conform::game_data().unwrap_or_else(|e| panic!("{e}"))).clone()
}

/// A headless game at the app's start.
pub fn headless() -> Game {
    Game::new(data(), Start::Level1)
}

/// The state hash the app writes, of a headless game.
pub fn hash(g: &Game) -> String {
    format!("{:016x}", fd_app::state_hash(g))
}

/// The headless game's picture as the capture holds it.
pub fn picture(g: &Game) -> Image {
    let p = g.picture();
    let mut rgba = vec![0; p.width as usize * p.height as usize * 4];
    fd_core::render::rgba(&p, &mut rgba);
    for px in rgba.chunks_exact_mut(4) {
        px[3] = 255;
    }
    Image { width: p.width as u32, height: p.height as u32, rgba }
}

/// Every line the Controls window showed, in order.
pub fn said(c: &Captured) -> Vec<String> {
    c.stderr.lines().filter_map(|l| l.strip_prefix("fd-app: controls: ")).map(String::from).collect()
}
