// SCRATCH: never committed.
use std::path::Path;
use std::thread;

use amiga_chips::Picture;
use amiga_oracle::FrameInput;
use fd_conform::{Case, Engine, Options, run_with};
use fd_core::level::EdgesCut;
use fd_core::{Event, Game, LAYOUT, Mem, Start, Transplant, Var};

struct W {
    game: Game,
    seen: Vec<String>,
    cuts: Vec<(usize, EdgesCut)>,
    inputs: Vec<FrameInput>,
    fwd: bool,
}
impl Transplant for W {
    fn transplant(&mut self, mem: &dyn Mem) {
        self.game.transplant(mem);
    }
    fn probe(&self, v: &Var) -> u32 {
        self.game.probe(v)
    }
}
impl Engine for W {
    fn pass(&mut self, input: FrameInput) {
        self.inputs.push(input);
        self.game.step(input);
        let s = self.game.state();
        let c = &s.edges.creatures;
        if let Ok(r) = std::env::var("BOX") {
            let v: Vec<usize> = r.split(',').map(|x| x.parse().unwrap()).collect();
            if self.seen.len() >= v[4] && self.seen.len() <= v[5] {
                let p = self.game.picture();
                let mut vals = std::collections::BTreeMap::new();
                for y in v[1]..v[3] { for x in v[0]..v[2] { *vals.entry(p.pixels[y * p.width as usize + x]).or_insert(0usize) += 1; } }
                std::fs::write(format!("{}/eng-{}.png", std::env::var("DIFF").unwrap(), self.seen.len()), fd_conform::bug::picture_png(&p)).unwrap();
                let s = self.game.state();
                println!("pass {} irq {} box {:?} draw {:?} cam {:x},{:x} resume {:?}", self.seen.len(), s.irq_count, vals, s.creature_draw, s.camera.x, s.camera.y, s.edges_resume);
            }
        }
        if std::env::var("SPR").is_ok() {
            let v = |n: &str| self.game.probe(fd_core::layout_var(n).unwrap());
            if v("spring_timer") == 6 {
                println!("spr pass {} irq {} spring_x {:x} kind {:x} char {:x},{:x}", self.seen.len(), v("irq_count"), v("spring_x"), v("spring_kind"), v("char_x"), v("char_y"));
            }
        }
        self.seen.push(format!(
            "irq {} x {:#x} y {:#x} cam {:#x},{:#x} score {:x} items {:x} lives {:x} hits {} active {:?} kinds {:?}",
            s.irq_count, s.char_x, s.char_y, s.camera.x, s.camera.y, s.score, s.items_left, s.lives, s.hits_taken, c.active, c.slots.map(|s| s.kind)
        ));
    }
    fn picture(&self) -> Option<Picture> {
        Some(self.game.picture())
    }
    fn events(&mut self) -> Vec<Event> {
        self.game.take_events()
    }
    fn off_play(&mut self, input: FrameInput) {
        Engine::off_play(&mut self.game, input)
    }
    fn game_over(&self) -> bool {
        Engine::game_over(&self.game)
    }
    fn level_end(&self) -> bool {
        Engine::level_end(&self.game)
    }
    fn cut_redraw(&self, cut: u8) -> Option<Picture> {
        Engine::cut_redraw(&self.game, cut)
    }
    fn foreground_cut(&mut self, at: Option<EdgesCut>) {
        if let Some(c) = at {
            self.cuts.push((self.seen.len(), c));
        }
        if self.fwd {
            let at = match std::env::var("STRIP").as_deref() {
                Ok("none") => at.map(|c| EdgesCut { defer_from: None, ..c }),
                Ok("fab") => at.or(Some(EdgesCut { next: 4, defer_from: Some(0) })),
                Ok("fabstart") => {
                    let mut g = self.game.clone();
                    let before = g.state().edges.creatures.active;
                    g.foreground();
                    if at.is_none() && g.state().edges.creatures.active != before {
                        Some(EdgesCut { next: 4, defer_from: Some(0) })
                    } else {
                        at
                    }
                }
                Ok("zero") => at.map(|c| EdgesCut { defer_from: c.defer_from.map(|_| 0), ..c }),
                _ => at,
            };
            Engine::foreground_cut(&mut self.game, at)
        }
    }
}

