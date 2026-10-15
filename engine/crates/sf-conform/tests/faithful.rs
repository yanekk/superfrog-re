//! The faithful checks (DESIGN §2.1 checks 1–3, T180): the tolerances on their own, then on the
//! oracle that T120's guards still hold and that the loosening did not blind the checks. The
//! oracle tests need what `runner.rs` needs.

use sf_conform::compare::{
    PassMatch, PassStep, PictureClass, PictureJudge, Side, classify, gameplay_vars, vars,
};
use sf_conform::sound_events::{Paula, SoundMatch};
use sf_conform::{Case, Mode, SoundEvent, Tolerance, sound_events};
use sf_core::Picture;
use sf_core::layout::{AmigaMemory, LAYOUT, Ram, VarId, def};
use sf_oracle::Rgb12Frame;
use sf_paula::PaulaWrite;

const T: Tolerance = Tolerance::OWNER;

/// 32×8 of colour 1 = `$123` in the engine's picture.
fn ours() -> Picture {
    let mut p = Picture {
        left: 0,
        top: 0x2c,
        width: 32,
        height: 8,
        pixels: vec![1; 256],
        palettes: vec![[0; 32]; 8],
        splits: Vec::new(),
    };
    p.palettes.iter_mut().for_each(|c| c[1] = 0x123);
    p
}

fn theirs(colour: u16) -> Rgb12Frame {
    Rgb12Frame {
        left: 0,
        top: 0x2c,
        width: 32,
        height: 8,
        pixels: vec![colour; 256],
        hires_detail: false,
    }
}

/// The original's picture with `n` pixels of a wrong sprite in it.
fn sprite(n: usize) -> Rgb12Frame {
    let mut t = theirs(0x123);
    t.pixels[..n].fill(0xf0f);
    t
}

#[test]
fn a_one_step_colour_difference_on_many_pixels_is_tolerated() {
    // Every pixel one step off in each of R, G and B, for as long as it lasts.
    for (colour, step) in [
        (0x234, true),
        (0x012, true),
        (0x224, true),
        (0x133, true),
        (0x125, false),
    ] {
        let class = classify(&ours(), &theirs(colour), &[], &T);
        match step {
            true => assert_eq!(class, PictureClass::ColourStep(256), "{colour:03x}"),
            false => assert_eq!(class, PictureClass::Glitch(256), "{colour:03x}"),
        }
    }
    let mut judge = PictureJudge::new(T, 100);
    for f in 0..100 {
        judge
            .frame(f, classify(&ours(), &theirs(0x234), &[], &T))
            .unwrap();
    }
    assert_eq!(judge.colour_step.len(), 100);
    assert_eq!(
        classify(&ours(), &theirs(0x123), &[], &T),
        PictureClass::Same
    );
}

#[test]
fn fewer_than_64_pixels_are_tolerated_however_long() {
    assert_eq!(
        classify(&ours(), &sprite(63), &[], &T),
        PictureClass::FewPixels(63)
    );
    assert_eq!(
        classify(&ours(), &sprite(64), &[], &T),
        PictureClass::Glitch(64)
    );
    let mut judge = PictureJudge::new(T, 100);
    for f in 0..100 {
        judge.frame(f, PictureClass::FewPixels(63)).unwrap();
    }
    assert_eq!(judge.few_pixels.len(), 100);
}

#[test]
fn a_wrong_sprite_for_2_frames_is_tolerated_and_counted_for_3_fails() {
    let wrong = classify(&ours(), &sprite(64), &[], &T);
    let mut judge = PictureJudge::new(T, 1000);
    judge.frame(10, wrong).unwrap();
    judge.frame(11, wrong).unwrap();
    judge.frame(12, PictureClass::Same).unwrap();
    judge.frame(13, wrong).unwrap();
    judge.frame(14, wrong).unwrap();
    assert_eq!(judge.glitches, vec![10, 11, 13, 14]);
    let why = judge.frame(15, wrong).unwrap_err();
    assert!(why.contains("3 frames in a row"), "{why}");
    // Rows left out (the panel's) are not counted.
    assert_eq!(
        classify(&ours(), &sprite(64), &[0..2], &T),
        PictureClass::Same
    );
}

