//! The frame bar's runner (translator DESIGN §2.7, T29): a level case on an engine that is a whole
//! machine, judged at the end of every frame on what the bar names: the case's state, the picture
//! in [`PLAY_WINDOW`] and Paula's writes of the frame, in order.
//!
//! **Why not [`crate::run_with`].** The pass runner pairs the original's pass ends with an engine
//! that knows only passes: it hands each pass the input that pass read ([`crate::PassInputs`]),
//! compares at [`crate::PASS_END`], and shows each pass in the frame after it. A machine running the
//! original's program has frames, not passes: it reads the joystick when its own code does, so it
//! takes the input of each frame as the oracle got it, and its state, picture and writes are those
//! of the frame's end, which is where an oracle step ends. So the two step together, one engine
//! frame an oracle step, and nothing of the hand engine's measured cuts (T71, T74, T80, T82) applies.
//!
//! **Every channel.** A run ends at its first difference unless [`Options::every_channel`] is set:
//! then a channel that differed is no longer compared, the others go on, and
//! [`Verdict::firsts`] holds each channel's first difference in the order met (a stop always ends
//! the run). A whole-machine engine can differ in one channel from the first frame (its timing
//! moves a write across a frame's end) and still be right in the others, which a first reading
//! needs to see.
//!
//! **The start** is the pass runner's: the first step after the sync point in which a pass ends.
//! The engine is transplanted at that step's end (`translated_engine::case_start` takes its start
//! there), and its first frame is the next step's.

use amiga_chips::{Picture, to_rgb12};
use amiga_conform::compare::{PictureClass, PictureJudge, classify, paula, picture_except};
use amiga_conform::{Mode, Tolerance, report};
use amiga_oracle::{FrameInput, PaulaWrite};
use fd_core::{Mem, Transplant, Var};


use crate::{
    Case, GAME_OVER_SCREEN, Goal, HIGH_SCORES, HUNK_BASES, Inputs, Live, NEXT_WORLD, OracleMem,
    Options, Outcome, PASS_END, PLAY_WINDOW, Verdict, layout_vars, reach_case,
};

/// An engine stepped a whole frame at a time: the translated program on its machine, or a stand-in.
pub trait FrameEngine: Transplant {
    /// Run one frame on `input`, the input the original was given in the same frame.
    fn frame(&mut self, input: FrameInput);
    /// The whole picture of the last frame (it is cut to [`PLAY_WINDOW`] here); `None` if none.
    fn picture(&self) -> Option<Picture>;
    /// Paula's writes in the last frame, in order; their beam positions are not compared.
    fn paula(&self) -> &[PaulaWrite];
    /// How the engine stopped (translator DESIGN §2.4: `untranslated: hN+$off` and the rest), once
    /// it has; no frame runs after it.
    fn stopped(&self) -> Option<String>;
}

/// Run level case `case` on `engine`, judged frame by frame.
pub fn run_frames(
    case: &Case,
    engine: &mut dyn FrameEngine,
    opts: &Options,
) -> Result<Verdict, String> {
    run_frames_inputs(case, &Inputs::load(case)?, engine, opts)
}

