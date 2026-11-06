//! A bug zip replayed headlessly (bug-recorder DESIGN §2.5, §2.7): the recording read out of the
//! zip, the engine started as it says (`Game::start_level`), stepped through its inputs unpaced as
//! the app plays them (`Game::step`, one a frame), and its last picture and place held against the
//! owner's screenshot and `stopped-at`. A mismatch is a result, not an error: on a fixed engine it
//! is the point. `sf-conform bug replay` (main.rs) is the CLI.

use sf_core::input::FrameInput;
use sf_core::layout::{AmigaMemory, Ram, VarId, def};
use sf_core::level::objects::{WIDE_BASE, WIDE_BYTES};
use sf_core::level::screen::WIDE_EXTRA;
use sf_core::recording::{Location, Recording, RecordingRng, Superfrog, ZIP_SCREENSHOT};
use sf_core::{Game, LoadEnd, Picture, Scene, View};
use sf_oracle::{Amiga, Rgb12Frame};

pub use amiga_conform::screenshot::{frame_png, picture_png, picture_rgb, screenshot_differs};

use crate::sync::{self, Synced};
use crate::{Case, Divergence, InputScript, Mode, Outcome, SyncPoint};

/// The recording and the screenshot's PNG bytes of a bug zip (`amiga_record::read_zip`), the
/// screenshot also refused when it is not a PNG: here, not after a replay that may run minutes. A
/// recording the parser or `validate` refuses is refused, a level no game starts at included,
/// which `Game::start_level` could not reach.
pub fn open_zip(bytes: &[u8]) -> Result<(Recording, Vec<u8>), String> {
    let (rec, png) = amiga_record::read_zip::<Superfrog>(bytes)?;
    amiga_conform::screenshot::decode_png(&png)
        .map_err(|e| format!("{ZIP_SCREENSHOT}: it is not a PNG: {e}"))?;
    Ok((rec, png))
}

/// What `bug compare` and `bug case` say to a recording with any widescreen in it (real-widescreen
/// DESIGN §2.8, D9): a widescreen game is ours, not the original's.
pub const WIDESCREEN_REFUSED: &str =
    "widescreen recording: there is no original to compare a widescreen game with (games/superfrog/docs/BUG-FIX.md)";

/// `Err(`[`WIDESCREEN_REFUSED`]`)` when any of `rec` was played in widescreen: there is no original
/// to compare it with, so neither the oracle is started nor a case written.
pub fn comparable(rec: &Recording) -> Result<(), String> {
    match rec.has_widescreen() {
        true => Err(WIDESCREEN_REFUSED.to_string()),
        false => Ok(()),
    }
}

/// The game set to widescreen `on` and drawing its view, the app's: the wide game draws its wide
/// picture (real-widescreen DESIGN §2.7, §2.8).
pub fn set_widescreen(game: &mut Game, on: bool) {
    game.set_widescreen(on);
    game.set_view(View {
        extra_columns_each_side: if on { WIDE_EXTRA } else { 0 },
    });
}

/// A recording played to its end.
pub struct Replayed {
    pub game: Game,
    /// [`Game::location`] after the last step.
    pub location: Location,
    /// The last step's picture (the level's first frame for a recording of no frames): what the app
    /// showed when STOP was pressed.
    pub last: Picture,
}

/// `rec` played as the app played it: [`Game::start_level`] with its setup and seed, then one
/// [`Game::step`] per recorded input. `each` sees every step's frame number (0 the first input's),
/// the game after it and the picture it drew, which is not always `Game::render`'s (a level frame
/// whose pass ends mid-frame shows the picture drawn then). The screenshot is the core's picture
/// at 1× (DESIGN §2.5): 4:3, and the wide picture while the recording is in widescreen, which is
/// set at frame 0 and switched before the step of each toggle (real-widescreen DESIGN §2.8).
pub fn replay(rec: &Recording, mut each: impl FnMut(u32, &Game, &Picture)) -> Replayed {
    let mut game = Game::start_level(crate::display_data(), &rec.setup, rec.rng());
    if rec.widescreen {
        set_widescreen(&mut game, true);
    }
    let mut play = Play { game, rec };
    let last = amiga_record::replay(&mut play, &rec.inputs, |k, p, pic| each(k, &p.game, pic));
    let game = play.game;
    Replayed {
        location: game.location(),
        game,
        last,
    }
}

/// A recording's game as the replay loop steps it: widescreen switched before the step of each
/// toggle.
struct Play<'a> {
    game: Game,
    rec: &'a Recording,
}

impl amiga_record::Replayable for Play<'_> {
    fn picture(&self) -> Picture {
        self.game.render(self.game.view())
    }

    fn step(&mut self, frame: u32, input: FrameInput) -> Picture {
        let wide = self.rec.widescreen_at(frame);
        if wide != self.game.widescreen() {
            set_widescreen(&mut self.game, wide);
        }
        self.game.step(input).picture
    }
}

