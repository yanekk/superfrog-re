//! The six-level map page the real `fd-routeview --all` writes (plan forest-dumb-map-page T08):
//! its data files against the owner's disk files, the command's bad arguments, and the page in
//! headless Chrome (amiga-browser) at 1280×800, 1920×1080 and 800×600: the switcher lists W1-1 to
//! W6-1, each level opens "Map only" with EXIT drawn over its `$ff` cells and the items needed as
//! `fd_map::items_needed` reads them, W6-1 reaches column 319, every layer toggles, W3-1's flames
//! are on the springs/blocks/exit layer, and with no `live.js` the page says "App not running" and
//! asks for nothing outside `file://`. Needs both disks and Chrome; either missing fails, never
//! skips. Screens go to the test's temp folder.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use amiga_browser::{Browser, Opts};

const BIN: &str = env!("CARGO_BIN_EXE_fd-routeview");
const IDS: [&str; 6] = ["W1-1", "W2-1", "W3-1", "W4-1", "W5-1", "W6-1"];
const ROWS: usize = 96;
const MISC: &str = "Springs, vanishing blocks, flames, the exit";

fn temp(name: &str) -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("levels-{name}"));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(BIN).args(args).output().unwrap()
}

/// `_MapaBobowN` as the disk holds it, unpacked: column by column, 96 rows a column.
fn object_file(world: usize) -> Vec<u8> {
    let dir = fd_tools::disk_dir().unwrap_or_else(|e| panic!("{e}"));
    fd_tools::game_file(&dir, &format!("disk2/_MapaBobow{world}")).unwrap_or_else(|e| panic!("{e}"))
}

/// The JSON array after `"key":` in `s`, brackets matched.
fn array<'a>(s: &'a str, key: &str) -> &'a str {
    let at = s
        .find(&format!("\"{key}\":["))
        .unwrap_or_else(|| panic!("no {key}"))
        + key.len()
        + 3;
    let mut depth = 0;
    for (i, ch) in s[at..].char_indices() {
        match ch {
            '[' => depth += 1,
            ']' if depth == 1 => return &s[at..at + i + 1],
            ']' => depth -= 1,
            _ => {}
        }
    }
    panic!("{key} not closed")
}

/// The first `n` numbers of each inner array of a `[[…],[…]]` array.
fn rows_of(a: &str, n: usize) -> Vec<Vec<usize>> {
    let inner = &a[1..a.len() - 1];
    if inner.is_empty() {
        return vec![];
    }
    inner
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split("],[")
        .map(|t| {
            t.split(',')
                .take(n)
                .map(|x| x.parse().unwrap_or_else(|_| panic!("{t}")))
                .collect()
        })
        .collect()
}

fn number(s: &str, key: &str) -> usize {
    let at = s
        .find(&format!("\"{key}\":"))
        .unwrap_or_else(|| panic!("no {key}"))
        + key.len()
        + 3;
    s[at..]
        .split(|c: char| !c.is_ascii_digit())
        .next()
        .unwrap()
        .parse()
        .unwrap()
}

