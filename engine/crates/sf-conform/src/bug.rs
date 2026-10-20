//! A bug zip replayed headlessly (bug-recorder DESIGN §2.5, §2.7): the recording read out of the
//! zip, the engine started as it says (`Game::start_level`), stepped through its inputs unpaced as
//! the app plays them (`Game::step`, one a frame), and its last picture and place held against the
//! owner's screenshot and `stopped-at`. A mismatch is a result, not an error: on a fixed engine it
//! is the point. `sf-conform bug replay` (main.rs) is the CLI.

use sf_core::input::FrameInput;
use sf_core::layout::{AmigaMemory, Ram, VarId, def};
use sf_core::recording::Location;
use sf_core::recording::{Recording, ZIP_RECORDING, ZIP_SCREENSHOT};
use sf_core::{Game, LoadEnd, Picture, Scene, View};
use sf_oracle::{Amiga, Rgb12Frame};

use crate::sync::{self, Synced};
use crate::{Case, Divergence, InputScript, Mode, Outcome, SyncPoint};

/// The largest entry read out of a bug zip. `recording.txt` at `MAX_FRAMES` with the input
/// changing every frame is ~60 MB; a 1× screenshot is well under 1 MB.
const MAX_ENTRY: usize = 256 << 20;

/// The recording and the screenshot's PNG bytes of a bug zip. Refuses, naming why, anything that
/// is not a zip, lacks either entry, holds a screenshot that is not a PNG (refused here, not
/// after a replay that may run minutes), or holds a recording the parser or
/// [`Recording::validate`] refuses (a level no game starts at included, which `Game::start_level`
/// could not reach).
pub fn read_zip(bytes: &[u8]) -> Result<(Recording, Vec<u8>), String> {
    let entries = sf_disk::zip::read_entries(
        bytes,
        |n| n == ZIP_RECORDING || n == ZIP_SCREENSHOT,
        MAX_ENTRY,
    )
    .map_err(|e| format!("not a bug zip: {e}"))?;
    let take = |name: &str| {
        entries
            .iter()
            .find(|e| e.name == name)
            .map(|e| e.bytes.clone())
            .ok_or_else(|| format!("not a bug zip: it has no {name}"))
    };
    let text = take(ZIP_RECORDING)?;
    let png = take(ZIP_SCREENSHOT)?;
    let text =
        String::from_utf8(text).map_err(|_| format!("{ZIP_RECORDING}: it is not UTF-8 text"))?;
    let rec = Recording::parse(&text).map_err(|e| format!("{ZIP_RECORDING}: {e}"))?;
    rec.validate()
        .map_err(|e| format!("{ZIP_RECORDING}: {e}"))?;
    decode_png(&png).map_err(|e| format!("{ZIP_SCREENSHOT}: it is not a PNG: {e}"))?;
    Ok((rec, png))
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
/// whose pass ends mid-frame shows the picture drawn then). The 4:3 view: the screenshot is the
/// core's picture at 1× (DESIGN §2.5).
pub fn replay(rec: &Recording, mut each: impl FnMut(u32, &Game, &Picture)) -> Replayed {
    let mut game = Game::start_level(crate::display_data(), &rec.setup, rec.seed);
    let mut last = game.render(View::default());
    for (k, &input) in rec.inputs.iter().enumerate() {
        let out = game.step(input);
        each(k as u32, &game, &out.picture);
        last = out.picture;
    }
    Replayed {
        location: game.location(),
        game,
        last,
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
    let mut game = Game::start_level(crate::display_data(), &rec.setup, rec.seed);
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
/// what the owner played, and a bug seen after this frame may not occur on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Left {
    /// The original's frame, counted from the start.
    pub frame: u32,
    /// The recording's step it was fed.
    pub step: u32,
    pub what: String,
}

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
    pub left: Option<Left>,
    /// The original's picture on its last frame: at STOP.
    pub stop: Option<Rgb12Frame>,
}

/// The recording's inputs placed at the frames the original reads them (DESIGN §2.9 as T00
/// amended it): `original` is at the recording's start (a `SyncPoint::Setup` with its setup and
/// seed) and is run through the whole recording. Frame for frame, step n's input on the
/// original's next frame; at a step the engine loaded at once, the original held with nothing
/// pressed until its load is over, the frames it spends under a screen it times itself counted
/// as the recording's steps played; at each level's first frame and the slot machine's entry,
/// the side that is early held for the other.
pub fn retime(rec: &Recording, original: &mut Synced) -> InputScript {
    let played = play(rec, &[]);
    let r = retime_played(rec, &played, &mut original.amiga);
    original.memory = sync::core_memory(original.amiga.memory());
    r.script
}

fn retime_played(rec: &Recording, played: &Played, a: &mut Amiga) -> Retimed {
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
        left: None,
        stop: None,
    };
    let mut frames: Vec<FrameInput> = Vec::new();
    let mut held_in_loads = 0u32;
    let mut cop = played.start.cop1lc();
    let mut disks = sync::Disks::at(&sync::core_memory(a.memory()));
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
    while n < steps {
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
            while !load_over(a, end, &mut seen) {
                step(a, FrameInput::NONE, of, &mut frames, &mut r, &mut cop);
                if cop.is_some_and(crate::timed_screen) {
                    under += 1;
                }
                k += 1;
                assert!(
                    k < LOAD_SEARCH,
                    "the original's load ({end:x?}) was not over within {LOAD_SEARCH} frames of the engine's (step {of})"
                );
            }
            held_in_loads += k;
        }
        let m = sync::core_memory(a.memory());
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
            if let Some(k) = (0..WATCHED.len()).find(|&k| theirs[k] != ours[k]) {
                r.left = Some(Left {
                    frame: at,
                    step: of,
                    what: format!(
                        "{} original {:x}, recording {:x}",
                        def(WATCHED[k]).name,
                        theirs[k],
                        ours[k]
                    ),
                });
            }
        }
    }
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
    let m = sync::core_memory(a.memory());
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
    pub fn same(&self) -> bool {
        self.legs.iter().all(|l| l.first.is_none())
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

    /// What `bug compare` prints: `same` or `differs at frame F (engine step S): …` first, then
    /// where the original left the recording, the legs, and the first divergence in full.
    pub fn report(&self) -> String {
        let mut out = String::new();
        match self.first() {
            None => out += "same\n",
            Some(d) => {
                let (f, s) = self.at(d);
                let text = d.to_string();
                let what = text.lines().nth(1).map_or("", str::trim);
                out += &format!("differs at frame {f} (engine step {s}): {what}\n");
            }
        }
        match &self.retimed.left {
            None => out += "the original stayed on the recording to its end\n",
            Some(l) => {
                out += &format!(
                    "the original left the recording at frame {} (engine step {}): {}; a bug after it may not occur on the original\n",
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
    let played = play(rec, pokes);
    let start = || -> Result<Synced, String> {
        let cap = 2 * rec.inputs.len() as u32 + LOAD_SEARCH;
        let (mut s, _) = sync::setup_start(&rec.setup, rec.seed, cap)?;
        for (addr, bytes) in pokes {
            s.amiga.write_memory(*addr, bytes);
        }
        s.memory = sync::core_memory(s.amiga.memory());
        Ok(s)
    };
    let mut original = start()?;
    let retimed = retime_played(rec, &played, &mut original.amiga);
    let mut case = case_of(rec, "bug compare", retimed.script.clone(), retimed.frames);
    case.mode = mode;
    if mode == Mode::Strict {
        case.alone = crate::Alone::Off;
    }
    let mut legs = crate::Legs::new(LEG_PASSES, Some((start()?, played.start.clone())));
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
/// note and where they pressed STOP) and the script, in the original's frames.
pub fn case_files(rec: &Recording, name: &str, zip: &str, r: &Retimed) -> (String, String) {
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
    let toml = format!(
        "# {note} (stopped at {stopped})\n\
         # From the bug zip {zip} (`sf-conform bug case`, bug-recorder DESIGN §2.10): the level started\n\
         # with the Start Level dialog's values, the recording's {} inputs retimed onto the original's\n\
         # frames ({} of them, the original's loads included; DESIGN §2.9).\n\
         sync = {{ level = \"{}\", lives = {}, points = {}, energy = {}, difficulty = \"{difficulty}\", spud = {}, wings = {}, seed = \"{:08x} {:08x}\" }}\n\
         script = \"{name}\"\n\
         frames = {}\n",
        rec.inputs.len(),
        r.original_frames,
        s.level,
        s.lives.count(),
        s.points,
        s.energy,
        s.spud,
        s.wings,
        rec.seed.lo,
        rec.seed.hi,
        r.frames,
    );
    let mut script = format!(
        "# {note}: the bug zip {zip}'s inputs on the original's frames (`sf-conform bug case`).\n"
    );
    for &(f, input) in &r.script.lines {
        script += &format!("{f}: {}\n", InputScript::format_input(input));
    }
    (toml, script)
}

/// `bug case`: `rec` retimed on the original and written as `cases/<name>.toml` and
/// `scripts/<name>.txt`. Refuses a name that is not lower-case letters, digits and `-`, or one
/// either folder already has.
pub fn write_case(
    rec: &Recording,
    name: &str,
    zip: &str,
    cases: &std::path::Path,
    scripts: &std::path::Path,
) -> Result<(std::path::PathBuf, std::path::PathBuf), String> {
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
    for p in [&case, &script] {
        if p.exists() {
            return Err(format!(
                "{}: already there; choose another name",
                p.display()
            ));
        }
    }
    let played = play(rec, &[]);
    let cap = 2 * rec.inputs.len() as u32 + LOAD_SEARCH;
    let (mut original, _) = sync::setup_start(&rec.setup, rec.seed, cap)?;
    let r = retime_played(rec, &played, &mut original.amiga);
    let (toml, text) = case_files(rec, name, zip, &r);
    for (p, t) in [(&script, &text), (&case, &toml)] {
        std::fs::write(p, t).map_err(|e| format!("{}: {e}", p.display()))?;
    }
    Ok((case, script))
}

/// The case a recording is compared as: the setup's start, `script` in the original's frames.
fn case_of(rec: &Recording, name: &str, script: InputScript, frames: u32) -> Case {
    let compare = crate::Compare::ALL;
    Case {
        name: name.to_string(),
        sync: SyncPoint::Setup {
            setup: rec.setup,
            seed: rec.seed,
        },
        script,
        frames,
        compare,
        exclude: Vec::new(),
        only_vars: None,
        resync: true,
        lag_limit: None,
        mode: Mode::Faithful,
        alone: crate::Alone::of(compare),
    }
}

/// `pic` as 8-bit RGB, each 4-bit channel times 17, as the app shows it (`sf_app::video`).
pub fn picture_rgb(pic: &Picture) -> Vec<u8> {
    sf_core::to_rgb12(pic)
        .into_iter()
        .flat_map(|c| {
            [
                (c >> 8 & 15) as u8 * 17,
                (c >> 4 & 15) as u8 * 17,
                (c & 15) as u8 * 17,
            ]
        })
        .collect()
}

/// `pic` as a PNG.
pub fn picture_png(pic: &Picture) -> Vec<u8> {
    crate::report::png(pic.width as u32, pic.height as u32, &picture_rgb(pic))
}

/// The original's picture as a PNG.
pub fn frame_png(f: &Rgb12Frame) -> Vec<u8> {
    let rgb: Vec<u8> = f
        .pixels
        .iter()
        .flat_map(|&c| {
            [
                (c >> 8 & 15) as u8 * 17,
                (c >> 4 & 15) as u8 * 17,
                (c & 15) as u8 * 17,
            ]
        })
        .collect();
    crate::report::png(f.width as u32, f.height as u32, &rgb)
}

/// How many pixels of the screenshot `png` differ from `pic`: 0 is equal. A screenshot of another
/// size differs in every pixel of the larger of the two. `Err` when `png` does not decode.
pub fn screenshot_differs(png: &[u8], pic: &Picture) -> Result<u64, String> {
    let (w, h, rgb) = decode_png(png).map_err(|e| format!("{ZIP_SCREENSHOT}: {e}"))?;
    let (pw, ph) = (pic.width as u32, pic.height as u32);
    if (w, h) != (pw, ph) {
        return Ok(u64::from(w * h).max(u64::from(pw * ph)));
    }
    let ours = picture_rgb(pic);
    Ok(rgb
        .chunks_exact(3)
        .zip(ours.chunks_exact(3))
        .filter(|(a, b)| a != b)
        .count() as u64)
}

/// A PNG as (width, height, 8-bit RGB). The app writes RGB (`sf_app::capture::png`); any other
/// colour type a PNG may hold is taken too, alpha dropped.
fn decode_png(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let mut dec = png::Decoder::new(std::io::Cursor::new(bytes));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut r = dec.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; r.output_buffer_size().ok_or("the PNG is too large")?];
    let info = r.next_frame(&mut buf).map_err(|e| e.to_string())?;
    buf.truncate(info.buffer_size());
    let rgb = match info.color_type {
        png::ColorType::Rgb => buf,
        png::ColorType::Rgba => buf
            .chunks_exact(4)
            .flat_map(|p| [p[0], p[1], p[2]])
            .collect(),
        png::ColorType::Grayscale => buf.iter().flat_map(|&g| [g, g, g]).collect(),
        png::ColorType::GrayscaleAlpha => buf
            .chunks_exact(2)
            .flat_map(|p| [p[0], p[0], p[0]])
            .collect(),
        png::ColorType::Indexed => {
            return Err("an indexed PNG, which EXPAND should have widened".into());
        }
    };
    Ok((info.width, info.height, rgb))
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
    fn a_screenshot_of_another_size_differs_everywhere() {
        let pic = Picture::blank(0, 0, 4, 2);
        let same = picture_png(&pic);
        assert_eq!(screenshot_differs(&same, &pic).unwrap(), 0);
        let other = picture_png(&Picture::blank(0, 0, 5, 3));
        assert_eq!(screenshot_differs(&other, &pic).unwrap(), 15);
        let mut lit = pic.clone();
        lit.palettes[1][0] = 0xFFF;
        assert_eq!(screenshot_differs(&picture_png(&lit), &pic).unwrap(), 4);
        assert!(screenshot_differs(b"not a png", &pic).is_err());
    }

    #[test]
    fn refusals_name_why() {
        let e = read_zip(b"hello").err().unwrap();
        assert!(e.starts_with("not a bug zip"), "{e}");
        let z = sf_disk::zip::write_stored(&[(ZIP_SCREENSHOT, b"x")]);
        assert!(read_zip(&z).err().unwrap().contains("no recording.txt"));
        let z = sf_disk::zip::write_stored(&[
            (ZIP_RECORDING, b"superfrog recording 2\n"),
            (ZIP_SCREENSHOT, b"x"),
        ]);
        let e = read_zip(&z).err().unwrap();
        assert!(e.starts_with("recording.txt: line 1"), "{e}");
    }
}