/// Bytes written into memory at frame 0 on every side: the recording's engine, the original and
/// the runner's engine (a test's hook: W1-1's exit forced, as `chain-w1-1-to-w1-2`'s sync point
/// forces it). A recording made in the app has none.
pub type Pokes = [(u32, Vec<u8>)];

/// What the original's run is held to the recording by (T00): the frog, the score, the lives, the
/// hearts and the level.
const WATCHED: [VarId; 7] = [
    VarId::FrogX,
    VarId::FrogY,
    VarId::Score,
    VarId::FrogLives,
    VarId::FrogHearts,
    VarId::World,
    VarId::Level,
];

/// The legs a recording's comparison is judged in, in the original's passes (T00: 8 holds on
/// W1-1 and W4-3 under lag, unpaced; the engine is paced within a leg here).
pub const LEG_PASSES: u64 = 8;

/// The most frames the original is held at an anchor (a level's first frame, the slot machine's
/// entry) the recording's engine has passed before the original is taken to have left the
/// recording.
const ANCHOR_HOLD: u32 = 5_000;
/// The furthest ahead, in steps, an anchor of the recording's is matched to the original's: past
/// it the original's level start is one the recording does not have (a restart after a death the
/// owner did not die).
const ANCHOR_AHEAD: u32 = 1_000;

/// The most frames the original may spend on one load the engine did at once (the runner's).
const LOAD_SEARCH: u32 = 20_000;

/// The recording played as the app played it, what retiming needs of each step: whether the
/// engine loaded at once on it, the scene after it and the watched state after it.
struct Played {
    load: Vec<Option<LoadEnd>>,
    scene: Vec<Scene>,
    watched: Vec<[u32; WATCHED.len()]>,
    /// The engine at the recording's start: the runner's engine, and the COP1LC retiming starts
    /// from.
    start: Game,
    /// The last step's picture: the engine's at STOP.
    last: Picture,
}

fn watched_engine(g: &Game) -> [u32; WATCHED.len()] {
    WATCHED.map(|id| {
        let d = def(id);
        g.peek(d.addr, d.width.len().min(4))
            .iter()
            .fold(0, |v, &b| v << 8 | b as u32)
    })
}

fn watched_original(m: &AmigaMemory) -> [u32; WATCHED.len()] {
    WATCHED.map(|id| def(id).read(m as &dyn Ram))
}

fn play(rec: &Recording, pokes: &Pokes) -> Played {
    let mut game = Game::start_level(crate::display_data(), &rec.setup, rec.rng());
    for (addr, bytes) in pokes {
        game.poke(*addr, bytes);
    }
    let start = game.clone();
    let mut p = Played {
        load: Vec::with_capacity(rec.inputs.len()),
        scene: Vec::with_capacity(rec.inputs.len()),
        watched: Vec::with_capacity(rec.inputs.len()),
        last: game.render(View::default()),
        start,
    };
    for &input in &rec.inputs {
        let out = game.step(input);
        p.load.push(out.load);
        p.scene.push(game.scene());
        p.watched.push(watched_engine(&game));
        p.last = out.picture;
    }
    p
}

/// Where the original's run left the recording's (T00): under lag the original does not play
/// what the owner played, and a bug seen after this frame may not occur on it. `frame` counts the
/// original's frames from the start; `step` is the recording's step it was fed. The toolkit's, as
/// Forest Dumb's compare reports it too.
pub use amiga_conform::bug_compare::Left;

/// Where the original was put back on the recording (bug-recorder T14): it had left it (`what`)
/// by the pass end its frame `frame` ended, and the recording's engine after step `step` was
/// written into it after that frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PutBack {
    /// The original's frame, counted from the start.
    pub frame: u32,
    pub step: u32,
    pub what: String,
}

/// The recording's engine, stepped as far as asked (bug-recorder T14): what the original is put
/// back to. Its modelled state is the owner's game at that step.
pub(crate) struct Follow {
    game: Game,
    /// Steps done.
    done: u32,
}

impl Follow {
    pub fn new(setup: &sf_core::recording::LevelSetup, seed: sf_core::Rng, pokes: &Pokes) -> Follow {
        let mut game = Game::start_level(crate::display_data(), setup, seed);
        for (addr, bytes) in pokes {
            game.poke(*addr, bytes);
        }
        Follow { game, done: 0 }
    }

    /// The engine after step `step`, `input(k)` step k's input (steps are only ever asked for in
    /// order).
    pub fn after(&mut self, step: u32, input: impl Fn(u32) -> FrameInput) -> &Game {
        while self.done <= step {
            self.game.step(input(self.done));
            self.done += 1;
        }
        &self.game
    }

