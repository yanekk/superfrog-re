//! The renderer's engine ports (`fd_core::render`, plans/forest-dumb-level-1 T10) against the original
//! on the routine rig, over 10 000 states of each generator (T37's for the drawing routines of
//! `re/translate.toml`, this task's `gen/status.rs` for the status bar's), on the main program from the
//! owner's disks; and the display at the level sync point against the original's.
//!
//! Each check loads the engine's typed state from the state's memory, runs the engine function and
//! stores what it changed back, so the rig compares every byte the original writes with the engine's.
//! No register is compared where no caller reads one: `game_frame` and `main_loop` reload theirs after
//! each call. `h0_9550` returns its colour in `d3`, which `h0_94fe` stores, and a digit's callers
//! read `d1` and `d2` after it.

use std::sync::{Arc, OnceLock};

use fd_conform::generators::{self, Gen};
use fd_conform::judge::{self, Cpu};
use fd_conform::routine::{RegsCompared, Rig, RigReport};
use fd_conform::sync::{LEVEL_1_SYNC, reach_level_1};
use fd_conform::{Live, game_data};
use fd_core::program::Program;
use fd_core::render::objects::{CharSprite, SPRITES};
use fd_core::render::palette::{self, Fade};
use fd_core::render::playfield::{self, Scroll};
use fd_core::render::status::{STATUS, STATUS_BYTES, Shown, Sources, Status};
use fd_core::render::{COLOURS, PALETTE, WINDOW};
use fd_core::{Game, Mem, PORTED, Start, Transplant};

const CASES: usize = 10_000;

fn h(hunk: usize, o: u32) -> u32 {
    generators::h(hunk, o)
}

/// The rig's memory: the program as loaded. None of these generators lays a base, so it is the same
/// for all, and the program's data a port reads (the fade table, the font, the icon table) is there.
static BASE: OnceLock<Arc<Vec<u8>>> = OnceLock::new();

fn program() -> Program {
    Program::memory(BASE.get().expect("BASE set before a check").clone())
}

fn put(cpu: &mut Cpu, writes: Vec<(u32, Vec<u8>)>, hunk: usize) {
    for (off, bytes) in writes {
        for (k, b) in bytes.into_iter().enumerate() {
            cpu.mem.wb(h(hunk, off) + k as u32, b);
        }
    }
}

// --- the character's sprites ------------------------------------------------------------------

fn sprite_position(cpu: &mut Cpu) {
    let m = &cpu.mem;
    let mut s = CharSprite {
        ctl: 0,
        frame: None,
    };
    s.position(
        m.rl(h(0, 0x1e5a)),
        m.rl(h(0, 0x1e5e)),
        m.rw(h(0, 0x1e62)),
        m.rw(h(0, 0x1e64)),
    );
    let ctl: Vec<_> = s
        .bytes(&|_, n| vec![0; n as usize])
        .into_iter()
        .filter(|(off, _)| (off - SPRITES) % 0xa8 == 0)
        .collect();
    put(cpu, ctl, 1);
}

fn sprite_copy(cpu: &mut Cpu) {
    let mut t = cpu.mem.rb(h(0, 0x9b04));
    let mut s = CharSprite {
        ctl: 0,
        frame: None,
    };
    s.copy(&mut t, cpu.mem.rl(h(0, 0x4ed8)));
    cpu.mem.wb(h(0, 0x9b04), t);
    let m = cpu.mem.clone();
    let frame = |a: u32, n: u32| (0..n).map(|k| m.rb(a.wrapping_add(k))).collect::<Vec<u8>>();
    let data: Vec<_> = s
        .bytes(&frame)
        .into_iter()
        .filter(|(off, _)| (off - SPRITES) % 0xa8 != 0)
        .collect();
    put(cpu, data, 1);
}

// --- the play screen's pointers ---------------------------------------------------------------

fn scroll(cpu: &mut Cpu) {
    let s = Scroll::from_camera(cpu.mem.rw(h(0, 0x1e62)), cpu.mem.rw(h(0, 0x1e64)));
    put(cpu, s.copper(), 1);
}

// --- the palette --------------------------------------------------------------------------------

