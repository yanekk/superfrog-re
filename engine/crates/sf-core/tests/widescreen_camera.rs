//! real-widescreen T04 (DESIGN §2.5, D3): in widescreen the camera stops 65 pixels earlier at a
//! level's left and right ends, so the wide view shows no more of the map than the original's view
//! can; the frog's sprites are hidden and parked 65 further out; the bonus frog reaches 65 further
//! each side. In 4:3 every clamp is the original's.

use sf_core::frog::anim::spr;
use sf_core::frog::{FROG, f};
use sf_core::input::{FrameInput, Joy};
use sf_core::level::camera::{CAMERA_X, FROG_SPRITE_LEFT, FROG_SPRITE_RIGHT, LEVEL_WIDTH};
use sf_core::level::screen::WIDE_EXTRA;
use sf_core::{Game, GameData, LevelId, Start, View};

const EXTRA: u16 = WIDE_EXTRA;
const WIDE_VIEW: View = View {
    extra_columns_each_side: WIDE_EXTRA,
};
/// The original's window, in lores pixels from `camera_x` (docs/12).
const WINDOW: u16 = 334;

fn data() -> &'static GameData {
    sf_conform::game_data()
}

fn level(id: &str, wide: bool) -> Game {
    let mut g = Game::new(data(), Start::Level(id.parse::<LevelId>().unwrap()));
    g.set_widescreen(wide);
    g
}

fn w(g: &Game, addr: u32) -> u16 {
    let b = g.peek(addr, 2);
    u16::from_be_bytes([b[0], b[1]])
}

fn set_w(g: &mut Game, addr: u32, v: u16) {
    g.poke(addr, &v.to_be_bytes());
}

/// Where sprite record `s`'s copper words put it, the 9-bit HSTART less $80: what the hardware
/// shows. The record's own x is rewritten by the camera after the copper words are, so only these
/// show whether `place_frog_sprites` hid it.
fn shown_x(g: &Game, s: u32) -> u16 {
    let b = g.peek(s + spr::COPPER, 4);
    let cop = u32::from_be_bytes([b[0], b[1], b[2], b[3]]);
    let hstart = (g.peek(cop + 0xb, 1)[0] as u16) << 1 | w(g, cop + 0xe) & 1;
    hstart.wrapping_sub(0x80) & 0x1ff
}

/// Sprite `s` is shown where the frog stands (the frog and camera at rest), not parked.
fn shown_where_it_stands(g: &Game, s: u32) -> bool {
    shown_x(g, s) == w(g, s) & 0x1ff
}

fn cx(g: &Game) -> u16 {
    w(g, CAMERA_X + 2)
}

fn joy(j: Joy) -> FrameInput {
    FrameInput::joy(j)
}

/// The leftmost picture column is the map's colour 0 from top to bottom over the play field's
/// lower half: what the renderer draws past the map's left edge.
fn left_column_blank(g: &Game) -> bool {
    let p = g.render(WIDE_VIEW);
    let (w, h) = (p.width as usize, p.height as usize);
    (h / 2..h).all(|y| p.pixels[y * w] == 0)
}

#[test]
fn at_w1_1s_start_the_wide_view_begins_at_level_x_16() {
    let (mut narrow, mut wide) = (level("W1-1", false), level("W1-1", true));
    // Entered narrow and switched before the first step: entered again, wide (Game::set_widescreen).
    assert_eq!(cx(&narrow), 0x10);
    assert_eq!(cx(&wide), 0x10 + EXTRA, "start_point's clamp");
    for _ in 0..3 {
        narrow.step(joy(Joy::NONE));
        wide.step(joy(Joy::NONE));
    }
    assert_eq!(cx(&narrow), 0x10);
    assert!(cx(&wide) >= 0x10 + EXTRA, "keep_inside's clamp: {}", cx(&wide));
    assert_eq!(cx(&wide) - EXTRA, 16, "the wide view's first column is level x 16");
    // The picture: a narrow camera under the wide view shows black past the map; the wide one
    // does not.
    assert!(left_column_blank(&narrow), "the check sees the map's edge");
    assert!(!left_column_blank(&wide), "no colour 0 from off the map");
}