    /// What the original is put back with: every modelled variable as the engine holds it. The
    /// CPU's place, its stack and the custom chips are not among them; at a pass end in a level
    /// the original's CPU is in `level_frame_loop`'s wait, as the engine's would be, and that
    /// holds (T14: W1-1 played to W1-2 stays on the recording after it).
    pub fn writes(&self) -> Vec<(u32, Vec<u8>)> {
        self.game
            .layout()
            .iter()
            // The wide draw lists are the engine's own (`engine_only`), outside the original's
            // memory: vAmiga refuses a write there.
            .filter(|d| !sf_core::layout::engine_only(d.id))
            // The extra slots' range (`WIDE_BASE`) is outside it too, and 0 in a 4:3 game, as the
            // original reads an address it never maps.
            .filter(|d| !(WIDE_BASE..WIDE_BASE + WIDE_BYTES).contains(&d.addr))
            .map(|d| (d.addr, self.game.peek(d.addr, d.width.len())))
            .collect()
    }
}

/// How many of the original's pass ends a put-back waits for the recording's engine to be at a
/// pass end too, before the original is taken to have left for good.
const PUT_BACK_WAIT: u32 = 8;

/// A recording's inputs placed on the original's frames ([`retime`]), and what the original did
/// with them.
#[derive(Debug, Clone)]
pub struct Retimed {
    /// What the original is fed on each of its frames.
    pub script: InputScript,
    /// The original's frames fed.
    pub original_frames: u32,
    /// The runner's frames: the original's less those it spent in loads the engine does at once,
    /// which the runner holds the original through itself (the case's `frames`).
    pub frames: u32,
    /// The recording's step each of the original's frames was fed (or held at).
    pub step_of: Vec<u32>,
    /// Where the original was put back on the recording, in order.
    pub put_back: Vec<PutBack>,
    /// Where the original left the recording and could not be put back: nothing after it is
    /// judged on the owner's game. A load the original never ends ends the retiming there.
    pub left: Option<Left>,
    /// The original's picture on its last frame: at STOP.
    pub stop: Option<Rgb12Frame>,
    /// The original's watched state (the frog, score, lives, hearts, level) on its last frame.
    pub stop_watched: Vec<(VarId, u32)>,
}

/// The recording's inputs placed at the frames the original reads them (DESIGN §2.9 as T00
/// amended it): `original` is at the recording's start (a `SyncPoint::Setup` with its setup and
/// seed) and is run through the whole recording. Frame for frame, step n's input on the
/// original's next frame; at a step the engine loaded at once, the original held with nothing
/// pressed until its load is over, the frames it spends under a screen it times itself counted
/// as the recording's steps played; at each level's first frame and the slot machine's entry,
/// the side that is early held for the other. At a pass end in a level where the original's
/// watched state is not the recording's, it is put back on the recording (bug-recorder T14).
pub fn retime(rec: &Recording, original: &mut Synced) -> InputScript {
    let played = play(rec, &[]);
    let r = retime_played(rec, &played, &[], &mut original.amiga);
    original.memory = original.amiga.memory();
    r.script
}

