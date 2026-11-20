//! Map page T12, the drill (plans/forest-dumb-map-page DESIGN §2): the page used as the owner uses
//! it, beside the real translated app, at 1280×800, 1920×1080 (the second screen) and 800×600, one
//! headless Chrome (`amiga-browser`) a size, all three open on the one page at once.
//!
//! - The title, scripted `menu show-map` writing the page into the app's home: "Not playing".
//! - The app quitting: "App not running" within 4 s.
//! - The one route (T84) through W1-1, fast but for its last [`PACED`] steps, world 2's load and SCENA
//!   screen, and [`whole::WORLD_2_PLAY`] steps of W2-1, as `translated_whole.rs` plays it: on every
//!   sample the dot drawn on the character's cell, the status line naming it, items left never
//!   rising, taken items gone, creatures moving; Follow keeping the dot in view, a hand scroll
//!   stopping it, the button resuming it; a level picked by hand holding; "Not playing" during the
//!   load, then W2-1 by itself; a resize, a reload and three pages at once mid-play.
//! - After the run: each world with Secrets on and off and every layer toggled.
//!
//! Every screen goes to the scratch folder (`$FD_MAP_DRILL_OUT`, else `$TMPDIR/fd-map-drill`),
//! never the repo, with `samples.tsv`, what each page showed at each sample, for the worker to judge.
//! Full and `#[ignore]`d: the owner's disks, the generated tree and Chrome, four to ten minutes
//! (release): `cargo test --release -q -p fd-e2e --test map_drill -- --ignored --nocapture`.

mod common;
mod whole;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use amiga_browser::{Browser, Opts};
use fd_e2e::{App, LaunchOpts};

/// W1-1's last steps played on the clock, so the page is driven by hand while the game plays at
/// its own pace (about 30 s).
const PACED: u64 = 1_500;
/// Every Chrome's whole session: the title run, the main run and the layers after it.
const BROWSER_LIMIT: Duration = Duration::from_secs(1800);
const SIZES: [(u32, u32); 3] = [(1280, 800), (1920, 1080), (800, 600)];
const IDS: [&str; 6] = ["W1-1", "W2-1", "W3-1", "W4-1", "W5-1", "W6-1"];

fn scratch() -> PathBuf {
    let d = std::env::var_os("FD_MAP_DRILL_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("fd-map-drill"));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// One open page: its Chrome, its size's name and its own sample log.
struct Page {
    b: Browser,
    name: String,
    shots: u32,
}

/// What a page shows at one moment.
#[derive(Debug, Clone)]
struct Sample {
    at: f64,
    state: String,
    stat: String,
    count: String,
    current: String,
    follow: bool,
    /// `window.LIVE` as last read, when the page holds one (it is cleared while a read is pending).
    level: Option<String>,
    playing: bool,
    actor: Option<(f64, f64)>,
    cell: Option<(i64, i64)>,
    actors: Vec<(f64, f64)>,
    gone: usize,
    items_left: Option<i64>,
    /// The actor's screen point inside the map box, and whether the canvas there is the dot's colour.
    on_screen: Option<bool>,
    dot_drawn: Option<bool>,
    view: (f64, f64, f64),
    /// A level's data still loading: the page draws no play until it is in.
    loading: bool,
}

/// The sample as one tab-separated line (state, status, counter, current, follow, level, playing,
/// actor x, y, cell c, r, creatures `x,y;…`, items gone, items left, on screen, dot drawn, scale,
/// ox, oy, loading), an empty field for none: this crate reads `eval`'s answer as a string alone.
const SAMPLE: &str = r#"(() => {
  const v = ROUTEVIEW.view(), L = window.LASTLIVE || null, box = document.getElementById('mapbox');
  let on = '', drawn = '';
  if (L && L.actor && L.playing && L.level === v.current) {
    const sx = L.actor[0] * v.scale + v.ox, sy = L.actor[1] * v.scale + v.oy;
    on = sx >= 0 && sy >= 0 && sx < box.clientWidth && sy < box.clientHeight;
    if (on) {
      const c = document.getElementById('cv'), d = devicePixelRatio || 1;
      const px = c.getContext('2d').getImageData(Math.round(sx * d), Math.round(sy * d), 1, 1).data;
      drawn = px[0] === 0x7d && px[1] === 0xff && px[2] === 0x6a;
    }
  }
  const f = x => x == null ? '' : String(x);
  return [document.getElementById('livebar').dataset.state, document.getElementById('livestat').textContent,
    document.getElementById('livecount').textContent, v.current, v.follow,
    f(L && L.level), f(L && L.playing), f(L && L.actor && L.actor[0]), f(L && L.actor && L.actor[1]),
    f(L && L.cell && L.cell[0]), f(L && L.cell && L.cell[1]),
    L ? (L.actors || []).map(a => a[0] + ',' + a[1]).join(';') : '',
    L ? (L.cells || []).filter(c => c[2] === 0).length : '',
    f(L && L.counters && L.counters.items_left), f(on), f(drawn), v.scale, v.ox, v.oy, !document.getElementById('loading').hidden].join('\t');
})()"#;