fn load_fade(cpu: &Cpu) -> (Fade, [u16; 32]) {
    let m = &cpu.mem;
    let fade = Fade {
        on: m.rb(h(0, 0x94b4)),
        pass: m.rl(h(0, 0x94b6)),
        now: std::array::from_fn(|i| m.rw(h(1, COLOURS + 4 * i as u32))),
        scratch: std::array::from_fn(|i| m.rw(h(0, 0x961e + 2 * i as u32))),
    };
    let to = std::array::from_fn(|i| m.rw(h(1, PALETTE + 4 * i as u32)));
    (fade, to)
}

fn store_fade(cpu: &mut Cpu, f: &Fade) {
    let m = &mut cpu.mem;
    m.wb(h(0, 0x94b4), f.on);
    m.wl(h(0, 0x94b6), f.pass);
    for (i, c) in f.now.iter().enumerate() {
        m.ww(h(1, COLOURS + 4 * i as u32), *c);
    }
    for (i, w) in f.scratch.iter().enumerate() {
        m.ww(h(0, 0x961e + 2 * i as u32), *w);
    }
}

fn fade_step(cpu: &mut Cpu) {
    let (mut f, to) = load_fade(cpu);
    f.step(&to, &program());
    store_fade(cpu, &f);
}

fn fade_all(cpu: &mut Cpu) {
    let (mut f, to) = load_fade(cpu);
    f.all(&to, cpu.d[6], &program());
    store_fade(cpu, &f);
}

fn fade_colour(cpu: &mut Cpu) {
    let (c, s) = palette::colour(
        cpu.d[0] as u16,
        cpu.d[1] as u16,
        cpu.d[6] as u16,
        &program(),
    );
    for (i, w) in s.iter().enumerate() {
        cpu.mem.ww(h(0, 0x961e + 2 * i as u32), *w);
    }
    cpu.d[3] = cpu.d[3] & 0xffff_0000 | c as u32;
}

// --- the status bar -----------------------------------------------------------------------------

/// The bar from the state's memory, `f` run on it with its sources read there, and the bytes it
/// changed stored back.
fn on_status(cpu: &mut Cpu, f: impl FnOnce(&mut Status, &Sources, &Cpu)) {
    let m = cpu.mem.clone();
    let word = |a: u32| m.rw(h(0, a));
    let before = Status {
        bytes: (0..STATUS_BYTES).map(|k| m.rb(h(4, STATUS + k))).collect(),
        shown: Shown {
            countdown: word(0x6a8c),
            lives: word(0x6a8e),
            items: word(0x6a90),
            score: m.rl(h(0, 0x6a92)),
        },
    };
    let mut s = before.clone();
    let h4 = |o: u32| m.rb(h(4, 0).wrapping_add(o));
    let p = program();
    f(
        &mut s,
        &Sources {
            h4: &h4,
            program: &p,
        },
        cpu,
    );
    for (k, (a, b)) in s.bytes.iter().zip(&before.bytes).enumerate() {
        if a != b {
            cpu.mem.wb(h(4, STATUS + k as u32), *a);
        }
    }
    let sh = s.shown;
    let mm = &mut cpu.mem;
    mm.ww(h(0, 0x6a8c), sh.countdown);
    mm.ww(h(0, 0x6a8e), sh.lives);
    mm.ww(h(0, 0x6a90), sh.items);
    mm.wl(h(0, 0x6a92), sh.score);
}

fn status_update(cpu: &mut Cpu) {
    on_status(cpu, |s, src, c| {
        let m = &c.mem;
        s.update(
            m.rw(h(0, 0x6a80)),
            m.rw(h(0, 0x6a84)),
            m.rl(h(0, 0x2a66)),
            m.rw(h(0, 0x6a88)),
            src,
        )
    });
}

fn status_score(cpu: &mut Cpu) {
    on_status(cpu, |s, src, c| s.score(c.mem.rl(h(0, 0x2a66)), src));
}

fn status_score_digit(cpu: &mut Cpu) {
    on_status(cpu, |s, src, c| s.score_digit(c.d[0], c.d[1], src));
}

fn status_digit(cpu: &mut Cpu) {
    on_status(cpu, |s, src, c| {
        s.digit(c.d[0], c.a[1].wrapping_sub(h(4, 0)), src)
    });
}

fn status_countdown(cpu: &mut Cpu) {
    on_status(cpu, |s, src, c| s.countdown(c.mem.rw(h(0, 0x6a80)), src));
}

fn status_items(cpu: &mut Cpu) {
    on_status(cpu, |s, src, c| s.items(c.mem.rw(h(0, 0x6a88)), src));
}