fn retime_played(rec: &Recording, played: &Played, pokes: &Pokes, a: &mut Amiga) -> Retimed {
    let steps = rec.inputs.len() as u32;
    // The recording's anchors: the step after which a level's first frame has been (kind 0) or
    // the slot machine has been entered (kind 1).
    let mut anchors: Vec<(u32, u8)> = Vec::new();
    for k in 1..steps as usize {
        let (was, now) = (&played.scene[k - 1], &played.scene[k]);
        if now.is_level() && !was.is_level() {
            anchors.push((k as u32, 0));
        }
        let slot = |s: &Scene| matches!(s, Scene::Slot { .. });
        if slot(now) && !slot(was) {
            anchors.push((k as u32, 1));
        }
    }
    let mut r = Retimed {
        script: InputScript::default(),
        original_frames: 0,
        frames: 0,
        step_of: Vec::new(),
        put_back: Vec::new(),
        left: None,
        stop: None,
        stop_watched: Vec::new(),
    };
    let mut follow = Follow::new(&rec.setup, rec.rng(), pokes);
    // The original's pass ends since it was seen off the recording with the recording's engine
    // mid-pass, and where it was first seen off: what is reported if it is never put back.
    let mut waited = 0u32;
    let mut off: Option<(u32, u32, String)> = None;
    let mut frames: Vec<FrameInput> = Vec::new();
    let mut held_in_loads = 0u32;
    let mut cop = played.start.cop1lc();
    let mut disks = sync::Disks::at(&a.memory());
    let mut was = (true, false);
    let (mut next, mut n, mut held) = (0usize, 0u32, 0u32);
    let mut step = |a: &mut Amiga,
                    input: FrameInput,
                    of: u32,
                    frames: &mut Vec<FrameInput>,
                    r: &mut Retimed,
                    cop: &mut Option<u32>| {
        let cap = sync::step_inserting(a, input)
            .unwrap_or_else(|e| panic!("oracle, frame {}: {e}", frames.len()));
        *cop = sync::cop1lc(*cop, &a.cpu_writes());
        disks.serve(a);
        frames.push(input);
        r.step_of.push(of);
        r.stop = Some(cap.picture);
    };
    'run: while n < steps {
        // The recording's engine is past its next anchor and the original has not met it.
        let pending = anchors.get(next).is_some_and(|&(s, _)| s < n);
        let of = if pending { n - 1 } else { n };
        let input = if pending {
            FrameInput::NONE
        } else {
            rec.inputs[of as usize]
        };
        step(a, input, of, &mut frames, &mut r, &mut cop);
        // The engine loaded at once on this step: the original's load runs on, nothing pressed.
        let mut under = 0;
        if !pending && let Some(end) = played.load[of as usize] {
            let mut seen = false;
            let mut k = 0;
            let (before, stop) = (frames.len(), r.stop.clone());
            while !load_over(a, end, &mut seen) {
                step(a, FrameInput::NONE, of, &mut frames, &mut r, &mut cop);
                if cop.is_some_and(crate::timed_screen) {
                    under += 1;
                }
                k += 1;
                if k >= LOAD_SEARCH {
                    // The original is not where the recording is (it never begins or ends the
                    // load the engine did): the retiming ends before this step, judged no
                    // further (T14; T12's dry run 2 panicked here).
                    frames.truncate(before - 1);
                    r.step_of.truncate(before - 1);
                    r.stop = stop;
                    r.left.get_or_insert(Left {
                        frame: before as u32 - 1,
                        step: of,
                        what: format!(
                            "the engine loads here ({end:x?}) and the original's load was not over within {LOAD_SEARCH} frames"
                        ),
                    });
                    break 'run;
                }
            }
            held_in_loads += k;
        }
        let m = a.memory();
        let cpu = a.cpu();
        let now = (
            sync::in_level(&m),
            sync::main_chain(&cpu, &m).contains(&sync::IN_SLOT_MACHINE),
        );
        let rose = match (now, was) {
            ((true, _), (false, _)) => Some(0),
            ((_, true), (_, false)) => Some(1),
            _ => None,
        };
        was = now;
        let at = frames.len() as u32 - 1;
        if let Some(kind) = rose
            && let Some(i) = (next..anchors.len()).find(|&i| anchors[i].1 == kind)
            && anchors[i].0 <= n + ANCHOR_AHEAD
        {
            next = i + 1;
            n = anchors[i].0 + 1;
            held = 0;
            continue;
        }
        if pending {
            held += 1;
            if held >= ANCHOR_HOLD {
                let s = anchors[next].0;
                r.left.get_or_insert(Left {
                    frame: at,
                    step: s,
                    what: format!(
                        "the original did not reach {} within {ANCHOR_HOLD} frames of the recording's step {s}",
                        if anchors[next].1 == 0 { "the level's first frame" } else { "the slot machine" }
                    ),
                });
                next += 1;
                held = 0;
            }
            continue;
        }
        n += 1 + under;
        // Held to the recording where both play a level, the original at a pass's end.
        if r.left.is_none()
            && now.0
            && played.scene[of as usize].is_level()
            && !sync::mid_pass(&cpu, &m)
        {
            let (theirs, ours) = (watched_original(&m), played.watched[of as usize]);
            match (0..WATCHED.len()).find(|&k| theirs[k] != ours[k]) {
                None => (waited, off) = (0, None),
                Some(k) => {
                    let what = format!(
                        "{} original {:x}, recording {:x}",
                        def(WATCHED[k]).name,
                        theirs[k],
                        ours[k]
                    );
                    // Put back where both sides are at a pass end: the engine's state is then
                    // one the original's loop can go on from.
                    let eng = follow.after(of, |k| rec.inputs[k as usize]);
                    let first = off.get_or_insert((at, of, what.clone()));
                    if !eng.mid_pass() {
                        for (addr, bytes) in follow.writes() {
                            a.write_memory(addr, &bytes);
                        }
                        r.put_back.push(PutBack {
                            frame: at,
                            step: of,
                            what,
                        });
                        (waited, off) = (0, None);
                    } else if waited >= PUT_BACK_WAIT {
                        // Not judged on the recording from where it was first seen off it.
                        let (frame, step, what) = first.clone();
                        r.left = Some(Left {
                            frame,
                            step,
                            what: format!(
                                "{what}, and the recording's engine was in the middle of a pass at each of the original's {PUT_BACK_WAIT} pass ends since, so it could not be put back"
                            ),
                        });
                    } else {
                        waited += 1;
                    }
                }
            }
        }
    }
    let m = a.memory();
    r.stop_watched = WATCHED.into_iter().zip(watched_original(&m)).collect();
    r.original_frames = frames.len() as u32;
    r.frames = r.original_frames - held_in_loads;
    for (f, i) in frames.into_iter().enumerate() {
        if r.script.lines.last().map(|l| l.1) != Some(i) {
            r.script.lines.push((f as u32, i));
        }
    }
    r
}

