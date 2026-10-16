//! The game's flow as the app uses it (T21, DESIGN §2.9): `Game::start_at` from a progress, and
//! `Game::progress`. Engine only but for the last test, which ties `start_at` to the original; the
//! flow's faithfulness to the original is `tests/chain.rs`'s and the whole-game case's.

use sf_core::input::FrameInput;
use sf_core::layout::presentation;
use sf_core::{Game, Progress, Rng, Scene, Start};

fn display() -> std::sync::Arc<sf_core::DisplayData> {
    sf_conform::display_data()
}

/// Steps `g` with nothing held until a level's (or the bonus level's) first frame plays.
fn to_level(g: &mut Game) -> u32 {
    let mut n = 0;
    while !matches!(g.scene(), Scene::Level | Scene::Bonus(_)) {
        g.step(FrameInput::NONE);
        n += 1;
        assert!(n < 8000, "no level within 8 000 frames");
    }
    n
}

/// Every world's start point, and the bonus level's, leads to its first level, with nothing
/// unported on the way, and `progress` says so.
#[test]
fn start_at_starts_each_world_at_its_first_level() {
    for world in 1..=7 {
        let at = Progress { world, level: 1 };
        let mut g = Game::start_at_with(display(), &at, Rng::default());
        assert!(g.frame() < 2000, "W{world}: START GAME not taken by frame {}", g.frame());
        to_level(&mut g);
        assert_eq!(g.progress(), at, "W{world}");
        assert_eq!(g.unported(), &[] as &[&str], "W{world}");
    }
}

/// Before a game, and in the attract demo, there is no progress.
#[test]
fn there_is_no_progress_before_a_game() {
    let mut g = Game::with_display(display(), Start::PowerOn { seed: Rng::default() });
    assert_eq!(g.progress(), Progress { world: 0, level: 0 });
    // The title idles into the attract demo (W1-1, played from a recording) after the award pages
    // and the hall of talent.
    for _ in 0..12000 {
        g.step(FrameInput::NONE);
        if g.scene() == Scene::Level {
            break;
        }
    }
    assert_eq!(g.scene(), Scene::Level, "the attract demo did not start");
    assert_eq!(g.progress(), Progress { world: 0, level: 0 });
}

/// The code typed on the options screen by a slower player than `start_at`'s: every press later.
fn typed(code: [u8; 6]) -> Game {
    let mut g = Game::with_display(display(), Start::PowerOn { seed: Rng::default() });
    // The title idles a while first.
    for _ in 0..700 {
        g.step(FrameInput::NONE);
    }
    let slow = sf_core::CodeTimes {
        options: 70,
        first_down: 130,
        down: 55,
        first_digit: 120,
        digit: 25,
        leave: 60,
    };
    g.type_code(Some(code), &slow);
    g
}

/// What the time a player spends on the title decides, which a code typed sooner or later leaves
/// different: the options screen's flashing colour and the code's (`opt_flash`, `password_buf`'s
/// first byte), the menu's key-click effects' finished state, the keyboard's last byte, the RNG,
/// which the title's blink draws from every idle frame, and the title's eye-blink timers and the
/// menu's flash counter where its fire is pressed (`title_blink`, `title_menu`: the same on two
/// engines whose fire waits for the menu, not on a script's fixed frames, T201).
const TITLE_TIME: [&str; 7] = [
    "opt_flash",
    "password_buf",
    "sfx_chan0",
    "kbd_toggle",
    "rng_state",
    "title_blink",
    "title_menu",
];

/// `start_at` is the code typed on the title (DESIGN §2.9): at each code's first level frame
/// every gameplay variable is the same but those the title's time decides ([`TITLE_TIME`]). (How
/// many frames that takes is not: `music_fade_wait` waits for the title's music to stop wherever
/// it had got to.)
#[test]
fn start_at_equals_the_code_typed_on_the_title() {
    let table = sf_core::front::password::table(&display().front);
    for world in [2u8, 5, 7] {
        let at = Progress { world, level: 1 };
        let code = sf_core::front::password::password_for(&table, &at).expect("a code").0;
        let mut a = Game::start_at_with(display(), &at, Rng::default());
        let mut b = typed(code);
        to_level(&mut a);
        to_level(&mut b);
        assert_eq!(a.progress(), at);
        assert_eq!(b.progress(), at, "W{world}: the typed code was not taken");
        assert_eq!(differ(&a, &b), Vec::<&str>::new(), "W{world}");
    }
}

/// The gameplay variables `g` holds that differ from `h`'s, but those the title's time decides.
fn differ(g: &Game, h: &Game) -> Vec<&'static str> {
    let mut differ: Vec<&str> = g
        .probe()
        .into_iter()
        .zip(h.probe())
        .filter(|((id, x), (_, y))| presentation(*id).is_none() && x != y)
        .map(|((id, _), _)| sf_core::LAYOUT.iter().find(|d| d.id == id).map_or("?", |d| d.name))
        .filter(|n| !TITLE_TIME.contains(n))
        .collect();
    differ.dedup();
    differ
}

/// `start_at` against the original (T201): `code-w6-1` types W6-1's code with `CODE_TIMES` on
/// both sides from power-on, and W6-1's files need disk 4 put in; the case is faithful and alone,
/// and at W6-1's first frame its engine holds what `start_at(W6-1)` holds at its own, but for
/// [`TITLE_TIME`].
#[test]
fn start_at_equals_the_code_typed_on_the_original() {
    let at = Progress { world: 6, level: 1 };
    let case = sf_conform::Case::load(&sf_conform::cases_dir().join("code-w6-1.toml"))
        .unwrap_or_else(|e| panic!("{e}"));
    let mut first: Option<Game> = None;
    let mut unported = Vec::new();
    if let Err(d) = sf_conform::run_with(&case, |_, g| {
        unported = g.unported().to_vec();
        if first.is_none() && g.scene() == Scene::Level {
            first = Some(g.clone());
        }
    }) {
        panic!("code-w6-1: {d}");
    }
    assert_eq!(unported, Vec::<&str>::new());
    let typed = first.expect("code-w6-1 did not reach a level");
    assert_eq!(typed.progress(), at, "code-w6-1 did not start W6-1");
    let mut a = Game::start_at_with(display(), &at, Rng::default());
    to_level(&mut a);
    assert_eq!(a.progress(), at);
    assert_eq!(differ(&a, &typed), Vec::<&str>::new());
}
