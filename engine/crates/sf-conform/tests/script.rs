//! The input-script parser (T12).

use sf_conform::InputScript;
use sf_core::input::{FrameInput, Joy, KeySet};

#[test]
fn holds_and_releases_last_until_the_next_line() {
    let s = InputScript::parse(
        "# walk right, jump, let go\n\
         120: R F   # right and fire\n\
         150: R\n\
         \n\
         200: -\n",
    )
    .unwrap();
    assert_eq!(s.input_at(0), FrameInput::NONE);
    assert_eq!(s.input_at(119), FrameInput::NONE);
    assert_eq!(s.input_at(120).joy, Joy::RIGHT | Joy::FIRE);
    assert_eq!(s.input_at(149).joy, Joy::RIGHT | Joy::FIRE);
    assert_eq!(s.input_at(150).joy, Joy::RIGHT);
    assert_eq!(s.input_at(200), FrameInput::NONE);
    assert_eq!(s.input_at(1_000_000), FrameInput::NONE);
    assert_eq!(s.lines.len(), 3);
}

#[test]
fn keys_mouse_and_every_direction() {
    let s = InputScript::parse("5: U D L K19 M K45\n6:\n").unwrap();
    let i = s.input_at(5);
    assert_eq!(i.joy, Joy::UP | Joy::DOWN | Joy::LEFT | Joy::PORT1_BUTTON);
    assert_eq!(i.keys, KeySet::key(0x19).with(0x45));
    assert_eq!(InputScript::format_input(i), "U D L M K19 K45");
    assert_eq!(s.input_at(6), FrameInput::NONE);
}

#[test]
fn comments_and_blank_lines_are_ignored() {
    let s = InputScript::parse("\n   # only a comment\n\n10: F # trailing\n").unwrap();
    assert_eq!(s.lines, vec![(10, FrameInput::joy(Joy::FIRE))]);
    assert_eq!(
        InputScript::parse("# nothing\n").unwrap(),
        InputScript::idle()
    );
}

#[test]
fn out_of_order_and_repeated_frames_are_rejected_with_the_line() {
    let e = InputScript::parse("10: R\n5: L\n").unwrap_err();
    assert_eq!(e.line, 2);
    assert!(e.why.contains("strictly increase"), "{e}");
    let e = InputScript::parse("10: R\n# x\n10: L\n").unwrap_err();
    assert_eq!(e.line, 3);
}

#[test]
fn a_script_with_only_frame_0() {
    let s = InputScript::parse("0: R").unwrap();
    assert_eq!(s.lines.len(), 1);
    assert!((0..1000).all(|f| s.input_at(f).joy == Joy::RIGHT));
}

#[test]
fn bad_lines_say_why() {
    for (text, says) in [
        ("R F", "expected"),
        ("x: R", "not a frame number"),
        ("1: Q", "not an input"),
        ("1: K80", "00..7F"),
        ("1: K1", "two hex digits"),
        ("-3: R", "not a frame number"),
        ("+3: R", "not a frame number"),
        ("1: K+1", "two hex digits"),
    ] {
        let e = InputScript::parse(text).unwrap_err();
        assert!(e.why.contains(says), "{text}: {e}");
        assert_eq!(e.line, 1);
    }
}

#[test]
fn the_committed_scripts_parse() {
    let dir = sf_conform::scripts_dir();
    let mut n = 0;
    for e in std::fs::read_dir(&dir).unwrap() {
        let p = e.unwrap().path();
        if p.extension().is_some_and(|x| x == "txt") {
            InputScript::load(&p).unwrap_or_else(|e| panic!("{e}"));
            n += 1;
        }
    }
    assert!(n >= 2, "{n} scripts in {}", dir.display());
    for e in std::fs::read_dir(sf_conform::cases_dir()).unwrap() {
        let p = e.unwrap().path();
        sf_conform::Case::load(&p).unwrap_or_else(|e| panic!("{e}"));
    }
}

