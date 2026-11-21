//! What `title_paths.rs` and `title_paths_short.rs` share (title plan T10, DESIGN §2.2, §2.6): the
//! title's paths played in the real app by key presses, its state equal to a headless run of the
//! translated program every step.
//!
//! A [`Run`] is the headless run (`fd_conform`'s `TranslatedEngine` from power-on, level 1 mounted,
//! as the app runs it): a test steps it phase by phase, choosing each step's input, and reads the
//! title's variables after every step to judge what the path did against `docs/16-title.md`. Then
//! [`Run::in_the_app`] feeds the same inputs to the app as host key presses through `--script` and
//! holds every step's input and state hash to the run's; the first difference fails naming the
//! step, both hashes and the run's variables there.
//!
//! The Amiga's Esc and HELP have no free key on the Mac (fd-app's Esc quits), so [`SETTINGS`] binds
//! them to F11 and F12, keys that are no Amiga key, so a press feeds the one Amiga key alone.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use amiga_input::{FrameInput, Joy, KeySet};
use fd_conform::FrameEngine;
use fd_conform::translated_engine::TranslatedEngine;
use fd_e2e::{App, Captured, Image, LaunchOpts};
use fd_translated::TranslatedGame;

use crate::common;

/// Chip RAM's size, where slow RAM starts in the engine's memory.
const CHIP: usize = 512 << 10;
/// Where slow RAM starts in the A500's address space.
const SLOW_BASE: usize = 0xC0_0000;

/// Fire at the disk 2 prompt, as `translated_app.rs` presses it: steps 20 to 29.
const PROMPT: std::ops::Range<u64> = 20..30;
/// The title is up and taking input from about step 184 (translator T37); the paths start here.
pub const TITLE: u64 = 300;
/// Steps after START before fire is pulsed for the SCENA screen: the title's fire let go first.
const PULSE_AFTER: u64 = 50;

/// The Amiga keys the paths press (raw codes, docs/16).
pub const ESC: u8 = 0x45;
pub const HELP: u8 = 0x5f;
/// Steps HELP is held. Its release byte must arrive before `load_world` hands the machine to the
/// OS (about three steps after the level's end): a later one enters the OS's level 2 handler, which
/// the machine does not model, and the run stops (plan FINDINGS, 2026-11-20; DESIGN §2.4).
const HELP_TAP: u64 = 2;
/// The host keys bound to them in [`SETTINGS`].
const ESC_HOST: &str = "F11";
const HELP_HOST: &str = "F12";
/// Esc and HELP bound as Game ▸ Controls… saves them (`game_over.rs` does the same for Esc).
pub const SETTINGS: &str = "[keys]\n\"key Esc\" = [\"F11\"]\n\"key $5f\" = [\"F12\"]\n";

/// The host key on the default keys for each joystick bit, as `common::key_lines`.
const JOY_KEYS: [(Joy, &str); 5] = [
    (Joy::UP, "Space"),
    (Joy::DOWN, "ArrowDown"),
    (Joy::LEFT, "ArrowLeft"),
    (Joy::RIGHT, "ArrowRight"),
    (Joy::FIRE, "AltLeft"),
];

/// The title's variables a step's failure prints and the tests read (`fd_core::LAYOUT`).
pub const VARS: [&str; 10] = [
    "title_cursor",
    "title_selected",
    "code_digit_index",
    "code_digits",
    "world_index",
    "cheat_flag",
    "quit_flag",
    "irq_count",
    "lives",
    "last_key_byte",
];

pub fn require_tree() {
    assert!(
        TranslatedGame::generated(),
        "no translated program built in: run `python3 -I games/forest-dumb/tools/translate.py --build`"
    );
}

/// A variable of `fd_core::LAYOUT` in RAM as the engine holds it, chip then slow.
pub fn var(m: &[u8], name: &str) -> u32 {
    let v = fd_core::layout_var(name).expect("in LAYOUT");
    let at = fd_core::HUNK_BASES[v.hunk as usize] + v.offset;
    (0..v.width.bytes() as u32).fold(0, |acc, k| {
        let a = ((at + k) & 0xFF_FFFF) as usize;
        let i = match a.checked_sub(SLOW_BASE) {
            Some(s) => CHIP + s,
            None => a,
        };
        acc << 8 | m[i] as u32
    })
}

