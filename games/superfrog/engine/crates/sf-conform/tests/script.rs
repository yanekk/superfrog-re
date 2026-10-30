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
        ("1: C8", "C<entry 0..27>.<digit 0..5>"),
        ("1: C28.0", "C<entry 0..27>.<digit 0..5>"),
        ("1: C8.6", "C<entry 0..27>.<digit 0..5>"),
        ("1: Cx.1", "C<entry 0..27>.<digit 0..5>"),
    ] {
        let e = InputScript::parse(text).unwrap_err();
        assert!(e.why.contains(says), "{text}: {e}");
        assert_eq!(e.line, 1);
    }
}

/// `C<entry>.<k>` is the number key of digit `k` of a `password_table` entry, as the table gives
/// it: 1..9 are raw keys 01..09, 0 is 0A; a table entry that is not digits is an error.
#[test]
fn a_code_digit_is_read_from_the_table() {
    let table = |e: u32, k: u32| match (e, k) {
        (8, 0) => b'7',
        (8, 1) => b'0',
        (3, 5) => b'1',
        _ => b'x',
    };
    let s = InputScript::parse_with("10: C8.0\n11: C8.1 F\n12: C3.5\n", &table).unwrap();
    assert_eq!(s.input_at(10).keys.codes().collect::<Vec<_>>(), vec![0x07]);
    assert_eq!(s.input_at(11).keys.codes().collect::<Vec<_>>(), vec![0x0a]);
    assert_eq!(s.input_at(11).joy, Joy::FIRE);
    assert_eq!(s.input_at(12).keys.codes().collect::<Vec<_>>(), vec![0x01]);
    let e = InputScript::parse_with("1: -\n2: C9.2\n", &table).unwrap_err();
    assert!(e.why.contains("entry 9 is not digits"), "{e}");
    assert_eq!(e.line, 2);
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

/// A case's own limit on frames in a row the original ends mid-pass (T120), with its reason.
#[test]
fn a_lag_limit_pin_needs_its_frames_and_a_note() {
    use sf_conform::{Case, LagLimit};
    let base = "sync = \"power-on\"\nframes = 3\n";
    assert_eq!(Case::parse("x", base).unwrap().lag_limit, None);
    let c = Case::parse(
        "x",
        &format!("{base}lag-limit = {{ frames = 133, note = \"T104\" }}"),
    )
    .unwrap();
    assert_eq!(
        c.lag_limit,
        Some(LagLimit {
            frames: 133,
            note: "T104".into()
        })
    );
    for bad in [
        "lag-limit = 133",
        "lag-limit = { frames = 133 }",
        "lag-limit = { frames = 133, note = \" \" }",
        "lag-limit = { note = \"x\" }",
        "lag-limit = { frames = -1, note = \"x\" }",
        "lag-limit = { frames = 1, note = \"x\", task = \"T1\" }",
    ] {
        assert!(Case::parse("x", &format!("{base}{bad}")).is_err(), "{bad}");
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
    // T201: power-on with an empty df1 connected.
    let c = Case::parse("x", "sync = \"power-on-df1\"\nframes = 3\n").unwrap();
    assert_eq!(c.sync, SyncPoint::PowerOnDf1);
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
    let c = Case::parse("x", "sync = { level = \"WB-1\", lives = 60 }\nframes = 3").unwrap();
    assert_eq!(
        c.sync,
        SyncPoint::Lives {
            level: "WB-1".parse().unwrap(),
            lives: 60
        }
    );
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
        "sync = { level = \"W1-1\", lives = 100 }\nframes = 3",
        "sync = { level = \"W1-1\", lives = \"9\" }\nframes = 3",
        "sync = { level = \"W1-1\", clock = \"0:03\", lives = 9 }\nframes = 3",
        "sync = { level = \"W1-1\", exit = 1 }\nframes = 3",
        "sync = { level = \"W1-1\", clock = \"0:03\", exit = true }\nframes = 3",
        "sync = \"power-on\"\nframes = 3\n[[exclude]]\nwhat = \"picture\"\nnote = \"n\"\ntask = \"T1\"",
        "sync = \"power-on\"\nframes = 3\n[[exclude]]\nwhat = \"picture\"\nframes = [5, 3]\nnote = \"n\"\ntask = \"T1\"",
    ] {
        assert!(Case::parse("x", bad).is_err(), "{bad}");
    }
}

/// T208: a stretch of a long script, its sync point that script's frame N with df1 connected and
/// the case playing the script on from there.
#[test]
fn a_case_plays_a_script_on_from_its_sync_frame() {
    use sf_conform::{Case, SyncPoint};
    let s = InputScript::parse("10: R\n20: F\n30: -\n").unwrap();
    let on = s.from_frame(25);
    assert_eq!(on.input_at(0).joy, Joy::FIRE);
    assert_eq!(on.input_at(4).joy, Joy::FIRE);
    assert_eq!(on.input_at(5), FrameInput::NONE);
    assert_eq!(on.lines.len(), 2);
    assert_eq!(s.from_frame(20).input_at(0).joy, Joy::FIRE);
    assert_eq!(s.from_frame(0).input_at(10).joy, Joy::RIGHT);
    let c = Case::parse(
        "x",
        "sync = { script = \"start-w1-1\", frame = 7000, df1 = true }\n\
         script = \"start-w1-1\"\nscript-from = 7000\nframes = 3\n",
    )
    .unwrap();
    assert_eq!(
        c.sync,
        SyncPoint::AtFrame {
            script: "start-w1-1".into(),
            frame: 7000,
            df1: true
        }
    );
    let whole = InputScript::load(&sf_conform::scripts_dir().join("start-w1-1.txt")).unwrap();
    assert_eq!(c.script, whole.from_frame(7000));
    let c = Case::parse(
        "x",
        "sync = { script = \"start-w1-1\", frame = 3 }\nframes = 3\n",
    )
    .unwrap();
    assert!(matches!(c.sync, SyncPoint::AtFrame { df1: false, .. }));
    for bad in [
        "sync = { script = \"start-w1-1\", frame = 3, df1 = 1 }\nframes = 3",
        "sync = \"power-on\"\nscript = \"start-w1-1\"\nscript-from = \"3\"\nframes = 3",
        "sync = \"power-on\"\nscript = \"start-w1-1\"\nscript-from = -1\nframes = 3",
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

/// `game-whole.txt` holds each leg's route as committed (T206 review): a route re-planned after
/// the whole game was assembled leaves the whole game playing the old one until it is regenerated.
#[test]
fn the_whole_game_splices_each_route_as_committed() {
    let dir = sf_conform::scripts_dir();
    let text = std::fs::read_to_string(dir.join("game-whole.txt")).unwrap();
    let whole = InputScript::parse(&text).unwrap();
    let mut legs = 0;
    for line in text.lines() {
        // `# W3-1 from frame 84090: chain-w3-1 (its frames 0..5175...`
        let Some((start, rest)) = line
            .strip_prefix("# ")
            .and_then(|l| l.split_once(" from frame "))
            .and_then(|(_, r)| r.split_once(": "))
        else {
            continue;
        };
        let start: u32 = start.parse().unwrap();
        let (name, rest) = rest.split_once(" (its frames 0..").unwrap();
        let end: u32 = rest
            .split(|c: char| !c.is_ascii_digit())
            .next()
            .unwrap()
            .parse()
            .unwrap();
        let leg = InputScript::load(&dir.join(format!("{name}.txt"))).unwrap();
        for t in 0..end {
            assert_eq!(
                whole.input_at(start + t),
                leg.input_at(t),
                "{name}'s frame {t} (game-whole's {})",
                start + t
            );
        }
        legs += 1;
    }
    assert_eq!(legs, 26, "W1-1 to the final level");
}
