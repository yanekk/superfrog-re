//! The recorder's shared half with a synthetic game: a header of a start room, a stop room and a
//! mark token `X`, written in an order of its own. No real game's type is in sight.

use amiga_chips::Picture;
use amiga_input::{FrameInput, Joy};
use amiga_record::{
    GameRecording, Head, HeaderReader, HeaderWriter, MAX_FRAMES, Recorder, Recording,
    RecordingError, Replay, Replayable, read_zip, replay, zip_bytes,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Toy;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Rooms {
    start: u8,
    stop: Option<u8>,
    marks: Vec<u32>,
}

impl GameRecording for Toy {
    type Header = Rooms;
    type Stop = u8;
    const MAGIC: &'static str = "toy recording";
    const VERSION: &'static str = "1";
    const TOKENS: &'static [&'static str] = &["X"];

    fn write_header(rec: &Recording<Self>, w: &mut HeaderWriter) {
        w.field("start", &rec.start.to_string());
        w.build(&rec.build);
        let seed: Vec<String> = rec.seed.iter().map(|b| format!("{b:02x}")).collect();
        w.field("seed", &seed.join(""));
        w.frames(rec.inputs.len());
        w.field("stop", &rec.stop.map_or("-".into(), |s| s.to_string()));
        w.note(&rec.note);
    }

    fn read_header(
        n: usize,
        version: &str,
        r: &mut HeaderReader<'_>,
    ) -> Result<Head<Self>, RecordingError> {
        if version != "1" {
            return Err(HeaderReader::error(n, format!("version `{version}`")));
        }
        let (n, start) = r.next("start")?;
        let start = start.parse().map_err(|_| HeaderReader::error(n, "start".into()))?;
        let build = r.build()?;
        let (n, seed) = r.next("seed")?;
        let seed = (0..seed.len() / 2)
            .map(|i| u8::from_str_radix(seed.get(2 * i..2 * i + 2).unwrap_or("?"), 16))
            .collect::<Result<_, _>>()
            .map_err(|_| HeaderReader::error(n, "seed".into()))?;
        let frames = r.frames()?;
        let (n, stop) = r.next("stop")?;
        let stop = match stop {
            "-" => None,
            s => Some(s.parse().map_err(|_| HeaderReader::error(n, "stop".into()))?),
        };
        let note = r.note()?;
        Ok(Head {
            build,
            seed,
            frames,
            note,
            header: Rooms {
                start,
                stop,
                marks: Vec::new(),
            },
        })
    }

    fn validate(rec: &Recording<Self>) -> Result<(), String> {
        match rec.marks.last() {
            Some(&m) if m as usize >= rec.inputs.len() => Err("mark past the end".into()),
            _ => Ok(()),
        }
    }

    fn frame_tokens(h: &Rooms, frame: u32) -> Vec<&'static str> {
        match h.marks.binary_search(&frame) {
            Ok(_) => vec!["X"],
            Err(_) => Vec::new(),
        }
    }

    fn read_frame_tokens(
        h: &mut Rooms,
        _: &str,
        frame: u32,
        tokens: &[&str],
    ) -> Result<(), String> {
        match tokens.len() {
            0 => Ok(()),
            1 => {
                h.marks.push(frame);
                Ok(())
            }
            _ => Err("`X` twice".into()),
        }
    }

    fn stop(rec: &mut Recording<Self>, at: u8) {
        rec.stop = Some(at);
    }
}

fn recorded(frames: u32) -> Recording<Toy> {
    let header = Rooms {
        start: 3,
        stop: None,
        marks: Vec::new(),
    };
    let mut r: Recorder<Toy> = Recorder::start(header, vec![0xa5, 0, 0xff], "b 1");
    for f in 0..frames {
        if f == 4 || f == 9 {
            r.recording_mut().unwrap().marks.push(f);
        }
        r.step(match f / 3 % 3 {
            0 => FrameInput::NONE,
            1 => FrameInput::joy(Joy::RIGHT),
            _ => FrameInput::key(0x19),
        });
    }
    r.finish(7, "a # note".into()).unwrap()
}

#[test]
fn recorder_to_text_to_parse_to_replay_round_trips() {
    let rec = recorded(12);
    assert_eq!(rec.validate(), Ok(()));
    let text = rec.to_text();
    assert_eq!(
        text,
        "toy recording 1\nstart 3\nbuild b 1\nseed a500ff\nframes 12\nstop 7\nnote a # note\n\
         inputs\n0: -\n3: R\n4: R X\n6: K19\n9: X\n"
    );
    let back = Recording::<Toy>::parse(&text).unwrap();
    assert_eq!(back, rec);
    assert_eq!(back.to_text(), text);
    let mut r = Replay::new(&back);
    let fed: Vec<FrameInput> = std::iter::from_fn(|| r.next()).collect();
    assert_eq!(fed, rec.inputs);
    assert_eq!(r.just_finished(), Some(12));
    assert_eq!(r.just_finished(), None);
    assert_eq!(r.recording().stop, Some(7));
}

