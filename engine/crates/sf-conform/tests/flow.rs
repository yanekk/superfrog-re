//! The game's flow as the app uses it (T21, DESIGN §2.9): `Game::start_at` from a progress, and
//! `Game::progress`. Engine only; the flow's faithfulness to the original is `tests/chain.rs`'s
//! and the whole-game case's.

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
/// first byte), the menu's key-click effects' finished state, the keyboard's last byte, and the
/// RNG, which the title's blink draws from every idle frame.
const TITLE_TIME: [&str; 5] = ["opt_flash", "password_buf", "sfx_chan0", "kbd_toggle", "rng_state"];

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
        let mut differ: Vec<&str> = a
            .probe()
            .into_iter()
            .zip(b.probe())
            .filter(|((id, x), (_, y))| presentation(*id).is_none() && x != y)
            .map(|((id, _), _)| sf_core::LAYOUT.iter().find(|d| d.id == id).map_or("?", |d| d.name))
            .filter(|n| !TITLE_TIME.contains(n))
            .collect();
        differ.dedup();
        assert_eq!(differ, Vec::<&str>::new(), "W{world}");
    }
}

/// The disks a game asks for on the way into a world (T21): W4's files are on disk 2, W6's music
/// and loading picture too, its tiles and maps on disk 4. Each prompt is raised once, as the
/// original's `disk_request` asks; nothing answers it, and the game goes on at once (the app has
/// every disk), as fast as with no prompt (W1, whose files are all on disk 1, the measure).
#[test]
fn disk_prompts_are_raised_as_the_game_asks_and_need_no_answer() {
    use sf_core::DiskPrompt::Insert;
    use sf_disk::DiskSlot::{D2, D4};
    let prompts = |world: u8| {
        let mut g = Game::start_at_with(display(), &Progress { world, level: 1 }, Rng::default());
        let mut seen = Vec::new();
        while !matches!(g.scene(), Scene::Level | Scene::Bonus(_)) {
            if let Some(p) = g.step(FrameInput::NONE).disk_prompt {
                seen.push(p);
            }
        }
        seen
    };
    assert_eq!(prompts(1), []);
    assert_eq!(prompts(4), [Insert(D2)]);
    assert_eq!(prompts(6), [Insert(D2), Insert(D4)]);
}
