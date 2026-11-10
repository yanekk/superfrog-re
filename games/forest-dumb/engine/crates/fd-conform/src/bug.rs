//! A bug zip replayed headlessly (DESIGN §2.8, §2.12): the recording read out of the zip, refused
//! when another build made it, the engine started at the level's start as the app starts it
//! (`Game::new(data, Start::Level1)`), stepped through its inputs unpaced as the app plays them
//! (one `Game::step` a frame), and its last picture held against the owner's screenshot. A
//! mismatch is a result, not an error: on a fixed engine it is the point. `fd-conform bug replay`
//! (main.rs) is the CLI.
//!
//! [`compare`] plays the recording on the original from the level sync point, its inputs retimed
//! onto the original's frames ([`retime`]), and judges the owner's game against it with the level
//! case runner (`run_inputs`). `fd-conform bug compare` is its CLI; the procedure is
//! `games/forest-dumb/docs/BUG-FIX.md`.
//!
//! **Retimed by pass, not by frame** (T15). The app runs one `Game::step` a frame, and each step is
//! one pass of `game_frame`, which reads the joystick once (`read_joystick`, docs/09). The original
//! runs a pass a vertical blank, but its pass often ends after the frame does, and its read falls
//! in the frame after its start on scrolling passes (T41, T67). So the recording's step `k` is the
//! original's pass `k + 2` (the sync point has run pass 1, `Game::new`'s `irq_count`), and each of
//! the original's frames is fed the joystick of the pass that reads next and the keys of the pass
//! that starts next ([`crate::PassInputs`]). That holds the original to the recording pass for
//! pass, except where two passes read in one frame: they read one joystick, and where the
//! recording held two, the second pass reads the first's ([`Lagged`]). The engine side plays the
//! recording's own inputs, so it is the owner's game, and a lagged pass is reported, not hidden.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use amiga_chips::Picture;
use amiga_conform::bug_compare::{Left, verdict_line};
use amiga_conform::script::InputScript;
use amiga_oracle::{FrameInput, Rgb12Frame};
use fd_core::level::EdgesCut;
use fd_core::recording::{ForestDumb, Location, Recording, ZIP_SCREENSHOT, check_build};
use fd_core::{Event, Game, GameData, Mem, Start, Transplant, Var, layout_var};

use crate::case::{Case, Compare, Exclusion};
use crate::{
    Engine, Inputs, Options, OracleMem, Outcome, PASS_END, PLAY_WINDOW, PassInputs, Verdict, sync,
};

pub use amiga_conform::screenshot::{frame_png, picture_png, screenshot_differs};

/// The recording and the screenshot's PNG bytes of a bug zip (`amiga_record::read_zip`), refused
/// unless `build` made it and the screenshot is a PNG: here, before a replay that may run minutes.
pub fn open_zip(bytes: &[u8], build: &str) -> Result<(Recording, Vec<u8>), String> {
    let (rec, png) = amiga_record::read_zip::<ForestDumb>(bytes)?;
    amiga_conform::screenshot::decode_png(&png)
        .map_err(|e| format!("{ZIP_SCREENSHOT}: it is not a PNG: {e}"))?;
    check_build(&rec, build)?;
    Ok((rec, png))
}

/// A recording played to its end.
pub struct Replayed {
    pub game: Game,
    /// [`Game::location`] after the last step.
    pub location: Location,
    /// The last step's picture (the level's first frame for a recording of no frames): what the
    /// app showed when Stop was pressed.
    pub last: Picture,
}

/// `rec` played as the app played it, from the level's start. `each` sees every step's frame
/// number (0 the first input's), the game after it and its picture.
pub fn replay(
    data: Arc<GameData>,
    rec: &Recording,
    each: impl FnMut(u32, &Game, &Picture),
) -> Replayed {
    // The only level a recording names today; the reader refuses any other (`LevelId`).
    let mut game = Game::new(data, Start::Level1);
    let last = amiga_record::replay(&mut game, &rec.inputs, each);
    Replayed {
        location: game.location(),
        game,
        last,
    }
}

/// `irq_count` in a new game: the sync point has run pass 1, so the recording's step `k` is pass
/// `k + 2` (`State::level_1`).
const START_PASS: u32 = 1;

