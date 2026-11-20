// SCRATCH: kept in plans/forest-dumb-level-1/prototype/t84-one-route/, never built by a crate.
// Plays a step script on the original with the case's engine beside it and the engine alone on the
// same steps, and prints where the two sides part: the passes that read another step's joystick
// than the engine alone gives them, the passes that cut a redraw, and the first pass the engine
// alone's character, items or creature slots are not the original's.
//
// SCRIPT=../../conform/scripts/route-W1-1.txt [MAXSTEPS=n] [VERBOSE=1]
//   cargo test --release -p fd-plan --test zz_measure -- --nocapture
use amiga_conform::script;
use amiga_input::FrameInput;
use fd_conform::{
    Case, HUNK_BASES, Live, NEXT_WORLD, OracleMem, PASS_END, Sampler, edges_cut, game_data,
    reach_case_counting, sync,
};
use fd_core::level::EdgesCut;
use fd_core::{Game, Mem, Start, Transplant, layout_var};
use fd_plan::game::PlanState;
use amiga_plan::PlanGame;

type Key = [u32; 20];
fn key(g: &Game) -> Key {
    let s = g.state();
    let c = &s.edges.creatures;
    let mut k = [0u32; 20];
    k[0] = s.char_x;
    k[1] = s.char_y;
    k[2] = s.lives as u32;
    k[3] = s.hits_taken as u32;
    k[4] = s.items_left as u32;
    for i in 0..5 {
        k[5 + 3 * i] = c.active[i] as u32;
        k[6 + 3 * i] = c.slots[i].x as u32;
        k[7 + 3 * i] = c.slots[i].y as u32;
    }
    k
}
fn read_script(path: &str) -> Vec<FrameInput> {
    let text = std::fs::read_to_string(path).unwrap();
    let sc = script::parse(&text).unwrap();
    let n = text
        .lines()
        .find_map(|l| l.strip_prefix("# frames "))
        .map(|n| n.parse().unwrap())
        .unwrap_or_else(|| sc.lines.last().unwrap().0);
    (0..n).map(|k| sc.input_at(k)).collect()
}