/// The frog walks right (put near the end, then held right): the camera stops at
/// `level_width − $168 − extra`, the frog walks on to its own clamp, `level_width − $40`.
fn drive_to_right_end(wide: bool) -> (Game, u16) {
    let mut g = level("W1-1", wide);
    for _ in 0..3 {
        g.step(joy(Joy::NONE));
    }
    let width = w(&g, LEVEL_WIDTH);
    set_w(&mut g, FROG + f::X, width - 0x200);
    // The camera closes on the frog 8 pixels a frame.
    for _ in 0..700 {
        g.step(joy(Joy::RIGHT));
    }
    (g, width)
}

#[test]
fn at_a_levels_right_end_the_camera_stops_65_earlier_and_the_frog_walks_on() {
    let (wide, width) = drive_to_right_end(true);
    assert_eq!(cx(&wide), width - 0x168 - EXTRA);
    // The wide view's end, window pixel 334 + 65: the original's at its limit.
    assert_eq!(cx(&wide) + WINDOW + EXTRA, width - 26);
    let (narrow, n_width) = drive_to_right_end(false);
    assert_eq!(cx(&narrow), n_width - 0x168, "4:3's keep_inside unchanged");
    // The frog walks as far in both, against W1-1's last wall short of its own clamp
    // (`level_width − $40`), off-centre into the right strip in widescreen.
    let frog = w(&wide, FROG + f::X);
    assert_eq!(frog, w(&narrow, FROG + f::X));
    assert!(frog <= width - 0x40 && frog > width - 0x60, "frog at {frog}");
    assert!(frog - cx(&wide) > WINDOW, "past the original's right edge");
}

#[test]
fn in_widescreen_the_frogs_sprites_stay_shown_past_150() {
    let (wide, _) = drive_to_right_end(true);
    let (l, r) = (w(&wide, FROG_SPRITE_LEFT), w(&wide, FROG_SPRITE_RIGHT));
    // The frog stands 0x128 + 65 from the camera: both sprites past $150, neither parked.
    assert!(r >= 0x150 && r < 0x150 + EXTRA, "right sprite at {r:#x}");
    assert!(l >= 0x128 && l < 0x150 + EXTRA, "left sprite at {l:#x}");
    assert!(shown_where_it_stands(&wide, FROG_SPRITE_LEFT), "left sprite parked");
    assert!(shown_where_it_stands(&wide, FROG_SPRITE_RIGHT), "right sprite parked");
    let (narrow, _) = drive_to_right_end(false);
    assert!(w(&narrow, FROG_SPRITE_RIGHT) < 0x150);
    assert!(shown_where_it_stands(&narrow, FROG_SPRITE_RIGHT));
}

/// At W1-1's left end the frog walks into the left strip: its left sprite stands at screen x −50,
/// shown in widescreen (hidden only at x + $20 + 65 < 0), where the original's bound (x + $20 < 0)
/// would park it.
#[test]
fn in_widescreen_the_frogs_sprites_stay_shown_left_of_minus_20() {
    let mut g = level("W1-1", true);
    for _ in 0..200 {
        g.step(joy(Joy::LEFT));
    }
    let (l, r) = (w(&g, FROG_SPRITE_LEFT) as i16, w(&g, FROG_SPRITE_RIGHT) as i16);
    assert!(l < -0x20 && l >= -0x20 - EXTRA as i16, "left sprite at {l}");
    assert_eq!(r, l + 0x10);
    assert!(shown_where_it_stands(&g, FROG_SPRITE_LEFT), "left sprite parked");
    assert!(shown_where_it_stands(&g, FROG_SPRITE_RIGHT), "right sprite parked");
}

