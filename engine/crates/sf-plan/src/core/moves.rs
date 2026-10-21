//! The macro moves the movement map is measured with (DESIGN §2.2): each family is a held joystick
//! sequence, then a tail held until the frog lands. What a move reaches is never computed here; the
//! map steps the engine with it (`core::map`).
//!
//! The set is T00's spike set (walk 6/12/24, jump 4/10/20, hop, jump up, run-up 8/16/32 then jump,
//! down, flight), plus mid-air steering, which T00 found W6-3's map stuck without (D18); a 32-frame
//! jump and floats (up held in the air), which W6-3's map needs to reach the ledge before switch
//! 48's gate (T04); and launches, whose long landing cap lets a pad's or spring's throw finish.

use sf_core::input::Joy;

/// A horizontal direction, or none (straight up / standing).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Dir {
    None,
    Left,
    Right,
}

impl Dir {
    pub fn joy(self) -> Joy {
        match self {
            Dir::None => Joy::NONE,
            Dir::Left => Joy::LEFT,
            Dir::Right => Joy::RIGHT,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MoveFamily {
    /// Hold the direction for `frames`, then nothing until landed (walking off an edge falls).
    Walk { dir: Dir, frames: u16 },
    /// Hold up and the direction for `hold` frames (a longer hold jumps higher), then `then` until
    /// landed. `then == dir` is a plain jump; `then != dir` steers in the air (D18, W6-3).
    Jump { dir: Dir, hold: u16, then: Dir },
    /// Jump holding up and `dir` for 16 frames, then keep up held with `then` until landed: up
    /// held in the air floats the frog in W6's low gravity (W6-3's route onto 161,6, T04).
    Float { dir: Dir, then: Dir },
    /// Run `run_up` frames, then jump holding the direction for 20, and keep it until landed.
    RunJump { dir: Dir, run_up: u16 },
    /// Hold down (crouch, or drop through what lets the frog through), then nothing.
    Drop,
    /// A short step, then nothing for up to `LAUNCH_CAP` frames: a pad or spring next to the frog
    /// throws it far, and the throw ends landed long after a jump's cap.
    Launch { dir: Dir },
    /// With wings: a jump, then fire pressed `flaps` times (each press a flap), holding `dir`
    /// throughout, or only during the jump when `hover`.
    Fly { dir: Dir, flaps: u8, hover: bool },
}

/// Frames a move may stay in the air after its holds before it is given up as never landing.
pub const LAND_CAP: u16 = 150;
/// The same for launches: a spring's throw can cross most of a screen.
pub const LAUNCH_CAP: u16 = 300;

/// A family as concrete inputs: `holds` played in order, then `tail` held until landed, at most
/// `land_cap` frames.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Macro {
    pub holds: Vec<(Joy, u16)>,
    pub tail: Joy,
    pub land_cap: u16,
    pub needs_wings: bool,
}

impl MoveFamily {
    pub fn needs_wings(self) -> bool {
        matches!(self, MoveFamily::Fly { .. })
    }

    pub fn expand(self) -> Macro {
        let up = |d: Dir| d.joy() | Joy::UP;
        let (holds, tail, land_cap) = match self {
            MoveFamily::Walk { dir, frames } => (vec![(dir.joy(), frames)], Joy::NONE, LAND_CAP),
            MoveFamily::Jump { dir, hold, then } => (vec![(up(dir), hold)], then.joy(), LAND_CAP),
            MoveFamily::Float { dir, then } => (vec![(up(dir), 16)], up(then), LAND_CAP),
            MoveFamily::RunJump { dir, run_up } => {
                (vec![(dir.joy(), run_up), (up(dir), 20)], dir.joy(), LAND_CAP)
            }
            MoveFamily::Drop => (vec![(Joy::DOWN, 8)], Joy::NONE, LAND_CAP),
            MoveFamily::Launch { dir } => (vec![(dir.joy(), 3)], Joy::NONE, LAUNCH_CAP),
            MoveFamily::Fly { dir, flaps, hover } => {
                let during = if hover { Joy::NONE } else { dir.joy() };
                let mut h = vec![(up(dir), 6)];
                for _ in 0..flaps {
                    h.push((during | Joy::FIRE, 4));
                    h.push((during, 4));
                }
                (h, Joy::NONE, LAND_CAP)
            }
        };
        Macro {
            holds,
            tail,
            land_cap,
            needs_wings: self.needs_wings(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveSet {
    pub families: Vec<MoveFamily>,
}

/// The map's move set; flight moves only `with wings` (they are tried only from a node holding
/// wings either way).
pub fn default_moves(wings: bool) -> MoveSet {
    use Dir::*;
    let mut v = Vec::new();
    for dir in [Right, Left] {
        for frames in [6, 12, 24] {
            v.push(MoveFamily::Walk { dir, frames });
        }
        for hold in [4, 10, 20, 32] {
            v.push(MoveFamily::Jump { dir, hold, then: dir });
        }
        // A hop: the jump's direction let go once it has left the ground.
        v.push(MoveFamily::Jump { dir, hold: 20, then: None });
        for run_up in [8, 16, 32] {
            v.push(MoveFamily::RunJump { dir, run_up });
        }
        v.push(MoveFamily::Launch { dir });
    }
    for hold in [4, 20] {
        v.push(MoveFamily::Jump { dir: None, hold, then: None });
    }
    // Mid-air steering: up first, then sideways; or sideways, then the other way.
    for hold in [6, 14, 24, 32] {
        for (dir, then) in [(None, Right), (None, Left), (Right, Left), (Left, Right)] {
            v.push(MoveFamily::Jump { dir, hold, then });
        }
    }
    for dir in [None, Right, Left] {
        for then in [Right, Left] {
            v.push(MoveFamily::Float { dir, then });
        }
    }
    v.push(MoveFamily::Drop);
    if wings {
        for dir in [Right, Left] {
            for flaps in [4, 8, 16] {
                for hover in [false, true] {
                    v.push(MoveFamily::Fly { dir, flaps, hover });
                }
            }
        }
    }
    MoveSet { families: v }
}
