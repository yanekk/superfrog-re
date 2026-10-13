//! The intro's divergence finder (T32, for T46-T48): a story case's oracle and engine side by
//! side, past the first divergence. `story_dbg <case.toml> <frames> [<addr:len>...]`:
//! - `SCAN=1`: one `MISS` line per frame whose picture or Paula writes differ (with the first
//!   differing variable); group them into ranges to see what is left.
//! - `BRIEF=1` (`EVENTS=1`, `ALLW=1`): per frame the original's call chain and the jobs, with
//!   both sides' DMACON/INTENA writes (or every write) and their lines.
//! - `FROM=n`: print from frame `n` on.
//! - otherwise: per frame the chain and the watched memory, both sides; `ROW=y,x` prints 24
//!   pixels of row `y` from `x`, both sides.
//! - with `BRIEF=1` or the default (T51): `BLITS=1` lists the original's BLTCON0/BLTSIZE writes and
//!   blitter events (begin, free, end; `+` past the frame's end) with their beam; `REGS=1` its CPU
//!   registers at the frame's end; `CMP=addr:len` (hex) the bytes there that differ, both sides.
use sf_conform::{Case, sync};
use sf_core::layout::Ram;
use sf_core::{Game, Start};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let case = Case::load(std::path::Path::new(&args[1])).unwrap();
    let frames: u32 = args[2].parse().unwrap();
    let watch: Vec<(u32, u32)> = args[3..]
        .iter()
        .map(|a| {
            let (x, n) = a.split_once(':').unwrap();
            (
                u32::from_str_radix(x.trim_start_matches("0x"), 16).unwrap(),
                u32::from_str_radix(n, 16).unwrap(),
            )
        })
        .collect();
    let synced = sync::reach(&case.sync, &case.script, frames + 20000).unwrap();
    let mut amiga = synced.amiga;
    let mut game = Game::with_display(
        sf_conform::display_data(),
        Start::Disk3 {
            seed: Default::default(),
        },
    );
    let mut of = 0u32;
    let mut irqs = Vec::new();
    for f in 0..frames {
        let input = case.script.input_at(of);
        let resumes = sync::intro_resumes(&amiga);
        let mut cap = sync::step_inserting(&mut amiga, input).unwrap();
        let mut entry = sync::handler_entry(&irqs, &amiga.irqs());
        let mut handler_end = sync::handler_end(&cap.paula, resumes);
        irqs = amiga.irqs();
        of += 1;
        let mut pacing = sync::pacing(&amiga.cpu(), &sync::core_memory(amiga.memory()), &cap.paula);
        // A two-frame step of the original: the engine runs both frames (as `sf_conform::run`).
        let mut held = Vec::new();
        if amiga.frames_run() == 2 {
            held = game.step_paced(input, pacing).paula;
        }
        let before = game.clone();
        if std::env::var("NOFEED").is_err() {
            game.set_handler_entry(entry);
            game.set_handler_end(handler_end);
        }
        let mut out = game.step_paced(input, pacing);
        out.paula.splice(0..0, held);
        if let Some(end) = out.load {
            let mut seen = false;
            let mut prev_mem = sync::core_memory(amiga.memory());
            loop {
                let m = sync::core_memory(amiga.memory());
                let chain = sync::main_chain(&amiga.cpu(), &m);
                let over = match end {
                    sf_core::LoadEnd::Reached(a) => chain.iter().take(4).any(|&x| x == a),
                    sf_core::LoadEnd::Left(a) => {
                        let inside = chain.contains(&a);
                        seen |= inside;
                        seen && !inside
                    }
                };
                if over {
                    break;
                }
                prev_mem = sync::core_memory(amiga.memory());
                let resumes = sync::intro_resumes(&amiga);
                cap = sync::step_inserting(&mut amiga, case.script.input_at(of)).unwrap();
                entry = sync::handler_entry(&irqs, &amiga.irqs());
                handler_end = sync::handler_end(&cap.paula, resumes);
                irqs = amiga.irqs();
                of += 1;
            }
            pacing = sync::pacing(&amiga.cpu(), &sync::core_memory(amiga.memory()), &cap.paula);
            let m = sync::core_memory(amiga.memory());
            let take = |g: &mut Game, from: &sf_core::AmigaMemory, runs: &[(u32, u32)]| {
                for &(addr, len) in runs {
                    let b: Vec<u8> = (addr..addr + len).map(|a| from.byte(a)).collect();
                    g.poke(addr, &b);
                }
            };
            let redo = |line: Option<u16>| {
                let mut g = before.clone();
                let runs = before
                    .runs_during_load()
                    .iter()
                    .map(|&(a, n)| (a, (a..a + n).map(|x| prev_mem.byte(x)).collect()))
                    .collect();
                g.set_load_state(runs);
                g.set_load_line(line);
                if std::env::var("NOFEED").is_err() {
                    g.set_handler_entry(entry);
                    g.set_handler_end(handler_end);
                }
                let o = g.step_paced(input, pacing);
                let leaves = g.load_leaves();
                take(&mut g, &m, leaves);
                take(&mut g, &m, &o.resync);
                (g, o)
            };
            (game, out) = redo(None);
            if let sf_core::LoadEnd::Left(_) = end {
                let fits = |g: &Game| {
                    g.layout()
                        .iter()
                        .flat_map(|d| d.pieces())
                        .zip(g.probe())
                        .all(|(p, (_, v))| p.read(&m) == v)
                };
                if !fits(&game) {
                    for line in (0..313u16).step_by(2) {
                        let (g, o) = redo(Some(line));
                        if fits(&g) {
                            println!("  load line {line}");
                            game = g;
                            out = o;
                            break;
                        }
                    }
                }
            }
        }
        let out_pic = out.picture.clone();
        let m = sync::core_memory(amiga.memory());
        if let Ok(spec) = std::env::var("PAL") {
            // `PAL=a-b`: per frame in a..b, each (original, engine) colour pair that differs,
            // with its pixel count and first row.
            let (a, b) = spec.split_once('-').unwrap();
            if (a.parse::<u32>().unwrap()..=b.parse::<u32>().unwrap()).contains(&f) {
                let ours = sf_core::to_rgb12(&out.picture);
                let (w, h) = (
                    out.picture.width.min(cap.picture.width),
                    out.picture.height.min(cap.picture.height),
                );
                // Per palette index: the original's colour where the engine's differs.
                let mut seen: std::collections::BTreeMap<u8, (u16, u16, u32)> = Default::default();
                for y in 0..h {
                    for x in 0..w {
                        let i = y as usize * out.picture.width as usize + x as usize;
                        let (t, o) = (cap.picture.pixel(x, y), ours[i]);
                        if t != o {
                            let e = seen.entry(out.picture.pixels[i]).or_insert((t, o, 0));
                            e.2 += 1;
                        }
                    }
                }
                let s: Vec<String> = seen
                    .iter()
                    .map(|(i, (t, o, n))| format!("{i}:{t:03x}/{o:03x}x{n}"))
                    .collect();
                println!("PAL F {f} {}", s.join("  "));
            }
        }
        if std::env::var("DRAWS").is_ok() {
            // `DRAWS=1` (T54): frames whose DMACON writes (a draw's `$8400` and `$0400` above all)
            // differ in value or order, both sides with their lines.
            let dmacon = |ws: &[sf_paula::PaulaWrite]| {
                ws.iter()
                    .filter(|w| w.reg == 0x96)
                    .map(|w| (w.value, w.vpos))
                    .collect::<Vec<_>>()
            };
            let (o, t) = (dmacon(&out.paula), dmacon(&cap.paula));
            if o.iter().map(|w| w.0).ne(t.iter().map(|w| w.0)) {
                println!("DRAW F {f} O {} ours {o:x?} orig {t:x?}", of - 1);
            }
            continue;
        }
        if std::env::var("SCAN").is_ok() {
            let pd = sf_conform::compare::picture(&out.picture, &cap.picture);
            let ad = sf_conform::compare::paula(&out.paula, &cap.paula);
            let vd = sf_conform::compare::vars(game.layout(), &game.probe(), &m, &[]);
            if pd.is_some() || ad.is_some() {
                println!(
                    "MISS F {f} O {} pic {} paula {} var {}",
                    of - 1,
                    pd.map_or(String::from("-"), |p| format!(
                        "{:?} rows {:?}",
                        p.first, p.rows
                    )),
                    ad.map_or(String::from("-"), |a| a
                        .lines()
                        .next()
                        .unwrap_or("")
                        .to_string()),
                    vd.map_or(String::from("-"), |v| v.0.name.to_string())
                );
            }
            continue;
        }
        let chain = sync::main_chain(&amiga.cpu(), &m);
        let rel: Vec<String> = chain
            .iter()
            .map(|&a| {
                if (0xc00fa0..0xc53100).contains(&a) {
                    format!("{:x}", a - 0xc00fa0)
                } else {
                    format!("@{a:x}")
                }
            })
            .collect();
        let t = game.story().and_then(|s| s.intro_clock()).unwrap_or(0);
        let jobs: Vec<String> = out
            .jobs
            .iter()
            .filter(|j| !j.inside.is_empty())
            .map(|j| format!("{:x}", j.inside[0].wrapping_sub(0xc00fa0)))
            .collect();
        if std::env::var("FROM").is_ok_and(|v| f < v.parse::<u32>().unwrap()) {
            continue;
        }
        if std::env::var("BRIEF").is_ok() {
            println!(
                "F {f} O {} t {t} chain {} jobs {} load {}",
                of - 1,
                rel.join("<"),
                jobs.join(","),
                out.load.map_or(String::new(), |l| format!("{l:x?}"))
            );

            let all = std::env::var("ALLW").is_ok();
            let ev = |ws: &[sf_paula::PaulaWrite]| {
                ws.iter()
                    .filter(|w| all || w.reg == 0x96 || w.reg == 0x9a)
                    .map(|w| format!("{:x}={:x}@{}", w.reg, w.value, w.vpos))
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            if std::env::var("IRQS").is_ok() {
                println!("  irqs {:?}", amiga.irqs());
            }
            if std::env::var("EVENTS").is_ok() {
                println!("  ours {}\n  orig {}", ev(&out.paula), ev(&cap.paula));
            }
            extras(&amiga, &game, &m);
            continue;
        }
        println!(
            "frame {f} (oracle {}) chain {:x?} lines {}",
            of - 1,
            chain,
            amiga.frame_lines()
        );
        if let Ok(spec) = std::env::var("ROW") {
            let (y, x0) = spec.split_once(',').unwrap();
            let (y, x0): (u16, u16) = (y.parse().unwrap(), x0.parse().unwrap());
            let ours = sf_core::to_rgb12(&out_pic);
            let o: Vec<String> = (x0..x0 + 24)
                .map(|x| {
                    format!(
                        "{:03x}",
                        ours[y as usize * out_pic.width as usize + x as usize]
                    )
                })
                .collect();
            let t: Vec<String> = (x0..x0 + 24)
                .map(|x| format!("{:03x}", cap.picture.pixel(x, y)))
                .collect();
            println!(
                "  row {y} orig {}\n  row {y} ours {}",
                t.join(" "),
                o.join(" ")
            );
        }
        for &(a, n) in &watch {
            let theirs: Vec<u8> = (a..a + n).map(|x| m.byte(x)).collect();
            let ours = game.peek(a, n);
            let hex = |b: &[u8]| {
                b.chunks(2)
                    .map(|w| format!("{:02x}{:02x}", w[0], w.get(1).copied().unwrap_or(0)))
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            println!("  {a:06x} orig {}", hex(&theirs));
            if theirs != ours {
                println!("  {a:06x} ours {}", hex(&ours));
            }
        }
        extras(&amiga, &game, &m);
    }
}

/// `BLITS=1`, `REGS=1` and `CMP=addr:len` (T51): what times the original's blits, its CPU
/// registers, and the bytes in a range that differ between it and the engine.
fn extras(amiga: &sf_oracle::Amiga, game: &Game, m: &sf_core::AmigaMemory) {
    if std::env::var("BLITS").is_ok() {
        let writes: Vec<String> = amiga
            .cpu_writes()
            .iter()
            .filter(|w| w.reg == 0x40 || w.reg == 0x58)
            .map(|w| {
                let name = if w.reg == 0x40 { "con0" } else { "size" };
                format!("{name}={:x}@{},{}", w.value, w.vpos, w.hpos)
            })
            .collect();
        println!("  blit writes {}", writes.join(" "));
        let events: Vec<String> = amiga
            .blit_events()
            .iter()
            .map(|e| {
                let kind = match e.kind {
                    sf_oracle::BlitEventKind::Begin => "begin",
                    sf_oracle::BlitEventKind::Free => "free",
                    sf_oracle::BlitEventKind::End => "end",
                };
                let next = if e.next { "+" } else { "" };
                format!("{kind}@{next}{},{}", e.vpos, e.hpos)
            })
            .collect();
        println!(
            "  blit events (frames run {}) {}",
            amiga.frames_run(),
            events.join(" ")
        );
    }
    if std::env::var("REGS").is_ok() {
        let r = amiga.cpu();
        let hex = |v: &[u32]| {
            v.iter()
                .map(|x| format!("{x:08x}"))
                .collect::<Vec<_>>()
                .join(" ")
        };
        println!(
            "  regs pc {:06x} sr {:04x} usp {:08x} ssp {:08x}\n  regs d {}\n  regs a {}",
            r.pc,
            r.sr,
            r.usp,
            r.ssp,
            hex(&r.d),
            hex(&r.a)
        );
    }
    if let Ok(spec) = std::env::var("CMP") {
        let (x, n) = spec.split_once(':').expect("CMP=addr:len");
        let addr = u32::from_str_radix(x.trim_start_matches("0x"), 16).unwrap();
        let len = u32::from_str_radix(n, 16).unwrap();
        let ours = game.peek(addr, len);
        let differ: Vec<String> = (addr..addr + len)
            .zip(&ours)
            .filter(|&(a, &o)| m.byte(a) != o)
            .map(|(a, &o)| format!("{a:06x}:{:02x}/{o:02x}", m.byte(a)))
            .collect();
        println!(
            "  cmp {addr:06x}+{len:x} {} differ (orig/ours) {}",
            differ.len(),
            differ.join(" ")
        );
    }
}