/// Frames the original may run without a pass reading the joystick before it is taken to have
/// left play: `main_loop` acts on game over and the level's end within 32 passes (docs/09), and a
/// pass in play reads within two frames of its start.
const STALL: u32 = 64;

/// Frames the original is given past twice the recording's steps.
const SLACK: u32 = 2_000;

/// What a bug compare does not judge yet, each with the task that makes it judged (DESIGN §2.2: no
/// gap is silent). None since T75: the creature slots, CLXDAT as `char_hit_check` copies it and the
/// `hits_taken` it counts are compared, the runner handing the engine each edge redraw the next
/// pass cut short, where the original starts a creature a pass later (docs/10). A gap added here
/// is read and counted, and printed in the report.
fn known_gaps() -> Vec<Exclusion> {
    Vec::new()
}

/// The variables a bug compare holds at every pass end: every play variable in `LAYOUT` less the
/// known gaps ([`known_gaps`]). `mt_counter` is the title's.
fn compared_vars() -> Vec<String> {
    let gaps = known_gaps();
    fd_core::LAYOUT
        .iter()
        .map(|v| v.name)
        .filter(|&n| n != "mt_counter" && !gaps.iter().any(|g| g.var.as_deref() == Some(n)))
        .map(str::to_string)
        .collect()
}

/// A pass whose joystick on the original is not the one the recording holds for it: it read in
/// the same frame as the pass before, which got that frame's input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lagged {
    /// The original's frame it read in.
    pub frame: u32,
    /// The recording's step it is.
    pub step: u32,
    pub recorded: FrameInput,
    pub read: FrameInput,
}

/// A recording's inputs placed on the original's frames ([`retime`]).
#[derive(Debug, Clone)]
pub struct Retimed {
    /// What the original is fed, frames counted from the level sync point.
    pub script: InputScript,
    /// The original's frames judged: up to the pass end of the recording's last step, or up to the
    /// last pass end in play before it left ([`Retimed::left`]).
    pub frames: u32,
    /// The recording's steps the original's passes read over [`Retimed::frames`].
    pub passes: u32,
    pub lagged: Vec<Lagged>,
    /// Where the original left play before the recording's end: nothing after it is judged.
    pub left: Option<Left>,
    /// The original's picture where the retiming ended: at Stop, or where it left.
    pub stop: Rgb12Frame,
    /// `irq_count` at the first pass end after the sync point: the case runner's pass 0.
    pub first_end: u32,
    /// The original's `char_x` and `char_y` at the last pass end judged: a bug case's goal.
    pub end: (u32, u32),
}

impl Retimed {
    /// The recording's step that pass `pass` of the case runner (counted from
    /// [`Retimed::first_end`]) is.
    pub fn step_of_pass(&self, pass: u32) -> u32 {
        (pass + self.first_end).saturating_sub(START_PASS + 1)
    }
}

fn var(name: &str) -> Result<&'static Var, String> {
    layout_var(name).ok_or_else(|| format!("no {name} in LAYOUT"))
}