fn status_lives(cpu: &mut Cpu) {
    on_status(cpu, |s, src, c| s.lives(c.mem.rw(h(0, 0x6a84)), src));
}

type Routine = (&'static str, u32, fn(&mut Cpu), RegsCompared);

/// The routines `re/translate.toml` gives T10: name, hunk 0 entry, adapter, registers compared.
fn translated() -> Vec<Routine> {
    let none = RegsCompared::NONE;
    vec![
        ("h0_3ab4", 0x3ab4, |_| playfield::blit_done(), none),
        ("char_sprite_position", 0x5462, sprite_position, none),
        ("h0_94ba", 0x94ba, fade_step, none),
        ("h0_94fe", 0x94fe, fade_all, none),
        ("h0_9550", 0x9550, fade_colour, RegsCompared::d(&[3])),
        ("h0_9754", 0x9754, scroll, none),
        ("char_sprite_copy", 0x9b06, sprite_copy, none),
    ]
}

/// The status bar's routines and the start-up's pointer routines, outside `re/translate.toml`.
fn status_routines() -> Vec<Routine> {
    let none = RegsCompared::NONE;
    let d12 = RegsCompared::d(&[1, 2]);
    vec![
        ("h0_6a96", 0x6a96, status_update, none),
        ("h0_6ae8", 0x6ae8, status_score, none),
        ("h0_6b70", 0x6b70, status_score_digit, d12),
        ("h0_6b78", 0x6b78, status_digit, d12),
        ("h0_6bc0", 0x6bc0, status_countdown, none),
        ("h0_6c20", 0x6c20, status_items, none),
        ("h0_6c80", 0x6c80, status_lives, none),
        (
            "h0_9a34",
            0x9a34,
            |c| put(c, fd_core::render::sprite_pointers(), 1),
            none,
        ),
        (
            "h0_988a",
            0x988a,
            |c| put(c, fd_core::render::status_pointers(), 1),
            none,
        ),
    ]
}

fn all() -> Vec<Routine> {
    let mut v = translated();
    v.extend(status_routines());
    v
}

fn rig() -> Rig {
    let r = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    BASE.get_or_init(|| r.memory_for(&generators::generator("h0_9754").unwrap()));
    r
}

#[test]
fn every_t10_routine_is_registered_and_checked_here() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../re/translate.toml");
    let text = std::fs::read_to_string(path).unwrap();
    let field = |block: &str, key: &str| {
        block.lines().find_map(|l| {
            let (k, v) = l.split_once('=')?;
            (k.trim() == key).then(|| v.trim().trim_matches('"').to_string())
        })
    };
    let mut t10: Vec<(String, u32)> = text
        .split("[[routine]]")
        .skip(1)
        .filter(|b| field(b, "port_task").as_deref() == Some("T10"))
        .map(|b| {
            let off = field(b, "offset").unwrap();
            (
                field(b, "name").unwrap(),
                u32::from_str_radix(off.trim_start_matches("0x"), 16).unwrap(),
            )
        })
        .collect();
    t10.sort();
    let mut here: Vec<(String, u32)> = translated()
        .iter()
        .map(|r| (r.0.to_string(), r.1))
        .collect();
    here.sort();
    assert_eq!(here, t10);
    for (name, off, _, _) in all() {
        assert!(
            PORTED
                .iter()
                .any(|p| p.name == name && p.hunk == 0 && p.offset == off),
            "{name} is not in PORTED"
        );
    }
}

#[test]
fn every_drawing_routine_matches_the_original() {
    let mut r = rig();
    let mut lines = Vec::new();
    for (name, off, port, regs) in all() {
        let g = generators::generator(name).unwrap();
        let rep: RigReport = judge::check_regs(&mut r, name, CASES, h(0, off), &g, regs, port)
            .unwrap_or_else(|f| panic!("{f}"));
        assert_eq!(
            (rep.passed, rep.hangs, rep.faults, rep.unmodelled_blits),
            (CASES, 0, 0, 0),
            "{name}"
        );
        lines.push(format!("{name}: {} paths", rep.paths));
    }
    println!("{}", lines.join("\n"));
}

