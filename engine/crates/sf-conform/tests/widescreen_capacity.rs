//! real-widescreen T03 (DESIGN §2.4, D4): widescreen's extra slots hold T00's heaviest scene
//! (`plans/real-widescreen/spike/README.md`), engine-only. The extra slots' own rules are
//! `sf-core/tests/widescreen_capacity.rs`.

use sf_core::Game;
use sf_core::level::objects::{SLOTS, WIDE_SLOTS, slot_at};

/// T00's heaviest scene (`plans/real-widescreen/spike/README.md`): W6-2's finishing route in
/// widescreen, the frog and hearts copied from the same route played in 4:3 after every frame
/// (the spike's ghost run), so the wide camera travels the whole route whatever the enemies do.
/// The live objects stay below the original's 20 plus [`WIDE_SLOTS`]: no spawn is refused or
/// evicts for want of a slot.
#[test]
fn t00s_heaviest_scene_fits() {
    use sf_core::frog::FROG;
    let route = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../plans/smart-planner/bench/routes/app/W6-2.txt"
    ))
    .expect("the bench's W6-2 route");
    let script = sf_conform::InputScript::parse(&route).expect("a route");
    let start = || {
        Game::start_level(
            sf_conform::display_data(),
            &sf_core::recording::LevelSetup { level: "W6-2".parse().unwrap(), ..Default::default() },
            sf_core::Rng::seeded(1, 1),
        )
    };
    let (mut lead, mut wide) = (start(), start());
    wide.set_widescreen(true);
    let hearts = sf_core::layout::h0(0x830);
    let mut peak = (0, 0);
    for f in 0..8491 {
        lead.step_unseen(script.input_at(f));
        wide.step_unseen(script.input_at(f));
        let frog: Vec<u8> = (0..0x100).map(|o| lead.state().b(FROG + o)).collect();
        wide.poke(FROG, &frog);
        wide.poke(hearts, &lead.state().w(hearts).to_be_bytes());
        let st = wide.state();
        let n = (0..SLOTS + WIDE_SLOTS).filter(|&k| st.l(slot_at(k) + 0x40) != 0).count();
        if n > peak.0 {
            peak = (n, f);
        }
        if !lead.scene().is_level() {
            break;
        }
    }
    eprintln!("W6-2 widescreen ghost: peak {} live at frame {}", peak.0, peak.1);
    assert!(peak.0 > 40, "the scene is the heavy one: {peak:?}");
    assert!(peak.0 < (SLOTS + WIDE_SLOTS) as usize, "below the slots there are: {peak:?}");
}
