//! The route planner on W1-1 (DESIGN §2.11): the toolkit's whole-level explorer (`amiga-plan`) on
//! fd-core's engine from the level's start ([`Start::Level1`], the app's and the map page's start).
//!
//! A route is the engine's input a step at a time, written as an input script (`0: -`, `12: R U`),
//! so `fd-routeview --script` draws it and a case can play it on the original ([`confirm`]).

pub mod confirm;
pub mod facts;
pub mod game;

use std::sync::Arc;

use amiga_conform::script::InputScript;
use amiga_input::FrameInput;
use amiga_plan::run::{RunOptions, plan_engine};
use amiga_plan::{Options, PlanGame, Reason, Route};
use fd_core::recording::Location;
use fd_core::{Game, GameData, Start};

pub use facts::facts;
pub use game::PlanState;

/// The level's start as the planner plays it.
pub fn start(data: &Arc<GameData>) -> PlanState {
    PlanState(Game::new(data.clone(), Start::Level1))
}

/// A search's end: the route to the exit, or the furthest route and why it stopped there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Planned {
    pub route: Route,
    /// `None` when the route reaches the exit.
    pub stalled: Option<Reason>,
    /// Where the route ends on the engine.
    pub end: Location,
}

/// Plans W1-1 from its start with `opt` within `run`'s budget and rounds.
pub fn plan(data: &Arc<GameData>, opt: Options, run: &RunOptions) -> Planned {
    let s = start(data);
    let f = facts(&data.level);
    let (route, stalled) = match plan_engine(s, f, opt, run) {
        Ok(r) => (r, None),
        Err((r, why)) => (r, Some(why)),
    };
    let end = replay(data, &route.inputs).location();
    Planned {
        route,
        stalled,
        end,
    }
}

/// `inputs` played on the engine from the level's start.
pub fn replay(data: &Arc<GameData>, inputs: &[FrameInput]) -> PlanState {
    let mut g = start(data);
    for &i in inputs {
        g.step(i);
    }
    g
}

/// `inputs` as an input script, `notes` as its leading comment lines. A script holds an input from
/// its line's frame until the next line, so the route ends in a line at its length that lets go:
/// the map page reads the last line's frame as the route's end and plays only its idle tail past it.
pub fn script_text(notes: &[String], inputs: &[FrameInput]) -> String {
    let mut out: String = notes.iter().map(|n| format!("# {n}\n")).collect();
    out.push_str(&script(inputs).to_text());
    out
}

/// `inputs` as an input script that lets go at the route's end, so whatever plays past it idles.
pub fn script(inputs: &[FrameInput]) -> InputScript {
    let mut s = InputScript::from_steps(inputs);
    let end = inputs.len() as u32;
    if s.lines.last().is_none_or(|l| l.0 != end) {
        s.lines.push((end, FrameInput::NONE));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use amiga_input::Joy;

    #[test]
    fn a_route_as_a_script_reads_back_step_for_step_and_lets_go_at_its_end() {
        let r = FrameInput::joy(Joy::RIGHT);
        let ru = FrameInput::joy(Joy::RIGHT | Joy::UP);
        let inputs = [r, r, ru, ru, ru, FrameInput::NONE, r];
        let text = script_text(&["seed 1".into()], &inputs);
        assert_eq!(text, "# seed 1\n0: R\n2: U R\n5: -\n6: R\n7: -\n");
        let s = amiga_conform::script::parse(&text).unwrap();
        let back: Vec<_> = (0..inputs.len() as u32).map(|k| s.input_at(k)).collect();
        assert_eq!(back, inputs);
        // A route ending idle still ends in a line at its length; an empty one is one line.
        assert!(script_text(&[], &[r, FrameInput::NONE]).ends_with("1: -\n2: -\n"));
        assert_eq!(script_text(&[], &[]), "0: -\n");
    }

    #[test]
    fn bcd_words_read_as_their_decimal_value() {
        assert_eq!(game::bcd(0x0282), 282);
        assert_eq!(game::bcd(0x0032), 32);
        assert_eq!(game::bcd(0x9999), 9999);
    }
}