/// A blinking frog's sprites are parked past the right of the screen: at $150 in 4:3, at $150 + 65
/// in widescreen, where the wide picture does not reach.
#[test]
fn a_hidden_frog_is_parked_past_the_wide_edge() {
    for (wide, at) in [(false, 0x150), (true, 0x150 + EXTRA)] {
        let mut g = level("W1-1", wide);
        for _ in 0..3 {
            g.step(joy(Joy::NONE));
        }
        set_w(&mut g, FROG + f::BLINK, 0x41);
        let mut parked = 0;
        for _ in 0..8 {
            g.step(joy(Joy::NONE));
            if w(&g, FROG + f::BLINK) & 1 != 0 {
                assert_eq!(w(&g, FROG_SPRITE_LEFT), at, "wide {wide}");
                assert_eq!(w(&g, FROG_SPRITE_RIGHT), at, "wide {wide}");
                assert_eq!(shown_x(&g, FROG_SPRITE_LEFT), at & 0x1ff, "wide {wide}");
                parked += 1;
            }
        }
        assert!(parked > 0, "the frog blinked");
    }
}

/// The bonus frog, held left then right, stops at `cx − 8` and `cx + $12a`, 65 further each side
/// in widescreen: it flies into the strips.
#[test]
fn the_bonus_frog_reaches_into_the_strips() {
    for (wide, extra) in [(false, 0), (true, EXTRA)] {
        let mut g = level("WB-1", wide);
        let screen_x = |g: &Game| w(g, FROG + f::X).wrapping_sub(cx(g)) as i16;
        // The furthest it gets each way: a bonus shot may knock it back off the clamp later
        // (in widescreen they fly into the strips, T05).
        let (mut least, mut most) = (i16::MAX, i16::MIN);
        for _ in 0..300 {
            g.step(joy(Joy::LEFT));
            least = least.min(screen_x(&g));
        }
        assert_eq!(least, -8 - extra as i16, "wide {wide}: left");
        for _ in 0..300 {
            g.step(joy(Joy::RIGHT));
            most = most.max(screen_x(&g));
        }
        assert_eq!(most, 0x12a + extra as i16, "wide {wide}: right");
        assert!(g.scene().is_level());
    }
}

/// The final level's camera stays at its start point (only `keep_inside` runs), x 949, well inside
/// both modes' clamps: widescreen does not move it, so the witch's path (from the camera) is the
/// original's. Her fight still starts: an engine-only run of 1 000 steps reaches her update.
#[test]
fn the_final_levels_camera_is_not_moved_and_the_witch_fight_starts() {
    use sf_core::bosses::witch::WITCH_UPDATE;
    use sf_core::level::objects::OBJ_SLOTS;
    let narrow = level("WW-1", false);
    let mut g = level("WW-1", true);
    assert_eq!(cx(&g), cx(&narrow));
    assert_eq!(cx(&g), 949);
    let mut fighting = false;
    for _ in 0..1000 {
        g.step(joy(Joy::NONE));
        assert_eq!(cx(&g), 949);
        fighting |= (0..20u32).any(|k| {
            let b = g.peek(OBJ_SLOTS + k * 0x7c + 4, 4);
            u32::from_be_bytes([b[0], b[1], b[2], b[3]]) == WITCH_UPDATE
        });
    }
    assert!(fighting, "the witch's fight began");
    assert!(g.scene().is_level());
}

/// A restart's `start_point` (run by the front end's `level_init`, real-widescreen T01 review)
/// uses the wide clamp: W1-1 run into its first pit (`conform/scripts/frog-pit.txt`), the death,
/// the fade and the restart, whose loop runs again from frame 332.
#[test]
fn a_restart_in_widescreen_starts_at_the_wide_clamp() {
    for (wide, at) in [(false, 0x10), (true, 0x10 + EXTRA)] {
        let mut g = level("W1-1", wide);
        let mut left = false;
        for i in 0..400 {
            g.step(joy(if i < 290 { Joy::RIGHT } else { Joy::NONE }));
            left |= !g.scene().is_level();
        }
        assert!(left && g.scene().is_level(), "wide {wide}: died and restarted");
        assert_eq!(cx(&g), at, "wide {wide}");
    }
}

