//! Shell: a guide script played at its own start, sampled for guide points (DESIGN §2.11). On the
//! original for `# timing: original` or no timing line; on the engine from the `start_level` state
//! for `app`, which T12 wires. T10 reuses it for the original's side of a parting.

use sf_conform::{InputScript, sync};
use sf_core::State;
use sf_core::input::FrameInput;
use sf_oracle::Amiga;

use crate::core::Facts;
use crate::core::repair::{Sampler, TracePoint};

/// The caller starts the oracle: `sync::reach(AtLevel)` for sets A and C, `sync::at_snapshot_file`
/// for a set-B leg (replaying `game-whole` to the leg would cost minutes). Shell, trace.rs.
///
/// Plays `frames` frames of `script` (its frame 0 the oracle's next) and samples the frog's cell,
/// the coins left and the switches pressed (`facts`, the level's, places them) before every 8th
/// frame, frame 0 first, until the level ends.
pub fn trace_original(
    amiga: &mut Amiga,
    facts: &Facts,
    script: &InputScript,
    frames: u32,
) -> Result<Vec<TracePoint>, String> {
    let mut s = Sampler::new(facts);
    for k in 0..=frames {
        // The original's memory is copied out only on a sampled frame: it is ~1 MB.
        if Sampler::due(k) && !s.sample(&State::transplant(&sync::core_memory(amiga.memory()))) {
            break;
        }
        if k == frames {
            break;
        }
        sync::step_inserting(amiga, script.input_at(k))?;
    }
    Ok(s.points)
}

/// A script's timing line (DESIGN §2.6): `original`, `app`, or none (`None`, read as original).
pub fn timing(text: &str) -> Option<&str> {
    text.lines()
        .next()?
        .trim()
        .strip_prefix("# timing:")?
        .split(|c: char| c.is_whitespace() || c == '.')
        .find(|w| !w.is_empty())
}

/// A script's inputs from its frame 0 to its last line's frame, for `follow_options`.
pub fn script_inputs(script: &InputScript) -> Vec<FrameInput> {
    let n = script.lines.last().map_or(0, |l| l.0);
    (0..n).map(|k| script.input_at(k)).collect()
}