/// Mutants, each a plausible wrong port: the rig must catch every one.
#[test]
fn wrong_ports_fail() {
    let mut r = rig();
    let mutants: [(&str, u32, fn(&mut Cpu), RegsCompared); 5] = [
        // The blink ignored: the sprites always copied.
        (
            "char_sprite_copy",
            0x9b06,
            |c| {
                let t = c.mem.rb(h(0, 0x9b04));
                c.mem.wb(h(0, 0x9b04), 0);
                sprite_copy(c);
                if t != 0 {
                    c.mem.wb(h(0, 0x9b04), t.wrapping_add(1));
                }
            },
            RegsCompared::NONE,
        ),
        // The scroll's word inside the line not inverted.
        (
            "h0_9754",
            0x9754,
            |c| {
                scroll(c);
                let x = c.mem.rw(h(0, 0x1e62));
                c.mem.ww(h(1, 0x1c2), (x & 0xf) * 0x11);
            },
            RegsCompared::NONE,
        ),
        // The fade's table read at the pass, not half of it.
        (
            "h0_94ba",
            0x94ba,
            |c| {
                let (mut f, to) = load_fade(c);
                if f.on != 0 {
                    f.all(&to, f.pass, &program());
                    f.pass = f.pass.wrapping_add(1);
                    if f.pass == 0x20 {
                        (f.pass, f.on) = (0, 0);
                    }
                } else {
                    f.pass = 0;
                }
                store_fade(c, &f);
            },
            RegsCompared::NONE,
        ),
        // Every number redrawn, changed or not.
        (
            "h0_6a96",
            0x6a96,
            |c| {
                on_status(c, |s, src, c| {
                    let m = &c.mem;
                    s.countdown(m.rw(h(0, 0x6a80)), src);
                    s.lives(m.rw(h(0, 0x6a84)), src);
                    s.score(m.rl(h(0, 0x2a66)), src);
                    s.items(m.rw(h(0, 0x6a88)), src);
                })
            },
            RegsCompared::NONE,
        ),
        // The lives drawn as their BCD digits, not as threes.
        (
            "h0_6c80",
            0x6c80,
            |c| {
                on_status(c, |s, src, c| {
                    let v = c.mem.rw(h(0, 0x6a84));
                    s.lives(v, src);
                    s.digit((v & 0xf) as u32, 0x8664, src);
                })
            },
            RegsCompared::NONE,
        ),
    ];
    for (name, off, port, regs) in mutants {
        let g = generators::generator(name).unwrap();
        let f = judge::check_regs(&mut r, name, 2_000, h(0, off), &g, regs, port).unwrap_err();
        assert!(f.failures > 0, "{name}: {f}");
    }
}

#[test]
fn the_status_generators_poke_only_what_their_routines_read() {
    for (name, ranges) in fd_conform::generators::status::READS {
        let g = generators::generator(name).unwrap();
        assert!(g.base.is_none(), "{name}");
        let mut rng = Gen::new(7);
        for _ in 0..500 {
            let s = (g.case)(&mut rng);
            for (a, bytes) in &s.pokes {
                for k in 0..bytes.len() as u32 {
                    assert!(
                        ranges.iter().any(|r| r.contains(&(a + k))),
                        "{name} pokes {:#x}",
                        a + k
                    );
                }
            }
        }
    }
}

#[test]
fn the_adapters_store_back_what_they_load() {
    // With every routine's effect removed, loading and storing changes nothing: the rig sees only
    // what the engine wrote.
    let r = rig();
    let base = r.memory_for(&generators::generator("h0_6a96").unwrap());
    let mut g = Gen::new(3);
    for name in ["h0_94fe", "h0_6a96"] {
        let generator = generators::generator(name).unwrap();
        for _ in 0..100 {
            let s = (generator.case)(&mut g);
            let before = judge::Mem::new(base.clone(), &s.pokes);
            let mut cpu = Cpu {
                d: s.d,
                a: s.a,
                mem: before.clone(),
            };
            if name == "h0_94fe" {
                let (f, _) = load_fade(&cpu);
                store_fade(&mut cpu, &f);
            } else {
                on_status(&mut cpu, |_, _, _| {});
            }
            assert_eq!(cpu.mem.changed_since(&before), vec![], "{name}");
        }
    }
}

// --- the display at the level sync point --------------------------------------------------------

fn game() -> Game {
    Game::new(game_data().unwrap_or_else(|e| panic!("{e}")), Start::Level1)
}

