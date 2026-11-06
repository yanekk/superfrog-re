//! The variables a conformance case compares, where each lives in the original's memory, and how an
//! engine is started from and read against them (plans/forest-dumb-level-1 DESIGN §2.1, T06).
//!
//! Every entry is a `kind = "var"` of `re/symbols.toml` with the same name, hunk, offset and size; a
//! test below holds the two together, so a case never compares the engine with some other byte.
//! `fd-conform` resolves a hunk to its base on the oracle; nothing here knows an address.

/// How many bytes a variable is, read big-endian as the 68000 stores it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Width {
    Byte,
    Word,
    Long,
}

impl Width {
    pub const fn bytes(self) -> usize {
        match self {
            Width::Byte => 1,
            Width::Word => 2,
            Width::Long => 4,
        }
    }
}

/// One compared variable: byte `offset` of the main program's hunk `hunk`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Var {
    pub name: &'static str,
    pub hunk: u8,
    pub offset: u32,
    pub width: Width,
}

const fn var(name: &'static str, hunk: u8, offset: u32, width: Width) -> Var {
    Var {
        name,
        hunk,
        offset,
        width,
    }
}

/// Every variable a case may name. A porting task appends what it models; the runner reads a
/// variable from the original and asks the engine for it through [`Transplant::probe`].
/// Measured or read in games/forest-dumb/docs/09-play-measured.md ("Variables"); the BCD ones are
/// compared as stored.
pub const LAYOUT: &[Var] = &[
    // The idle title's song tick (forest-dumb-start), the first case's one variable.
    var("mt_counter", 4, 0x195c9, Width::Byte),
    var("char_x", 0, 0x1e5a, Width::Long),
    var("char_y", 0, 0x1e5e, Width::Long),
    var("camera_x", 0, 0x1e62, Width::Word),
    var("camera_y", 0, 0x1e64, Width::Word),
    var("char_screen_x", 0, 0x4d78, Width::Word),
    // The camera's hand-off to `main_loop`'s edge redraws (T09, docs/09 "The camera"): a latch a
    // direction, and where the camera was when that edge was crossed.
    var("edge_at_right_x", 0, 0x5186, Width::Word),
    var("edge_at_right_y", 0, 0x5188, Width::Word),
    var("edge_at_left_x", 0, 0x518a, Width::Word),
    var("edge_at_left_y", 0, 0x518c, Width::Word),
    var("edge_at_up_x", 0, 0x518e, Width::Word),
    var("edge_at_up_y", 0, 0x5190, Width::Word),
    var("edge_at_down_x", 0, 0x5192, Width::Word),
    var("edge_at_down_y", 0, 0x5194, Width::Word),
    var("edge_latch_right", 0, 0x5196, Width::Word),
    var("edge_latch_left", 0, 0x5198, Width::Word),
    var("edge_latch_up", 0, 0x519a, Width::Word),
    var("edge_latch_down", 0, 0x519c, Width::Word),
    var("lives", 0, 0x6a84, Width::Word),
    var("hits_taken", 0, 0x3b26, Width::Word),
    var("countdown", 0, 0x6a80, Width::Long),
    var("items_left", 0, 0x6a88, Width::Word),
    var("score", 0, 0x2a66, Width::Long),
    var("world_index", 0, 0x6dfe, Width::Long),
    // The level-3 handler's count: one a pass in play, so it numbers the passes (fd-conform
    // matches the engine's pass n with the original's by it).
    var("irq_count", 0, 0x1e6a, Width::Word),
    var("paused", 0, 0x2100, Width::Word),
    var("level_exit_touched", 0, 0x2696, Width::Word),
    var("give_up", 0, 0x21b2, Width::Word),
    // The character's animation pickers and the joystick read (T51).
    var("char_walk_t", 0, 0x4be4, Width::Byte),
    var("char_facing", 0, 0x4ece, Width::Byte),
    var("char_probe_1", 0, 0x4ed1, Width::Byte),
    var("char_probe_2", 0, 0x4ed2, Width::Byte),
    // A pointer as the original stores it: equal only with tables loaded at the oracle's bases
    // (`character::anim`).
    var("char_anim_ptr", 0, 0x4ed8, Width::Long),
    var("joy_left", 0, 0x8df2, Width::Word),
    var("joy_right", 0, 0x8df4, Width::Word),
    var("joy_up", 0, 0x8df6, Width::Word),
    var("joy_down", 0, 0x8df8, Width::Word),
    var("input_log_index", 0, 0x8ec0, Width::Long),
    // The character's movement (T08). The probe's sixteen samples as four longs: LAYOUT has no
    // wider width.
    var("char_jump_3b28", 0, 0x3b28, Width::Word),
    var("char_jump_41b4", 0, 0x41b4, Width::Word),
    var("char_jump_41b6", 0, 0x41b6, Width::Word),
    var("char_dying_t", 0, 0x4b60, Width::Word),
    var("char_dying", 0, 0x4b62, Width::Word),
    var("char_rising", 0, 0x4ecf, Width::Byte),
    var("char_falling", 0, 0x4ed0, Width::Byte),
    var("char_probe_3", 0, 0x4ed3, Width::Byte),
    var("char_probe_4", 0, 0x4ed4, Width::Byte),
    var("char_probe_5", 0, 0x4ed5, Width::Byte),
    var("char_probe_6", 0, 0x4ed6, Width::Byte),
    var("char_slide_t", 0, 0x4108, Width::Byte),
    var("char_jumped", 0, 0x4109, Width::Byte),
    var("char_jump_t", 0, 0x410a, Width::Byte),
    var("char_run_left", 0, 0x4710, Width::Long),
    var("char_run_right", 0, 0x4714, Width::Long),
    var("char_samples_0", 0, 0x54f0, Width::Long),
    var("char_samples_4", 0, 0x54f4, Width::Long),
    var("char_samples_8", 0, 0x54f8, Width::Long),
    var("char_samples_c", 0, 0x54fc, Width::Long),
    var("char_update_t", 0, 0x3e2c, Width::Long),
    var("char_unhurt_t", 0, 0x9b04, Width::Byte),
    // What ends a life or the game (T23): the countdown's pass count and the keys play acts on.
    var("countdown_tick", 0, 0x20ac, Width::Long),
    var("last_key_byte", 0, 0x1f78, Width::Byte),
    var("pause_key_down", 0, 0x2102, Width::Word),
    var("esc_down", 0, 0x21b0, Width::Word),
    var("cheat_flag", 0, 0x780c, Width::Word),
];

