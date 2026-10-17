//! T202: assembles the whole-game script (`conform/scripts/game-whole.txt`) by running the
//! original from power-on with an empty df1 connected (`power-on-df1`: W6-2..4 play their first map
//! set, as the engine does). START GAME from the title menu, then each level's completion script
//! spliced at the frame that level's first frame plays, found by running the original; between
//! levels nothing is held but a press in each tally's wait (fire takes the points; at the slot
//! leg's tally up, then up and fire, takes the slot machine, whose rounds fire one frame in every
//! 80 until it returns). Each disk the original asks for goes in. It stops at the end of the first
//! frame the main program is in `end_seq_load`, and fails, naming the leg, on game over or a leg
//! that never begins.
//!
//! `whole_game [--out <script.txt>] [--slot <leg>] [--scratch <dir>] [--from <leg>] [--until <leg>]`
//! (`--until`: stop at the end of the frame before `<leg>`'s first)
//!
//! A snapshot of the original at each leg's start goes to `--scratch` (default
//! `$TMPDIR/sf-whole-game`), with the script so far; `--from <leg>` resumes there, so a leg's
//! route can be re-planned without replaying the game before it. `--check <script.txt>` instead
//! replays a script from power-on and says the frame `end_seq_load` is reached on.

use std::path::PathBuf;

use sf_conform::InputScript;
use sf_conform::sync::{Disks, core_memory, main_chain, step_inserting};
use sf_core::input::{FrameInput, Joy};
use sf_core::layout::{AmigaMemory, Ram, h0};
use sf_disk::DiskSlot;
use sf_oracle::{Amiga, OracleConfig, Snapshot};

const LOGIC_RUNNING: u32 = h0(0x9d6);
const LEVEL_STATE: u32 = h0(0xa06);
const FROG_LIVES: u32 = h0(0x82e);
const SCORE: u32 = h0(0x834);
const WORLD: u32 = h0(0x40da);
const LEVEL: u32 = h0(0x40de);
/// Set by `world_load_lw` and `level_init_lw` for the final level.
const FINAL_LEVEL_FLAG: u32 = h0(0x1e);
/// `level_tally`'s wait for the choice (`sync.rs`'s `TALLY_WAIT`), and its slot-picked flag.
const TALLY_WAIT: std::ops::Range<u32> = h0(0x467e)..h0(0x46e2);
const TALLY_SLOT: u32 = h0(0x4798);
const IN_SLOT_MACHINE: u32 = h0(0x478e);
/// `level_exit_fade`'s return address from `game_over_seq`.
const IN_GAME_OVER: u32 = h0(0x7600);
/// `top_loop`'s call of `end_seq_load` returns here.
const IN_END_SEQ_LOAD: u32 = h0(0x7b6);
/// With df1 connected the boot reaches the title this much sooner (`sync.rs`'s `DF1_LEAD`).
const DF1_LEAD: u32 = 219;
/// The oracle's frame cap: well past a whole game.
const CAP: u32 = 600_000;
/// Frames a leg may take to begin after the last one's play ended (tally, world end picture,
/// loads, disk swaps, the slot machine).
const BETWEEN: u32 = 12_000;

struct Leg {
    name: &'static str,
    /// As the original numbers it (the bonus level is world 7); `None` for the final level.
    at: Option<(u32, u32)>,
    script: &'static str,
    /// The script ends with its own tally press (`<t>: F`, `<t+4>: -`), left out: the assembler
    /// presses in the tally's wait itself.
    tally_press: bool,
}

const fn leg(name: &'static str, w: u32, l: u32, script: &'static str, tally: bool) -> Leg {
    Leg {
        name,
        at: Some((w, l)),
        script,
        tally_press: tally,
    }
}