/// The original's load is over, as the runner decides it.
fn load_over(a: &Amiga, end: LoadEnd, seen: &mut bool) -> bool {
    let m = a.memory();
    let chain = sync::main_chain_deep(&a.cpu(), &m, usize::MAX);
    match end {
        LoadEnd::Reached(x) => chain.iter().take(4).any(|&c| c == x),
        LoadEnd::Left(x) => {
            let inside = chain.contains(&x);
            *seen |= inside;
            *seen && !inside
        }
    }
}

/// One leg of the comparison: the runner's frames `from..to`, and its first divergence.
#[derive(Debug, Clone)]
pub struct Leg {
    pub from: u32,
    pub to: u32,
    pub first: Option<Divergence>,
}

/// A recording compared with the original ([`compare`]).
pub struct Compared {
    pub retimed: Retimed,
    pub outcome: Outcome,
    pub legs: Vec<Leg>,
    /// The engine's picture at STOP (the recording's last step).
    pub engine_stop: Picture,
}

impl Compared {
    /// No leg differs and the original was on the recording to its end (put back where it left
    /// it): never true over frames that were not judged on the owner's game (T14).
    pub fn same(&self) -> bool {
        self.first().is_none() && self.retimed.left.is_none()
    }

    /// The first divergence, in the first leg that differs.
    pub fn first(&self) -> Option<&Divergence> {
        self.legs.iter().find_map(|l| l.first.as_ref())
    }

    /// Where `d` is: (the original's frame, the recording's step it was fed).
    pub fn at(&self, d: &Divergence) -> (u32, u32) {
        let f = d.oracle_frame.unwrap_or(d.frame);
        let s = self
            .retimed
            .step_of
            .get(f as usize)
            .or(self.retimed.step_of.last());
        (f, s.copied().unwrap_or(0))
    }

    /// What `bug compare` prints: `same`, `differs at frame F (engine step S): …`, or `same up
    /// to frame F (engine step S), not judged after it` first; then where the original was put
    /// back on the recording and where it left it for good, the legs, and the first divergence
    /// in full.
    pub fn report(&self) -> String {
        let mut out = String::new();
        let text = self.first().map(|d| d.to_string());
        let first = self.first().zip(text.as_deref()).map(|(d, text)| {
            let (f, s) = self.at(d);
            (f, s, text.lines().nth(1).map_or("", str::trim))
        });
        out += &amiga_conform::bug_compare::verdict_line(first, self.retimed.left.as_ref());
        out += "\n";
        let put = &self.retimed.put_back;
        if let Some(p) = put.first() {
            out += &format!(
                "the original left the recording {} time{} and was put back on it, the recording's engine written into it at the pass end it was seen: first at frame {} (engine step {}): {}; judged from the owner's state after each\n",
                put.len(),
                if put.len() == 1 { "" } else { "s" },
                p.frame,
                p.step,
                p.what
            );
        }
        match &self.retimed.left {
            None => out += "the original was on the recording to its end\n",
            Some(l) => {
                out += &format!(
                    "the original left the recording at frame {} (engine step {}) and could not be put back: {}; nothing after it was judged on the recording\n",
                    l.frame, l.step, l.what
                );
            }
        }
        let bad: Vec<String> = self
            .legs
            .iter()
            .filter_map(|l| l.first.as_ref().map(|d| self.at(d).0.to_string()))
            .collect();
        out += &format!(
            "legs of {LEG_PASSES} passes: {}, {}\n",
            self.legs.len(),
            if bad.is_empty() {
                "every one same".to_string()
            } else {
                format!("{} differ, first at frames {}", bad.len(), bad.join(", "))
            }
        );
        let o = &self.outcome;
        if o.late > 0 || !o.resynced.is_empty() {
            out += &format!(
                "the original lagged on {} frames; the engine re-synced from it after a lag {} times{}\n",
                o.late,
                o.resynced.len(),
                o.resynced.first().map_or(String::new(), |(f, v)| format!(
                    ", first at frame {f} ({v})"
                ))
            );
        }
        if let Some(d) = self.first() {
            out += &d.to_string();
        }
        out
    }
}

/// `rec` played on the original from the same start with its inputs retimed ([`retime`]), and
/// judged by `mode`'s checks in legs ([`LEG_PASSES`]; DESIGN §2.8, §2.9 as T00 amended it).
pub fn compare(rec: &Recording, mode: Mode) -> Result<Compared, String> {
    compare_with(rec, mode, &[], |_, _| {})
}