/// `rec` placed on the original's frames from the level sync point (the module's note): run on
/// the original until the pass of its last step has ended, or until the original leaves play (a
/// pass that read no input, `world_index` past W1-1, or [`STALL`] frames with no read).
pub fn retime(rec: &Recording) -> Result<Retimed, String> {
    let steps = rec.inputs.len() as u32;
    let mut am = sync::reach_level_1(2 * steps + SLACK)?;
    let mut t = PassInputs::anchor(&am)?;
    if t.next_start() != START_PASS + 1 {
        return Err(format!(
            "the sync point's irq_count is {}, a new game's {START_PASS}",
            t.next_start() - 1
        ));
    }
    am.watch_pcs(&t.pcs());
    am.watch_write(Some(PASS_END));
    let (irq, world) = (var("irq_count")?, var("world_index")?);
    let first_pass = START_PASS + 1;
    let last_pass = START_PASS + steps;
    let of = |pass: u32| -> Option<FrameInput> {
        pass.checked_sub(first_pass)
            .and_then(|k| rec.inputs.get(k as usize).copied())
    };
    let mut fed: Vec<FrameInput> = Vec::new();
    let mut lagged = Vec::new();
    let mut taken = START_PASS;
    let mut first_end = None;
    // Frames fed up to the last pass end seen in play: what is judged if the original leaves.
    let mut judged = 0usize;
    let mut since_read = 0u32;
    let mut left = None;
    let mut end = (0, 0);
    let (char_x, char_y) = (var("char_x")?, var("char_y")?);
    let out_of_play = |frame: u32, pass: u32, what: String| {
        Some(Left {
            frame,
            step: pass.saturating_sub(first_pass).min(steps.saturating_sub(1)),
            what,
        })
    };
    while steps > 0 {
        let input = FrameInput {
            joy: of(t.next_read()).unwrap_or(FrameInput::NONE).joy,
            keys: of(t.next_start()).unwrap_or(FrameInput::NONE).keys,
        };
        am.step(input).map_err(|e| e.to_string())?;
        fed.push(input);
        let frame = am.frame();
        if let Err(e) = t.step(&am, input) {
            // The pairing of passes and inputs no longer holds: judged up to the last pass end.
            left = out_of_play(
                frame,
                taken + 1,
                format!("the original's passes could no longer be paired with inputs: {e}"),
            );
            break;
        }
        let mut read = false;
        while let Some(got) = t.take(taken + 1) {
            taken += 1;
            read = true;
            if let Some(want) = of(taken).filter(|&w| w != got) {
                lagged.push(Lagged {
                    frame,
                    step: taken - first_pass,
                    recorded: want,
                    read: got,
                });
            }
        }
        since_read = if read { 0 } else { since_read + 1 };
        if let Some(mem) = am.watched_memory() {
            let m = OracleMem(&mem);
            let ended = m.value(irq);
            first_end.get_or_insert(ended);
            if ended > taken {
                left = out_of_play(
                    frame,
                    taken + 1,
                    format!(
                        "pass {ended} of the original ended without reading the joystick: it left \
                         play (game over, the level's end or a load)"
                    ),
                );
                break;
            }
            if m.value(world) != 0 {
                left = out_of_play(
                    frame,
                    ended,
                    format!(
                        "world_index is {} on the original: the level ended there",
                        m.value(world)
                    ),
                );
                break;
            }
            judged = fed.len();
            end = (m.value(char_x), m.value(char_y));
            if ended >= last_pass {
                break;
            }
        }
        if since_read > STALL {
            left = out_of_play(
                frame,
                taken + 1,
                format!(
                    "no pass of the original read the joystick in {STALL} frames: it left play"
                ),
            );
            break;
        }
    }
    if left.is_none() {
        judged = fed.len();
    }
    let passes = taken.min(last_pass).saturating_sub(START_PASS);
    Ok(Retimed {
        script: InputScript::from_steps(&fed[..judged]),
        frames: judged as u32,
        passes,
        lagged,
        left,
        stop: am.picture_in(PLAY_WINDOW.0, PLAY_WINDOW.1),
        first_end: first_end.unwrap_or(START_PASS + 1),
        end,
    })
}

/// The owner's game as the case runner drives it: `Game::new`, as the app starts it, stepped
/// through the recording's own inputs, one step a pass, never the inputs the original read. It is
/// not taken from the original: `Game::new` equals the original at the runner's first pass end
/// (T69), and where it does not, what differs there is kept for the report and judged from then on.
/// `before_step(step, game)` runs before each step (a test plants a mutation there).
struct Owners<'a> {
    game: Game,
    inputs: &'a [FrameInput],
    done: u32,
    before_step: &'a mut dyn FnMut(u32, &mut Game),
    /// (variable, the owner's game's value, the original's) at the first pass end.
    start_differs: Vec<(&'static str, u32, u32)>,
}

impl Owners<'_> {
    fn one(&mut self) {
        let k = self.done;
        (self.before_step)(k, &mut self.game);
        let input = self
            .inputs
            .get(k as usize)
            .copied()
            .unwrap_or(FrameInput::NONE);
        self.game.step(input);
        self.done += 1;
    }
}

impl Transplant for Owners<'_> {
    fn transplant(&mut self, mem: &dyn Mem) {
        self.start_differs = fd_core::LAYOUT
            .iter()
            .filter(|v| v.name != "mt_counter")
            .map(|v| (v.name, self.game.probe(v), mem.value(v)))
            .filter(|&(_, ours, theirs)| ours != theirs)
            .collect();
        // No transplant: the owner's game keeps its own start. Should the runner's first end be
        // past pass 1 (two passes ended in one step), the game catches up on the recording's inputs.
        let ended = layout_var("irq_count").map_or(START_PASS, |v| mem.value(v));
        while self.done < ended.saturating_sub(START_PASS) {
            self.one();
        }
    }

    fn probe(&self, v: &Var) -> u32 {
        self.game.probe(v)
    }
}

