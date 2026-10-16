//! Check 4, plays alone (DESIGN §2.1, T182): the moments of play a run goes through, read off
//! either side's state frame by frame, and the judge that holds the engine's to the original's.
//!
//! Both sides go through one function: a [`FrameState`] is read from the same `LAYOUT` variables
//! (the original's memory, the engine's state), and [`Milestones`] derives every milestone from
//! the changes between two of them. The original keeps no count of kills or items taken, so a
//! derivation fault shows on both sides alike and cannot make them differ.
//!
//! What says which screen is up is the copper list the frame starts with (COP1LC): the original's
//! as its CPU last wrote the register, the engine's as its program keeps it
//! ([`sf_core::Game::cop1lc`]). The blank list a screen fades through says nothing: the screen
//! before it stays the scene.

use std::fmt;

use sf_core::layout::{MAP_ROW_CELLS, VarId, def, h2};

/// How far apart, in frames of play, two sides' milestones may be (the owner's bar, DESIGN §2.1:
/// never changed in code to make a case green).
pub const MILESTONE_FRAMES: u64 = 10;

/// A screen of the game, as its copper list (and, in a level, what plays there) says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SceneKind {
    /// The story intro on disk 3 (its own program and memory map).
    Story,
    /// The title and its menus (`TITLE_COPPER`, h2+$7158).
    Title,
    /// The attract screens' award pages (h2+$6110) and hall of talent (h2+$6ff0).
    Awards,
    HallOfTalent,
    /// The attract demo: a level played by its recording (`attract_mode` set).
    AttractDemo,
    /// A level started, by the original's `world` and `level` numbers (the bonus level is world
    /// 7, [`SceneKind::BonusLevel`]).
    Level {
        world: u16,
        level: u16,
    },
    BonusLevel {
        level: u16,
    },
    /// The last life lost: `game_over_seq` runs in the level it was lost in.
    GameOver,
    /// The level-complete tally (h2+$6268).
    Tally,
    /// The slot machine (h2+$65e4).
    SlotMachine,
    /// The end sequence (h2+$5efc).
    EndSequence,
    /// A screen with a copper list not named above (a world's pictures, the progress map, until
    /// the engine shows them): its address.
    Other(u32),
}

/// One moment of play (DESIGN §2.1 check 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Milestone {
    Scene(SceneKind),
    /// An object coin taken (`coins_collected` up one), with the coins then still needed for
    /// the exit.
    CoinTaken {
        left: u16,
    },
    /// A marker cleared from the level map that no coin and no kill explains: an item, a coin
    /// drawn in the tiles, a checkpoint. `kind` is the marker, `cell` its column and row.
    ItemTaken {
        kind: u16,
        cell: (u16, u16),
    },
    /// An enemy's hit points gone below none; `kind` is its init routine (`+$00`), as `h0+$x`'s
    /// offset.
    EnemyKilled {
        kind: u32,
    },
    /// `frog_lives` down, to `lives` (negative: the game is over).
    LifeLost {
        lives: u16,
    },
    /// `level_state` set to end the level with the frog alive: the exit (or the bonus level's or
    /// the witch's end), by `world` and `level` numbers.
    LevelExit {
        world: u16,
        level: u16,
    },
    /// The tally began, with the score the level ended on.
    Tally {
        score: u32,
    },
    /// The run's last frame.
    End {
        score: u32,
        lives: u16,
        coins: u16,
    },
}

impl fmt::Display for Milestone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Milestone::Scene(s) => write!(f, "scene {s:x?}"),
            Milestone::CoinTaken { left } => write!(f, "coin taken ({left} left for the exit)"),
            Milestone::ItemTaken { kind, cell } => {
                write!(f, "item taken (marker {kind} at {},{})", cell.0, cell.1)
            }
            Milestone::EnemyKilled { kind } => write!(f, "enemy killed (init h0+${kind:x})"),
            Milestone::LifeLost { lives } => write!(f, "life lost ({} left)", *lives as i16),
            Milestone::LevelExit { world, level } => write!(f, "level exit (W{world}-{level})"),
            Milestone::Tally { score } => write!(f, "tally (score {score})"),
            Milestone::End {
                score,
                lives,
                coins,
            } => write!(
                f,
                "end (score {score}, {} lives, {coins} coins)",
                *lives as i16
            ),
        }
    }
}

