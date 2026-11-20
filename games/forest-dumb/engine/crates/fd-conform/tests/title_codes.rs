//! The title's read held on the original (forest-dumb-title T01, `docs/16-title.md`): every code
//! KODY takes, a wrong one, the rows, Esc, KONIEC, game over through the high scores (with and
//! without a name) back to the title, and the song's first loop. Each run starts from a snapshot of
//! the idle title at boot-to-level1's frame 4990 and plays a committed script
//! (`conform/scripts/title-*.txt`, `game-over-*.txt`), its frames counted from there. A step is
//! named by the frame it starts at, as the scripts name it. Full: the owner's disks and Kickstart.
//!
//! `world_index` and `cheat_flag` are read at the end of the step that enters `load_world`: the
//! step `title_code_world` runs in, and nothing between the two writes either.

use std::path::Path;
use std::sync::OnceLock;

use amiga_conform::script::{self, InputScript};
use amiga_oracle::{Amiga, OracleConfig, Snapshot, kickstart_path};
use fd_conform::{Case, HUNK_BASES, Inputs, conform_dir, disks};
use fd_core::layout_var;

/// The idle title the scripts start from (world-2-start's sync frame).
const START: u32 = 4990;
/// boot-to-level1's disk change.
const SWAP: u32 = 3600;
/// The longest run here: the song's first loop, and game over's way back with room to spare.
const CAP: u32 = 15_200;

const ENTRY: u32 = 0x0;
const LOAD_WORLD: u32 = 0x578;
const QUIT_TO_DOS: u32 = 0x35c;
/// `quit_to_dos`'s `rts`, the return to AmigaDOS.
const QUIT_RTS: u32 = 0x3ca;
const SESSION_LOOP: u32 = 0x158;
/// `wait_vpos_wrap`, which `quit_to_dos` calls, and the routine after it.
const WAIT_VPOS_WRAP: u32 = 0x620;
const WAIT_VPOS_WRAP_END: u32 = 0x62e;
const TITLE_SCREEN: u32 = 0x6e06;
const TITLE_PASS: u32 = 0x6e6e;
const GAME_OVER_SCREEN: u32 = 0x4c4;
/// `high_scores` past its first raster wait, `h0+$89d0` being a loop of its own.
const HIGH_SCORES_IN: u32 = 0x89dc;
const NAME_ENTRY: u32 = 0x8b1a;

/// The high-score entries' first byte; each is `hiscore_name_N`, 12 bytes, then `hiscore_score_N`.
const HISCORE_NAME_1: u32 = 0x84b6;

fn h0(off: u32) -> u32 {
    HUNK_BASES[0] + off
}

fn in_program(pc: u32) -> bool {
    (HUNK_BASES[0]..HUNK_BASES[0] + 0x5be68).contains(&pc)
        || (HUNK_BASES[4]..HUNK_BASES[4] + 0x1c94c).contains(&pc)
}

/// `LAYOUT`'s variable `name` on the oracle: its address and width.
fn at(name: &str) -> (u32, usize) {
    let v = layout_var(name).unwrap_or_else(|| panic!("{name} is not in LAYOUT"));
    (HUNK_BASES[v.hunk as usize] + v.offset, v.width.bytes())
}

fn read(am: &Amiga, addr: u32, len: usize) -> u32 {
    am.peek(addr, len)
        .iter()
        .fold(0, |acc, &b| acc << 8 | b as u32)
}

/// The idle title at `START`, and the stack pointer the entry left after saving the caller's
/// registers: the first line sampled in the program's code in the entry's step.
struct Title {
    snap: Snapshot,
    entry_sp: u32,
    /// The long above the 60 bytes the entry saved: its caller's return address.
    ret: u32,
}