/// An Amiga key held: its bit in a frame's key set.
pub fn key(code: u8) -> FrameInput {
    FrameInput {
        joy: Joy::NONE,
        keys: KeySet(1u128 << code),
    }
}

/// The raw code of a digit's key (docs/16: keys 1 to 9 are $01 to $09, 0 is $0A).
pub fn digit(d: u8) -> u8 {
    if d == 0 { 0x0A } else { d }
}

/// The host key feeding Amiga key `code` in the app with [`SETTINGS`].
fn host_key(code: u8) -> &'static str {
    match code {
        0x01..=0x0A => [
            "Digit1", "Digit2", "Digit3", "Digit4", "Digit5", "Digit6", "Digit7", "Digit8",
            "Digit9", "Digit0",
        ][code as usize - 1],
        ESC => ESC_HOST,
        HELP => HELP_HOST,
        _ => panic!("no host key chosen for Amiga key ${code:02x}"),
    }
}

/// The headless run of the translated program and what it was fed.
pub struct Run {
    e: TranslatedEngine,
    /// Step `n` (from 1) at index `n - 1`.
    pub inputs: Vec<FrameInput>,
    /// `fd_app::memory_hash` after each step, as the app's `hashes.txt` writes it.
    pub hashes: Vec<String>,
    /// [`VARS`] after each step.
    pub vars: Vec<[u32; VARS.len()]>,
    /// The picture after each step, kept only while [`Run::keep_pictures`] is on.
    pub pictures: BTreeMap<u64, Image>,
    keep_pictures: bool,
    /// The program's exit (KONIEC or Esc), the step it was seen after.
    pub exited: Option<u64>,
    /// The first step after which level 1 is in RAM (`fd_app::in_level_1`), as the app reports
    /// `level 1 begins at step N`.
    pub level_1_begins: Option<u64>,
}

impl Run {
    /// From power-on with level 1 mounted, fire at the disk 2 prompt, idle to [`TITLE`].
    pub fn to_the_title() -> Run {
        require_tree();
        let disks = fd_conform::disks::load().unwrap_or_else(|e| panic!("{e}"));
        let mut e = TranslatedEngine::power_on(&disks).unwrap_or_else(|e| panic!("{e}"));
        e.mount_level_1(common::data());
        let mut r = Run {
            e,
            inputs: Vec::new(),
            hashes: Vec::new(),
            vars: Vec::new(),
            pictures: BTreeMap::new(),
            keep_pictures: false,
            exited: None,
            level_1_begins: None,
        };
        while r.steps() < TITLE {
            let n = r.steps() + 1;
            let i = if PROMPT.contains(&n) {
                FrameInput::joy(Joy::FIRE)
            } else {
                FrameInput::NONE
            };
            r.step(i);
        }
        r
    }

    /// RAM after the last step, chip then slow.
    pub fn ram(&self) -> &[u8] {
        self.e.ram().expect("a frame has run")
    }

    pub fn steps(&self) -> u64 {
        self.inputs.len() as u64
    }

    /// Keep each step's picture from now on (`false` stops).
    pub fn keep_pictures(&mut self, on: bool) {
        self.keep_pictures = on;
    }

    /// One step on `input`. A stop fails, naming the step; the program's own exit ends the run.
    pub fn step(&mut self, input: FrameInput) {
        assert!(self.exited.is_none(), "stepped after the program's exit");
        self.e.frame(input);
        let n = self.steps() + 1;
        self.inputs.push(input);
        if let Some(stop) = self.e.stopped() {
            assert!(
                stop.contains("exit: the program returned from its entry"),
                "the headless run stopped at step {n}: {stop}"
            );
            self.exited = Some(n);
        }
        let m = self.e.ram().expect("a frame has run");
        self.hashes.push(format!(
            "{:016x}",
            fd_app::memory_hash(&m[..CHIP], &m[CHIP..])
        ));
        self.vars.push(VARS.map(|v| var(m, v)));
        if self.level_1_begins.is_none() && fd_app::in_level_1(&m[..CHIP], &m[CHIP..]) {
            self.level_1_begins = Some(n);
        }
        if self.keep_pictures {
            let p = fd_app::canvas(&FrameEngine::picture(&self.e).expect("a frame has run"));
            let mut rgba = vec![0; p.width as usize * p.height as usize * 4];
            fd_core::render::rgba(&p, &mut rgba);
            for px in rgba.chunks_exact_mut(4) {
                px[3] = 255;
            }
            self.pictures.insert(
                n,
                Image {
                    width: p.width as u32,
                    height: p.height as u32,
                    rgba,
                },
            );
        }
    }