/// What one frame's end shows of play, read off a side's `LAYOUT` variables.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FrameState {
    /// In the story intro, whose memory is not the game's: nothing else is read.
    pub story: bool,
    /// The copper list the next frame starts with; `None` when the side does not say.
    pub cop1lc: Option<u32>,
    pub logic_running: u16,
    pub attract: bool,
    pub world: u16,
    pub level: u16,
    pub level_state: i32,
    pub lives: u16,
    pub dying: u16,
    pub coins_left: u16,
    pub coins_collected: u32,
    pub score: u32,
    pub obj_slots: Vec<u8>,
    pub level_map: Vec<u8>,
    /// The frame ended in the middle of a main-loop pass (the side's own mid-pass test). A cleared
    /// marker is an item taken only if it is still clear at a frame that ends a pass, two passes
    /// on ([`ITEM_SETTLE`]; T115): a crumbling platform's spawn clears the marker under it and the
    /// next pass can write it back (W3-3 4953–4954 on the original, which the engine run alone
    /// does within one pass).
    pub mid_pass: bool,
}

/// The copper lists that name a screen (docs/12, docs/13).
const TITLE_COPPER: u32 = h2(0x7158);
const AWARD_COPPER: u32 = h2(0x6110);
const HALL_COPPER: u32 = h2(0x6ff0);
const TALLY_COPPER: u32 = h2(0x6268);
const SLOT_COPPER: u32 = h2(0x65e4);
const END_COPPER: u32 = h2(0x5efc);
/// The black list screens fade through.
const BLANK_COPPER: u32 = h2(0x6fd8);

/// `obj_slots`: 20 slots of `$7c` bytes (docs/09).
const SLOTS: usize = 20;
const SLOT_SIZE: usize = 0x7c;

impl FrameState {
    /// The story intro's frame.
    pub fn story() -> FrameState {
        FrameState {
            story: true,
            ..FrameState::default()
        }
    }

    /// A frame read from a side's memory: `read(addr, len)` gives `len` bytes at the original's
    /// address `addr`.
    pub fn read(read: &dyn Fn(u32, u32) -> Vec<u8>, cop1lc: Option<u32>) -> FrameState {
        let bytes = |id: VarId| {
            let d = def(id);
            read(d.addr, d.width.len())
        };
        let word = |id: VarId| be_w(&read(def(id).addr, 2), 0);
        let long = |id: VarId| be_l(&read(def(id).addr, 4), 0);
        FrameState {
            story: false,
            cop1lc,
            logic_running: word(VarId::LogicRunning),
            attract: long(VarId::AttractMode) != 0,
            world: long(VarId::World) as u16,
            level: long(VarId::Level) as u16,
            level_state: long(VarId::LevelState) as i32,
            lives: word(VarId::FrogLives),
            dying: word(VarId::FrogDying),
            coins_left: word(VarId::CoinsLeftForExit),
            coins_collected: long(VarId::CoinsCollected),
            score: long(VarId::Score),
            obj_slots: bytes(VarId::ObjSlots),
            level_map: bytes(VarId::LevelMap),
            mid_pass: false,
        }
    }

    /// The screen this frame is on; `None` when it does not say (the blank list, no list).
    pub fn scene(&self) -> Option<SceneKind> {
        if self.story {
            return Some(SceneKind::Story);
        }
        Some(match self.cop1lc? {
            sf_core::render::LEVEL_COPPER if self.attract => SceneKind::AttractDemo,
            sf_core::render::LEVEL_COPPER if (self.lives as i16) < 0 => SceneKind::GameOver,
            sf_core::render::LEVEL_COPPER if self.world == 7 => {
                SceneKind::BonusLevel { level: self.level }
            }
            sf_core::render::LEVEL_COPPER => SceneKind::Level {
                world: self.world,
                level: self.level,
            },
            TITLE_COPPER => SceneKind::Title,
            AWARD_COPPER => SceneKind::Awards,
            HALL_COPPER => SceneKind::HallOfTalent,
            TALLY_COPPER => SceneKind::Tally,
            SLOT_COPPER => SceneKind::SlotMachine,
            END_COPPER => SceneKind::EndSequence,
            BLANK_COPPER => return None,
            other => SceneKind::Other(other),
        })
    }