impl Engine for Owners<'_> {
    fn pass(&mut self, _original_read: FrameInput) {
        self.one();
    }

    fn picture(&self) -> Option<Picture> {
        Some(self.game.picture())
    }

    fn events(&mut self) -> Vec<Event> {
        self.game.take_events()
    }

    /// Without it the owner's game keeps its own rule and has a creature a pass before the
    /// original wherever a pass cut the redraw that starts it (T75: `enemy-hop`'s inputs, step 92).
    fn foreground_cut(&mut self, at: Option<EdgesCut>) {
        self.game.cut_foreground(at);
    }
}

/// A recording compared with the original ([`compare`]).
pub struct Compared {
    pub retimed: Retimed,
    /// The case runner's verdict; `None` when nothing was there to judge (no pass ended in play).
    pub verdict: Option<Verdict>,
    /// The engine's picture at Stop (the recording's last step).
    pub engine_stop: Picture,
    /// (variable, `Game::new`'s value, the original's) where the engine's own start differs from
    /// the original at the first pass end; empty since T69.
    pub start_differs: Vec<(&'static str, u32, u32)>,
}

impl Compared {
    /// The first difference, as (the original's frame, the recording's step, what differs).
    pub fn first(&self) -> Option<(u32, u32, String)> {
        let r = &self.retimed;
        match &self.verdict.as_ref()?.outcome {
            Outcome::Differs {
                pass,
                frame,
                var,
                engine,
                oracle,
            } => Some((
                *frame,
                r.step_of_pass(*pass),
                format!("{var} is {engine:#x} in the engine, {oracle:#x} in the original"),
            )),
            Outcome::PictureDiffers { pass, frame, why } => {
                Some((*frame, r.step_of_pass(*pass), format!("picture: {why}")))
            }
            Outcome::Unported { pass, frame, .. } => Some((
                *frame,
                r.step_of_pass(*pass),
                self.verdict.as_ref()?.outcome.to_string(),
            )),
            // No goal is set and an unused known gap is no difference.
            _ => None,
        }
    }

    /// Where judging ended before the recording's end: where the original left play, or, when
    /// nothing was judged, at the sync point.
    pub fn left(&self) -> Option<Left> {
        if let Some(l) = &self.retimed.left {
            return Some(l.clone());
        }
        match self.verdict.as_ref().map(|v| &v.outcome) {
            None => Some(Left {
                frame: sync::LEVEL_1_SYNC,
                step: 0,
                what: "the recording has no pass to compare".to_string(),
            }),
            Some(Outcome::ComparedNothing { what }) => Some(Left {
                frame: sync::LEVEL_1_SYNC,
                step: 0,
                what: format!("compared nothing: {what}"),
            }),
            _ => None,
        }
    }

    /// No difference, and judged to the recording's end.
    pub fn same(&self) -> bool {
        self.first().is_none() && self.left().is_none()
    }