    /// `n` steps on `input`.
    pub fn hold(&mut self, input: FrameInput, n: u64) {
        for _ in 0..n {
            self.step(input);
        }
    }

    /// `input` held `down` steps, then nothing for `after`.
    pub fn press(&mut self, input: FrameInput, down: u64, after: u64) {
        self.hold(input, down);
        self.hold(FrameInput::NONE, after);
    }

    /// Steps on `input(n)` until `done` holds after one, at most `limit` steps; the step it held.
    pub fn until(
        &mut self,
        what: &str,
        limit: u64,
        mut input: impl FnMut(u64) -> FrameInput,
        mut done: impl FnMut(&Run) -> bool,
    ) -> u64 {
        let from = self.steps();
        loop {
            let n = self.steps() + 1;
            assert!(
                n <= from + limit,
                "{what}: not by step {} (from step {from})",
                from + limit
            );
            self.step(input(n));
            if done(self) {
                return n;
            }
        }
    }

    /// [`VARS`] entry `name` after step `n`.
    pub fn at(&self, n: u64, name: &str) -> u32 {
        let k = VARS.iter().position(|v| *v == name).expect("in VARS");
        self.vars[n as usize - 1][k]
    }

    /// [`VARS`] entry `name` after the last step.
    pub fn now(&self, name: &str) -> u32 {
        self.at(self.steps(), name)
    }

    /// The steps from `from` on where `name` changed, with its new value.
    pub fn changes(&self, name: &str, from: u64) -> Vec<(u64, u32)> {
        (from.max(2)..=self.steps())
            .filter(|&n| self.at(n, name) != self.at(n - 1, name))
            .map(|n| (n, self.at(n, name)))
            .collect()
    }

    /// The four digits typed on KODY, from the title's START row: Down to KODY, fire to start the
    /// entry, each digit's key held six steps and let go for six (a digit counts on its release),
    /// Up back to START. Checks the entry against docs/16 as it goes; `code_digits` at the end.
    pub fn type_code(&mut self, code: [u8; 4]) -> u32 {
        assert_eq!(self.now("title_cursor"), 0, "typing a code starts on START");
        self.press(FrameInput::joy(Joy::DOWN), 10, 20);
        assert_eq!(self.now("title_cursor"), 1, "Down let go: KODY");
        self.press(FrameInput::joy(Joy::FIRE), 10, 10);
        assert_ne!(
            self.now("title_selected"),
            0,
            "fire let go on KODY: the entry is on"
        );
        for (k, &d) in code.iter().enumerate() {
            assert_eq!(
                self.now("code_digit_index"),
                k as u32,
                "before digit {}",
                k + 1
            );
            self.press(key(digit(d)), 6, 6);
        }
        assert_eq!(
            self.now("title_selected"),
            0,
            "the fourth digit ends the entry"
        );
        assert_eq!(self.now("title_cursor"), 1, "still on KODY");
        self.press(FrameInput::joy(Joy::UP), 10, 10);
        assert_eq!(self.now("title_cursor"), 0, "Up let go: START");
        self.now("code_digits")
    }

    /// Fire on START, then the world's load and its SCENA screen with fire ten steps in twenty
    /// from [`PULSE_AFTER`] steps on, until play goes on: `irq_count`, still on the title, through
    /// the load and the SCENA screen, counts again. The step play's first pass ran.
    pub fn start_and_play(&mut self) -> u64 {
        assert_eq!(self.now("title_cursor"), 0, "START");
        let from = self.steps();
        let still = self.now("irq_count");
        self.until(
            "play after START",
            3_000,
            |n| {
                let k = n - from - 1;
                if k < 10 || (k >= PULSE_AFTER && (k - PULSE_AFTER) % 20 < 10) {
                    FrameInput::joy(Joy::FIRE)
                } else {
                    FrameInput::NONE
                }
            },
            |r| r.now("irq_count") != still,
        )
    }