fn title() -> &'static Title {
    static TITLE: OnceLock<Title> = OnceLock::new();
    TITLE.get_or_init(|| {
        let case =
            Case::load(Path::new("cases/world-2-start.toml")).unwrap_or_else(|e| panic!("{e}"));
        let inputs = Inputs::load(&case).unwrap_or_else(|e| panic!("{e}"));
        let mut am = machine();
        am.watch_pcs(&[h0(ENTRY)]);
        am.set_profile(true);
        let mut entry_sp: Option<u32> = None;
        for frame in 0..START {
            if frame == SWAP {
                am.insert_disk(0, 1);
            }
            am.step(inputs.at(frame)).unwrap();
            if entry_sp.is_none() && am.pc_hits().iter().any(|&(_, n)| n > 0) {
                entry_sp = am
                    .line_samples()
                    .iter()
                    .find(|l| in_program(l.pc))
                    .map(|l| l.sp);
                am.set_profile(false);
            }
        }
        let entry_sp = entry_sp.expect("the entry ran before the title");
        Title {
            snap: am.snapshot(),
            entry_sp,
            ret: read(&am, entry_sp + 60, 4),
        }
    })
}

fn machine() -> Amiga {
    let kickstart = kickstart_path().unwrap_or_else(|e| panic!("{e}"));
    let disks = disks::load().unwrap_or_else(|e| panic!("{e}"));
    Amiga::power_on(&OracleConfig::new(kickstart, disks, CAP)).unwrap_or_else(|e| panic!("{e}"))
}