    /// In a level's play, with its map and objects live (`logic_running`, not the demo).
    fn playing(&self) -> bool {
        !self.story && self.logic_running != 0 && !self.attract
    }

    fn slot(&self, k: usize) -> &[u8] {
        &self.obj_slots[k * SLOT_SIZE..(k + 1) * SLOT_SIZE]
    }
}

fn be_w(b: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([b[at], b[at + 1]])
}

fn be_l(b: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

/// Derives milestones frame by frame: one [`FrameState`] after another, each compared with the
/// one before. Keeps only the last frame, so a long run costs one frame's state.
#[derive(Debug, Clone, Default)]
pub struct Milestones {
    prev: Option<FrameState>,
    scene: Option<SceneKind>,
    pub list: Vec<(u64, Milestone)>,
    /// Items whose marker went, with the frame: kept, at it, if the marker is still clear at the
    /// first frame ending a pass [`ITEM_SETTLE`] frames on ([`FrameState::mid_pass`]).
    pending: Vec<(u64, Milestone)>,
}

/// Frames (two 25 Hz passes) a cleared marker must stay clear, to a frame ending a pass, to be an
/// item taken.
pub const ITEM_SETTLE: u64 = 4;

impl Milestones {
    /// Frame `frame` of play ended in `s`.
    pub fn frame(&mut self, frame: u64, s: FrameState) {
        let mut out = Vec::new();
        self.derive(&s, &mut out);
        for m in out {
            if matches!(m, Milestone::ItemTaken { .. }) {
                self.pending.push((frame, m));
            } else {
                self.list.push((frame, m));
            }
        }
        if !s.mid_pass {
            self.settle(&s, |f| f + ITEM_SETTLE <= frame);
        }
        self.prev = Some(s);
    }

    /// The pending items `due` (by the frame each went) as `s` (a frame ending a pass, or the
    /// run's last) shows their cells: each still clear is taken, in its frame's place in the list;
    /// one back is dropped.
    fn settle(&mut self, s: &FrameState, due: impl Fn(u64) -> bool) {
        let (now, later): (Vec<_>, Vec<_>) = std::mem::take(&mut self.pending)
            .into_iter()
            .partition(|&(f, _)| due(f));
        self.pending = later;
        for (f, m) in now {
            let Milestone::ItemTaken { cell, .. } = m else {
                continue;
            };
            let i = 2 * (cell.1 as usize * MAP_ROW_CELLS as usize + cell.0 as usize);
            if s.level_map.get(i + 1).is_some_and(|&b| b & 0x3f == 0) {
                let at = self.list.partition_point(|&(g, _)| g <= f);
                self.list.insert(at, (f, m));
            }
        }
    }

    fn derive(&mut self, s: &FrameState, out: &mut Vec<Milestone>) {
        let mut push = |m| out.push(m);
        if let Some(scene) = s.scene()
            && self.scene != Some(scene)
        {
            self.scene = Some(scene);
            push(Milestone::Scene(scene));
            if scene == SceneKind::Tally {
                push(Milestone::Tally { score: s.score });
            }
        }
        let Some(p) = &self.prev else {
            return;
        };
        if !p.story && !s.story && p.world == s.world && p.level == s.level {
            if (s.lives as i16) < (p.lives as i16) {
                push(Milestone::LifeLost { lives: s.lives });
            }
            // The exit's 1 is often counted down to 0 by the same pass, before the frame ends;
            // a death sets `$190` with the frog dying.
            if p.level_state < 0 && s.level_state >= 0 && s.dying == 0 && p.playing() {
                push(Milestone::LevelExit {
                    world: s.world,
                    level: s.level,
                });
            }
        }
        if p.playing() && s.playing() && p.world == s.world && p.level == s.level {
            // Each coin with its own count, as taking it left it: two taken in one frame on one
            // side and a frame apart on the other are the same milestones.
            for k in 1..=s.coins_collected.saturating_sub(p.coins_collected).min(99) {
                let left = p.coins_left.saturating_sub(k as u16).max(s.coins_left);
                push(Milestone::CoinTaken { left });
            }
            // Kills: a slot that was an enemy (hit points 0 or more, not immune) and still holds
            // the same object, its hit points now below none.
            let mut killed_cells = Vec::new();
            for k in 0..SLOTS {
                let (a, b) = (p.slot(k), s.slot(k));
                let (cell, init) = (be_l(a, 0x40), be_l(a, 0));
                let (hp_was, hp) = (be_w(a, 0x1c) as i16, be_w(b, 0x1c) as i16);
                if cell != 0
                    && be_l(b, 0x40) == cell
                    && be_l(b, 0) == init
                    && hp_was >= 0
                    && hp_was != 0x7d00
                    && hp < 0
                {
                    killed_cells.push(cell);
                    push(Milestone::EnemyKilled {
                        kind: init.wrapping_sub(sf_core::layout::H0),
                    });
                }
            }
            // Items: a marker cleared from the map, not a coin's and not a kill's.
            let map = def(VarId::LevelMap).addr;
            for (i, (was, now)) in p.level_map.chunks(2).zip(s.level_map.chunks(2)).enumerate() {
                let (was, now) = (be_w(was, 0), be_w(now, 0));
                let marker = was & 0x3f;
                if marker == 0 || now & 0x3f != 0 || matches!(marker, 8 | 9) {
                    continue;
                }
                if killed_cells.contains(&(map + 2 * i as u32)) {
                    continue;
                }
                let cell = (i as u32 % MAP_ROW_CELLS, i as u32 / MAP_ROW_CELLS);
                push(Milestone::ItemTaken {
                    kind: marker,
                    cell: (cell.0 as u16, cell.1 as u16),
                });
            }
        }
    }

    /// The run is over: its last frame's [`Milestone::End`].
    pub fn end(&mut self, frame: u64) -> &[(u64, Milestone)] {
        if let Some(s) = self.prev.take() {
            self.settle(&s, |_| true);
            self.prev = Some(s);
        }
        if let Some(s) = &self.prev {
            self.list.push((
                frame,
                Milestone::End {
                    score: s.score,
                    lives: s.lives,
                    coins: s.coins_collected as u16,
                },
            ));
        }
        &self.list
    }
}

/// Every milestone of `frames` (frame `k` of play ending in `frames[k]`), the last frame's
/// [`Milestone::End`] among them.
pub fn milestones(frames: &[FrameState]) -> Vec<(u64, Milestone)> {
    let mut m = Milestones::default();
    for (k, s) in frames.iter().enumerate() {
        m.frame(k as u64, s.clone());
    }
    m.end(frames.len().saturating_sub(1) as u64);
    m.list
}

/// Where the two sides' milestones first part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MilestoneDiff {
    /// The frame of play it is at.
    pub frame: u64,
    /// The original's milestone the engine did not meet, or the engine's the original has not.
    pub theirs: Option<Milestone>,
    pub ours: Option<Milestone>,
    pub why: String,
}