    /// HELP, under the cheat, until the level ends (`world_index` one up); the step it did.
    ///
    /// `help_key` touches the exit in the one pass that reads HELP's press, and `main_loop` tests
    /// for the level's end only after a pass whose `irq_count` is a multiple of 32 (docs/09,
    /// `level-end.toml` presses it for pass 224), so a press at any other pass is lost. The first
    /// press measures how many passes after it the exit is touched; the second is aimed so that
    /// pass is a multiple of 32, as a player would try again. HELP is tapped for [`HELP_TAP`] steps.
    pub fn help_until_the_level_ends(&mut self) -> u64 {
        let world = self.now("world_index");
        for _ in 0..2 {
            let before = self.now("irq_count");
            let from = self.steps();
            let touched = self.until(
                "HELP touching the exit under the cheat",
                60,
                |n| {
                    if n - from <= HELP_TAP {
                        key(HELP)
                    } else {
                        FrameInput::NONE
                    }
                },
                |r| var(r.ram(), "level_exit_touched") != 0,
            );
            if self.steps() - from < HELP_TAP {
                self.hold(key(HELP), HELP_TAP - (self.steps() - from));
            }
            // main_loop's check follows the touching pass when the level ends.
            for _ in 0..40 {
                self.step(FrameInput::NONE);
                if self.now("world_index") != world {
                    return self.steps();
                }
            }
            // Passes from the press to the touch; wait until a press lands it on a multiple of 32.
            let lag = self.at(touched, "irq_count").wrapping_sub(before);
            self.until(
                "an irq_count to aim HELP from",
                200,
                |_| FrameInput::NONE,
                |r| (r.now("irq_count") + lag) % 32 == 0,
            );
        }
        panic!(
            "HELP twice under the cheat and the level did not end: {}",
            self.describe(self.steps())
        )
    }

    /// The run in the app: its inputs as host key presses on [`SETTINGS`], unpaced, the frames'
    /// PNGs written only when `frames`; every step's input and state held to the run's. The app
    /// comes back with its capture: its home, the capture's folder, goes when it is dropped.
    pub fn in_the_app(&self, frames: bool, timeout_s: u64) -> (App, Captured) {
        self.in_the_app_with(InApp {
            frames,
            timeout_s,
            ..InApp::default()
        })
    }

    /// [`Run::in_the_app`] with `how`'s scale, script lines and focus losses (title plan T11's drill).
    pub fn in_the_app_with(&self, how: InApp) -> (App, Captured) {
        let InApp {
            frames,
            timeout_s,
            scale,
            ref extra,
            ref blur_at,
            fast,
        } = how;
        let steps = self.steps();
        let mut app = App::launch_translated(LaunchOpts {
            // Past an exit, so the app reaches it and quits on its own, not on the cap.
            max_frames: if self.exited.is_some() {
                steps + 50
            } else {
                steps
            },
            timeout_s,
            scale,
            extra_args: if frames {
                vec![]
            } else {
                vec!["--capture-hashes".into()]
            },
            ..Default::default()
        });
        let bugs = app.home.join("test-bugs");
        app.opts
            .extra_args
            .extend(["--bugs-dir".into(), bugs.display().to_string()]);
        std::fs::write(app.home.join("settings.toml"), SETTINGS).unwrap();
        // The app reads a script's events in file order, so an added line goes in among the keys'
        // at its step (a stable sort keeps a step's own order), not after the last.
        let mut lines: Vec<String> = self.key_lines(blur_at).lines().map(String::from).collect();
        lines.extend(extra.lines().map(String::from));
        lines.sort_by_key(|l| {
            l.split(':')
                .next()
                .and_then(|n| n.parse::<u64>().ok())
                .unwrap_or_else(|| panic!("not an `N: event` line: {l}"))
        });
        let c = app.run_script_unread(&format!(
            "{}{}\n",
            if fast { format!("fast-until {steps}\n") } else { String::new() },
            lines.join("\n")
        ));
        assert!(c.exit.success(), "{}", c.stderr);
        assert!(
            c.stderr
                .contains("fd-app: playing the translated program from power-on"),
            "{}",
            c.stderr
        );
        assert!(!c.stderr.contains("stopped at step"), "{}", c.stderr);
        assert_eq!(zips(&bugs), Vec::<PathBuf>::new(), "no bug zip");
        let fed = common::inputs(&c);
        let hashes = common::hashes(&c);
        // An exit ends the app at its step (`menu quit after step N`): no hash after it.
        let last = self.exited.unwrap_or(steps);
        assert!(
            hashes.len() as u64 > last,
            "a hash for the start and steps 1 to {last}"
        );
        for n in 1..=last {
            let i = fed.get(&n).copied().unwrap_or(FrameInput::NONE);
            let want = self.inputs[n as usize - 1];
            assert_eq!(i, want, "the input step {n} saw in the app");
            let (app, headless) = (&hashes[n as usize], &self.hashes[n as usize - 1]);
            assert!(
                app == headless,
                "the state after step {n}: app {app}, headless {headless}; headless {}",
                self.describe(n)
            );
        }
        (app, c)
    }