    /// What `bug compare` prints: the verdict line first (`amiga_conform::bug_compare`), then
    /// the lag, where judging ended, what was compared and the known gaps.
    pub fn report(&self) -> String {
        let r = &self.retimed;
        let first = self.first();
        let left = self.left();
        let mut out = verdict_line(
            first.as_ref().map(|(f, s, w)| (*f, *s, w.as_str())),
            left.as_ref(),
        );
        out += "\n";
        match r.lagged.first() {
            None => {
                out += &format!(
                    "the original read the recording's input on each of its {} passes\n",
                    r.passes
                )
            }
            Some(l) => {
                out += &format!(
                    "lag: on {} of {} passes the original read other input than the recording's (two \
                     passes read in one frame), first at frame {} (engine step {}): recorded {}, read \
                     {}; a difference from there on may be the lag's, not the engine's\n",
                    r.lagged.len(),
                    r.passes,
                    l.frame,
                    l.step,
                    InputScript::format_input(l.recorded),
                    InputScript::format_input(l.read),
                )
            }
        }
        out += &format!(
            "the original ran {} frames for {} passes from the sync point\n",
            r.frames, r.passes
        );
        if self.verdict.is_some() {
            out += &match self.start_differs.as_slice() {
                [] => "the engine's own start equals the original's at the first pass end\n"
                    .to_string(),
                d => {
                    let vars: Vec<String> = d
                        .iter()
                        .map(|(n, ours, theirs)| format!("{n} {ours:#x} (original {theirs:#x})"))
                        .collect();
                    format!(
                        "the engine's own start differs from the original's at the first pass end in \
                         {}: {}; a difference below may come from the start, not the recording\n",
                        d.len(),
                        vars.join(", ")
                    )
                }
            };
        }
        match &left {
            None => out += "the original played the recording to its end\n",
            Some(l) => {
                out += &format!(
                    "judging ended at frame {} (engine step {}): {}; nothing after it was judged\n",
                    l.frame, l.step, l.what
                )
            }
        }
        if let Some(v) = &self.verdict {
            out += &format!(
                "compared: {} pass ends ({} unseen), {} pictures; tolerated {} colour steps, {} few \
                 pixels, {} glitches\n",
                v.passes, v.unseen, v.pictures, v.tolerated[0], v.tolerated[1], v.tolerated[2]
            );
            for (e, n) in &v.excluded {
                out += &format!(
                    "not judged yet: {} ({}), differed on {n} {}\n",
                    e.what,
                    e.task,
                    if e.var.is_some() { "passes" } else { "frames" }
                );
            }
            for (pass, e) in &v.events {
                out += &format!("event at engine step {}: {e:?}\n", r.step_of_pass(*pass));
            }
        }
        out
    }
}

/// `rec` compared with the original: retimed ([`retime`]), then run as a level case from the sync
/// point, every play variable at each pass end and the picture each frame within the
/// owner's tolerance, less the known gaps. A picture difference's PNG goes to `diff_dir`.
pub fn compare(
    data: Arc<GameData>,
    rec: &Recording,
    diff_dir: Option<PathBuf>,
) -> Result<Compared, String> {
    compare_with(data, rec, diff_dir, |_, _| {})
}

/// [`compare`], `before_step(step, game)` called before the owner's game runs each step.
pub fn compare_with(
    data: Arc<GameData>,
    rec: &Recording,
    diff_dir: Option<PathBuf>,
    mut before_step: impl FnMut(u32, &mut Game),
) -> Result<Compared, String> {
    let retimed = retime(rec)?;
    let engine_stop = replay(data.clone(), rec, |_, _, _| {}).last;
    let case = Case {
        name: "bug-compare".to_string(),
        script: sync::LEVEL_1_SCRIPT.to_string(),
        sync_frame: sync::LEVEL_1_SYNC,
        swap: None,
        frames: retimed.frames,
        input: None,
        compare: Compare {
            vars: true,
            picture: true,
        },
        vars: compared_vars(),
        goal: None,
        exclude: known_gaps(),
        set: Vec::new(),
    };
    let mut start_differs = Vec::new();
    let verdict = if retimed.frames == 0 {
        None
    } else {
        let inputs = Inputs::with_after(&case, retimed.script.clone())?;
        let mut owners = Owners {
            game: Game::new(data, Start::Level1),
            inputs: &rec.inputs,
            done: 0,
            before_step: &mut before_step,
            start_differs: Vec::new(),
        };
        let opts = Options {
            diff_dir,
            ..Options::default()
        };
        let v = crate::run_inputs(&case, &inputs, &mut owners, &opts)?;
        start_differs = owners.start_differs;
        Some(v)
    };
    Ok(Compared {
        retimed,
        verdict,
        engine_stop,
        start_differs,
    })
}

impl Compared {
    /// The level case that runs what this compare judged (`conform/cases/<name>.toml`, its
    /// `input` the script `<name>`, which is [`Retimed::script`]): every compared
    /// variable and the picture, the known gaps that differed here excluded (one that hid nothing
    /// would fail the case), its goal the original character's place at the last pass end judged. The engine side is fd-core's `Game` transplanted at the first pass
    /// end, as in the compare, but stepped on the inputs the original read: the same as the
    /// recording's except at [`Retimed::lagged`] passes.
    pub fn case_text(&self, name: &str, note: &str) -> String {
        let quoted = |v: &[String]| {
            v.iter()
                .map(|n| format!("\"{n}\""))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let gaps: Vec<String> = self
            .verdict
            .iter()
            .flat_map(|v| &v.excluded)
            .filter(|(_, n)| *n > 0)
            .map(|(e, _)| {
                let what = e.what.split(" (").next().unwrap_or(&e.what);
                format!("{{ what = \"{what}\", task = \"{}\" }}", e.task)
            })
            .collect();
        let mut text = format!(
            "# {note}\n\
             # From a bug zip (`fd-conform bug compare`, games/forest-dumb/docs/BUG-FIX.md): the\n\
             # recording's inputs on the original's frames from the level sync point.\n\
             sync = {{ script = \"{}\", frame = {} }}\n\
             input = \"{name}\"\n\
             frames = {}\n\
             compare = [\"vars\", \"picture\"]\n\
             vars = [{}]\n\
             # Where the original's character is at the last pass end judged.\n\
             goal = {{ all = [{{ var = \"char_x\", value = {:#x} }}, {{ var = \"char_y\", value = {:#x} }}], \
             by_frame = {} }}\n",
            sync::LEVEL_1_SCRIPT,
            sync::LEVEL_1_SYNC,
            self.retimed.frames,
            quoted(&compared_vars()),
            self.retimed.end.0,
            self.retimed.end.1,
            sync::LEVEL_1_SYNC + self.retimed.frames,
        );
        if !gaps.is_empty() {
            text += &format!("exclude = [{}]\n", gaps.join(", "));
        }
        text
    }
}

/// Where `bug compare` writes its pictures for `zip` unless told: the game's `build/bugs/<zip
/// stem>/`, git-ignored, because the pictures are disk-derived.
pub fn default_out(zip: &Path) -> Result<PathBuf, String> {
    let stem = zip
        .file_stem()
        .ok_or_else(|| format!("{}: no file name to name the folder by", zip.display()))?;
    Ok(crate::conform_dir().join("../../build/bugs").join(stem))
}

#[cfg(test)]
mod tests {
    use super::*;
    use fd_core::recording::{LevelId, recorder};