const LEGS: [Leg; 26] = [
    leg("W1-1", 1, 1, "complete-w1-1", true),
    leg("W1-2", 1, 2, "chain-w1-2", false),
    leg("W1-3", 1, 3, "chain-w1-3", false),
    leg("W1-4", 1, 4, "chain-w1-4", false),
    leg("W2-1", 2, 1, "chain-w2-1", false),
    leg("W2-2", 2, 2, "chain-w2-2", false),
    leg("W2-3", 2, 3, "chain-w2-3", false),
    leg("W2-4", 2, 4, "chain-w2-4", false),
    leg("W3-1", 3, 1, "chain-w3-1", false),
    leg("W3-2", 3, 2, "chain-w3-2", false),
    leg("W3-3", 3, 3, "enemies-w3-3-complete", true),
    leg("W3-4", 3, 4, "enemies-w3-4-complete", true),
    leg("W4-1", 4, 1, "complete-w4-1", true),
    leg("W4-2", 4, 2, "complete-w4-2", true),
    leg("W4-3", 4, 3, "complete-w4-3", true),
    leg("W4-4", 4, 4, "complete-w4-4", true),
    leg("W5-1", 5, 1, "enemies-w5-1-complete", true),
    leg("W5-2", 5, 2, "enemies-w5-2-complete", true),
    leg("W5-3", 5, 3, "enemies-w5-3-complete", true),
    leg("W5-4", 5, 4, "enemies-w5-4-complete", true),
    leg("WB-1", 7, 1, "chain-bonus-to-w6-1", false),
    leg("W6-1", 6, 1, "enemies-w6-1-complete", false),
    leg("W6-2", 6, 2, "enemies-w6-2-complete", false),
    leg("W6-3", 6, 3, "enemies-w6-3-complete", false),
    leg("W6-4", 6, 4, "enemies-w6-4-complete", false),
    Leg {
        name: "WW-1",
        at: None,
        script: "final-win",
        tally_press: false,
    },
];

fn word(m: &AmigaMemory, a: u32) -> u32 {
    (m.byte(a) as u32) << 8 | m.byte(a + 1) as u32
}

fn long(m: &AmigaMemory, a: u32) -> u32 {
    word(m, a) << 16 | word(m, a + 2)
}

fn in_level(m: &AmigaMemory) -> bool {
    word(m, LOGIC_RUNNING) != 0 && (long(m, LEVEL_STATE) as i32) < 0
}

/// Whether the original's frame just ended is `leg`'s first (its level's loop running).
fn begun(leg: &Leg, m: &AmigaMemory) -> bool {
    if !in_level(m) {
        return false;
    }
    let fin = word(m, FINAL_LEVEL_FLAG) != 0;
    match leg.at {
        Some((w, l)) => !fin && long(m, WORLD) == w && long(m, LEVEL) == l,
        None => fin,
    }
}

/// A leg's script and the frame its play ends on (its own tally press left out).
fn leg_script(leg: &Leg) -> (InputScript, u32) {
    let mut s = InputScript::load(&sf_conform::case::script_path(leg.script))
        .unwrap_or_else(|e| panic!("{e}"));
    if leg.tally_press {
        let n = s.lines.len();
        let ok = n >= 3
            && s.lines[n - 2].1 == FrameInput::joy(Joy::FIRE)
            && s.lines[n - 1].1 == FrameInput::NONE
            && s.lines[n - 2].0 - s.lines[n - 3].0 >= 30;
        assert!(ok, "{}: does not end with its tally's fire", leg.script);
        s.lines.truncate(n - 2);
    }
    let end = s.lines.last().map_or(0, |l| l.0 + 1);
    (s, end)
}

fn power_on() -> Amiga {
    let cfg = OracleConfig::locate(CAP)
        .unwrap_or_else(|e| panic!("{e}"))
        .with_boot(DiskSlot::D1)
        .with_drives(2);
    Amiga::power_on(&cfg).unwrap_or_else(|e| panic!("{e}"))
}

/// START GAME from the title, every press from the title on [`DF1_LEAD`] frames sooner.
fn menu() -> InputScript {
    let mut s = InputScript::load(&sf_conform::case::script_path("start-w1-1"))
        .unwrap_or_else(|e| panic!("{e}"));
    for l in &mut s.lines {
        if l.0 >= 4600 {
            l.0 -= DF1_LEAD;
        }
    }
    s
}

/// What the assembler holds between legs, from the original's state at the last frame's end.
struct Presser {
    /// Frames left of the press being held, and the frames since the last one ended.
    held: u32,
    since: u32,
    /// Frames in the slot machine.
    slot_frames: u32,
}