/// `--all` writes six data files; each level's objects are its file's non-zero cells over the
/// used columns, value for value, its exit the `$ff` cells, its items needed `fd_map::items_needed`.
#[test]
fn all_writes_six_levels_matching_their_files() {
    let out = temp("all");
    let r = run(&["--all", "--out", out.to_str().unwrap()]);
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    let said = String::from_utf8_lossy(&r.stdout);
    assert_eq!(
        said.trim(),
        format!("{}: 6 levels", out.join("index.html").display())
    );
    let index = std::fs::read_to_string(out.join("index.html")).unwrap();
    assert!(
        index.contains(r#""live":{"file":"live.js""#),
        "live mode off"
    );
    assert!(!index.contains("://"), "the page names a URL");
    for (i, id) in IDS.iter().enumerate() {
        let js = std::fs::read_to_string(out.join(format!("data/{id}.js"))).unwrap();
        let file = object_file(i + 1);
        let cols = number(&js, "cols");
        assert_eq!(cols, if i == 5 { 320 } else { 240 }, "{id}");
        let objects = rows_of(array(&js, "objects"), 3);
        let want = file[..cols * ROWS].iter().filter(|&&v| v != 0).count();
        assert_eq!(objects.len(), want, "{id}: object count");
        for o in &objects {
            assert_eq!(
                file[o[0] * ROWS + o[1]] as usize,
                o[2],
                "{id}: cell {},{}",
                o[0],
                o[1]
            );
        }
        let exit = rows_of(array(&js, "exit"), 2);
        let ff: Vec<Vec<usize>> = (0..cols * ROWS)
            .filter(|&k| file[k] == 0xff)
            .map(|k| vec![k / ROWS, k % ROWS])
            .collect();
        assert_eq!(exit, ff, "{id}: exit");
        let needed = fd_map::items_needed(&format!("_MapaBobow{}", i + 1), &file).unwrap();
        assert_eq!(number(&js, "coins_needed"), needed as usize, "{id}");
        assert!(
            index.contains(&format!(
                r#""id":"{id}","label":"{id}","group":"W{}","status":"map""#,
                i + 1
            )),
            "{id}: not map only"
        );
    }
}

#[test]
fn bad_arguments_exit_2_naming_what() {
    let out = temp("bad");
    let o = out.to_str().unwrap();
    let r = run(&["--level", "W9-1", "--out", o]);
    assert_eq!(r.status.code(), Some(2));
    let e = String::from_utf8_lossy(&r.stderr);
    assert!(
        e.contains("W9-1") && IDS.iter().all(|id| e.contains(id)),
        "{e}"
    );
    for args in [
        &["--level", "W2-1", "--all"][..],
        &[][..],
        &["--bogus"][..],
        &["--level"][..],
    ] {
        assert_eq!(run(args).status.code(), Some(2), "{args:?}");
    }
    let r = run(&["--all", "--script", "/nonexistent/script.txt", "--out", o]);
    assert_eq!(r.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&r.stderr).contains("/nonexistent/script.txt"));
    assert!(!out.exists(), "a bad call wrote a page");
}

