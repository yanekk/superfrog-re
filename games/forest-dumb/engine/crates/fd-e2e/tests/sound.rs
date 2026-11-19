//! Translator T38 (DESIGN §2.10): the app plays the translated program's sound, each frame's Paula
//! writes through `amiga-paula`. The app's capture (`audio.wav`, `audio-frames.txt`) is compared
//! sample for sample with a headless run of `fd-translated` on the same inputs, mixed by the same
//! `fd_app::audio::Sound`.
//!
//! Full: needs the owner's disks and the generated tree (`translate.py --build`).

mod common;

use std::collections::BTreeMap;

use amiga_input::FrameInput;
use fd_app::audio::{SILENT_RATE, Sound};
use fd_e2e::{App, Captured, LaunchOpts};
use fd_translated::{Control, RUN_STACK, Start, TranslatedGame};
use pcode_machine::{Disks, Input};

/// One frame of a headless run: its samples and how many Paula writes made them.
struct Heard {
    samples: Vec<[i16; 2]>,
    writes: usize,
    /// In level 1 at the frame's end (`fd_app::in_level_1`).
    level_1: bool,
}

/// The translated program from power-on, level 1 mounted, as the app runs it, for `inputs.len()`
/// frames, frame `i` on `inputs[i]`: each frame's sound at [`SILENT_RATE`], a scripted app's.
fn headless(inputs: Vec<FrameInput>) -> Vec<Heard> {
    assert!(
        TranslatedGame::generated(),
        "no translated program built in: run `python3 -I games/forest-dumb/tools/translate.py --build`"
    );
    let disks = fd_conform::disks::load().unwrap_or_else(|e| panic!("{e}"));
    let program = fd_conform::os_run::program(&disks[0]).unwrap_or_else(|e| panic!("{e}"));
    let disk2 = disks[1].clone();
    let data = common::data();
    std::thread::Builder::new()
        .stack_size(RUN_STACK)
        .spawn(move || {
            let mut g = TranslatedGame::new(Start::PowerOn {
                program: &program,
                disks: Disks { df0: disk2 },
            });
            g.mount_level_1(data);
            let mut sound = Sound::new(SILENT_RATE, g.machine().dmacon());
            let mut heard = Vec::new();
            let Some(&first) = inputs.first() else {
                return heard;
            };
            g.set_input(Input::from(first));
            g.run(&mut |out| {
                assert!(
                    out.stop.is_none(),
                    "stopped at frame {}: {:?}",
                    heard.len() + 1,
                    out.stop
                );
                let mut samples = Vec::new();
                sound.frame_out(out, &mut samples);
                heard.push(Heard {
                    samples,
                    writes: out.paula.len(),
                    level_1: fd_app::in_level_1(out.chip, out.slow),
                });
                match inputs.get(heard.len()) {
                    Some(&i) => Control::Next(Input::from(i)),
                    None => Control::Stop,
                }
            });
            heard
        })
        .expect("a thread")
        .join()
        .unwrap_or_else(|e| std::panic::resume_unwind(e))
}

/// The app's sound by step: `audio.wav`'s samples cut at `audio-frames.txt`'s counts.
fn captured(c: &Captured) -> BTreeMap<u64, Vec<[i16; 2]>> {
    let wav = std::fs::read(&c.audio_wav).expect("audio.wav");
    // The capture's WAV is the canonical 44-byte header (amiga_app::capture::wav), then the data.
    assert_eq!(&wav[..4], b"RIFF");
    assert_eq!(
        u32::from_le_bytes(wav[24..28].try_into().unwrap()),
        SILENT_RATE,
        "its rate"
    );
    let mut samples = wav[44..].chunks_exact(4).map(|s| {
        [
            i16::from_le_bytes([s[0], s[1]]),
            i16::from_le_bytes([s[2], s[3]]),
        ]
    });
    let frames = std::fs::read_to_string(c.dir.join("audio-frames.txt")).expect("audio-frames.txt");
    let by_step = frames
        .lines()
        .map(|l| {
            let mut w = l.split(' ').map(|v| v.parse::<u64>().unwrap());
            let n = w.next().unwrap();
            let count: u64 = w.sum();
            (n, samples.by_ref().take(count as usize).collect::<Vec<_>>())
        })
        .collect();
    assert_eq!(
        samples.next(),
        None,
        "audio-frames.txt accounts for every sample"
    );
    by_step
}