    /// [`VARS`] after step `n`, for a failure's message.
    pub fn describe(&self, n: u64) -> String {
        VARS.iter()
            .map(|v| format!("{v}={:#x}", self.at(n, v)))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// The inputs as script lines: a host key goes down or up where its input changes, landing
    /// before the step that reads it (`N:` is seen by step N), everything let go after the last.
    /// At a step of `blur_at` the window loses focus in place of the keys' `up` lines: the app lets
    /// go of every key held then, so the run's inputs must let go of them all there.
    fn key_lines(&self, blur_at: &[u64]) -> String {
        let mut out = String::new();
        let mut held = FrameInput::NONE;
        for (k, i) in self
            .inputs
            .iter()
            .copied()
            .chain([FrameInput::NONE])
            .enumerate()
        {
            let n = k + 1;
            if blur_at.contains(&(n as u64)) {
                assert!(
                    i.joy.0 & held.joy.0 == 0 && i.keys.0 & held.keys.0 == 0,
                    "step {n}: a focus loss lets go of every key held"
                );
                out.push_str(&format!("{n}: blur\n"));
            }
            let lost = blur_at.contains(&(n as u64));
            for (bit, host) in JOY_KEYS {
                let (was, is) = (held.joy.0 & bit.0 != 0, i.joy.0 & bit.0 != 0);
                if was != is && !(lost && was) {
                    out.push_str(&format!("{n}: {} {host}\n", if is { "down" } else { "up" }));
                }
            }
            for code in 0..128u8 {
                let (was, is) = (held.keys.0 >> code & 1 != 0, i.keys.0 >> code & 1 != 0);
                if was != is && !(lost && was) {
                    let how = if is { "down" } else { "up" };
                    out.push_str(&format!("{n}: {how} {}\n", host_key(code)));
                }
            }
            held = i;
        }
        out
    }
}

/// How [`Run::in_the_app_with`] runs the app: [`Run::in_the_app`]'s at scale 1 by default.
pub struct InApp {
    /// Every frame's PNG written, else hashes and inputs only.
    pub frames: bool,
    pub timeout_s: u64,
    /// `--scale`; `None` launches at the app's own default.
    pub scale: Option<u32>,
    /// Script lines added after the key presses: `N: resize W H`, `N: shot`.
    pub extra: String,
    /// Steps at which the window loses focus (`N: blur`), letting go of every key.
    pub blur_at: Vec<u64>,
    /// Unpaced to the last step (`fast-until`); a run taking shots is paced, since a shot is of
    /// a window drawn, and a fast-forward draws none.
    pub fast: bool,
}

impl Default for InApp {
    fn default() -> InApp {
        InApp {
            frames: false,
            timeout_s: 180,
            scale: Some(1),
            extra: String::new(),
            blur_at: Vec::new(),
            fast: true,
        }
    }
}

/// The zips in `bugs`, none when the folder was never made.
fn zips(bugs: &std::path::Path) -> Vec<PathBuf> {
    std::fs::read_dir(bugs)
        .map(|d| d.flatten().map(|e| e.path()).collect())
        .unwrap_or_default()
}

/// The app's frame `n` is the run's picture there: the drawn screen, not only the state.
pub fn drawn_alike(c: &Captured, r: &Run, from: u64, to: u64) {
    for n in from..=to {
        let want = r
            .pictures
            .get(&n)
            .unwrap_or_else(|| panic!("no picture kept at step {n}"));
        assert!(
            c.read_frame(n) == *want,
            "the picture after step {n}; headless {}",
            r.describe(n)
        );
    }
}