/// [`compare`], `pokes` written into every side at frame 0 and `before_step(frame, game)` called
/// before the runner's engine steps each frame (a test plants a divergence there).
pub fn compare_with(
    rec: &Recording,
    mode: Mode,
    pokes: &Pokes,
    before_step: impl FnMut(u32, &mut Game),
) -> Result<Compared, String> {
    compare_inner(rec, mode, pokes, pokes, before_step)
}

/// [`compare`], `pokes` written into the recording's engine alone: a test's recording the
/// original cannot follow, which the app could not make.
pub fn compare_played(rec: &Recording, mode: Mode, pokes: &Pokes) -> Result<Compared, String> {
    compare_inner(rec, mode, pokes, &[], |_, _| {})
}

fn compare_inner(
    rec: &Recording,
    mode: Mode,
    played_pokes: &Pokes,
    pokes: &Pokes,
    before_step: impl FnMut(u32, &mut Game),
) -> Result<Compared, String> {
    comparable(rec)?;
    let played = play(rec, played_pokes);
    let start = || -> Result<Synced, String> {
        let cap = 2 * rec.inputs.len() as u32 + LOAD_SEARCH;
        let (mut s, _) = sync::setup_start(&rec.setup, rec.rng(), cap)?;
        for (addr, bytes) in pokes {
            s.amiga.write_memory(*addr, bytes);
        }
        s.memory = s.amiga.memory();
        Ok(s)
    };
    let mut original = start()?;
    let retimed = retime_played(rec, &played, played_pokes, &mut original.amiga);
    let mut case = case_of(rec, "bug compare", &retimed);
    if let Some(p) = case.put_back.as_mut() {
        p.pokes = played_pokes.to_vec();
    }
    case.mode = mode;
    if mode == Mode::Strict {
        case.alone = crate::Alone::Off;
    }
    // The runner's engine: the recording's start, poked as every side is.
    let mut engine = Game::start_level(crate::display_data(), &rec.setup, rec.rng());
    for (addr, bytes) in pokes {
        engine.poke(*addr, bytes);
    }
    let mut legs = crate::Legs::new(LEG_PASSES, Some((start()?, engine)));
    let mut found = Vec::new();
    let outcome = crate::run_inner(
        &case,
        before_step,
        &mut |_, _, _| {},
        Some(&mut found),
        false,
        None,
        Some(&mut legs),
    )
    .expect("a run in legs collects its divergences");
    let mut retimed = retimed;
    if let Some((frame, why)) = legs.cut.take()
        && retimed.left.is_none()
    {
        let step = retimed.step_of.get(frame as usize).copied().unwrap_or(0);
        retimed.left = Some(Left {
            frame,
            step,
            what: format!("the judging run's engine loaded and {why}"),
        });
    }
    let mut from = vec![0];
    from.extend(legs.starts.iter().copied());
    let mut legs: Vec<Leg> = from
        .iter()
        .enumerate()
        .map(|(i, &f)| Leg {
            from: f,
            to: from.get(i + 1).copied().unwrap_or(retimed.frames),
            first: None,
        })
        .collect();
    // Check 4's divergence, judged at the run's end, may name a frame of play past the last.
    for d in found {
        let leg = legs
            .iter_mut()
            .rev()
            .find(|l| l.from <= d.frame)
            .expect("leg 0 is from 0");
        leg.first.get_or_insert(d);
    }
    Ok(Compared {
        retimed,
        outcome,
        legs,
        engine_stop: played.last,
    })
}

