//! Plans a slot machine case's input script (T74) on the engine, from the case's sync point: the
//! attract loop ended by fire on frame 40, then each menu played as the plan says, in turn, and
//! any menu after the plan's spin (or take); the jackpot's flash ended by fire. Prints the script
//! (frames the original's: its loads take 758) and what the slot machine gave.
//!
//! `cargo run --release --example slot_plan -- <start script> <plan>...`
//!
//! A plan's steps, one menu each: `spin` (the leftmost stop: spin, take or collect), `hold1`..
//! `hold3` (a reel's hold toggled), `gamble`, `right` and `wrong` (a hi-lo guess, higher or lower
//! as the engine's next number makes it right or wrong; tried on a clone).
use sf_conform::Case;
use sf_core::Game;
use sf_core::input::{FrameInput, Joy};
use sf_core::layout::h0;

/// The original's frame for the engine's: its loads take 758.
const LOADS: u32 = 758;
/// The menu's pointer stop (h0+$6064), the stops (h0+$606a), the pointer's x, the hi-lo number,
/// the number as the guess was made.
const CURSOR: u32 = h0(0x6064);
const STOPS: u32 = h0(0x606a);
const POINTER_X: u32 = h0(0x11b66);
const HILO: u32 = h0(0x49ce);
const HILO_GUESSED: u32 = h0(0x5988);
/// The win (h0+$57b2): 150 000 once the jackpot's code is up.
const WIN: u32 = h0(0x57b2);

struct Run {
    g: Game,
    f: u32,
    lines: Vec<(u32, &'static str)>,
}

impl Run {
    fn step(&mut self, input: FrameInput) {
        self.g.step(input);
        self.f += 1;
    }

    /// `key` held three frames from now, then nothing.
    fn press(&mut self, key: &'static str, joy: Joy) {
        self.lines.push((self.f + LOADS, key));
        for _ in 0..3 {
            self.step(FrameInput::joy(joy));
        }
        self.lines.push((self.f + LOADS, "-"));
    }

    /// Whether a menu reads the joystick on this frame: a move tried on a clone moves the pointer.
    fn live(&self) -> bool {
        let s = self.g.state();
        let c = s.l(CURSOR);
        let joy = if c == STOPS + 8 { Joy::LEFT } else { Joy::RIGHT };
        let mut t = self.g.clone();
        t.step(FrameInput::joy(joy));
        t.state().l(CURSOR) != c
    }

    fn wait_live(&mut self) {
        let from = self.f;
        while !self.live() {
            assert!(self.f < from + 2_000, "no menu from frame {from}");
            assert!(self.g.slot_outcome().is_none(), "returned waiting for a menu");
            self.step(FrameInput::NONE);
        }
    }

    /// The menu's pointer to stop `k` (0..5), then fire.
    fn pick(&mut self, k: u32) {
        let x = |r: &Run| r.g.state().w(POINTER_X);
        let target = [0x26, 0x60, 0x90, 0xc0, 0x116][k as usize];
        self.wait_live();
        // The leftmost stop is any x left of $46.
        let at = if k == 0 { x(self) < 0x46 } else { x(self) == target };
        eprintln!(
            "frame {}: stop {k}, coins {}",
            self.f,
            self.g.state().w(h0(0x49c6))
        );
        if !at {
            if k == 0 {
                self.press("L", Joy::LEFT);
                self.wait_live();
            } else {
                for _ in 0..k {
                    self.press("R", Joy::RIGHT);
                    self.wait_live();
                }
            }
        }
        assert!(k == 0 || x(self) == target, "frame {}", self.f);
        self.press("F", Joy::FIRE);
    }

    fn clone(&self) -> Run {
        Run {
            g: self.g.clone(),
            f: self.f,
            lines: self.lines.clone(),
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let start = &args[1];
    let plan: Vec<&str> = args[2..].iter().map(|s| s.as_str()).collect();
    let toml = format!("sync = {{ slot = \"slot-none\", start = \"{start}\" }}\nframes = 1\n");
    let case = Case::parse("plan", &toml).unwrap();
    let mut first = None;
    let _ = sf_conform::run_with(&case, |f, g| {
        if f == 0 {
            first = Some(g.clone());
        }
    });
    let mut r = Run {
        g: first.expect("the sync point"),
        f: 0,
        lines: Vec::new(),
    };
    while r.f < 40 {
        r.step(FrameInput::NONE);
    }
    r.lines.push((r.f + LOADS, "F"));
    r.step(FrameInput::joy(Joy::FIRE));
    r.lines.push((r.f + LOADS, "-"));
    let mut steps = plan.into_iter();
    let mut jackpot_fired = false;
    while r.g.slot_outcome().is_none() {
        assert!(r.f < 20_000, "never returned");
        if !jackpot_fired && r.g.state().l(WIN) == 150_000 {
            jackpot_fired = true;
            r.lines.push((r.f + LOADS, "F"));
            for _ in 0..10 {
                r.step(FrameInput::joy(Joy::FIRE));
            }
            r.lines.push((r.f + LOADS, "-"));
            continue;
        }
        if !r.live() {
            r.step(FrameInput::NONE);
            continue;
        }
        match steps.next().unwrap_or("spin") {
            "spin" => r.pick(0),
            "hold1" => r.pick(1),
            "hold2" => r.pick(2),
            "hold3" => r.pick(3),
            "gamble" => r.pick(4),
            g @ ("right" | "wrong") => {
                // Higher (stop 1) on a clone: right when the number did not fall.
                let mut t = r.clone();
                t.pick(1);
                for _ in 0..150 {
                    t.step(FrameInput::NONE);
                }
                let s = t.g.state();
                let higher_right = s.w(HILO) >= s.w(HILO_GUESSED);
                let higher = higher_right == (g == "right");
                eprintln!(
                    "frame {}: {} -> {}, {}",
                    r.f,
                    s.w(HILO_GUESSED),
                    s.w(HILO),
                    if higher { "higher" } else { "lower" }
                );
                r.pick(if higher { 1 } else { 3 });
            }
            other => panic!("no step {other}"),
        }
    }
    let o = r.g.slot_outcome().unwrap();
    println!("# frames = {}  ({:?})", r.f - 1, o);
    for (f, k) in r.lines {
        println!("{f}: {k}");
    }
    let u = r.g.unported();
    if !u.is_empty() {
        eprintln!("unported: {u:?}");
    }
}