#[test]
fn the_glitch_budget_of_1_percent_fails_when_exceeded() {
    let wrong = PictureClass::Glitch(500);
    // 300 frames: 3 tolerated, apart.
    let mut judge = PictureJudge::new(T, 300);
    for f in [5, 50, 100] {
        judge.frame(f, wrong).unwrap();
        judge.frame(f + 1, PictureClass::Same).unwrap();
    }
    let why = judge.frame(200, wrong).unwrap_err();
    assert!(why.contains("1%"), "{why}");
    // A case under 100 frames has no budget at all.
    assert!(PictureJudge::new(T, 99).frame(0, wrong).is_err());
}

fn w(reg: u16, value: u16) -> PaulaWrite {
    PaulaWrite {
        reg,
        value,
        hpos: 0,
        vpos: 0,
    }
}

/// A sample started on channel 2 the way the sound driver does: DMA off, registers, DMA on.
fn start(addr: u32, vol: u16) -> Vec<PaulaWrite> {
    vec![
        w(0x96, 0x0004),
        w(0xC0, (addr >> 16) as u16),
        w(0xC2, addr as u16),
        w(0xC4, 0x400),
        w(0xC6, 254),
        w(0xC8, vol),
        w(0x96, 0x8004),
    ]
}

#[test]
fn sound_events_from_writes() {
    let e = sound_events(&start(0x2_3456, 0xf4fd));
    assert_eq!(
        e,
        vec![
            SoundEvent::Stop { ch: 2 },
            SoundEvent::Start {
                ch: 2,
                addr: 0x2_3456,
                len: 0x400,
                period: 254,
                vol: 64
            }
        ]
    );
    // `clr.w AUD3VOL`'s bus words (T131/T146): $f4fd and $33fc both play 64; $0605 plays 5.
    let mut p = Paula::default();
    p.frame(&[w(0xD8, 0x33fc), w(0x96, 0x8008)]);
    assert_eq!(p.frame(&[w(0xD8, 0xf4fd)]), vec![]);
    assert_eq!(
        p.frame(&[w(0xD8, 0x0605)]),
        vec![SoundEvent::Volume { ch: 3, vol: 5 }]
    );
    // A beam position is not a sound.
    let mut moved = start(0x2_3456, 40);
    moved.iter_mut().for_each(|x| (x.vpos, x.hpos) = (200, 99));
    assert_eq!(sound_events(&moved), sound_events(&start(0x2_3456, 40)));
}

/// Feeds one script of writes per frame to each side; the first difference, if any.
fn compare(theirs: &[Vec<PaulaWrite>], ours: &[Vec<PaulaWrite>]) -> Result<SoundMatch, String> {
    let (mut pt, mut po) = (Paula::default(), Paula::default());
    let mut m = SoundMatch::new(T.sound_frames);
    let none = Vec::new();
    for f in 0..theirs.len().max(ours.len()) + 10 {
        let t = pt.frame(theirs.get(f).unwrap_or(&none));
        let o = po.frame(ours.get(f).unwrap_or(&none));
        m.frame(f as u32, &t, &o)?;
    }
    Ok(m)
}

fn at(frame: usize, writes: Vec<PaulaWrite>) -> Vec<Vec<PaulaWrite>> {
    let mut v = vec![Vec::new(); frame];
    v.push(writes);
    v
}

#[test]
fn sound_events_reordered_in_a_frame_or_shifted_2_frames_match() {
    let theirs = at(5, start(0x2_3456, 40));
    // Registers written in another order around the DMA's start: the same sound.
    let s = start(0x2_3456, 40);
    let reordered = vec![s[0], s[3], s[2], s[1], s[6], s[5], s[4]];
    let m = compare(&theirs, &at(5, reordered)).unwrap();
    assert_eq!((m.matched, m.shifts), (2, 0));
    for shift in [3, 4, 6, 7] {
        let m = compare(&theirs, &at(shift, start(0x2_3456, 40)))
            .unwrap_or_else(|e| panic!("shift to {shift}: {e}"));
        assert_eq!((m.matched, m.shifts), (2, 2), "shift to {shift}");
    }
}