/// `bug case`'s files for `rec` retimed as `r`, named `name`, from the zip `zip`: the case
/// (`SyncPoint::Setup`'s form, the script by name, the runner's frames; its first line the owner's
/// note and where they pressed STOP) and the script, in the original's frames; and where the
/// original was put back on the recording (T14), the recording's inputs by engine step, the
/// script `<name>-steps` the case's `put-back` names.
pub fn case_files(
    rec: &Recording,
    name: &str,
    zip: &str,
    r: &Retimed,
) -> (String, String, Option<String>) {
    let one_line = |t: &str| t.split_whitespace().collect::<Vec<_>>().join(" ");
    let stopped = rec.stopped_at.map_or("-".to_string(), |l| l.to_string());
    let note = match one_line(&rec.note) {
        n if n.is_empty() => "(no note)".to_string(),
        n => n,
    };
    let s = &rec.setup;
    let difficulty = match s.difficulty {
        sf_core::recording::Difficulty::Easy => "easy",
        sf_core::recording::Difficulty::Normal => "normal",
    };
    let mut toml = format!(
        "# {note} (stopped at {stopped})\n\
         # From the bug zip {zip} (`sf-conform bug case`, bug-recorder DESIGN §2.10): the level started\n\
         # with the Start Level dialog's values, the recording's {} inputs retimed onto the original's\n\
         # frames ({} of them, the original's loads included; DESIGN §2.9).\n",
        rec.inputs.len(),
        r.original_frames,
    );
    if !r.put_back.is_empty() {
        toml += "# Where the original's lag took it off the recording it is put back on it (T14): after\n\
                 # `at`'s frames the recording's engine after its step is written into the original.\n";
    }
    if let Some(l) = &r.left {
        toml += &format!(
            "# The original left the recording at frame {} (engine step {}) for good: {}.\n",
            l.frame, l.step, l.what
        );
    }
    toml += &format!(
        "sync = {{ level = \"{}\", lives = {}, points = {}, energy = {}, difficulty = \"{difficulty}\", spud = {}, wings = {}, seed = \"{:08x} {:08x}\" }}\n\
         script = \"{name}\"\n\
         frames = {}\n",
        s.level,
        s.lives.count(),
        s.points,
        s.energy,
        s.spud,
        s.wings,
        rec.rng().lo,
        rec.rng().hi,
        r.frames,
    );
    let mut steps = None;
    if !r.put_back.is_empty() {
        let at: Vec<String> = r
            .put_back
            .iter()
            .map(|p| format!("[{}, {}]", p.frame + 1, p.step))
            .collect();
        toml += &format!(
            "put-back = {{ steps = \"{name}-steps\", at = [{}] }}\n",
            at.join(", ")
        );
        let mut text = format!(
            "# {note}: the bug zip {zip}'s inputs by engine step (`sf-conform bug case`), what the\n\
             # case {name} puts the original back to.\n"
        );
        for &(f, input) in &steps_script(rec).lines {
            text += &format!("{f}: {}\n", InputScript::format_input(input));
        }
        steps = Some(text);
    }
    let mut script = format!(
        "# {note}: the bug zip {zip}'s inputs on the original's frames (`sf-conform bug case`).\n"
    );
    for &(f, input) in &r.script.lines {
        script += &format!("{f}: {}\n", InputScript::format_input(input));
    }
    (toml, script, steps)
}

/// `bug case`: `rec` retimed on the original and written as `cases/<name>.toml`,
/// `scripts/<name>.txt` and, if the original is put back on the recording, `scripts/<name>-steps.txt`;
/// the paths written. Refuses a name that is not lower-case letters, digits and `-`, or one
/// either folder already has.
pub fn write_case(
    rec: &Recording,
    name: &str,
    zip: &str,
    cases: &std::path::Path,
    scripts: &std::path::Path,
) -> Result<Vec<std::path::PathBuf>, String> {
    comparable(rec)?;
    if name.is_empty()
        || !name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err(format!(
            "{name:?}: a case's name is lower-case letters, digits and -"
        ));
    }
    let case = cases.join(format!("{name}.toml"));
    let script = scripts.join(format!("{name}.txt"));
    let steps = scripts.join(format!("{name}-steps.txt"));
    for p in [&case, &script, &steps] {
        if p.exists() {
            return Err(format!(
                "{}: already there; choose another name",
                p.display()
            ));
        }
    }
    let played = play(rec, &[]);
    let cap = 2 * rec.inputs.len() as u32 + LOAD_SEARCH;
    let (mut original, _) = sync::setup_start(&rec.setup, rec.rng(), cap)?;
    let r = retime_played(rec, &played, &[], &mut original.amiga);
    let (toml, text, steps_text) = case_files(rec, name, zip, &r);
    let mut files = vec![(script, text)];
    files.extend(steps_text.map(|t| (steps, t)));
    files.push((case, toml));
    for (p, t) in &files {
        std::fs::write(p, t).map_err(|e| format!("{}: {e}", p.display()))?;
    }
    let mut paths: Vec<_> = files.into_iter().map(|(p, _)| p).collect();
    paths.rotate_right(1);
    Ok(paths)
}

/// The recording's inputs as a script of engine steps.
fn steps_script(rec: &Recording) -> InputScript {
    InputScript {
        lines: amiga_conform::script::InputScript::from_steps(&rec.inputs).lines,
    }
}

/// The case a recording retimed as `r` is compared as: the setup's start, the script in the
/// original's frames, the original put back where `r` put it back.
fn case_of(rec: &Recording, name: &str, r: &Retimed) -> Case {
    let compare = crate::Compare::ALL;
    Case {
        name: name.to_string(),
        sync: SyncPoint::Setup {
            setup: rec.setup,
            seed: rec.rng(),
        },
        script: r.script.clone(),
        frames: r.frames,
        compare,
        exclude: Vec::new(),
        only_vars: None,
        resync: true,
        lag_limit: None,
        mode: Mode::Faithful,
        alone: crate::Alone::of(compare),
        put_back: (!r.put_back.is_empty()).then(|| crate::PutBacks {
            steps: steps_script(rec),
            at: r.put_back.iter().map(|p| (p.frame + 1, p.step)).collect(),
            pokes: Vec::new(),
        }),
    }
}