fn load(name: &str) -> InputScript {
    let path = conform_dir().join("scripts").join(format!("{name}.txt"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    script::parse(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// A run from the idle title on a script, watching `pcs` (hunk 0 offsets).
struct Run {
    am: Amiga,
    script: InputScript,
    pcs: Vec<u32>,
}

impl Run {
    fn new(name: &str, pcs: &[u32]) -> Run {
        let mut am = machine();
        am.restore(&title().snap);
        assert_eq!(am.frame(), START);
        let pcs: Vec<u32> = pcs.to_vec();
        am.watch_pcs(&pcs.iter().map(|&p| h0(p)).collect::<Vec<_>>());
        Run {
            am,
            script: load(name),
            pcs,
        }
    }

    /// Run the step starting at the next frame; the frame it started at.
    fn step(&mut self) -> u32 {
        let frame = self.am.frame();
        let input = script::input_at(&self.script.lines, frame - START);
        self.am.step(input).unwrap();
        frame
    }

    fn hit(&self, off: u32) -> bool {
        self.am
            .pc_hits()
            .iter()
            .any(|&(pc, n)| pc == h0(off) && n > 0)
    }

    /// Steps until one runs `off`; that step's frame.
    fn until(&mut self, off: u32, by: u32) -> u32 {
        assert!(self.pcs.contains(&off));
        while self.am.frame() < by {
            let frame = self.step();
            if self.hit(off) {
                return frame;
            }
        }
        panic!("h0+${off:x} not run by frame {by}");
    }

    fn var(&self, name: &str) -> u32 {
        let (addr, len) = at(name);
        read(&self.am, addr, len)
    }
}

/// A code script to `load_world`: (its step, `world_index`, `cheat_flag`, the digits).
fn code(name: &str, cheat_before: u16) -> (u32, u32, u32, u32) {
    let mut run = Run::new(name, &[LOAD_WORLD]);
    run.am.poke(at("cheat_flag").0, &cheat_before.to_be_bytes());
    let frame = run.until(LOAD_WORLD, START + 300);
    (
        frame,
        run.var("world_index"),
        run.var("cheat_flag"),
        run.var("code_digits"),
    )
}

#[test]
fn each_code_starts_its_world() {
    // (script, world_index, cheat_flag, the digits as stored): measured 2026-11-20.
    for (name, world, cheat, digits) in [
        ("title-code-w1", 0, 0, 0x0005_0506),
        ("title-code-w3", 2, 0, 0x0204_0204),
        ("title-code-w5", 4, 0, 0x0705_0301),
        ("title-code-cheat", 0, 1, 0x0204_0000),
    ] {
        assert_eq!(
            code(name, 0),
            (START + 138, world, cheat, digits),
            "{name}: (load_world's step, world_index, cheat_flag, code_digits)"
        );
    }
    // A code clears a cheat an earlier session left.
    assert_eq!(code("title-code-w3", 1).2, 0, "2424 clears cheat_flag");
}

#[test]
fn a_wrong_code_starts_world_1_and_keeps_the_cheat() {
    assert_eq!(
        code("title-code-wrong", 0),
        (START + 138, 0, 0, 0x0102_0304)
    );
    assert_eq!(
        code("title-code-wrong", 1),
        (START + 138, 0, 1, 0x0102_0304)
    );
}

#[test]
fn the_rows_move_on_release_and_wrap() {
    let mut run = Run::new("title-rows", &[]);
    // The step each change is first seen at the end of, and the row it leaves.
    let want = [
        (5020, 1),
        (5060, 2),
        (5072, 0),
        (5100, 2),
        (5120, 1),
        (5140, 0),
    ];
    let mut seen = Vec::new();
    let mut row = run.var("title_cursor");
    assert_eq!(row, 0);
    while run.am.frame() < START + 170 {
        let frame = run.step();
        let now = run.var("title_cursor");
        if now != row {
            seen.push((frame, now));
            row = now;
        }
        assert_eq!(run.var("title_selected"), 0, "step {frame}: no row chosen");
    }
    assert_eq!(seen, want);
}

/// The quit's steps and that it returns to the caller with the stack the entry found: the last line
/// in the program's code has the entry's stack pointer, and the program never runs again.
fn quits(name: &str, quit_step: u32) {
    let mut run = Run::new(name, &[QUIT_TO_DOS, QUIT_RTS, TITLE_PASS, ENTRY]);
    // The return may fall on the next step's first lines, before any is sampled: both steps' lines.
    run.am.set_profile(true);
    while run.am.frame() < quit_step {
        run.step();
    }
    assert_eq!(run.step(), quit_step);
    assert!(
        run.hit(QUIT_TO_DOS),
        "{name}: quit_to_dos in step {quit_step}"
    );
    assert_eq!(run.var("quit_flag"), 1, "{name}: quit_flag");
    let mut lines = run.am.line_samples();
    assert_eq!(run.step(), quit_step + 1);
    assert!(run.hit(QUIT_RTS), "{name}: the return in the next step");
    lines.extend(run.am.line_samples());
    run.am.set_profile(false);
    // The last line in the program's code is in quit_to_dos, at the stack session_loop runs on,
    // which is the entry's, or in the wait_vpos_wrap it calls, one return address deeper.
    let last = lines.iter().rposition(|l| in_program(l.pc)).unwrap();
    let (pc, sp) = (lines[last].pc - HUNK_BASES[0], lines[last].sp);
    let deeper = if (WAIT_VPOS_WRAP..WAIT_VPOS_WRAP_END).contains(&pc) {
        4
    } else {
        assert!(
            (QUIT_TO_DOS..=QUIT_RTS).contains(&pc),
            "{name}: last in h0+${pc:x}"
        );
        0
    };
    assert_eq!(
        sp + deeper,
        title().entry_sp,
        "{name}: the stack at h0+${pc:x}"
    );
    // The caller's return address is still where the entry's caller put it, and the caller runs.
    assert_eq!(
        lines[last].stack[(title().entry_sp + 60 - sp) as usize / 4],
        title().ret
    );
    assert!(
        lines[last + 1].pc >= 0xf8_0000,
        "{name}: then Kickstart's code"
    );
    for _ in 0..50 {
        let frame = run.step();
        assert!(
            !run.hit(TITLE_PASS) && !run.hit(ENTRY),
            "{name}: the program ran again in step {frame}"
        );
    }
}

#[test]
fn esc_quits_in_the_step_it_is_pressed() {
    quits("title-esc", 5010);
}

#[test]
fn koniec_quits_after_fires_release() {
    quits("title-koniec", 5061);
    let mut run = Run::new("title-koniec", &[]);
    while run.am.frame() <= 5060 {
        let frame = run.step();
        assert_eq!(
            run.var("title_selected"),
            (frame == 5060) as u32,
            "step {frame}"
        );
    }
}

/// The steps game over's way passes through, and the high-score table at the title.
fn game_over(name: &str, score: Option<u32>) -> (Vec<u32>, Vec<(String, u32)>) {
    let pcs = [
        LOAD_WORLD,
        GAME_OVER_SCREEN,
        HIGH_SCORES_IN,
        NAME_ENTRY,
        SESSION_LOOP,
        TITLE_SCREEN,
    ];
    let mut run = Run::new(name, &pcs);
    // World 2, as the game over cases play it (world-2-start's `set`).
    run.am
        .poke_at(h0(LOAD_WORLD), at("world_index").0, &1u32.to_be_bytes());
    if let Some(s) = score {
        run.am
            .poke_at(h0(LOAD_WORLD), at("score").0, &s.to_be_bytes());
    }
    let mut steps = Vec::new();
    while !run.hit(TITLE_SCREEN) {
        assert!(
            run.am.frame() < START + 3000,
            "{name}: no title by {}",
            START + 3000
        );
        let frame = run.step();
        for &p in &pcs {
            if run.hit(p) && p != LOAD_WORLD {
                steps.push(frame);
            }
        }
    }
    let table = (0..8)
        .map(|e| {
            // Through LAYOUT's rows, as the game over cases compare it; the raw bytes must agree.
            let b: Vec<u8> = ["0", "4", "8"]
                .iter()
                .flat_map(|o| run.var(&format!("hiscore_name_{}+${o}", e + 1)).to_be_bytes())
                .collect();
            assert_eq!(b, run.am.peek(h0(HISCORE_NAME_1 + 16 * e), 12));
            let name = String::from_utf8_lossy(&b).trim_end().to_string();
            (name, run.var(&format!("hiscore_score_{}", e + 1)))
        })
        .collect();
    (steps, table)
}

const TABLE: [(&str, u32); 8] = [
    ("eagle soft", 0x0020_0000),
    ("hijo", 0x0017_5000),
    ("axel d.", 0x0015_0000),
    ("bald hors", 0x0012_5000),
    ("dan", 0x0010_0000),
    ("k.m.", 0x0007_5000),
    ("comanche", 0x0005_0000),
    ("druid", 0x0002_5000),
];

fn table(last: (&str, u32)) -> Vec<(String, u32)> {
    let mut t: Vec<(String, u32)> = TABLE.iter().map(|&(n, s)| (n.to_string(), s)).collect();
    t[7] = (last.0.to_string(), last.1);
    t
}

#[test]
fn game_over_goes_through_the_high_scores_to_the_title() {
    // game_over_screen, high_scores, session_loop, title_screen.
    let (steps, t) = game_over("game-over-title", None);
    assert_eq!(steps, [6278, 6400, 6500, 7447]);
    assert_eq!(t, table(TABLE[7]), "a score of 0 enters nothing");
}

#[test]
fn a_high_score_types_a_name_into_the_table() {
    // name entry at once with high_scores.
    let (steps, t) = game_over("game-over-name", Some(0x0003_0000));
    assert_eq!(steps, [6278, 6400, 6400, 6600, 7548]);
    assert_eq!(t, table(("ac", 0x0003_0000)));
}

#[test]
fn the_song_first_returns_to_position_0_at_15013() {
    let mut run = Run::new("title-rows", &[]);
    // title-rows does nothing after 150; the rows do not touch the music.
    let pos = |r: &Run| r.var("mt_songpos");
    let mut top = 0;
    loop {
        let frame = run.step();
        let p = pos(&run);
        if p == 0 && top > 0 {
            assert_eq!((frame, top), (15_013, 26));
            break;
        }
        top = top.max(p);
        assert!(frame < 15_100, "no loop by {frame}");
    }
}