#[test]
fn the_zip_round_trips_byte_for_byte() {
    let rec = recorded(20);
    let zip = zip_bytes(&rec, b"png").unwrap();
    let (back, png) = read_zip::<Toy>(&zip).unwrap();
    assert_eq!((&back, png.as_slice()), (&rec, b"png".as_slice()));
    assert_eq!(zip_bytes(&back, &png).unwrap(), zip);
    let mut bad = rec.clone();
    bad.marks.push(20);
    assert!(zip_bytes(&bad, b"").unwrap_err().contains("mark past"));
    assert!(read_zip::<Toy>(b"hello").unwrap_err().starts_with("not a bug zip"));
}

#[test]
fn refusals_name_their_line() {
    let good = recorded(12).to_text();
    for (text, line, says) in [
        (good.replace("toy recording 1", "toy recording 2"), 1, "version `2`"),
        (good.replace("frames 12", "frames 4320001"), 5, "longest recording"),
        (good.replace("frames 12", "frames x"), 5, "not a frame count"),
        (good.replace("build b 1\n", ""), 3, "missing `build`"),
        (good.replace("4: R X", "4: R X X"), 11, "`X` twice"),
        (good.replace("4: R X", "4: R Q"), 11, "not an input"),
        (good.replace("inputs\n", "inputs x\n"), 8, "stands alone"),
        (good.replace("6: K19", "3: K19"), 12, "strictly increase"),
        (format!("{good}12: R\n"), 14, "past the recording's 12"),
        (good[..good.find("note").unwrap()].to_string(), 7, "missing `note`"),
    ] {
        let e = Recording::<Toy>::parse(&text).unwrap_err();
        assert_eq!((e.line, e.why.contains(says)), (line, true), "{says}: {e}");
    }
    for cut in 0..good.len() {
        let _ = Recording::<Toy>::parse(&good[..cut]);
    }
}

#[test]
fn a_full_recorder_holds_no_more_than_a_day() {
    let header = Rooms {
        start: 0,
        stop: None,
        marks: Vec::new(),
    };
    let mut r: Recorder<Toy> = Recorder::start(header, Vec::new(), "b");
    assert_eq!(r.frames(), Some(0));
    r.recording_mut().unwrap().inputs = vec![FrameInput::NONE; MAX_FRAMES as usize - 1];
    assert!(!r.full());
    r.step(FrameInput::NONE);
    assert!(r.full());
    r.step(FrameInput::NONE);
    let rec = r.finish(1, String::new()).unwrap();
    assert_eq!(rec.inputs.len(), MAX_FRAMES as usize);
    assert_eq!(rec.validate(), Ok(()));
    let mut over = rec.clone();
    over.inputs.push(FrameInput::NONE);
    assert!(over.validate().unwrap_err().contains("longest recording"));
    let mut off: Recorder<Toy> = Recorder::default();
    off.step(FrameInput::NONE);
    assert_eq!((off.frames(), off.is_recording(), off.full()), (None, false, false));
    assert_eq!(off.finish(0, String::new()), None);
}

/// A game whose picture is one pixel holding the sum of the joystick bits it was stepped with.
struct Counter(u32);

impl Replayable for Counter {
    fn picture(&self) -> Picture {
        let mut p = Picture::blank(0, 0, 1, 1);
        p.pixels[0] = self.0 as u8;
        p
    }
    fn step(&mut self, frame: u32, input: FrameInput) -> Picture {
        self.0 += input.joy.bits() as u32 + frame;
        self.picture()
    }
}

#[test]
fn the_replay_loop_steps_every_input_and_returns_the_last_picture() {
    let inputs = [FrameInput::joy(Joy::RIGHT), FrameInput::NONE, FrameInput::joy(Joy::FIRE)];
    let mut seen = Vec::new();
    let last = replay(&mut Counter(0), &inputs, |k, g, p| seen.push((k, g.0, p.pixels[0])));
    let r = Joy::RIGHT.bits() as u32;
    let f = Joy::FIRE.bits() as u32;
    let sums = [r, r + 1, r + 1 + f + 2];
    assert_eq!(seen, (0..3).map(|k| (k, sums[k as usize], sums[k as usize] as u8)).collect::<Vec<_>>());
    assert_eq!(last.pixels[0], sums[2] as u8);
    assert_eq!(replay(&mut Counter(5), &[], |_, _, _| unreachable!()).pixels[0], 5);
}
