//! Every case in `engine/conform/cases/` that is green under the faithful checks (DESIGN §2.1)
//! and that no other test runs (T181): each run as `sf-conform run` runs it, checks 1–4 (a
//! `render` case is exempt from check 4). The cases with their own assertions live with their
//! subjects (`frog_cases.rs`, `enemies_*`, `front.rs`, ...); a case red today is not here, and its
//! FINDINGS row names its owner. The slow ones are ignored, named in `engine/conform/README.md`.

fn green(name: &str) {
    let case = sf_conform::Case::load(&sf_conform::cases_dir().join(format!("{name}.toml")))
        .unwrap_or_else(|e| panic!("{e}"));
    if let Err(d) = sf_conform::run(&case) {
        panic!("{name}: {d}");
    }
}

/// One test per case: `case!(fn_name, "case-name")`, `#[ignore = ...]` before it for a slow one.
macro_rules! case {
    ($(#[$m:meta])* $f:ident, $name:literal) => {
        $(#[$m])*
        #[test]
        fn $f() {
            green($name);
        }
    };
}

// The attract demo, the front end and game over.
case!(#[ignore = "slow: about 4 min"] attract_demo_3, "attract-demo-3");
case!(front_noskip, "front-noskip");
case!(front_options_all, "front-options-all");
case!(front_options, "front-options");
case!(front_skip_bounce, "front-skip-bounce");
case!(front_skip_left, "front-skip-left");
case!(front_skip_wait, "front-skip-wait");
case!(game_over_esc_w1_1, "game-over-esc-w1-1");
case!(game_over_fire_w1_1, "game-over-fire-w1-1");
case!(game_over_timeup_w1_1, "game-over-timeup-w1-1");
case!(story_boot, "story-boot");

// The frog alone in each world, objects frozen; W4-1, W4-2 and W6-1 explored (W1-4's completion is
// `frog_cases::complete_w1_4`).
case!(enemies_w4_1, "enemies-w4-1");
case!(enemies_w4_2, "enemies-w4-2");
case!(enemies_w6_1_explore, "enemies-w6-1-explore");
case!(frog_w2_1, "frog-w2-1");
case!(frog_w3_1, "frog-w3-1");
case!(frog_w4_1, "frog-w4-1");
case!(frog_w5_1, "frog-w5-1");
case!(frog_w6_1, "frog-w6-1");

// Levels completed, green since T212, each guarding where the original's frame ended inside a
// step: W2-3 and W6-1 the panel's steps (the clock digit after a blank in the score digit), W6-1
// `bob_draw`'s camera latches, W2-2 `obj_despawn_if_offscreen`'s points, W4-2 the drawer's steps
// by bob (a swing's links freed a pass after a blank's spawn). 10 s to a minute each.
case!(complete_w4_2, "complete-w4-2");
case!(enemies_w6_1_complete, "enemies-w6-1-complete");
case!(w2_2_complete, "w2-2-complete");
case!(w2_3_complete, "w2-3-complete");

// The slot machine's other prizes and the hi-lo game.
case!(slot_gamble, "slot-gamble");
case!(slot_hilo_1, "slot-hilo-1");
case!(slot_hilo_2, "slot-hilo-2");
case!(slot_hilo_3, "slot-hilo-3");
case!(slot_hilo_4, "slot-hilo-4");
case!(slot_hilo_5, "slot-hilo-5");
case!(slot_hilo_6, "slot-hilo-6");
case!(slot_hilo_7, "slot-hilo-7");
case!(slot_hilo_8, "slot-hilo-8");
case!(slot_hilo_wrong, "slot-hilo-wrong");
case!(slot_hold_20000, "slot-hold-20000");
case!(slot_hold_jackpot, "slot-hold-jackpot");

// The renderer on every level but W1-1 (`render_vs_oracle.rs` has W1-1), 400 frames each.
case!(render_final, "render-final");
case!(render_w1_2, "render-w1-2");
case!(render_w1_3, "render-w1-3");
case!(render_w1_4, "render-w1-4");
case!(render_w2_1, "render-w2-1");
case!(render_w2_2, "render-w2-2");
case!(render_w2_3, "render-w2-3");
case!(render_w2_4, "render-w2-4");
case!(render_w3_1, "render-w3-1");
case!(render_w3_2, "render-w3-2");
case!(render_w3_3, "render-w3-3");
case!(render_w3_4, "render-w3-4");
case!(render_w4_1, "render-w4-1");
case!(render_w4_2, "render-w4-2");
case!(render_w4_3, "render-w4-3");
case!(render_w4_4, "render-w4-4");
case!(render_w5_1, "render-w5-1");
case!(render_w5_2, "render-w5-2");
case!(render_w5_3, "render-w5-3");
case!(render_w5_4, "render-w5-4");
case!(render_w6_1, "render-w6-1");
case!(render_w6_2, "render-w6-2");
case!(render_w6_2_alt, "render-w6-2-alt");
case!(render_w6_3, "render-w6-3");
case!(render_w6_3_alt, "render-w6-3-alt");
case!(render_w6_4, "render-w6-4");
case!(render_w6_4_alt, "render-w6-4-alt");
case!(render_wb_1, "render-wb-1");

// Stretches of `game-whole` (T208): W6-4's start after W6-3, and the overlay before the final
// level's load. About 9 min each to the sync point the first time, then seconds from the cache.
// W2-2's start after W2-1 (T209), about 2 min the first time.
case!(#[ignore = "slow: about 2 min uncached"] whole_w2_2_start, "whole-w2-2-start");
case!(#[ignore = "slow: about 9 min uncached"] whole_w6_4_start, "whole-w6-4-start");
case!(#[ignore = "slow: about 9 min uncached"] whole_final_load, "whole-final-load");

// Planned routes that lag for long runs while the frog takes coins in the vertical blank (T29):
// `tile_anim_draw`'s queue slots freed only as the original's loop frees them.
case!(tile_queue_w2_4, "tile-queue-w2-4");
case!(tile_queue_w6_2, "tile-queue-w6-2");
