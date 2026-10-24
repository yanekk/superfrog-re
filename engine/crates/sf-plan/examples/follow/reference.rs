//! The reference run: a fixed route played once in its own timing (the original from its sync
//! point), every frame's input and frog recorded. Both new route forms are cut from it: A's
//! triggers (the frog where the input changes) and B's guide points (every 8th frame). Kept under
//! `~/.cache/superfrog-re/route-follower/` (derived, regenerable).

use std::path::PathBuf;

use sf_core::input::{FrameInput, Joy, KeySet};
use sf_plan::core::{Facts, GuidePoint};

use crate::machine::Machine;
use crate::play::Obs;

#[derive(Debug, Clone, Copy)]
pub struct RefFrame {
    pub input: FrameInput,
    /// The frog before the frame's input.
    pub o: Obs,
    /// The frame ended mid-pass (the original lagging).
    pub mid: bool,
}

pub fn dir() -> PathBuf {
    let home = std::env::var_os("HOME").expect("HOME");
    PathBuf::from(home).join(".cache/superfrog-re/route-follower")
}

pub fn path(tag: &str) -> PathBuf {
    dir().join(format!("{tag}.ref"))
}

/// Plays `inputs` on `m` until the level has ended past its exit (or the inputs end), recording.
pub fn record(m: &mut dyn Machine, facts: &Facts, inputs: &[FrameInput]) -> Result<Vec<RefFrame>, String> {
    let mut out = Vec::new();
    let mut played = false;
    for &x in inputs {
        let o = Obs::of(m.state(), facts);
        if played && !o.playing() {
            break;
        }
        played |= o.playing();
        m.step(x)?;
        out.push(RefFrame { input: x, o, mid: m.mid_pass() });
    }
    Ok(out)
}

pub fn save(tag: &str, frames: &[RefFrame]) -> std::io::Result<()> {
    std::fs::create_dir_all(dir())?;
    let mut s = String::from("# k joy keys x y landed facing coins hearts lives dying level_state switches mid\n");
    for (k, r) in frames.iter().enumerate() {
        let o = &r.o;
        s += &format!(
            "{k} {} {} {} {} {} {} {} {} {} {} {} {} {}\n",
            r.input.joy.0,
            r.input.keys.0,
            o.x,
            o.y,
            o.landed as u8,
            o.facing,
            o.coins,
            o.hearts,
            o.lives,
            o.dying as u8,
            o.level_state,
            o.switches,
            r.mid as u8
        );
    }
    std::fs::write(path(tag), s)
}

pub fn load(tag: &str) -> Result<Vec<RefFrame>, String> {
    let p = path(tag);
    let text = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
    let mut out = Vec::new();
    for l in text.lines().filter(|l| !l.starts_with('#')) {
        let v: Vec<i64> = l.split_whitespace().map(|w| w.parse().unwrap()).collect();
        let (x, y) = (v[3] as i16, v[4] as i16);
        let o = Obs {
            x,
            y,
            cell: cell_of(x, y),
            landed: v[5] != 0,
            facing: v[6] as u16,
            coins: v[7] as u16,
            hearts: v[8] as u16,
            lives: v[9] as u16,
            dying: v[10] != 0,
            level_state: v[11] as u32,
            switches: v[12] as u32,
        };
        out.push(RefFrame {
            input: FrameInput { joy: Joy(v[1] as u8), keys: KeySet(v[2] as u128) },
            o,
            mid: v[13] != 0,
        });
    }
    Ok(out)
}

/// `frog_cell`'s formula on a position (unclamped is fine on a played route).
pub fn cell_of(x: i16, y: i16) -> (u16, u16) {
    (((x as i32 + 0x17).div_euclid(16)).clamp(0, 299) as u16, ((y as i32 - 0x10).div_euclid(16)).clamp(0, 83) as u16)
}

/// B's route: the frog's cell, coins left and switches every 8 frames while playing.
pub fn guide(frames: &[RefFrame]) -> Vec<GuidePoint> {
    frames
        .iter()
        .step_by(8)
        .filter(|r| r.o.playing())
        .map(|r| GuidePoint { cell: r.o.cell, coins_left: Some(r.o.coins), switches: Some(r.o.switches) })
        .collect()
}

/// The pixel positions behind the guide points (B1 steers by them; B2/B3 score by them).
pub fn guide_px(frames: &[RefFrame]) -> Vec<(i16, i16, u16, u32)> {
    frames
        .iter()
        .step_by(8)
        .filter(|r| r.o.playing())
        .map(|r| (r.o.x, r.o.y, r.o.coins, r.o.switches))
        .collect()
}

/// Frames the reference ended mid-pass on (P4's lag at the original's own places).
pub fn lag_frames(frames: &[RefFrame]) -> std::collections::BTreeSet<u32> {
    frames.iter().enumerate().filter(|(_, r)| r.mid).map(|(k, _)| k as u32).collect()
}