impl Presser {
    fn input(&mut self, a: &Amiga, slot_leg: bool) -> FrameInput {
        let m = core_memory(a.memory());
        let chain = main_chain(&a.cpu(), &m);
        let in_slot = chain.contains(&IN_SLOT_MACHINE);
        if in_slot {
            self.slot_frames += 1;
            // One frame of fire in every 80 (`slot-fire`'s rounds).
            return if self.slot_frames % 80 == 0 {
                FrameInput::joy(Joy::FIRE)
            } else {
                FrameInput::NONE
            };
        }
        if self.held > 0 {
            self.held -= 1;
            let up = slot_leg && word(&m, TALLY_SLOT) == 1;
            return FrameInput::joy(if up { Joy(Joy::UP.0 | Joy::FIRE.0) } else { Joy::FIRE });
        }
        self.since += 1;
        if !TALLY_WAIT.contains(&chain[0]) {
            return FrameInput::NONE;
        }
        if slot_leg && word(&m, TALLY_SLOT) != 1 {
            // Up lights the slot machine's line (as `SyncPoint::Slot`).
            self.since = 0;
            return FrameInput::joy(Joy::UP);
        }
        if self.since < 8 {
            return FrameInput::NONE;
        }
        self.held = 3;
        self.since = 0;
        FrameInput::joy(if slot_leg { Joy(Joy::UP.0 | Joy::FIRE.0) } else { Joy::FIRE })
    }
}

struct Args {
    out: PathBuf,
    slot: String,
    scratch: PathBuf,
    from: Option<String>,
    check: Option<PathBuf>,
    diff: Option<String>,
    ablate: bool,
    until: Option<String>,
}

fn args() -> Args {
    let mut a = Args {
        out: sf_conform::scripts_dir().join("game-whole.txt"),
        slot: "W1-1".into(),
        scratch: std::env::temp_dir().join("sf-whole-game"),
        from: None,
        check: None,
        diff: None,
        ablate: false,
        until: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        if k == "--ablate" {
            a.ablate = true;
            continue;
        }
        let v = it.next().unwrap_or_else(|| panic!("{k} takes a value"));
        match k.as_str() {
            "--out" => a.out = v.into(),
            "--slot" => a.slot = v,
            "--scratch" => a.scratch = v.into(),
            "--from" => a.from = Some(v),
            "--check" => a.check = Some(v.into()),
            "--diff" => a.diff = Some(v),
            "--until" => a.until = Some(v),
            _ => panic!("unknown argument {k}"),
        }
    }
    a
}

/// The frame inputs so far, as script lines with the legs' comments.
struct Record {
    inputs: Vec<FrameInput>,
    notes: Vec<(u32, String)>,
}

impl Record {
    fn text(&self) -> String {
        let mut t = String::new();
        let mut notes = self.notes.iter().peekable();
        let mut last: Option<FrameInput> = None;
        for (f, &i) in self.inputs.iter().enumerate() {
            let f = f as u32;
            let mut noted = false;
            while let Some((_, n)) = notes.next_if(|(at, _)| *at <= f) {
                t += &format!("# {n}\n");
                noted = true;
            }
            if last != Some(i) || noted && f > 0 {
                t += &format!("{f}: {}\n", InputScript::format_input(i));
                last = Some(i);
            }
        }
        for (_, n) in notes {
            t += &format!("# {n}\n");
        }
        t
    }

    fn save(&self, path: &std::path::Path) {
        let mut b = Vec::new();
        for i in &self.inputs {
            b.push(i.joy.bits());
        }
        std::fs::write(path.with_extension("inputs"), b).unwrap();
        let notes: String = self.notes.iter().map(|(f, n)| format!("{f}\t{n}\n")).collect();
        std::fs::write(path.with_extension("notes"), notes).unwrap();
    }

    fn load(path: &std::path::Path) -> Record {
        let b = std::fs::read(path.with_extension("inputs")).unwrap();
        let inputs = b
            .into_iter()
            .map(|j| FrameInput::joy(Joy(j)))
            .collect();
        let notes = std::fs::read_to_string(path.with_extension("notes"))
            .unwrap()
            .lines()
            .map(|l| {
                let (f, n) = l.split_once('\t').unwrap();
                (f.parse().unwrap(), n.to_string())
            })
            .collect();
        Record { inputs, notes }
    }
}

fn save_snapshot(path: &std::path::Path, s: &Snapshot) {
    sf_conform::sync::save_snapshot_file(path, s).unwrap();
}