impl Page {
    fn open(index: &Path, (w, h): (u32, u32)) -> Page {
        let mut b = Browser::launch(Opts {
            limit: BROWSER_LIMIT,
            width: w,
            height: h,
        })
        .unwrap_or_else(|e| panic!("amiga-browser: {e}"));
        b.open(index).unwrap();
        Page {
            b,
            name: format!("{w}x{h}"),
            shots: 0,
        }
    }

    fn sample(&mut self, t0: Instant) -> Sample {
        let line = self.eval_str(SAMPLE);
        let f: Vec<&str> = line.split('\t').collect();
        assert_eq!(f.len(), 20, "{line}");
        let num = |k: usize| f[k].parse::<f64>().ok();
        let bool_ = |k: usize| (!f[k].is_empty()).then(|| f[k] == "true");
        Sample {
            at: t0.elapsed().as_secs_f64(),
            state: f[0].into(),
            stat: f[1].into(),
            count: f[2].into(),
            current: f[3].into(),
            follow: f[4] == "true",
            level: (!f[5].is_empty()).then(|| f[5].to_string()),
            playing: f[6] == "true",
            actor: num(7).zip(num(8)),
            cell: num(9).zip(num(10)).map(|(c, r)| (c as i64, r as i64)),
            actors: f[11]
                .split(';')
                .filter(|a| !a.is_empty())
                .map(|a| {
                    let (x, y) = a.split_once(',').unwrap();
                    (x.parse().unwrap(), y.parse().unwrap())
                })
                .collect(),
            gone: f[12].parse().unwrap_or(0),
            items_left: num(13).map(|n| n as i64),
            on_screen: bool_(14),
            dot_drawn: bool_(15),
            view: (num(16).unwrap(), num(17).unwrap(), num(18).unwrap()),
            loading: f[19] == "true",
        }
    }

    fn shot(&mut self, dir: &Path, what: &str) {
        self.shots += 1;
        let p = dir.join(format!("{}-{:03}-{what}.png", self.name, self.shots));
        self.b.screenshot(&p).unwrap();
    }

    fn eval_str(&mut self, js: &str) -> String {
        self.b.eval(js).unwrap().as_str().unwrap_or_default().to_string()
    }

    /// A real click on the element `js` finds, scrolled into view first.
    fn click(&mut self, js: &str) {
        let r = self
            .b
            .eval(&format!(
                "(() => {{ const e = {js}; if (!e) return null; e.scrollIntoView({{block: 'center'}}); const r = e.getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 2]; }})()"
            ))
            .unwrap();
        assert!(!r.is_null(), "{}: nothing found by {js}", self.name);
        self.b
            .click(r[0].as_f64().unwrap(), r[1].as_f64().unwrap())
            .unwrap();
    }

    fn pick(&mut self, id: &str) {
        self.click(&format!(
            "[...document.querySelectorAll('#worlds .rt')].find(e => e.textContent.endsWith('{id}'))"
        ));
    }