    /// BUG-FIX.md runs `cargo run -p fd-conform -- bug ...`; with `fd-judge` beside it cargo
    /// refuses to pick a binary unless the manifest names one.
    #[test]
    fn cargo_run_picks_the_runner() {
        assert!(include_str!("../Cargo.toml").contains("default-run = \"fd-conform\""));
    }

    #[test]
    fn the_bug_compare_judges_every_play_variable_but_the_gaps() {
        let vars = compared_vars();
        for v in [
            "char_x",
            "camera_x",
            "lives",
            "score",
            "world_index",
            "irq_count",
        ] {
            assert!(vars.iter().any(|n| n == v), "{v}");
        }
        for v in [
            "countdown",
            "give_up",
            "paused",
            "countdown_tick",
            "esc_down",
        ] {
            assert!(vars.iter().any(|n| n == v), "{v}");
        }
        // The creatures and contact, judged since T75.
        for v in ["hits_taken", "clxdat_copy", "creature_slots+$0"] {
            assert!(vars.iter().any(|n| n == v), "{v}");
        }
        assert!(!vars.iter().any(|n| n == "mt_counter"));
        // Every gap names its task.
        assert!(known_gaps().iter().all(|g| g.task.starts_with('T')));
    }

    fn zip(build: &str, png: &[u8]) -> Vec<u8> {
        let rec = recorder(LevelId::W1_1, build)
            .finish(
                Location {
                    level: LevelId::W1_1,
                    col: 7,
                    row: 7,
                },
                String::new(),
            )
            .unwrap();
        amiga_record::zip_bytes(&rec, png).unwrap()
    }

    #[test]
    fn a_zip_is_refused_naming_why() {
        let png = amiga_app_png();
        assert!(open_zip(&zip("b", &png), "b").is_ok());
        let e = open_zip(&zip("a", &png), "b").unwrap_err();
        assert!(e.contains("`a`") && e.contains("`b`"), "{e}");
        let e = open_zip(&zip("b", b"not a png"), "b").unwrap_err();
        assert!(e.starts_with("screenshot.png: it is not a PNG"), "{e}");
        assert!(
            open_zip(b"hello", "b")
                .unwrap_err()
                .starts_with("not a bug zip")
        );
    }

    /// A 1x1 PNG, as the conformance harness writes one.
    fn amiga_app_png() -> Vec<u8> {
        amiga_conform::report::png(1, 1, &[0, 0, 0])
    }
}