#[test]
fn scratch() {
    let names = std::env::var("SCR").unwrap();
    let fwd = std::env::var("NOFWD").is_err();
    let frames: Option<u32> = std::env::var("FRAMES").ok().map(|f| f.parse().unwrap());
    let runs: Vec<_> = names
        .split(',')
        .map(|n| {
            let n = n.to_string();
            thread::spawn(move || {
                let mut case = Case::load(Path::new(&format!("cases/{n}.toml"))).unwrap();
                if std::env::var("NOPIC").is_ok() { case.compare.picture = false; }
                if let Some(f) = frames {
                    case.frames = f;
                }
                for v in LAYOUT.iter().filter(|v| v.name.starts_with("creature_")) {
                    if !case.vars.iter().any(|c| c == v.name) {
                        case.vars.push(v.name.to_string());
                    }
                }
                let mut w = W {
                    game: Game::new(fd_conform::game_data().unwrap(), Start::Level1),
                    seen: Vec::new(),
                    cuts: Vec::new(),
                    inputs: Vec::new(),
                    fwd,
                };
                let v = run_with(&case, &mut w, &Options { diff_dir: std::env::var("DIFF").ok().map(std::path::PathBuf::from), ..Options::default() });
                let mut out = format!("=== {n}\n");
                match v {
                    Ok(v) => {
                        let t = format!("{v}");
                        for l in t.lines().take(4) {
                            out += &format!("{}\n", &l[..l.len().min(700)]);
                        }
                        out += &format!("goal_met {:?} passes {} unseen {}\n", v.goal_met, v.passes, v.unseen);
                    }
                    Err(e) => out += &format!("ERR {e}\n"),
                }
                let mut last = String::new();
                for (k, s) in w.seen.iter().enumerate() {
                    let key: String = s.split(" score ").nth(1).unwrap().split(" hits ").next().unwrap().to_string()
                        + s.split(" active ").nth(1).unwrap();
                    if key != last && std::env::var("TRACE").is_ok() {
                        out += &format!("  {k}: {s}\n");
                    }
                    last = key;
                }
                out += &format!("  last: {}\n", w.seen.last().cloned().unwrap_or_default());
                if std::env::var("LAG").is_ok() {
                    let sname = case.input.clone().unwrap();
                    let sc = amiga_conform::script::parse(&std::fs::read_to_string(fd_conform::conform_dir().join("scripts").join(format!("{sname}.txt"))).unwrap()).unwrap();
                    // Pass k (0-based after the transplant pass) against script steps k+off.
                    for off in 0..3u32 {
                        let diff: Vec<usize> = w.inputs.iter().enumerate().filter(|(k, i)| sc.input_at(*k as u32 + off) != **i).map(|(k, _)| k).collect();
                        out += &format!("  off {off}: {} of {} pass inputs differ from the script's step: {:?}\n", diff.len(), w.inputs.len(), &diff[..diff.len().min(60)]);
                    }
                    // Engine alone on the script, against the case's engine, by irq.
                    let mut g = Game::new(fd_conform::game_data().unwrap(), Start::Level1);
                    let lead = case.sync_frame - 6301;
                    let mut alone = std::collections::HashMap::new();
                    for f in 0..lead + case.frames {
                        g.step(if f < lead { FrameInput::NONE } else { sc.input_at(f - lead) });
                        let st = g.state();
                        alone.insert(st.irq_count, (st.char_x, st.char_y, st.lives));
                    }
                    for s in &w.seen {
                        let irq: u16 = s.split(' ').nth(1).unwrap().parse().unwrap();
                        let x = u32::from_str_radix(s.split(" x 0x").nth(1).unwrap().split(' ').next().unwrap(), 16).unwrap();
                        let y = u32::from_str_radix(s.split(" y 0x").nth(1).unwrap().split(' ').next().unwrap(), 16).unwrap();
                        if let Some(a) = alone.get(&irq) {
                            if (a.0, a.1) != (x, y) {
                                out += &format!("  first engine-alone difference at irq {irq}: alone {:x?} case {x:x},{y:x}\n", a);
                                break;
                            }
                        }
                    }
                    out += &format!("  cuts: {:?}\n", w.cuts.iter().map(|c| (c.0, c.1.next, c.1.defer_from.map(|d| d != 0))).collect::<Vec<_>>());
                }
                out += &format!("  cuts with defer: {:?}\n", w.cuts.iter().filter(|c| c.1.defer_from.is_some()).count());
                out
            })
        })
        .collect();
    for r in runs {
        println!("{}", r.join().unwrap());
    }
}