/// One level, map only; and `--script --all` puts the route on W1-1 beside five map-only levels.
#[test]
fn one_level_and_a_route_beside_the_others() {
    let out = temp("one");
    let r = run(&["--level", "W6-1", "--out", out.to_str().unwrap()]);
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    assert!(
        String::from_utf8_lossy(&r.stdout)
            .trim()
            .ends_with(": 1 level")
    );
    let index = std::fs::read_to_string(out.join("index.html")).unwrap();
    assert!(index.contains(r#""id":"W6-1""#) && !index.contains(r#""id":"W1-1""#));

    let out = temp("route");
    let script = fd_conform::conform_dir().join("scripts/stretch-run.txt");
    let r = run(&[
        "--script",
        script.to_str().unwrap(),
        "--all",
        "--out",
        out.to_str().unwrap(),
    ]);
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    let said = String::from_utf8_lossy(&r.stdout);
    assert!(said.contains(": 6 levels, W1-1's route "), "{said}");
    let index = std::fs::read_to_string(out.join("index.html")).unwrap();
    assert!(index.contains(r#""id":"W1-1","label":"W1-1","group":"W1","status":"unknown""#));
    assert_eq!(index.matches(r#""status":"map""#).count(), 5);
    let w1 = std::fs::read_to_string(out.join("data/W1-1.js")).unwrap();
    assert!(array(&w1, "track").len() > 100, "W1-1 has no route");
}

// ---------------------------------------------------------------------------------------------
// In Chrome
// ---------------------------------------------------------------------------------------------

/// Errors the page raises, collected from its first line on.
const CATCH: &str = "<script>window.ERRS=[];addEventListener('error',e=>ERRS.push(String(e.message)));\
addEventListener('unhandledrejection',e=>ERRS.push(String(e.reason)));\
{const ce=console.error;console.error=(...a)=>{ERRS.push(a.join(' '));ce(...a)}}</script>";

fn site() -> PathBuf {
    let out = temp("chrome");
    let r = run(&["--all", "--out", out.to_str().unwrap()]);
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    let index = out.join("index.html");
    let html = std::fs::read_to_string(&index).unwrap();
    std::fs::write(
        &index,
        html.replacen("<body>", &format!("<body>{CATCH}"), 1),
    )
    .unwrap();
    out
}

fn eval_str(b: &mut Browser, js: &str) -> String {
    b.eval(js).unwrap().as_str().unwrap_or_default().to_string()
}

fn eval_i(b: &mut Browser, js: &str) -> i64 {
    b.eval(js)
        .unwrap()
        .as_i64()
        .unwrap_or_else(|| panic!("{js}: not a number"))
}

fn click(b: &mut Browser, js: &str) {
    let r = b
        .eval(&format!(
            "(() => {{ const e = {js}; if (!e) return null; e.scrollIntoView({{block: 'center'}}); const r = e.getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 2]; }})()"
        ))
        .unwrap();
    assert!(!r.is_null(), "nothing found by {js}");
    b.click(r[0].as_f64().unwrap(), r[1].as_f64().unwrap())
        .unwrap();
}

fn chip(name: &str) -> String {
    format!("[...document.querySelectorAll('#legend .chip')].find(b => b.textContent === '{name}')")
}

fn wait_level(b: &mut Browser, id: &str) {
    b.wait_for(
        &format!("document.getElementById('lvl').textContent === '{id}' && document.getElementById('loading').hidden"),
        Duration::from_secs(5),
    )
    .unwrap_or_else(|e| panic!("never showed {id}: {e}"));
}

/// Pure white pixels on the canvas over level pixels `x0..x1`, `y0..y1`. The page dims the map, so
/// no tile is pure white: only a mark of the springs/blocks/exit layer (`#ffffff`) or EXIT's label.
fn white(b: &mut Browser, (x0, y0, x1, y1): (i32, i32, i32, i32)) -> i64 {
    eval_i(
        b,
        &format!(
            "(() => {{ const v = ROUTEVIEW.view(), c = document.getElementById('cv'), d = devicePixelRatio || 1; \
             const sx = p => Math.round((p * v.scale + v.ox) * d), sy = p => Math.round((p * v.scale + v.oy) * d); \
             const a = Math.max(0, sx({x0})), t = Math.max(0, sy({y0})), z = Math.min(c.width, sx({x1})), w = Math.min(c.height, sy({y1})); \
             if (z <= a || w <= t) return -1; \
             const px = c.getContext('2d').getImageData(a, t, z - a, w - t).data; let n = 0; \
             for (let i = 0; i < px.length; i += 4) if (px[i] === 255 && px[i+1] === 255 && px[i+2] === 255) n++; return n; }})()"
        ),
    )
}

/// A digest of the whole canvas.
fn digest(b: &mut Browser) -> i64 {
    eval_i(
        b,
        "(() => { const c = document.getElementById('cv'), d = c.getContext('2d').getImageData(0, 0, c.width, c.height).data; \
         let h = 0; for (let i = 0; i < d.length; i++) h = (h * 31 + d[i]) | 0; return h; })()",
    )
}

/// The cells around `(c, r)`, in level pixels: a cell's mark is drawn wider than the cell when the
/// view is small, and EXIT's label above it.
fn around(c: i64, r: i64, w: i64, h: i64) -> (i32, i32, i32, i32) {
    (
        ((c - 2) * 16) as i32,
        ((r - 4) * 16) as i32,
        ((c + w + 2) * 16) as i32,
        ((r + h + 2) * 16) as i32,
    )
}

fn errors(b: &mut Browser) -> Vec<String> {
    let v = b.eval("window.ERRS").unwrap();
    v.as_array()
        .unwrap()
        .iter()
        .map(|s| s.to_string())
        .collect()
}

fn drive(root: &Path, (w, h): (u32, u32), needed: &[u16]) {
    let wh = format!("{w}x{h}");
    let mut b = Browser::launch(Opts {
        width: w,
        height: h,
        ..Opts::default()
    })
    .unwrap_or_else(|e| panic!("amiga-browser: {e}"));
    let shot = |b: &mut Browser, name: &str| {
        b.screenshot(&root.join(format!("{wh}-{name}.png")))
            .unwrap()
    };
    b.open(&root.join("index.html")).unwrap();
    wait_level(&mut b, "W1-1");

    // No live.js written yet: the page says so and draws no character.
    b.wait_for(
        "document.getElementById('livebar').textContent.includes('App not running')",
        Duration::from_secs(3),
    )
    .unwrap_or_else(|_| {
        panic!(
            "{wh}: status {:?}",
            eval_str(&mut b, "document.getElementById('livebar').textContent")
        )
    });

    // The switcher lists the six levels in order.
    let listed = b
        .eval("[...document.querySelectorAll('#worlds .rt')].map(e => e.textContent.slice(1))")
        .unwrap();
    let listed: Vec<&str> = listed
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(listed, IDS, "{wh}: the switcher");

    for (i, id) in IDS.iter().enumerate() {
        click(
            &mut b,
            &format!(
                "[...document.querySelectorAll('#worlds .rt')].find(e => e.textContent.endsWith('{id}'))"
            ),
        );
        wait_level(&mut b, id);
        shot(&mut b, id);
        let verdict = eval_str(&mut b, "document.getElementById('verdict').textContent");
        assert!(verdict.starts_with("Map only"), "{wh} {id}: {verdict}");
        assert!(
            !b.eval("document.getElementById('scrub').checkVisibility()")
                .unwrap()
                .as_bool()
                .unwrap(),
            "{wh} {id}: a timeline on a map-only level"
        );
        // The items-needed line.
        let stat = eval_str(
            &mut b,
            "(() => { const d = [...document.querySelectorAll('#stats dt')].find(e => e.textContent === 'Coins the exit needs'); return d ? d.nextElementSibling.textContent : ''; })()",
        );
        assert_eq!(stat, needed[i].to_string(), "{wh} {id}: items needed");
        // EXIT over the $ff cells: white there with the layer on, none with it off.
        let (c0, r0) = (
            eval_i(&mut b, &format!("ROUTE_DATA['{id}'].exit[0][0]")),
            eval_i(&mut b, &format!("ROUTE_DATA['{id}'].exit[0][1]")),
        );
        let near = around(c0, r0, 2, 3);
        let on = white(&mut b, near);
        click(&mut b, &chip(MISC));
        let off = white(&mut b, near);
        click(&mut b, &chip(MISC));
        assert!(
            on > 0 && off == 0,
            "{wh} {id}: exit at {c0},{r0}: {on} white with the layer on, {off} off"
        );
    }

    // W6-1 is drawn to its last column: a click there names column 319.
    click(
        &mut b,
        "[...document.querySelectorAll('#worlds .rt')].find(e => e.textContent.endsWith('W6-1'))",
    );
    wait_level(&mut b, "W6-1");
    let at = b
        .eval(
            "(() => { document.getElementById('mapbox').scrollIntoView({block: 'center'}); \
             const v = ROUTEVIEW.view(), r = document.getElementById('mapbox').getBoundingClientRect(); \
             return [r.x + (319 * 16 + 8) * v.scale + v.ox, r.y + (48 * 16 + 8) * v.scale + v.oy]; })()",
        )
        .unwrap();
    let (x, y) = (at[0].as_f64().unwrap(), at[1].as_f64().unwrap());
    assert!(
        x < w as f64 && y > 0.0 && y < h as f64,
        "{wh}: column 319 off screen at {x},{y}"
    );
    b.click(x, y).unwrap();
    let readout = eval_str(&mut b, "document.getElementById('readout').textContent");
    assert!(readout.starts_with("W6-1:319,"), "{wh}: {readout}");

    // W3-1: each layer with objects changes the map when switched off and back; its flames are on
    // the springs/blocks/exit layer.
    click(
        &mut b,
        "[...document.querySelectorAll('#worlds .rt')].find(e => e.textContent.endsWith('W3-1'))",
    );
    wait_level(&mut b, "W3-1");
    let layers: [(&str, &str); 5] = [
        ("Coins", "o[3] === 'coin'"),
        ("Creatures", "o[3] === 'enemy'"),
        ("Bonuses", "o[3] === 'pickup'"),
        ("Water", "o[3] === 'hazard'"),
        (
            MISC,
            "!['coin', 'enemy', 'pickup', 'hazard'].includes(o[3])",
        ),
    ];
    // The pointer leaves the map first (the click on column 319 left a hover mark there).
    click(&mut b, &chip("Coins"));
    click(&mut b, &chip("Coins"));
    let base = digest(&mut b);
    for (name, is) in layers {
        let n = eval_i(
            &mut b,
            &format!("ROUTE_DATA['W3-1'].objects.filter(o => {is}).length"),
        );
        click(&mut b, &chip(name));
        let off = digest(&mut b);
        click(&mut b, &chip(name));
        assert_eq!(digest(&mut b), base, "{wh}: {name} back on");
        if n > 0 {
            assert_ne!(off, base, "{wh}: {name} off changed nothing ({n} objects)");
        }
    }
    let flames = b
        .eval("ROUTE_DATA['W3-1'].objects.filter(o => o[2] === 0x34).map(o => [o[0], o[1], o[3]])")
        .unwrap();
    let flames = flames.as_array().unwrap();
    assert_eq!(flames.len(), 31, "{wh}: W3-1's flames");
    // A flame alone: no other object of the layer within the cells `around` looks at.
    let lone = eval_str(
        &mut b,
        "(() => { const O = ROUTE_DATA['W3-1'].objects, misc = o => !['coin', 'enemy', 'pickup', 'hazard'].includes(o[3]); \
         const f = O.find(o => o[2] === 0x34 && !O.some(p => p !== o && misc(p) && Math.abs(p[0] - o[0]) <= 3 && p[1] - o[1] >= -5 && p[1] - o[1] <= 3)); \
         return f ? f[0] + ',' + f[1] + ',' + f[3] : ''; })()",
    );
    let f: Vec<&str> = lone.split(',').collect();
    assert_eq!(f.len(), 3, "{wh}: no lone flame in W3-1");
    assert_eq!(f[2], "flame");
    let near = around(f[0].parse().unwrap(), f[1].parse().unwrap(), 1, 1);
    let on = white(&mut b, near);
    click(&mut b, &chip(MISC));
    let off = white(&mut b, near);
    click(&mut b, &chip(MISC));
    shot(&mut b, "W3-1-layers");
    assert!(
        on > 0 && off == 0,
        "{wh}: flame at {lone}: {on} white on, {off} off"
    );

    let wide = b
        .eval("document.documentElement.scrollWidth <= innerWidth")
        .unwrap();
    assert_eq!(
        wide.as_bool(),
        Some(true),
        "{wh}: the page scrolls sideways"
    );
    assert_eq!(errors(&mut b), Vec::<String>::new(), "{wh}");
    assert_eq!(
        b.blocked_requests(),
        Vec::<String>::new(),
        "{wh}: a request outside file://"
    );
}

#[test]
fn the_six_levels_in_chrome_at_three_sizes() {
    let root = site();
    let needed: Vec<u16> = (1..=6)
        .map(|w| fd_map::items_needed("x", &object_file(w)).unwrap())
        .collect();
    for size in [(1280, 800), (1920, 1080), (800, 600)] {
        drive(&root, size, &needed);
    }
}