fn load_snapshot(path: &std::path::Path) -> Snapshot {
    sf_conform::sync::load_snapshot_file(path).unwrap_or_else(|e| panic!("{e}"))
}

fn status(m: &AmigaMemory) -> String {
    format!(
        "lives {}, score {}",
        word(m, FROG_LIVES) as i16,
        long(m, SCORE)
    )
}

fn check(path: &std::path::Path) {
    let s = InputScript::load(path).unwrap_or_else(|e| panic!("{e}"));
    let mut a = power_on();
    let mut disks: Option<Disks> = None;
    for f in 0..CAP {
        step_inserting(&mut a, s.input_at(f)).unwrap_or_else(|e| panic!("frame {f}: {e}"));
        let m = core_memory(a.memory());
        disks.get_or_insert_with(|| Disks::at(&m)).serve(&mut a);
        let chain = main_chain(&a.cpu(), &m);
        if chain.contains(&IN_GAME_OVER) {
            println!("game over at frame {f} ({})", status(&m));
            std::process::exit(1);
        }
        if chain.contains(&IN_END_SEQ_LOAD) {
            println!("end_seq_load at frame {f} ({})", status(&m));
            return;
        }
    }
    println!("end_seq_load not reached in {CAP} frames");
    std::process::exit(1);
}

/// `--diff <leg>`: the leg's script played from its start in the chain (the snapshot `--scratch`
/// holds) and from its own sync point (`AtLevel`), side by side: the `LAYOUT` variables that differ
/// at its first frame, then the first frame the frog's position differs and what differs then.
fn diff(args: &Args, name: &str) {
    use sf_core::frog::{FROG, f};
    let i = LEGS.iter().position(|l| l.name == name).unwrap_or_else(|| panic!("no leg {name}"));
    let leg = &LEGS[i];
    let (s, end) = leg_script(leg);
    let mut chain = power_on();
    chain.restore(&load_snapshot(&args.scratch.join(format!("leg-{i:02}.snap"))));
    let id: sf_core::LevelId = name.parse().unwrap();
    let mut fresh = sf_conform::sync::reach(
        &sf_conform::SyncPoint::AtLevel(id),
        &InputScript::idle(),
        end + 100,
    )
    .unwrap_or_else(|e| panic!("{e}"))
    .amiga;
    let differ = |a: &Amiga, b: &Amiga| -> Vec<String> {
        let (ma, mb) = (core_memory(a.memory()), core_memory(b.memory()));
        sf_core::layout::LAYOUT
            .iter()
            .filter(|v| v.width.len() <= 64)
            .filter(|v| (0..v.width.len()).any(|k| ma.byte(v.addr + k) != mb.byte(v.addr + k)))
            .map(|v| v.name.to_string())
            .collect()
    };
    println!("at the start: {:?}", differ(&chain, &fresh));
    for v in sf_core::layout::LAYOUT.iter().filter(|v| v.width.len() <= 8) {
        let val = |a: &Amiga| {
            let m = core_memory(a.memory());
            (0..v.width.len()).map(|k| format!("{:02x}", m.byte(v.addr + k))).collect::<String>()
        };
        if val(&chain) != val(&fresh) {
            println!("  {}: chain {} fresh {}", v.name, val(&chain), val(&fresh));
        }
    }
    let pos = |a: &Amiga| {
        let m = core_memory(a.memory());
        (word(&m, FROG + f::X), word(&m, FROG + f::Y))
    };
    let (chain0, fresh0) = (chain.snapshot(), fresh.snapshot());
    // The fresh frog's path, and the frame each run's path parts from it (`end` if never).
    let mut path = Vec::new();
    for k in 0..end {
        step_inserting(&mut fresh, s.input_at(k)).unwrap();
        path.push(pos(&fresh));
    }
    let parts = |a: &mut Amiga| -> u32 {
        for k in 0..end {
            step_inserting(a, s.input_at(k)).unwrap();
            if pos(a) != path[k as usize] {
                return k;
            }
        }
        end
    };
    let k = parts(&mut chain);
    if k == end {
        println!("the frog's path is the same to {end}");
        return;
    }
    fresh.restore(&fresh0);
    for j in 0..=k {
        step_inserting(&mut fresh, s.input_at(j)).unwrap();
    }
    println!("frame {k}: frog {:?} in the chain, {:?} fresh", pos(&chain), path[k as usize]);
    println!("  differ: {:?}", differ(&chain, &fresh));
    if !args.ablate {
        return;
    }
    // `--ablate`: which differences decide it. Each variable differing at the start (any width)
    // set to the fresh value alone in the chain's snapshot, then every one but it, and the frame
    // the path parts then.
    fresh.restore(&fresh0);
    let fm = core_memory(fresh.memory());
    let cm = core_memory({
        chain.restore(&chain0);
        chain.memory()
    });
    let vars: Vec<_> = sf_core::layout::LAYOUT
        .iter()
        .filter(|v| (0..v.width.len()).any(|k| fm.byte(v.addr + k) != cm.byte(v.addr + k)))
        .collect();
    let value = |v: &sf_core::layout::VarDef| -> Vec<u8> {
        (0..v.width.len()).map(|k| fm.byte(v.addr + k)).collect()
    };
    let run = |set: &[&&sf_core::layout::VarDef]| -> u32 {
        let mut a = power_on();
        a.restore(&chain0);
        for v in set {
            a.poke(v.addr, &value(v));
        }
        parts(&mut a)
    };
    let all: Vec<_> = vars.iter().collect();
    println!("all {} set fresh: parts at {}", all.len(), run(&all));
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let next = std::sync::atomic::AtomicUsize::new(0);
    let mut res: Vec<(usize, u32, u32)> = std::thread::scope(|sc| {
        let hs: Vec<_> = (0..threads)
            .map(|_| {
                sc.spawn(|| {
                    let mut out = Vec::new();
                    loop {
                        let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some(v) = vars.get(i) else { return out };
                        let but: Vec<_> =
                            vars.iter().enumerate().filter(|(j, _)| *j != i).map(|(_, v)| v).collect();
                        out.push((i, run(&[v]), run(&but)));
                    }
                })
            })
            .collect();
        hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
    });
    res.sort();
    let mut deciding = Vec::new();
    for &(i, alone, but) in &res {
        println!("  {:<20} alone: {alone:>5}   all but it: {but:>5}", vars[i].name);
        if but < end || alone > k {
            deciding.push(&vars[i]);
        }
    }
    let names: Vec<_> = deciding.iter().map(|v| v.name).collect();
    println!("{names:?} alone: parts at {}", run(&deciding));
}