fn loud(s: &[[i16; 2]]) -> bool {
    s.iter().any(|&[l, r]| l != 0 || r != 0)
}

/// Fire at the disk 2 prompt, as `translated_app.rs`: the title follows, from about step 184.
const PAST_THE_PROMPT: &str = "20: down AltLeft\n30: up AltLeft\n";

/// Steps run over the title.
const TITLE: u64 = 400;

/// The app's inputs by step `1..=to`, as the capture says it fed them.
fn fed(c: &Captured, to: u64) -> Vec<FrameInput> {
    let i = common::inputs(c);
    (1..=to)
        .map(|n| i.get(&n).copied().unwrap_or_default())
        .collect()
}

#[test]
fn the_titles_first_300_frames_of_sound_are_the_same_on_two_headless_runs() {
    // Fire at the prompt, as the app's scripts press it, so the title's music is in the 300.
    let mut inputs = vec![FrameInput::NONE; 300];
    for i in &mut inputs[19..29] {
        i.joy.0 = common::FIRE;
    }
    let (a, b) = (headless(inputs.clone()), headless(inputs));
    assert_eq!(a.len(), 300);
    for (n, (a, b)) in a.iter().zip(&b).enumerate() {
        assert_eq!(a.samples, b.samples, "frame {}", n + 1);
    }
    assert!(
        a.iter().any(|h| loud(&h.samples)),
        "the title is not silent"
    );
}

#[test]
fn the_apps_sound_over_the_title_is_heard_and_equals_the_headless_runs() {
    let mut app = App::launch_translated(LaunchOpts {
        max_frames: TITLE,
        timeout_s: 180,
        extra_args: vec!["--capture-hashes".into()],
        ..Default::default()
    });
    let c = app.run_script_unread(PAST_THE_PROMPT);
    assert!(c.exit.success(), "{}", c.stderr);
    let app = captured(&c);
    assert_eq!(app.len() as u64, TITLE, "a line of audio-frames.txt a step");
    let heard = headless(fed(&c, TITLE));
    for (n, h) in (1..=TITLE).zip(&heard) {
        assert_eq!(app[&n], h.samples, "step {n}'s samples");
    }
    let first = (1..=TITLE).find(|n| loud(&app[n]));
    assert!(first.is_some(), "the title is silent in the app");
}

/// Steps run in level 1: a jump at step 100, a slide at 200 (down while running, docs/09's
/// `h0_3ea8`, whose third step starts AUD2 in the original).
const LEVEL_1_STEPS: u64 = 300;

#[test]
fn the_apps_sound_in_level_1_equals_the_headless_runs_frame_for_frame() {
    let mut app = App::launch_translated(LaunchOpts {
        max_frames: LEVEL_1_STEPS,
        timeout_s: 180,
        extra_args: vec!["--capture-hashes".into(), "--level".into(), "1".into()],
        ..Default::default()
    });
    let c = app.run_script_unread(
        "100: down Space\n110: up Space\n180: down ArrowRight\n200: down ArrowDown\n230: up ArrowDown\n240: up ArrowRight\n",
    );
    assert!(c.exit.success(), "{}", c.stderr);
    let headless_frames: u64 = c
        .stderr
        .lines()
        .find_map(|l| l.strip_prefix("fd-app: level 1 reached headless after "))
        .and_then(|l| l.split(' ').next())
        .unwrap_or_else(|| panic!("{}", c.stderr))
        .parse()
        .unwrap();
    // The app's way to level 1, mixed and not played, then its steps.
    let mut inputs: Vec<FrameInput> = (0..headless_frames).map(fd_app::to_level_1_input).collect();
    inputs.extend(fed(&c, LEVEL_1_STEPS));
    let heard = headless(inputs);
    let (to_1, play) = heard.split_at(headless_frames as usize);
    assert!(
        to_1.last().unwrap().level_1 && play.iter().all(|h| h.level_1),
        "in level 1 throughout"
    );
    let app = captured(&c);
    assert_eq!(app.len() as u64, LEVEL_1_STEPS);
    for (n, h) in (1..=LEVEL_1_STEPS).zip(play) {
        assert_eq!(app[&n], h.samples, "step {n}'s samples");
        assert!(
            h.writes > 0,
            "step {n}: the sound driver writes every frame of play"
        );
    }
}