/// The variable called `name`.
pub fn layout_var(name: &str) -> Option<&'static Var> {
    LAYOUT.iter().find(|v| v.name == name)
}

/// The original's memory as the main program's hunks see it. The runner implements it over the
/// oracle's RAM; a test implements it over a few bytes.
pub trait Mem {
    /// `len` bytes from byte `offset` of hunk `hunk`.
    fn read(&self, hunk: u8, offset: u32, len: usize) -> Vec<u8>;

    /// Variable `v`'s value, big-endian, zero-extended.
    fn value(&self, v: &Var) -> u32 {
        self.read(v.hunk, v.offset, v.width.bytes())
            .iter()
            .fold(0, |acc, &b| acc << 8 | b as u32)
    }
}

/// An engine that a case starts from the original's state and then reads variable by variable.
pub trait Transplant {
    /// Take the state the engine models from the original's memory at a pass's end.
    fn transplant(&mut self, mem: &dyn Mem);
    /// The engine's value of `v`, as the original stores it (zero-extended to 32 bits).
    fn probe(&self, v: &Var) -> u32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_variable_is_the_one_symbols_toml_records() {
        // The committed map is the one source of an address; a copy that drifted from it would make
        // a case compare the engine with some other byte.
        let map = include_str!("../../../../re/symbols.toml");
        for v in LAYOUT {
            let entry = map
                .split("[[sym]]")
                .find(|s| s.contains(&format!("name = \"{}\"\n", v.name)))
                .unwrap_or_else(|| panic!("{} is not in symbols.toml", v.name));
            for want in [
                format!("hunk = {}\n", v.hunk),
                format!("offset = {:#x}\n", v.offset),
                "kind = \"var\"\n".to_string(),
                format!("size = {}\n", v.width.bytes()),
            ] {
                assert!(entry.contains(&want), "{}: no `{want}` in {entry}", v.name);
            }
        }
    }

    #[test]
    fn names_are_unique() {
        for (i, v) in LAYOUT.iter().enumerate() {
            assert!(
                LAYOUT[i + 1..].iter().all(|w| w.name != v.name),
                "{} twice",
                v.name
            );
            assert_eq!(layout_var(v.name), Some(v));
        }
        assert_eq!(layout_var("no_such"), None);
    }

    struct Bytes(Vec<u8>);

    impl Mem for Bytes {
        fn read(&self, hunk: u8, offset: u32, len: usize) -> Vec<u8> {
            assert_eq!(hunk, 0);
            self.0[offset as usize..offset as usize + len].to_vec()
        }
    }

    #[test]
    fn a_value_is_read_big_endian_at_its_width() {
        let m = Bytes(vec![0x12, 0x34, 0x56, 0x78, 0x9a]);
        let at = |offset, width| m.value(&var("v", 0, offset, width));
        assert_eq!(at(0, Width::Byte), 0x12);
        assert_eq!(at(1, Width::Word), 0x3456);
        assert_eq!(at(1, Width::Long), 0x3456_789a);
    }
}