#[test]
fn measure() {
    let verbose = std::env::var("VERBOSE").is_ok();
    let max_steps: usize = std::env::var("MAXSTEPS").map(|s| s.parse().unwrap()).unwrap_or(30000);
    let s = read_script(&std::env::var("SCRIPT").unwrap());
    let data = game_data().unwrap();
    let case = Case::parse(
        "measure",
        &format!(
            "sync = {{ script = \"{}\", frame = {} }}\nframes = 32000\ncompare = [\"vars\"]\nvars = [\"char_x\"]\ngoal = {{ level_end = true, by_frame = 38301 }}\n",
            sync::LEVEL_1_SCRIPT,
            sync::LEVEL_1_SYNC
        ),
    )
    .unwrap();
    let irq = layout_var("irq_count").unwrap();
    let next_world = HUNK_BASES[NEXT_WORLD.0 as usize] + NEXT_WORLD.1;
    let (mut am, mut read) = reach_case_counting(&case, 0, &[next_world, Sampler::CAMERA_UPDATE]).unwrap();
    am.watch_write(Some(PASS_END));
    let mut sampler = Sampler::default();
    let mut profiling = false;
    let mut edges: Vec<(u32, Option<EdgesCut>)> = Vec::new();
    let mut engine = Game::new(data.clone(), Start::Level1);
    let mut at: Option<u32> = None;
    // The engine alone: alone_keys[p] is its key after pass p (step p - 2).
    let mut alone = PlanState(Game::new(data.clone(), Start::Level1));
    let mut alone_keys: Vec<Key> = vec![[0; 20]; 2];
    let mut alone_lost = None;
    for (k, &i) in s.iter().enumerate() {
        alone.0.step(i);
        alone.0.take_events();
        alone_keys.push(key(&alone.0));
        if alone_lost.is_none() && (alone.lives() != 32 || alone.dying()) {
            alone_lost = Some(k);
        }
    }
    println!("the engine alone: a life lost at step {alone_lost:?}");
    let (mut late, mut other_input, mut cuts, mut passes) = (0u32, 0u32, 0u32, 0u32);
    let mut lag_hist = [0u32; 5];
    let mut parted: Option<u32> = None;
    let mut char_parted: Option<u32> = None;
    let mut prev_lag = 0i64;
    let watch: (u32, u32) = std::env::var("WATCH").map(|w| { let v: Vec<u32> = w.split(',').map(|x| x.parse().unwrap()).collect(); (v[0], v[1]) }).unwrap_or((1, 0));
    let cvars: Vec<_> = ["creature_active+$0", "creature_active+$1", "creature_active+$2", "creature_active+$3", "creature_active+$4", "creature_slots+$0", "creature_slots+$8", "creature_slots+$10", "creature_slots+$18", "creature_slots+$20"].iter().map(|v| layout_var(v).unwrap()).collect();
    let mut lags: std::collections::HashMap<u32, i64> = Default::default();
    let mut csv = String::from("pass,lag,cells,drawn,active,cut,camx,camy,spr\n");
    let mut transitions = 0;
    for t in 0..s.len().min(max_steps) {
        let input = s[t];
        let nr = read.next_read();
        let started = Live(&am).value(irq);
        let sample = sampler.samples(&am, &case);
        if sample != profiling {
            am.set_profile(sample);
            profiling = sample;
        }
        am.step(input).unwrap();
        let now = Live(&am).value(irq);
        if sample && now == started.wrapping_add(1) {
            edges.push((now, edges_cut(&am)));
            if watch.0 <= now && now <= watch.1 {
                println!("  pass {now} starts in step {t}: pc {:x?} {:?}", fd_conform::interrupted_pc(&am).map(|p| p.wrapping_sub(HUNK_BASES[0])), edges_cut(&am));
            }
        }
        sampler.stepped(&am);
        read.step(&am, input).unwrap();
        for p in nr..read.next_read() {
            // Pass p read step t; the engine alone gives it step p - 2.
            let lag = t as i64 - (p as i64 - 2);
            lag_hist[(lag + 1).clamp(0, 4) as usize] += 1;
            if lag != prev_lag {
                transitions += 1;
            }
            prev_lag = lag;
            lags.insert(p, lag);
            if lag != 0 {
                late += 1;
                let own = s.get(p as usize - 2).copied().unwrap_or(FrameInput::NONE);
                if own.joy != input.joy {
                    other_input += 1;
                    if verbose || other_input <= 20 {
                        println!("  pass {p} read step {t} ({:x}), its own step {} holds {:x}", input.joy.0, p - 2, own.joy.0);
                    }
                }
            }
        }
        if let Some(mem) = am.watched_memory() {
            let theirs = OracleMem(&mem);
            let end = theirs.value(irq);
            match at {
                None => {
                    engine.transplant(&theirs);
                    at = Some(end);
                    read.drop_through(end);
                }
                Some(last) if end > last => {
                    for n in last + 1..=end {
                        let pi = read.take(n).expect("a pass ended that never read its input");
                        let cut = edges.iter().find(|(p, _)| *p == n).and_then(|c| c.1);
                        edges.retain(|(p, _)| *p > n);
                        cuts += cut.is_some() as u32;
                        {
                            let st = engine.state();
                            let list = (st.animation.phase as usize + 1) & 3;
                            let cells = st.edges.cells.used[list].iter().filter(|&&f| f != 0).count();
                            let drawn = st.creature_draw.drawn.iter().filter(|&&f| f != 0).count();
                            let active = st.edges.creatures.active.iter().filter(|&&f| f != 0).count();
                            csv.push_str(&format!("{n},{},{cells},{drawn},{active},{},{},{},{}\n", lags.get(&n).copied().unwrap_or(-9), cut.is_some() as u8, st.camera.x, st.camera.y, st.frame_flag));
                        }
                        engine.cut_foreground(cut);
                        engine.step(pi);
                        engine.take_events();
                        passes += 1;
                        if watch.0 <= n && n <= watch.1 {
                            let e: Vec<String> = cvars.iter().map(|v| format!("{:x}", engine.probe(v))).collect();
                            println!("  pass {n} ended (step {t}, cut {cut:?}): engine {}", e.join(" "));
                            if n == end {
                                let o: Vec<String> = cvars.iter().map(|v| format!("{:x}", theirs.value(v))).collect();
                                println!("                         original {}", o.join(" "));
                            }
                        }
                        let (a, b) = (alone_keys.get(n as usize), key(&engine));
                        if let Some(a) = a {
                            if char_parted.is_none() && (a[0] != b[0] || a[1] != b[1] || a[4] != b[4]) {
                                char_parted = Some(n);
                                println!("  the character parts at pass {n}: alone {:x},{:x} items {}; the original {:x},{:x} items {}", a[0], a[1], a[4], b[0], b[1], b[4]);
                            }
                            if *a != b {
                                if parted.is_none() || verbose {
                                    println!("  pass {n}: alone {a:x?}\n            orig  {b:x?}");
                                }
                                parted.get_or_insert(n);
                            }
                        }
                    }
                    at = Some(end);
                }
                Some(_) => {}
            }
        }
        if am.pc_hits().iter().any(|&(p, n)| p == next_world && n > 0) {
            println!("the original ran next_world in step {t}");
            break;
        }
        let p = PlanState(engine.clone());
        if p.lives() != 32 || p.dying() {
            println!("the original loses a life at step {t}");
            break;
        }
    }
    if let Ok(f) = std::env::var("CSV") {
        std::fs::write(f, csv).unwrap();
    }
    println!("{passes} passes; reads by lag -1,0,1,2,3+: {lag_hist:?}; {transitions} lag changes; {late} late, {other_input} of them another input; {cuts} cuts; first parted {parted:?}, character {char_parted:?}");
}