/// The frames `bug replay` writes: `from..to` (to excluded), every `every`th from `from`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FramePick {
    pub from: u32,
    pub to: u32,
    pub every: u32,
}

/// How many frames `bug replay` writes when `--frames` is not given: the last this many.
pub const DEFAULT_LAST: u32 = 250;

impl FramePick {
    /// `--frames A..B` (`None`: the last [`DEFAULT_LAST`] of `frames`) and `--every N` (`None`:
    /// 1), refused when not numbers, `A > B` or `N` is 0. A range past the recording's end writes
    /// only the frames it has.
    pub fn parse(
        range: Option<&str>,
        every: Option<&str>,
        frames: u32,
    ) -> Result<FramePick, String> {
        let (from, to) = match range {
            None => (frames.saturating_sub(DEFAULT_LAST), frames),
            Some(r) => {
                let bad =
                    || format!("--frames {r:?}: expected A..B, frames from A up to B, B left out");
                let (a, b) = r.split_once("..").ok_or_else(bad)?;
                let (a, b): (u32, u32) =
                    (a.parse().map_err(|_| bad())?, b.parse().map_err(|_| bad())?);
                if a > b {
                    return Err(bad());
                }
                (a, b)
            }
        };
        let every = match every {
            None => 1,
            Some(n) => n
                .parse()
                .ok()
                .filter(|&n: &u32| n > 0)
                .ok_or_else(|| format!("--every {n:?}: expected a whole number above 0"))?,
        };
        Ok(FramePick { from, to, every })
    }

    pub fn wants(&self, frame: u32) -> bool {
        (self.from..self.to).contains(&frame) && (frame - self.from) % self.every == 0
    }
}

/// `bug replay`'s default folder for `zip`'s frames: `~/.cache/superfrog-re/bugs/<zip stem>/`,
/// outside the repo (the pictures are disk-derived, RE §2.3).
pub fn default_out(zip: &std::path::Path) -> Result<std::path::PathBuf, String> {
    let home = std::env::var_os("HOME").ok_or("HOME is not set; give --out DIR")?;
    let stem = zip.file_stem().ok_or_else(|| {
        format!(
            "{}: no file name to name the folder by; give --out DIR",
            zip.display()
        )
    })?;
    Ok(std::path::PathBuf::from(home)
        .join(".cache/superfrog-re/bugs")
        .join(stem))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sf_core::recording::ZIP_RECORDING;

    #[test]
    fn frame_picks() {
        let p = FramePick::parse(None, None, 1000).unwrap();
        assert_eq!(
            p,
            FramePick {
                from: 750,
                to: 1000,
                every: 1
            }
        );
        assert_eq!(FramePick::parse(None, None, 10).unwrap().from, 0);
        let p = FramePick::parse(Some("10..20"), Some("5"), 1000).unwrap();
        let got: Vec<u32> = (0..1000).filter(|&f| p.wants(f)).collect();
        assert_eq!(got, [10, 15]);
        for (r, e) in [
            (Some("20..10"), None),
            (Some("x..2"), None),
            (Some("5"), None),
            (None, Some("0")),
        ] {
            assert!(FramePick::parse(r, e, 100).is_err(), "{r:?} {e:?}");
        }
    }

    #[test]
    fn refusals_name_why() {
        let e = open_zip(b"hello").err().unwrap();
        assert!(e.starts_with("not a bug zip"), "{e}");
        let z = sf_disk::zip::write_stored(&[(ZIP_SCREENSHOT, b"x")]);
        assert!(open_zip(&z).err().unwrap().contains("no recording.txt"));
        let z = sf_disk::zip::write_stored(&[
            (ZIP_RECORDING, b"superfrog recording 3\n"),
            (ZIP_SCREENSHOT, b"x"),
        ]);
        let e = open_zip(&z).err().unwrap();
        assert!(e.starts_with("recording.txt: line 1"), "{e}");
        let rec = Recording {
            build: "b".into(),
            seed: sf_core::recording::seed_bytes(sf_core::Rng::seeded(1, 2)),
            note: String::new(),
            inputs: vec![FrameInput::NONE; 3],
            header: sf_core::recording::Header::new(Default::default(), false),
        };
        let z = amiga_record::zip_bytes(&rec, b"not a png").unwrap();
        let e = open_zip(&z).err().unwrap();
        assert!(e.starts_with("screenshot.png: it is not a PNG"), "{e}");
        let z = amiga_record::zip_bytes(&rec, &picture_png(&Picture::blank(0, 0, 2, 2))).unwrap();
        assert_eq!(open_zip(&z).unwrap().0, rec);
    }
}
