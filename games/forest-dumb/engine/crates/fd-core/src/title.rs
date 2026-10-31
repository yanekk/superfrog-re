//! The placeholder scene for the idle title screen and its one modelled variable, `mt_counter`
//! (games/forest-dumb/docs/07-executable-map.md, "The first case's variable").

use amiga_input::FrameInput;

/// `mt_speed`, the ticks a row: `mt_init` sets it and this song never changes it inside the measured
/// window, which covers all its positions. A later speed command would need the song from the disk.
const MT_SPEED: u8 = 6;

/// The hunk [`LAYOUT`]'s addresses are relative to: the main program's hunk 4.
pub const LAYOUT_HUNK: u32 = 4;

/// Where the harness reads each modelled variable in the original's memory, relative to the start
/// of [`LAYOUT_HUNK`]. `fd-conform` resolves the hunk's base.
pub const LAYOUT: &[(&str, u32)] = &[("mt_counter", 0x195c9)];

/// The idle title screen's state, advanced once a frame.
///
/// [`TitleScene::new`] is the state at the case's sync point, oracle frame 4922, the first after
/// `mt_init`, and not the first title frame 4921: the counter is 0 at both, so starting a frame
/// early would run one ahead for ever.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TitleScene {
    mt_counter: u8,
}

impl TitleScene {
    pub fn new() -> Self {
        TitleScene { mt_counter: 0 }
    }

    /// Advance one frame, as `mt_music` does: add one, clear on reaching the speed.
    ///
    /// The input is ignored because the scene models only the idle title. The counter was measured
    /// the same with an earlier click, but fire at the title leaves for the game, which this
    /// placeholder does not model, so a case feeding it input after the sync point says nothing.
    pub fn frame(&mut self, _input: FrameInput) {
        self.mt_counter += 1;
        if self.mt_counter == MT_SPEED {
            self.mt_counter = 0;
        }
    }

    /// The modelled `mt_counter`, one byte in the original.
    pub fn variable(&self) -> u8 {
        self.mt_counter
    }
}

impl Default for TitleScene {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amiga_input::Joy;

    /// The map's measured window: frames 4922 to 20 000.
    const MEASURED_FRAMES: u32 = 20_000 - 4922;

    #[test]
    fn starts_at_zero_on_the_sync_frame() {
        assert_eq!(TitleScene::new().variable(), 0);
        assert_eq!(TitleScene::default(), TitleScene::new());
    }

    #[test]
    fn first_cycle_is_the_measured_values() {
        let mut scene = TitleScene::new();
        let mut seen = vec![scene.variable()];
        for _ in 0..12 {
            scene.frame(FrameInput::NONE);
            seen.push(scene.variable());
        }
        assert_eq!(seen, [0, 1, 2, 3, 4, 5, 0, 1, 2, 3, 4, 5, 0]);
    }

    #[test]
    fn follows_the_measured_formula_through_the_whole_window() {
        // The map's evidence: `(n - 4922) % 6` for every frame, with no exception.
        let mut scene = TitleScene::new();
        for k in 0..=MEASURED_FRAMES {
            assert_eq!(u32::from(scene.variable()), k % 6, "frame {}", 4922 + k);
            scene.frame(FrameInput::NONE);
        }
    }

    #[test]
    fn two_runs_on_the_same_input_agree_every_frame() {
        let inputs = [
            FrameInput::NONE,
            FrameInput::joy(Joy::UP),
            FrameInput::joy(Joy::FIRE),
        ];
        let (mut a, mut b) = (TitleScene::new(), TitleScene::new());
        for k in 0..1000 {
            let input = inputs[k % inputs.len()];
            a.frame(input);
            b.frame(input);
            assert_eq!(a.variable(), b.variable(), "frame {k}");
        }
    }

    #[test]
    fn layout_is_the_address_the_map_records() {
        // The committed map is the one source of the address; a copy that drifted from it would
        // make the case compare our counter with some other byte.
        let map = include_str!("../../../../re/symbols.toml");
        let (name, offset) = LAYOUT[0];
        let entry = map
            .split("[[sym]]")
            .find(|s| s.contains(&format!("name = \"{name}\"")))
            .expect("the variable is in symbols.toml");
        assert!(
            entry.contains(&format!("hunk = {LAYOUT_HUNK}\n")),
            "{entry}"
        );
        assert!(
            entry.contains(&format!("offset = {offset:#x}\n")),
            "{entry}"
        );
        assert!(entry.contains("size = 1\n"), "{entry}");
    }
}