/// The scroller's first drawn column (h0+$1662c), which `start_point` sets from its clamped
/// camera before `keep_inside` first runs: only it shows `start_point`'s copy of the clamp.
fn first_drawn_column(g: &Game) -> u16 {
    w(g, sf_core::layout::h0(0x1662c))
}

/// `start_point`'s own clamp, both ends: W3-1 starts at x 69, inside the 4:3 clamp and left of
/// widescreen's; a W1-1 restart whose start (h0+$a1a) is put at x 4700 starts at the right clamp.
/// Each starts, and draws its first column, from the clamped camera.
#[test]
fn start_points_copy_of_the_clamp_moves_in_by_65() {
    for (wide, at) in [(false, 69), (true, 0x10 + EXTRA)] {
        let g = level("W3-1", wide);
        assert_eq!(cx(&g), at, "wide {wide}");
        assert_eq!(first_drawn_column(&g), at & 0xfff0, "wide {wide}: W3-1's first column");
    }
    for wide in [false, true] {
        let mut g = level("W1-1", wide);
        g.poke(sf_core::layout::h0(0xa1a), &4700u32.to_be_bytes());
        let right = w(&g, LEVEL_WIDTH) - 0x168 - if wide { EXTRA } else { 0 };
        // The camera as `start_point` leaves it, before the level's loop (and `keep_inside`) runs.
        let mut start = None;
        for i in 0..400 {
            g.step(joy(if i < 290 { Joy::RIGHT } else { Joy::NONE }));
            if start.is_none() && cx(&g) > 4000 {
                assert!(!g.scene().is_level(), "wide {wide}: the restart is still loading");
                start = Some((cx(&g), first_drawn_column(&g)));
            }
        }
        assert!(g.scene().is_level(), "wide {wide}: restarted");
        assert_eq!(start, Some((right, right & 0xfff0)), "wide {wide}: the restart's start");
        assert_eq!(cx(&g), right, "wide {wide}");
    }
}

/// real-widescreen T10: a level from [`Game::start_level`] (the app's Start Level and every bug
/// replay) switched to widescreen before its first step is started again in widescreen, so it
/// starts at the wide clamp and shows nothing past the map; switched back before a step it is the
/// 4:3 start again; switched after a step it plays on (DESIGN §2.7).
#[test]
fn a_dialog_started_level_switched_before_its_first_step_starts_at_the_wide_clamp() {
    use sf_core::recording::LevelSetup;
    let display = sf_conform::display_data();
    let seed = sf_core::Rng::seeded(3, 4);
    let narrow = Game::start_level(display.clone(), &LevelSetup::default(), seed);
    let mut g = Game::start_level(display.clone(), &LevelSetup::default(), seed);
    g.set_view(WIDE_VIEW);
    g.set_widescreen(true);
    assert_eq!(cx(&narrow), 0x10);
    assert_eq!(cx(&g), 0x10 + EXTRA, "start_point's wide clamp");
    assert!(g.widescreen() && g.view() == WIDE_VIEW, "the mode on and the view kept");
    assert_eq!(g.frame(), narrow.frame());
    for n in 0..40 {
        g.step(joy(Joy::NONE));
        assert!(!left_column_blank(&g), "frame {n}: colour 0 from off the map");
        assert_eq!(cx(&g), 0x10 + EXTRA, "frame {n}");
    }
    let mut back = Game::start_level(display.clone(), &LevelSetup::default(), seed);
    back.set_widescreen(true);
    back.set_widescreen(false);
    assert!(back.state() == narrow.state(), "switched back: the 4:3 start");
    let mut late = narrow.clone();
    late.step(joy(Joy::NONE));
    let frame = late.frame();
    late.set_widescreen(true);
    assert_eq!((late.frame(), cx(&late)), (frame, 0x10), "after a step: played on, not started again");
}