/// [`run_frames`] on `inputs` rather than the scripts the case names.
pub fn run_frames_inputs(
    case: &Case,
    inputs: &Inputs,
    engine: &mut dyn FrameEngine,
    opts: &Options,
) -> Result<Verdict, String> {
    if !case.is_level() {
        return Err(format!(
            "{}: the frame runner runs level cases; a title case runs with `run`",
            case.name
        ));
    }
    let vars = layout_vars(&case.vars)?;
    let goal: Vec<(&Var, u32)> = match &case.goal {
        Some(Goal::All { all, .. }) => all
            .iter()
            .map(|(v, value)| Ok((layout_vars(std::slice::from_ref(v))?[0], *value)))
            .collect::<Result<_, String>>()?,
        _ => Vec::new(),
    };
    let rows: Vec<std::ops::Range<u16>> = case
        .exclude
        .iter()
        .filter(|e| e.var.is_none() && e.cols.is_none())
        .map(|e| e.rows.clone())
        .collect();
    if case.exclude.iter().any(|e| e.var.is_some() || e.cols.is_some()) {
        return Err(format!(
            "{}: the frame runner leaves out whole rows only; a box or a variable exclusion is the \
             pass runner's",
            case.name
        ));
    }
    let scene_pcs =
        [GAME_OVER_SCREEN, HIGH_SCORES, NEXT_WORLD].map(|(h, o)| HUNK_BASES[h as usize] + o);
    let mut pcs: Vec<u32> = opts.register.iter().map(crate::coverage::address).collect();
    pcs.extend(scene_pcs);
    let mut am = reach_case(case, 0)?;
    am.watch_pcs(&pcs);
    am.watch_write(Some(PASS_END));

    let mut verdict = Verdict::new(opts.register);
    verdict.excluded = case.exclude.iter().map(|e| (e.clone(), 0)).collect();
    let tol = Tolerance::OWNER;
    let mut judge = PictureJudge::new(tol, case.frames);
    // Transplanted yet, and the engine frames run since.
    let mut started = false;
    let mut run = 0u32;
    // The channels still compared: state, Paula's writes, the picture. With
    // `Options::every_channel` a channel that differed is dropped and the run goes on with the rest.
    let mut live = [case.compare.vars, case.compare.writes, case.compare.picture];

    'steps: for step in 0..case.frames {
        let input = inputs.at(case.sync_frame + step);
        let capture = am.step(input).map_err(|e| e.to_string())?;
        let frame = am.frame();
        verdict.coverage.add(opts.register, am.pc_hits());
        if !started {
            if let Some(mem) = am.watched_memory() {
                engine.transplant(&OracleMem(&mem));
                started = true;
                am.watch_write(None);
            }
            continue;
        }
        engine.frame(input);
        run += 1;
        // `pass` in an outcome counts the engine's frames from the transplant.
        let pass = run;
        if let Some(stop) = engine.stopped() {
            verdict.firsts.push(Outcome::Stopped { pass, frame, stop });
            break;
        }
        let theirs = Live(&am);
        let mut differs: Vec<(usize, Outcome)> = Vec::new();
        if live[0] {
            let first = vars.iter().find_map(|v| {
                let (ours, theirs) = (engine.probe(v), theirs.value(v));
                (ours != theirs).then(|| Outcome::Differs {
                    pass,
                    frame,
                    var: v.name.to_string(),
                    engine: ours,
                    oracle: theirs,
                })
            });
            match first {
                Some(o) => differs.push((0, o)),
                None => verdict.passes += 1,
            }
        }
        if live[1] {
            match paula(engine.paula(), &capture.paula) {
                Some(why) => differs.push((1, Outcome::WritesDiffer { pass, frame, why })),
                None => {
                    verdict.writes += capture.paula.len() as u32;
                    verdict.write_frames += 1;
                }
            }
        }
        if live[2] {
            let pic = am.picture_in(PLAY_WINDOW.0, PLAY_WINDOW.1);
            let whole = engine
                .picture()
                .ok_or("the case compares the picture but the engine draws none")?;
            let ours = whole.rewindow(pic.left as i32, pic.top, pic.width, pic.height);
            let colours = to_rgb12(&ours);
            for (i, e) in case.exclude.iter().enumerate() {
                let differs = (e.rows.start..e.rows.end.min(pic.height)).any(|y| {
                    (0..pic.width)
                        .any(|x| colours[(y * pic.width + x) as usize] != pic.pixel(x, y))
                });
                verdict.excluded[i].1 += differs as u32;
            }
            let class = classify(&ours, &pic, &rows, &tol);
            let failed = match opts.mode {
                _ if class == PictureClass::Same => judge.frame(frame, class).err(),
                Mode::Strict => Some(format!("{class:?}, strict")),
                Mode::Faithful => judge.frame(frame, class).err(),
            };
            verdict.pictures += 1;
            verdict.tolerated = [
                judge.colour_step.len() as u32,
                judge.few_pixels.len() as u32,
                judge.glitches.len() as u32,
            ];
            if let Some(mut why) = failed {
                let shown = picture_except(&ours, &pic, &rows);
                if let Some((x, y, o, e)) = shown.as_ref().and_then(|m| m.first) {
                    why += &format!(
                        "; first at ({x}, {y}): {o:03x} in the original, {e:03x} in the engine"
                    );
                }
                if let (Some(dir), Some(m)) = (&opts.diff_dir, shown) {
                    let d = report::write_diff_png(dir, &case.name, frame, &ours, &pic, m);
                    why += &format!("; {}", d.path.display());
                }
                differs.push((2, Outcome::PictureDiffers { pass, frame, why }));
            }
        }
        for (k, o) in differs {
            live[k] = false;
            verdict.firsts.push(o);
            if !opts.every_channel || !live.contains(&true) {
                break 'steps;
            }
        }
        if let Some(Goal::All { by_frame, .. }) = &case.goal
            && verdict.goal_met.is_none()
            && frame <= *by_frame
            && goal
                .iter()
                .all(|(v, value)| engine.probe(v) == *value && theirs.value(v) == *value)
        {
            verdict.goal_met = Some(frame);
        }
        // The scenes are the program's own on both sides, so they are judged by the frame's state
        // and picture above; here they only end the comparison where the pass runner ends it.
        let ran = |pc: u32| am.pc_hits().iter().any(|&(p, n)| p == pc && n > 0);
        if ran(scene_pcs[2]) {
            verdict.level_end = Some(frame);
            break;
        }
        if ran(scene_pcs[0]) && verdict.game_over.is_none() {
            verdict.game_over = Some((frame, None));
        }
        if ran(scene_pcs[1]) {
            verdict.game_over = Some((verdict.game_over.map_or(frame, |g| g.0), Some(frame)));
            break;
        }
    }
    if let Some(first) = verdict.firsts.first() {
        verdict.outcome = first.clone();
        return Ok(verdict);
    }

    let nothing = |what: &str| {
        Some(Outcome::ComparedNothing {
            what: what.to_string(),
        })
    };
    let only_writes = case.compare.writes && !case.compare.vars && !case.compare.picture;
    let failed = if !started {
        nothing("no pass of the original ended after the sync point")
    } else if run == 0 {
        nothing("the engine ran no frame after the transplant")
    } else if only_writes && verdict.writes == 0 {
        nothing("neither side wrote to Paula in any compared frame")
    } else {
        match &case.goal {
            Some(Goal::All { all, by_frame }) if verdict.goal_met.is_none() => {
                let want: Vec<String> = all.iter().map(|(v, n)| format!("{v} = {n}")).collect();
                Some(Outcome::GoalMissed {
                    goal: format!("{} by oracle frame {by_frame}", want.join(", ")),
                })
            }
            Some(Goal::GameOver { by_frame }) => match verdict.game_over {
                Some((_, Some(left))) if left <= *by_frame => {
                    verdict.goal_met = Some(left);
                    None
                }
                _ => Some(Outcome::GoalMissed {
                    goal: format!(
                        "the game over screen left for the high scores by oracle frame {by_frame}"
                    ),
                }),
            },
            Some(Goal::LevelEnd { by_frame }) => match verdict.level_end {
                Some(end) if end <= *by_frame => {
                    verdict.goal_met = Some(end);
                    None
                }
                _ => Some(Outcome::GoalMissed {
                    goal: format!("the level's end (`next_world`) by oracle frame {by_frame}"),
                }),
            },
            _ => None,
        }
    };
    let unused = || {
        verdict
            .excluded
            .iter()
            .find(|(_, n)| *n == 0)
            .map(|(e, _)| Outcome::ExclusionUnused {
                what: e.what.clone(),
                task: e.task.clone(),
            })
    };
    verdict.outcome = failed.or_else(unused).unwrap_or(Outcome::Same);
    Ok(verdict)
}