#[test]
fn sound_events_3_frames_apart_or_a_different_sample_fail() {
    let theirs = at(5, start(0x2_3456, 40));
    for shift in [2, 8] {
        let e = compare(&theirs, &at(shift, start(0x2_3456, 40))).unwrap_err();
        assert!(e.contains("channel 2"), "{e}");
    }
    let e = compare(&theirs, &at(5, start(0x2_8000, 40))).unwrap_err();
    assert!(e.contains("$028000"), "{e}");
    // Played at another volume, or not at all.
    assert!(compare(&theirs, &at(5, start(0x2_3456, 41))).is_err());
    assert!(compare(&theirs, &[]).is_err());
    // A sound the original does not make.
    assert!(compare(&[], &theirs).is_err());
}

#[test]
fn state_is_matched_by_pass_count() {
    let mut m = PassMatch::default();
    // On time on both sides: compared at each pass's end.
    assert!(m.frame(0, 1, 1).compare_now);
    assert_eq!(m.frame(1, 0, 0), PassStep::default());
    // The engine's pass ends a frame later: the original's state is held, then compared.
    assert_eq!(
        m.frame(2, 1, 0),
        PassStep {
            hold: Some(Side::Theirs),
            ..PassStep::default()
        }
    );
    assert_eq!(
        m.frame(3, 0, 1),
        PassStep {
            compare_held: true,
            ..PassStep::default()
        }
    );
    // A frame behind for good: each catch-up compares the held state and holds the next.
    assert_eq!(m.frame(4, 0, 1).hold, Some(Side::Ours));
    assert_eq!(
        m.frame(5, 1, 1),
        PassStep {
            compare_held: true,
            hold: Some(Side::Ours),
            ..PassStep::default()
        }
    );
    assert_eq!(m.skews, vec![2, 4, 5]);
    // Two passes apart is a difference.
    assert_eq!(m.frame(6, 0, 1).apart, Some((3, 5)));
    m.level();
    assert!(m.frame(7, 1, 1).compare_now);
    // The original's end of the pass held never seen (it ended two in a frame): not compared.
    assert_eq!(m.frame(8, 0, 1).hold, Some(Side::Ours));
    assert_eq!(
        m.frame(9, 2, 1),
        PassStep {
            compare_now: true,
            ..PassStep::default()
        }
    );
}

/// Memory covering every `LAYOUT` variable, all zero.
fn memory() -> AmigaMemory {
    AmigaMemory {
        chip: vec![0; 512 * 1024],
        slow: vec![0; 512 * 1024],
    }
}

fn probe(m: &AmigaMemory) -> Vec<(VarId, u32)> {
    LAYOUT
        .iter()
        .flat_map(|d| d.pieces())
        .map(|p| (p.id, p.read(m as &dyn Ram)))
        .collect()
}

#[test]
fn a_presentation_variable_differing_is_not_a_state_difference() {
    let case = Case::parse("state", "sync = \"power-on\"\nframes = 1").unwrap();
    let theirs = memory();
    let mut engine = memory();
    engine.chip[def(VarId::LevelCopper).addr as usize + 6] = 0x55;
    engine.chip[def(VarId::PlayfieldBuffer1).addr as usize + 100] = 0x0f;
    let p = probe(&engine);
    assert!(
        vars(LAYOUT, &p, &theirs, &[]).is_some(),
        "frame-exact sees it"
    );
    assert_eq!(gameplay_vars(&case, LAYOUT, &p, &theirs, &[]), None);
    // The frog's x is gameplay.
    let x = def(VarId::FrogX).addr - sf_core::layout::SLOW_BASE;
    engine.slow[x as usize + 1] = 1;
    let (d, t, o) = gameplay_vars(&case, LAYOUT, &probe(&engine), &theirs, &[]).unwrap();
    assert_eq!((d.name, t, o), ("frog_x", 0, 1));
}