    fn wait(&mut self, js: &str, within: Duration, what: &str) {
        if let Err(e) = self.b.wait_for(js, within) {
            let st = self.eval_str("document.getElementById('livestat').textContent");
            panic!("{}: {what}: {e} (status {st:?})", self.name);
        }
    }

    /// A wheel turn over the map's middle: a scroll by hand.
    fn hand_scroll(&mut self) {
        let r = self
            .b
            .eval("(() => { const b = document.getElementById('mapbox'); b.scrollIntoView({block: 'center'}); const r = b.getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 2]; })()")
            .unwrap();
        self.b
            .scroll(r[0].as_f64().unwrap(), r[1].as_f64().unwrap(), 0.0, 120.0)
            .unwrap();
    }

    fn errors(&mut self) -> String {
        self.eval_str("JSON.stringify(window.ERRS || [])")
    }
}

/// Errors the page raises, from its first line on (as `levels_chrome.rs`); and `LASTLIVE`, the last
/// frame `live.js` set, since the page clears `window.LIVE` while each read is pending.
const CATCH: &str = "<script>window.ERRS=[];addEventListener('error',e=>ERRS.push(String(e.message)));\
addEventListener('unhandledrejection',e=>ERRS.push(String(e.reason)));\
{const ce=console.error;console.error=(...a)=>{ERRS.push(a.join(' '));ce(...a)}}\
{let l;Object.defineProperty(window,'LIVE',{configurable:true,get(){return l},set(v){l=v;if(v)window.LASTLIVE=v}})}</script>";

fn log(out: &mut std::fs::File, page: &str, phase: &str, s: &Sample) {
    writeln!(
        out,
        "{:.2}\t{page}\t{phase}\t{}\t{}\t{}\tfollow={}\tlive={:?}/{}\tcell={:?}\tactor={:?}\tactors={}\tgone={}\titems={:?}\ton={:?}\tdot={:?}\tview={:.2},{:.0},{:.0}\tloading={}",
        s.at, s.state, s.stat, s.count, s.follow, s.level, s.playing, s.cell, s.actor,
        s.actors.len(), s.gone, s.items_left, s.on_screen, s.dot_drawn, s.view.0, s.view.1, s.view.2, s.loading
    )
    .unwrap();
}