#[test]
fn case_files_parse_and_refuse_what_they_do_not_know() {
    use sf_conform::{Case, Compare, SyncPoint};
    let c = Case::parse("x", "sync = \"power-on\"\nframes = 3\n").unwrap();
    assert_eq!(
        (c.sync, c.frames, c.compare),
        (SyncPoint::PowerOn, 3, Compare::ALL)
    );
    let c = Case::parse(
        "x",
        "sync = { level = \"W1-1\" }\nframes = 1\ncompare = [\"paula\"]",
    )
    .unwrap();
    assert_eq!(c.sync, SyncPoint::AtLevel("W1-1".parse().unwrap()));
    assert!(c.compare.paula && !c.compare.vars);
    let c = Case::parse(
        "x",
        "sync = { level = \"W1-1\", clock = \"0:03\" }\nframes = 3",
    )
    .unwrap();
    assert_eq!(
        c.sync,
        SyncPoint::Clock {
            level: "W1-1".parse().unwrap(),
            seconds: 3
        }
    );
    let c = Case::parse("x", "sync = { level = \"W1-1\", exit = true }\nframes = 3").unwrap();
    assert_eq!(c.sync, SyncPoint::Exit("W1-1".parse().unwrap()));
    let c = Case::parse(
        "x",
        "sync = \"power-on\"\nframes = 9\n[[exclude]]\nwhat = \"picture\"\nframes = [3, 5]\n\
         note = \"n\"\ntask = \"T1\"\n",
    )
    .unwrap();
    assert_eq!(
        (2..7).map(|f| c.picture_excluded(f)).collect::<Vec<_>>(),
        [false, true, true, true, false]
    );
    for bad in [
        "frames = 3",
        "sync = \"power-on\"",
        "sync = \"power-on\"\nframes = 0",
        "sync = \"power-on\"\nframes = 3\ncolour = 1",
        "sync = \"power-on\"\nframes = 3\ncompare = [\"sound\"]",
        "sync = { level = \"W9-1\" }\nframes = 3",
        "sync = { script = \"boot-to-title\" }\nframes = 3",
        "sync = { level = \"W1-1\", clock = \"0:003\" }\nframes = 3",
        "sync = { level = \"W1-1\", clock = \"0:3\" }\nframes = 3",
        "sync = { level = \"W1-1\", clock = \"10:00\" }\nframes = 3",
        "sync = { level = \"W1-1\", clock = \"0:60\" }\nframes = 3",
        "sync = { level = \"W1-1\", exit = 1 }\nframes = 3",
        "sync = { level = \"W1-1\", clock = \"0:03\", exit = true }\nframes = 3",
        "sync = \"power-on\"\nframes = 3\n[[exclude]]\nwhat = \"picture\"\nnote = \"n\"\ntask = \"T1\"",
        "sync = \"power-on\"\nframes = 3\n[[exclude]]\nwhat = \"picture\"\nframes = [5, 3]\nnote = \"n\"\ntask = \"T1\"",
    ] {
        assert!(Case::parse("x", bad).is_err(), "{bad}");
    }
}

#[test]
fn a_slot_case_names_its_script_and_maybe_a_start() {
    use sf_conform::{Case, SyncPoint};
    let c = Case::parse("s", "sync = { slot = \"slot-none\" }\nframes = 3\n").unwrap();
    assert_eq!(
        c.sync,
        SyncPoint::Slot {
            script: "slot-none".into(),
            start: None
        }
    );
    let c = Case::parse(
        "s",
        "sync = { slot = \"slot-none\", start = \"start-w1-1-late\" }\nframes = 3\n",
    )
    .unwrap();
    assert_eq!(
        c.sync,
        SyncPoint::Slot {
            script: "slot-none".into(),
            start: Some("start-w1-1-late".into())
        }
    );
    assert!(Case::parse("s", "sync = { slot = \"x\", frame = 3 }\nframes = 3\n").is_err());
}