/// T120's guard holds in faithful mode: an original held mid-pass past the limit fails the case
/// rather than leaving it green comparing nothing (`items-run-w1-1`'s longest run, 18 from 303).
#[test]
fn a_faithful_case_fails_once_the_original_stays_mid_pass_past_its_limit() {
    let full = Case::load(&sf_conform::cases_dir().join("items-run-w1-1.toml")).unwrap();
    assert_eq!(full.mode, Mode::Faithful);
    let pinned = |frames: u32| Case {
        frames: 330,
        lag_limit: Some(sf_conform::LagLimit {
            frames,
            note: "test".into(),
        }),
        ..full.clone()
    };
    let o = sf_conform::run(&pinned(18)).unwrap_or_else(|d| panic!("{d}"));
    assert_eq!((o.mode, o.longest_mid_pass), (Mode::Faithful, (303, 18)));
    let d = sf_conform::run(&pinned(17)).expect_err("a run past the limit was green");
    assert_eq!((d.frame, d.lag), (303 + 17, Some((303, 17))), "{d}");
}

/// The loosening did not blind the checks: the frog's jump one unit stronger than the original's
/// fails a frog case in faithful mode, on the frog's state at a pass's end.
#[test]
fn a_jump_off_by_one_fails_a_frog_case_faithfully() {
    let case = Case {
        frames: 260,
        ..Case::load(&sf_conform::cases_dir().join("frog-run-w1-1.toml")).unwrap()
    };
    assert_eq!(case.mode, Mode::Faithful);
    sf_conform::run(&case).unwrap_or_else(|d| panic!("unbroken: {d}"));
    let vy = def(VarId::FrogVy).addr;
    let mut broken = None;
    let d = sf_conform::run_with(&case, |f, game| {
        let v = i16::from_be_bytes(game.peek(vy, 2).try_into().unwrap());
        if broken.is_none() && v < 0 {
            broken = Some(f);
            game.poke(vy, &(v - 1).to_be_bytes());
        }
    })
    .expect_err("a stronger jump was faithful");
    let from = broken.expect("the frog never jumped");
    assert!((from..from + 3).contains(&d.frame), "broken at {from}: {d}");
    let (v, _, _) = d.first_var.unwrap_or_else(|| panic!("{d}"));
    assert!(
        v.name.starts_with("frog_") || v.name.starts_with("camera_"),
        "{d}"
    );
}

/// The title music stopped (`music_on` planted 0) fails the title faithfully, on its sound: the
/// events the original's music makes, the engine does not.
#[test]
fn silence_fails_the_title_on_its_sound() {
    let case = Case {
        compare: sf_conform::Compare {
            paula: true,
            ..sf_conform::Compare::default()
        },
        name: "sound-test".into(),
        ..Case::load(&sf_conform::cases_dir().join("title-rng.toml")).unwrap()
    };
    sf_conform::run(&case).unwrap_or_else(|d| panic!("the music alone: {d}"));
    let d = sf_conform::run_with(&case, |f, game| {
        if f == 3 {
            game.poke(sf_core::layout::h0(0x13420), &[0, 0]);
        }
    })
    .expect_err("silence matched the title music");
    let s = d.sound_diff.clone().unwrap_or_else(|| panic!("{d}"));
    assert!(s.contains("channel") && s.contains("the engine"), "{s}");
    assert!((3..=20).contains(&d.frame), "{d}");
}

/// The owner's call (2026-10-15, T180): a period or volume change followed by its channel's stop
/// within 2 frames need not be met (the stop that much sooner leaves it unheard); counted. A
/// change heard for longer must be.
#[test]
fn a_change_just_before_its_channel_stops_need_not_be_met() {
    let mut theirs = at(5, start(0x2_3456, 40));
    theirs.extend([vec![], vec![w(0xC8, 30)], vec![], vec![w(0x96, 0x0004)]]);
    // The engine stops channel 2 a frame early, never turning it down.
    let mut ours = at(5, start(0x2_3456, 40));
    ours.extend([vec![], vec![], vec![w(0x96, 0x0004)]]);
    let m = compare(&theirs, &ours).unwrap();
    assert_eq!((m.matched, m.trailing, m.shifts), (3, 1, 1));
    // Turned down 3 frames before the stop: heard, and missed.
    let mut heard = at(5, start(0x2_3456, 40));
    heard.extend([vec![w(0xC8, 30)], vec![], vec![], vec![w(0x96, 0x0004)]]);
    let mut ours = at(5, start(0x2_3456, 40));
    ours.extend([vec![], vec![], vec![], vec![w(0x96, 0x0004)]]);
    assert!(compare(&heard, &ours).is_err());
}