#[test]
fn the_palette_is_the_one_the_program_holds_and_the_original_fades_to() {
    let g = game();
    let am = reach_level_1(0).unwrap_or_else(|e| panic!("{e}"));
    let to = Live(&am).read(1, PALETTE, 128);
    let want: [u16; 32] = std::array::from_fn(|i| u16::from_be_bytes([to[4 * i], to[4 * i + 1]]));
    assert_eq!(g.data().display.palette(), want);
    // Not all one colour: the read found the palette, not padding.
    assert!(
        want.iter().filter(|&&c| c != want[0]).count() > 10,
        "{want:x?}"
    );
}

#[test]
fn the_display_equals_the_original_at_the_sync_point() {
    let am = reach_level_1(0).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(am.frame(), LEVEL_1_SYNC);
    let g = game();
    let mut t = game();
    t.transplant(&Live(&am));
    let (e, o) = (g.display(), t.display());
    assert_eq!(e.fade, o.fade);
    assert_eq!(e.sprite, o.sprite);
    assert_eq!(e.status.shown, o.status.shown);
    assert!(e.status.bytes == o.status.bytes, "the status bar's buffer");
    // The play screen is level_init's first view, T65's to port: the original's is drawn.
    assert!(o.screen.bytes.iter().any(|&b| b != 0));
}

#[test]
fn the_first_frame_is_drawn_with_the_status_bar() {
    let g = game();
    let p = g.picture();
    let (strt, stop) = WINDOW;
    assert_eq!((p.left, p.top), ((strt & 0xff) as i32, strt >> 8));
    assert_eq!((p.width, p.height), (320, 0x129 - 0x2f));
    // The bar's 56 rows from line 241 show its picture; the play area's (empty until T65) do not.
    let row = |y: usize| &p.pixels[y * 320..(y + 1) * 320];
    let bar = 241 - 0x2f;
    let drawn = (bar..bar + 56)
        .filter(|&y| row(y).iter().any(|&i| i != 0))
        .count();
    assert!(drawn > 40, "{drawn} of the bar's 56 rows drawn");
    let mut rgba = vec![0u8; 320 * p.height as usize * 4];
    g.picture_rgba(&mut rgba);
    assert!(rgba.chunks(4).all(|px| px[3] == 0xff));
    let _ = stop;
}

#[test]
fn after_a_transplant_the_picture_is_the_originals_still_and_scrolled() {
    // Worker check of the renderer (agreement over runs is T44's): given the original's display
    // memory, the engine draws the frame the oracle shows, the character's sprites and the status bar
    // included: every pixel when still. Play's first frames are not shown (the oracle's picture at the sync
    // point is its blank), so the checks start once the palette has faded in: still at frame 6340,
    // and at 6640 after running right, where the view has scrolled and bobs are cut over tiles
    // (level_files.rs's script and frames).
    let sc = amiga_conform::script::parse("6345: R\n6400: R F\n6410: R\n6600: -\n").unwrap();
    let mut am = reach_level_1(340).unwrap_or_else(|e| panic!("{e}"));
    for frame in [6340, 6640] {
        while am.frame() < frame {
            let at = am.frame();
            am.step(amiga_conform::script::input_at(&sc.lines, at))
                .unwrap_or_else(|e| panic!("{e}"));
        }
        let mut g = game();
        g.transplant(&Live(&am));
        let ours = amiga_chips::to_rgb12(&g.picture());
        let theirs = am.picture_in(WINDOW.0, WINDOW.1);
        assert_eq!(theirs.pixels.len(), ours.len());
        let differ = (0..ours.len())
            .filter(|&k| ours[k] != theirs.pixels[k])
            .count();
        // Moving, the original rewrites the sprites and cuts its creatures while the frame is drawn,
        // and a transplant holds memory as the frame ends: 1 211 pixels at 6640, all in rows 62 to
        // 128 around the character and a creature (measured 2026-11-06). Matching what the beam saw
        // is T44's; until then the scrolled frame is held at that bound.
        let bound = if frame == 6340 { 0 } else { 1_211 };
        assert!(differ <= bound, "frame {frame}: {differ} pixels differ");
        // Not a blank both sides agree on: the play area has its colours and the camera moved.
        let mut c = ours[..320 * 180].to_vec();
        c.sort_unstable();
        c.dedup();
        assert!(c.len() > 8, "frame {frame}: {} colours", c.len());
        if frame == 6640 {
            assert_ne!(g.state().camera.x, 0);
        }
    }
}