impl fmt::Display for MilestoneDiff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "frame {} of play: {}", self.frame, self.why)
    }
}

/// Check 4's judge: the engine's milestones `ours` equal the original's `theirs`, in the same
/// order (but for milestones within `tol` frames of each other, which may come either way round),
/// each within `tol` frames of play. `Ok` with how many matched; else the earliest that differs.
pub fn judge(
    theirs: &[(u64, Milestone)],
    ours: &[(u64, Milestone)],
    tol: u64,
) -> Result<usize, MilestoneDiff> {
    let mut used = vec![false; ours.len()];
    let mut matched: Vec<(usize, usize)> = Vec::new();
    let mut diffs: Vec<MilestoneDiff> = Vec::new();
    for (i, &(ft, mt)) in theirs.iter().enumerate() {
        let k =
            (0..ours.len()).find(|&k| !used[k] && ours[k].1 == mt && ours[k].0.abs_diff(ft) <= tol);
        match k {
            Some(k) => {
                used[k] = true;
                matched.push((i, k));
            }
            None => {
                let near = ours
                    .iter()
                    .enumerate()
                    .filter(|&(k, o)| !used[k] && o.0 + tol >= ft)
                    .map(|(_, o)| *o)
                    .next();
                let found = ours.iter().find(|o| o.1 == mt);
                diffs.push(MilestoneDiff {
                    frame: ft,
                    theirs: Some(mt),
                    ours: near.map(|o| o.1),
                    why: format!(
                        "the original's {mt} has no match in the engine within {tol} frames{}{}",
                        match found {
                            Some((fo, _)) => format!(" (the engine's nearest like it: frame {fo})"),
                            None => String::new(),
                        },
                        match near {
                            Some((fo, mo)) => format!("; the engine's next: {mo} at frame {fo}"),
                            None => "; the engine has none after".into(),
                        }
                    ),
                });
            }
        }
    }
    for (k, &(fo, mo)) in ours.iter().enumerate() {
        if !used[k] {
            diffs.push(MilestoneDiff {
                frame: fo,
                theirs: None,
                ours: Some(mo),
                why: format!("the engine's {mo} has no match in the original within {tol} frames"),
            });
        }
    }
    // In the original's order: a match more than `tol` frames before an earlier one's is out of
    // order.
    let mut latest: Option<(u64, Milestone)> = None;
    for &(i, k) in &matched {
        let (fo, mo) = ours[k];
        if let Some((fl, ml)) = latest
            && fo + tol < fl
        {
            diffs.push(MilestoneDiff {
                frame: theirs[i].0,
                theirs: Some(theirs[i].1),
                ours: Some(mo),
                why: format!(
                    "out of order: the engine's {mo} at frame {fo} comes more than {tol} frames before its {ml} at frame {fl}, which the original has first"
                ),
            });
        }
        if latest.is_none_or(|(fl, _)| fo > fl) {
            latest = Some((fo, mo));
        }
    }
    match diffs.into_iter().min_by_key(|d| d.frame) {
        Some(d) => Err(d),
        None => Ok(matched.len()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sf_core::layout::LEVEL_MAP_BYTES;

    fn coins(n: u16) -> (u64, Milestone) {
        (0, Milestone::CoinTaken { left: n })
    }

    fn at(f: u64, m: Milestone) -> (u64, Milestone) {
        (f, m)
    }

    const DEATH: Milestone = Milestone::LifeLost { lives: 2 };
    const EXIT: Milestone = Milestone::LevelExit { world: 1, level: 1 };

    #[test]
    fn equal_lists_pass_and_a_shift_of_ten_frames_passes_eleven_fails() {
        let theirs = vec![at(100, DEATH), at(400, EXIT)];
        assert_eq!(judge(&theirs, &theirs, MILESTONE_FRAMES), Ok(2));
        for shift in [10, 11] {
            let ours = vec![at(100 + shift, DEATH), at(400, EXIT)];
            let r = judge(&theirs, &ours, MILESTONE_FRAMES);
            match shift {
                10 => assert_eq!(r, Ok(2)),
                _ => {
                    let d = r.unwrap_err();
                    assert_eq!(d.frame, 100);
                    assert_eq!(d.theirs, Some(DEATH));
                    let text = d.to_string();
                    assert!(
                        text.contains("frame 100") && text.contains("life lost"),
                        "{text}"
                    );
                }
            }
            let early = vec![at(100 - shift, DEATH), at(400, EXIT)];
            assert_eq!(
                judge(&theirs, &early, MILESTONE_FRAMES).is_ok(),
                shift == 10
            );
        }
    }

    #[test]
    fn a_missing_or_extra_milestone_fails_with_its_frame_and_kind() {
        let theirs = vec![at(100, DEATH), at(400, EXIT)];
        let d = judge(&theirs, &[at(400, EXIT)], MILESTONE_FRAMES).unwrap_err();
        assert_eq!((d.frame, d.theirs, d.ours), (100, Some(DEATH), Some(EXIT)));
        assert!(d.to_string().contains("frame 100 of play") && d.to_string().contains("life lost"));
        let extra = vec![at(100, DEATH), at(250, coins(3).1), at(400, EXIT)];
        let d = judge(&theirs, &extra, MILESTONE_FRAMES).unwrap_err();
        assert_eq!((d.frame, d.theirs, d.ours), (250, None, Some(coins(3).1)));
        assert!(d.to_string().contains("coin taken"), "{d}");
        // A different value is both: the original's missing, the engine's extra.
        let other = vec![at(100, Milestone::LifeLost { lives: 1 }), at(400, EXIT)];
        assert_eq!(
            judge(&theirs, &other, MILESTONE_FRAMES).unwrap_err().frame,
            100
        );
    }

    #[test]
    fn milestones_within_ten_frames_may_swap_order_further_apart_not() {
        let coin = coins(4).1;
        let theirs = vec![at(100, DEATH), at(105, coin)];
        assert_eq!(
            judge(&theirs, &[at(104, coin), at(106, DEATH)], MILESTONE_FRAMES),
            Ok(2)
        );
        // Each within ten of its own, but the engine's order the other way round by 11 frames.
        let theirs = vec![at(100, DEATH), at(109, coin)];
        let d = judge(&theirs, &[at(99, coin), at(110, DEATH)], MILESTONE_FRAMES).unwrap_err();
        assert!(d.why.contains("out of order"), "{d}");
    }

    #[test]
    fn repeats_are_matched_one_for_one() {
        let c = coins(0).1;
        let theirs = vec![at(10, c), at(10, c), at(12, c)];
        assert_eq!(
            judge(&theirs, &[at(11, c), at(12, c), at(13, c)], 10),
            Ok(3)
        );
        assert!(judge(&theirs, &[at(11, c), at(12, c)], 10).is_err());
    }

    fn level_frame() -> FrameState {
        FrameState {
            cop1lc: Some(sf_core::render::LEVEL_COPPER),
            logic_running: 1,
            world: 1,
            level: 1,
            level_state: -1,
            lives: 3,
            coins_left: 2,
            obj_slots: vec![0; SLOTS * SLOT_SIZE],
            level_map: vec![0; LEVEL_MAP_BYTES as usize],
            ..FrameState::default()
        }
    }

    /// A crumbling platform's spawn (T115, W3-3 4953 on the original): the marker under it (62)
    /// cleared in a frame ending mid-pass, still clear at the next pass's end, and written back
    /// the frame after, is no item. An item cleared and still clear at a pass's end two passes on
    /// is one, at the frame it went, and so is one still clear at the run's end.
    #[test]
    fn a_cleared_marker_is_an_item_only_if_still_clear_two_passes_on() {
        let at = |c: u32, r: u32| (2 * (r * MAP_ROW_CELLS + c)) as usize;
        let put = |s: &mut FrameState, i: usize, v: u16| {
            s.level_map[i..i + 2].copy_from_slice(&v.to_be_bytes())
        };
        let (platform, item, last) = (at(127, 47), at(30, 5), at(40, 6));
        let mut s = level_frame();
        put(&mut s, platform, 0xae80 | 62);
        put(&mut s, item, 0x3e00 | 55);
        put(&mut s, last, 0x3e00 | 55);
        let mut f = vec![s.clone()];
        // Frame 1 ends mid-pass, the platform's marker clear; frame 2 ends the pass, still clear,
        // and the item goes; frame 3 the platform's marker is back.
        s.mid_pass = true;
        put(&mut s, platform, 0xae80);
        f.push(s.clone());
        s.mid_pass = false;
        put(&mut s, item, 0x3e00);
        f.push(s.clone());
        put(&mut s, platform, 0xae80 | 62);
        f.push(s.clone());
        // Frames 4 to 7 end passes, the item's marker still clear; the last item goes in frame 7,
        // the run's last.
        for k in 4..=7 {
            if k == 7 {
                put(&mut s, last, 0x3e00);
            }
            f.push(s.clone());
        }
        let items: Vec<(u64, Milestone)> = milestones(&f)
            .into_iter()
            .filter(|(_, m)| matches!(m, Milestone::ItemTaken { .. }))
            .collect();
        assert_eq!(
            items,
            vec![
                (
                    2,
                    Milestone::ItemTaken {
                        kind: 55,
                        cell: (30, 5)
                    }
                ),
                (
                    7,
                    Milestone::ItemTaken {
                        kind: 55,
                        cell: (40, 6)
                    }
                ),
            ]
        );
    }

    /// A frame ending mid-pass settles nothing (T115 review): a marker cleared in frame 1, still
    /// clear in frame 5 (due, but mid-pass) and written back in frame 6 (the pass's end), is no
    /// item.
    #[test]
    fn a_frame_ending_mid_pass_settles_no_item() {
        let i = (2 * (47 * MAP_ROW_CELLS + 127)) as usize;
        let put =
            |s: &mut FrameState, v: u16| s.level_map[i..i + 2].copy_from_slice(&v.to_be_bytes());
        let mut s = level_frame();
        put(&mut s, 0xae80 | 62);
        let mut f = vec![s.clone()];
        put(&mut s, 0xae80);
        for k in 1..=5 {
            s.mid_pass = k == 5;
            f.push(s.clone());
        }
        s.mid_pass = false;
        put(&mut s, 0xae80 | 62);
        f.push(s.clone());
        f.push(s.clone());
        assert!(
            !milestones(&f)
                .iter()
                .any(|(_, m)| matches!(m, Milestone::ItemTaken { .. }))
        );
    }

    #[test]
    fn the_derivation_reads_coins_kills_items_deaths_and_the_exit() {
        let mut f = vec![level_frame()];
        // An enemy in slot 3 from cell (5, 2), and an item's marker 55 at (7, 1).
        let map = def(VarId::LevelMap).addr;
        let cell = |c: u32, r: u32| map + 2 * (r * MAP_ROW_CELLS + c);
        let put_l =
            |b: &mut [u8], at: usize, v: u32| b[at..at + 4].copy_from_slice(&v.to_be_bytes());
        let put_w =
            |b: &mut [u8], at: usize, v: u16| b[at..at + 2].copy_from_slice(&v.to_be_bytes());
        let s3 = 3 * SLOT_SIZE;
        put_l(&mut f[0].obj_slots, s3, sf_core::layout::h0(0x259ae));
        put_l(&mut f[0].obj_slots, s3 + 0x40, cell(5, 2));
        put_w(&mut f[0].obj_slots, s3 + 0x1c, 0x20);
        let item = (cell(7, 1) - map) as usize;
        put_w(&mut f[0].level_map, item, 0x3e00 | 55);
        let enemy = (cell(5, 2) - map) as usize;
        put_w(&mut f[0].level_map, enemy, 0x0400 | 1);
        // Frame 1: a coin; frame 2: the kill (its marker cleared too) and the item.
        let mut s = f[0].clone();
        s.coins_collected = 1;
        s.coins_left = 1;
        f.push(s.clone());
        put_w(&mut s.obj_slots, s3 + 0x1c, 0xffe0);
        put_w(&mut s.level_map, enemy, 0x0400);
        put_w(&mut s.level_map, item, 0x3e00);
        f.push(s.clone());
        // Frame 3: a death; frame 4: the exit.
        s.lives = 2;
        f.push(s.clone());
        s.level_state = 0;
        f.push(s.clone());
        assert_eq!(
            milestones(&f),
            vec![
                (0, Milestone::Scene(SceneKind::Level { world: 1, level: 1 })),
                (1, Milestone::CoinTaken { left: 1 }),
                (2, Milestone::EnemyKilled { kind: 0x259ae }),
                (
                    2,
                    Milestone::ItemTaken {
                        kind: 55,
                        cell: (7, 1)
                    }
                ),
                (3, Milestone::LifeLost { lives: 2 }),
                (4, Milestone::LevelExit { world: 1, level: 1 }),
                (
                    4,
                    Milestone::End {
                        score: 0,
                        lives: 2,
                        coins: 1
                    }
                ),
            ]
        );
    }

    #[test]
    fn a_death_ends_the_level_without_an_exit() {
        let mut f = vec![level_frame()];
        let mut s = f[0].clone();
        s.level_state = 0x190;
        s.dying = 1;
        s.lives = 2;
        f.push(s);
        assert_eq!(
            milestones(&f)[1..],
            [
                (1, Milestone::LifeLost { lives: 2 }),
                (1, Milestone::End { score: 0, lives: 2, coins: 0 }),
            ]
        );
    }

    #[test]
    fn a_blank_copper_list_keeps_the_scene_and_the_tally_says_its_score() {
        let mut f = vec![level_frame()];
        let mut s = f[0].clone();
        s.cop1lc = Some(BLANK_COPPER);
        s.logic_running = 0;
        f.push(s.clone());
        s.cop1lc = Some(TALLY_COPPER);
        s.score = 1234;
        f.push(s.clone());
        s.cop1lc = Some(h2(0x1000));
        f.push(s);
        let m = milestones(&f);
        assert_eq!(
            m[1..4],
            [
                (2, Milestone::Scene(SceneKind::Tally)),
                (2, Milestone::Tally { score: 1234 }),
                (3, Milestone::Scene(SceneKind::Other(h2(0x1000)))),
            ]
        );
    }
}
