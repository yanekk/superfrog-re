//! real-widescreen T04 (DESIGN §2.5, D3): in widescreen the camera stops 65 pixels earlier at a
//! level's left and right ends, so the wide view shows no more of the map than the original's view
//! can; the frog's sprites are hidden and parked 65 further out; the bonus frog reaches 65 further
//! each side. In 4:3 every clamp is the original's.

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
    let (narrow, _) = drive_to_right_end(false);
    assert!(w(&narrow, FROG_SPRITE_RIGHT) < 0x150);
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
        for _ in 0..300 {
            g.step(joy(Joy::LEFT));
        }
        assert_eq!(screen_x(&g), -8 - extra as i16, "wide {wide}: left");
        for _ in 0..300 {
            g.step(joy(Joy::RIGHT));
        }
        assert_eq!(screen_x(&g), 0x12a + extra as i16, "wide {wide}: right");
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