#[test]
#[ignore = "full: the map page beside the translated app through W1-1 into world 2, 4 to 10 min (release); cargo test --release -q -p fd-e2e --test map_drill -- --ignored --nocapture"]
fn map_drill() {
    whole::require_tree();
    let dir = scratch();
    let mut tsv = std::fs::File::create(dir.join("samples.tsv")).unwrap();
    eprintln!("map drill: screens in {}", dir.display());

    // ---- the title: Show Map writes the page; "Not playing"; the app quits: "App not running".
    let mut app = App::launch_translated(LaunchOpts {
        max_frames: 600,
        timeout_s: 120,
        ..Default::default()
    });
    let map = app.home.join("map");
    let index = map.join("index.html");
    app.start_script("5: menu show-map\n");
    let t0 = Instant::now();
    while !(index.exists() && map.join("live.js").exists()) {
        assert!(t0.elapsed() < Duration::from_secs(60), "Show Map wrote no page");
        std::thread::sleep(Duration::from_millis(100));
    }
    // Let the page's write finish (data files after index.html), then catch its errors.
    std::thread::sleep(Duration::from_millis(1500));
    let html = std::fs::read_to_string(&index).unwrap();
    std::fs::write(&index, html.replacen("<body>", &format!("<body>{CATCH}"), 1)).unwrap();
    let mut pages: Vec<Page> = SIZES.iter().map(|&s| Page::open(&index, s)).collect();
    for p in &mut pages {
        p.wait(
            "document.getElementById('livestat').textContent === 'Not playing'",
            Duration::from_secs(5),
            "the title is not play",
        );
        let s = p.sample(t0);
        log(&mut tsv, &p.name, "title", &s);
        assert_eq!(s.on_screen, None, "{}: a dot on the title", p.name);
        p.shot(&dir, "title");
    }
    let c = app.finish_unread();
    assert!(c.exit.success(), "{}", c.stderr);
    // Every page in turn, read without a screenshot between, until all say it; then the screens.
    let quit = Instant::now();
    let mut seen = [None; 3];
    while seen.iter().any(Option::is_none) {
        for (i, p) in pages.iter_mut().enumerate() {
            if seen[i].is_none() && p.eval_str("document.getElementById('livestat').textContent") == "App not running" {
                seen[i] = Some(quit.elapsed().as_secs_f64());
            }
        }
        assert!(quit.elapsed() < Duration::from_secs(4), "App not running not shown within 4 s of the quit: {seen:?}");
        std::thread::sleep(Duration::from_millis(50));
    }
    for p in &mut pages {
        p.shot(&dir, "quit");
    }
    eprintln!("map drill: App not running after the quit, by page: {seen:?} s");

    // ---- the main run: W1-1's route, world 2's load, W2-1's play.
    let plan = whole::plan(common::route().len(), true);
    let end = plan.end.expect("the route ends level 1");
    let world_2 = plan.world_2.expect("world 2's play begins");
    let steps = plan.steps();
    let fast = end - PACED;
    eprintln!(
        "map drill: level 1 from step {}, ends {end}, world 2's play from {world_2}, {steps} steps, paced from {fast}",
        plan.begins
    );
    app.opts.max_frames = steps;
    app.opts.timeout_s = 1300;
    app.start_script(&format!(
        "fast-until {fast}\n{}",
        whole::key_lines(&plan.inputs)
    ));
    let t0 = Instant::now();
    for p in &mut pages {
        p.wait(
            "document.getElementById('livebar').dataset.state === 'here'",
            Duration::from_secs(120),
            "W1-1's play shown",
        );
    }
    let (big, small) = (1, 2);
    // Pick W3-1 by hand on the laptop page: it holds while W1-1 plays.
    pages[0].pick("W3-1");
    // Every page sampled each 300 ms through W1-1; the second screen's Follow tried by hand.
    let mut w1: Vec<Vec<Sample>> = vec![Vec::new(); 3];
    let mut scrolled: Option<(f64, (f64, f64, f64))> = None;
    let mut resumed = false;
    let (mut resized, mut reloaded) = (false, false);
    let mut loading: Vec<Sample> = Vec::new();
    loop {
        let mut any_w1 = false;
        for (i, p) in pages.iter_mut().enumerate() {
            let s = p.sample(t0);
            log(&mut tsv, &p.name, "w1", &s);
            if s.stat.starts_with("Playing W1-1") {
                any_w1 = true;
            }
            if s.level.is_some() && !s.playing {
                loading.push(s.clone());
            }
            w1[i].push(s);
            if w1[i].len() % 15 == 1 {
                p.shot(&dir, "w1");
            }
        }
        let n = w1[big].len();
        let at = t0.elapsed().as_secs_f64();
        // Once play is on the clock (the dot moves less than a screen a sample), try Follow by hand.
        if n == 20 {
            pages[big].hand_scroll();
            let s = pages[big].sample(t0);
            assert!(!s.follow, "a hand scroll left Follow on");
            scrolled = Some((at, s.view));
            pages[big].shot(&dir, "scrolled");
        }
        if n == 35 && scrolled.is_some() {
            pages[big].click("document.getElementById('zfollow')");
            resumed = true;
            pages[big].shot(&dir, "follow-again");
        }
        if n == 40 && !resized {
            pages[small].b.viewport(1100, 700).unwrap();
            resized = true;
        }
        if n == 50 && !reloaded {
            pages[small].b.viewport(800, 600).unwrap();
            let idx = index.clone();
            pages[small].b.open(&idx).unwrap();
            reloaded = true;
            pages[small].shot(&dir, "reloaded");
        }
        if !any_w1 && n > 5 {
            break;
        }
        assert!(t0.elapsed() < Duration::from_secs(1000), "W1-1 never ended on the page");
        std::thread::sleep(Duration::from_millis(300));
    }
    eprintln!("map drill: W1-1 left after {:.1} s", t0.elapsed().as_secs_f64());

    // ---- world 2's load: "Not playing", then W2-1 by itself on every page, the hand pick included.
    let mut w2: Vec<Vec<Sample>> = vec![Vec::new(); 3];
    let mut saw_not_playing = [false; 3];
    loop {
        let mut done = true;
        for (i, p) in pages.iter_mut().enumerate() {
            let s = p.sample(t0);
            log(&mut tsv, &p.name, "w2", &s);
            if s.stat == "Not playing" {
                saw_not_playing[i] = true;
            }
            if s.state != "here" || s.current != "W2-1" {
                done = false;
            }
            if w2[i].len() % 15 == 0 {
                p.shot(&dir, "w2");
            }
            w2[i].push(s);
        }
        if done && w2[0].len() > 10 {
            break;
        }
        assert!(t0.elapsed() < Duration::from_secs(1100), "the pages never showed W2-1's play");
        std::thread::sleep(Duration::from_millis(300));
    }
    // W2-1's play to the run's end.
    loop {
        let mut playing = false;
        for (i, p) in pages.iter_mut().enumerate() {
            let s = p.sample(t0);
            log(&mut tsv, &p.name, "w2", &s);
            playing |= s.state == "here";
            if w2[i].len() % 15 == 0 {
                p.shot(&dir, "w2");
            }
            w2[i].push(s);
        }
        if !playing {
            break;
        }
        assert!(t0.elapsed() < Duration::from_secs(1200), "W2-1 never stopped");
        std::thread::sleep(Duration::from_millis(300));
    }
    let c = app.finish_unread();
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(!c.stderr.contains("live map"), "{}", c.stderr);
    let quit = Instant::now();
    for p in &mut pages {
        p.wait(
            "document.getElementById('livestat').textContent === 'App not running'",
            Duration::from_secs(5),
            "App not running after the run",
        );
        p.shot(&dir, "after");
    }
    eprintln!("map drill: after the run, App not running in {:.1} s", quit.elapsed().as_secs_f64());

    // ---- judge the samples.
    for (i, p) in pages.iter().enumerate() {
        let name = &p.name;
        let here: Vec<&Sample> = w1[i].iter().chain(&w2[i]).filter(|s| s.state == "here" && !s.loading).collect();
        // The laptop page shows W3-1, picked by hand, through W1-1: its play is W2-1's alone.
        assert!(here.len() > 10, "{name}: {} samples of play shown", here.len());
        for s in &here {
            let (x, y) = s.actor.expect("play has an actor");
            assert_eq!(s.cell, Some(((x / 16.0).floor() as i64, (y / 16.0).floor() as i64)), "{name}: the dot's cell at {:.1}", s.at);
            let (c, r) = s.cell.unwrap();
            assert!(s.stat.ends_with(&format!("character at cell {c},{r}")), "{name}: status {:?}", s.stat);
            assert!(s.count.starts_with("Items left "), "{name}: counter {:?}", s.count);
            if s.follow && s.level.as_deref() == Some(s.current.as_str()) {
                assert_eq!(s.on_screen, Some(true), "{name}: Follow on, the dot off screen at {:.1}", s.at);
            }
            if s.on_screen == Some(true) {
                assert_eq!(s.dot_drawn, Some(true), "{name}: no dot drawn where the character is at {:.1}", s.at);
            }
        }
        for level in ["W1-1", "W2-1"] {
            let items: Vec<i64> = here.iter().filter(|s| s.level.as_deref() == Some(level)).filter_map(|s| s.items_left).collect();
            assert!(items.windows(2).all(|w| w[1] <= w[0]), "{name} {level}: items left rose: {items:?}");
        }
        // Play moved to W2-1 by itself: the view zoomed in on the character, as at load.
        let first_w2 = here.iter().find(|s| s.level.as_deref() == Some("W2-1")).expect("W2-1 shown in play");
        assert!(first_w2.view.0 >= 1.5, "{name}: W2-1 shown at scale {:.2}, not zoomed in on the character", first_w2.view.0);
        assert!(saw_not_playing[i], "{name}: no \"Not playing\" during world 2's load");
        if i == 0 {
            continue;
        }
        let w1_here: Vec<&&Sample> = here.iter().filter(|s| s.level.as_deref() == Some("W1-1")).collect();
        assert!(w1_here.first().unwrap().items_left > w1_here.last().unwrap().items_left, "{name}: items left never fell");
        assert!(w1_here.first().unwrap().gone < w1_here.last().unwrap().gone, "{name}: no item vanished");
        let moving = here.windows(2).any(|w| !w[0].actors.is_empty() && !w[1].actors.is_empty() && w[0].actors != w[1].actors);
        assert!(moving, "{name}: no creature in view moved");
    }
    // The hand pick held through W1-1, with its status naming both.
    let held: Vec<&Sample> = w1[0].iter().filter(|s| s.playing && s.level.as_deref() == Some("W1-1")).skip(3).collect();
    assert!(held.iter().all(|s| s.current == "W3-1" && s.state == "elsewhere"), "the hand pick of W3-1 did not hold");
    assert!(held.iter().all(|s| s.stat == "Playing W1-1 · showing W3-1"), "{:?}", held.last().map(|s| &s.stat));
    // Follow: off after the scroll, the view still until the click; on and the dot in view after.
    let (at, view) = scrolled.expect("Follow was never tried by hand");
    assert!(resumed);
    let after: Vec<&Sample> = w1[big].iter().filter(|s| s.at > at).collect();
    let (off, on): (Vec<&&Sample>, Vec<&&Sample>) = after.iter().partition(|s| !s.follow);
    assert!(!off.is_empty() && off.iter().all(|s| s.view == view), "the view moved with Follow off");
    assert!(on.iter().filter(|s| s.state == "here" && !s.loading).all(|s| s.on_screen == Some(true)), "Follow resumed without the dot in view");

    // ---- after the run: each world, Secrets on and off and every layer, at every size.
    for p in &mut pages {
        for id in IDS {
            p.pick(id);
            p.wait(
                &format!("document.getElementById('lvl').textContent === '{id}' && document.getElementById('loading').hidden"),
                Duration::from_secs(5),
                id,
            );
            p.shot(&dir, &format!("{id}-plain"));
            p.click("[...document.querySelectorAll('#legend .chip')].find(b => b.textContent === 'Secrets')");
            p.shot(&dir, &format!("{id}-secrets"));
            p.click("[...document.querySelectorAll('#legend .chip')].find(b => b.textContent === 'Secrets')");
            let chips = p.b.eval("[...document.querySelectorAll('#legend .chip')].filter(b => !b.hidden).map(b => b.textContent)").unwrap();
            let chips: Vec<String> = chips.as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
            for name in &chips {
                let find = format!("[...document.querySelectorAll('#legend .chip')].find(b => b.textContent === {name:?})");
                p.click(&find);
                if id == "W1-1" || id == "W6-1" {
                    p.shot(&dir, &format!("{id}-toggled-{}", name.replace(|c: char| !c.is_ascii_alphanumeric(), "")));
                }
                p.click(&find);
            }
        }
        // A torn read of live.js raises this alone, and the page keeps its last frame (live_chrome.rs).
        let errs = p.errors();
        assert!(errs == "[]" || errs.trim_matches(['[', ']']).split(',').all(|e| e == "\"Script error.\""), "{}: the page raised errors: {errs}", p.name);
        assert!(p.b.blocked_requests().iter().all(|u| u.contains("fonts.g")), "{}: {:?}", p.name, p.b.blocked_requests());
    }
    drop(pages);
    eprintln!("map drill: done; screens and samples.tsv in {}", dir.display());
}