fn main() {
    let args = args();
    if let Some(p) = &args.check {
        return check(p);
    }
    if let Some(leg) = &args.diff {
        return diff(&args, leg);
    }
    std::fs::create_dir_all(&args.scratch).unwrap();
    let t0 = std::time::Instant::now();
    let mut a = power_on();
    let mut first = 0;
    let mut rec = Record {
        inputs: Vec::new(),
        notes: vec![(
            0,
            "START GAME from the title (start-w1-1, with df1 connected every press from the \
             title on 219 frames sooner)"
                .into(),
        )],
    };
    if let Some(from) = &args.from {
        first = LEGS
            .iter()
            .position(|l| l.name == from)
            .unwrap_or_else(|| panic!("no leg {from}"));
        let p = args.scratch.join(format!("leg-{first:02}"));
        a.restore(&load_snapshot(&p.with_extension("snap")));
        rec = Record::load(&p);
        println!("resumed at {from}, frame {}", rec.inputs.len());
    }
    let mut disks = Disks::at(&core_memory(a.memory()));
    let menu = menu();
    // The leg being played: (its index, the frame its first frame is, its script, its play's end).
    let mut playing: Option<(usize, u32, InputScript, u32)> = None;
    let mut next = first;
    // When `next` was due to begin by.
    let mut due = rec.inputs.len() as u32 + if first == 0 { 20_000 } else { BETWEEN };
    let mut presser = Presser {
        held: 0,
        since: 99,
        slot_frames: 0,
    };
    let mut was_in_level = false;
    if args.from.is_some() {
        // The snapshot is the end of the frame the leg was seen to begin on: its script plays
        // from the next frame on, with no frame stepped first (which put it a frame late).
        let leg = &LEGS[first];
        let start = rec.inputs.len() as u32;
        let (s, end) = leg_script(leg);
        if rec.notes.last().is_some_and(|n| n.0 == start) {
            rec.notes.pop();
        }
        let m = core_memory(a.memory());
        rec.notes.push((start, leg_note(leg, start, end, &m)));
        println!("{}", rec.notes.last().unwrap().1);
        playing = Some((first, start, s, end));
        next = first + 1;
        due = start + end + BETWEEN;
        was_in_level = true;
    }
    loop {
        let f = rec.inputs.len() as u32;
        if f >= CAP {
            panic!("end_seq_load not reached in {CAP} frames");
        }
        let input = match &playing {
            Some((_, start, s, end)) if f - start < *end => s.input_at(f - start),
            Some((i, ..)) => presser.input(&a, LEGS[*i].name == args.slot),
            None => menu.input_at(f),
        };
        step_inserting(&mut a, input).unwrap_or_else(|e| panic!("oracle, frame {f}: {e}"));
        disks.serve(&mut a);
        rec.inputs.push(input);
        let m = core_memory(a.memory());
        let chain = main_chain(&a.cpu(), &m);
        let now_in_level = in_level(&m);
        if was_in_level && !now_in_level && word(&m, LOGIC_RUNNING) == 0 {
            if let Some((i, start, ..)) = &playing {
                println!("  {} loop ended at its frame {} ({})", LEGS[*i].name, f - start, status(&m));
            }
        }
        was_in_level = now_in_level;
        if chain.contains(&IN_GAME_OVER) {
            let leg = playing.as_ref().map_or("the menu", |p| LEGS[p.0].name);
            let note = format!("game over in {leg} at the end of frame {f} ({})", status(&m));
            println!("{note}");
            rec.notes.push((f, note));
            write(&args.out, &rec);
            std::process::exit(1);
        }
        if next < LEGS.len() && begun(&LEGS[next], &m) {
            let leg = &LEGS[next];
            let start = f + 1;
            let (s, end) = leg_script(leg);
            if args.until.as_deref() == Some(leg.name) {
                let note = format!(
                    "{} begins at frame {start} ({}); the script stops here (--until)",
                    leg.name,
                    status(&m)
                );
                println!("{note}  [{:.0?}]", t0.elapsed());
                rec.notes.push((start, note));
                break;
            }
            let note = leg_note(leg, start, end, &m);
            println!("{note}  [{:.0?}]", t0.elapsed());
            rec.notes.push((start, note));
            let p = args.scratch.join(format!("leg-{next:02}"));
            save_snapshot(&p.with_extension("snap"), &a.snapshot());
            rec.save(&p);
            playing = Some((next, start, s, end));
            presser = Presser {
                held: 0,
                since: 99,
                slot_frames: 0,
            };
            next += 1;
            due = start + end + BETWEEN;
            continue;
        }
        if chain.contains(&IN_END_SEQ_LOAD) {
            let note = format!(
                "end_seq_load reached at the end of frame {f} ({}); nothing held after",
                status(&m)
            );
            println!("{note}  [{:.0?}]", t0.elapsed());
            rec.notes.push((f, note));
            break;
        }
        if f > due {
            let name = LEGS.get(next).map_or("end_seq_load", |l| l.name);
            let note = format!("{name} did not begin by frame {due} ({})", status(&m));
            println!("{note}");
            rec.notes.push((f, note));
            write(&args.out, &rec);
            std::process::exit(1);
        }
    }
    write(&args.out, &rec);
}

fn leg_note(leg: &Leg, start: u32, end: u32, m: &AmigaMemory) -> String {
    format!(
        "{} from frame {start}: {} (its frames 0..{end}{}), {}",
        leg.name,
        leg.script,
        if leg.tally_press { ", its own tally press left out" } else { "" },
        status(m)
    )
}

/// The script as far as it was assembled.
fn write(out: &std::path::Path, rec: &Record) {
    let head = "# The whole game (T202), assembled by `examples/whole_game` on the original from power-on with an\n\
                # empty df1 connected (`power-on-df1`): each level's route spliced at the frame its first frame\n\
                # plays (a `chain-*` route was planned from the original's state there, T205); between levels\n\
                # nothing held but a press in each tally's wait (fire; at the slot leg's tally up, then up and\n\
                # fire, and in the slot machine one frame of fire in every 80).\n";
    std::fs::write(out, format!("{head}{}", rec.text())).unwrap();
    println!("wrote {}", out.display());
}
