//! The layout table: each original variable the engine models, with its address and width in the
//! original's memory (DESIGN §3.3). Transplant ([`State::transplant`]) and probe
//! ([`State::probe`]) are both driven from [`LAYOUT`], so a variable is either fully mapped or not
//! mapped at all; there is no third state in which a half-mapped variable could hide a divergence.
//!
//! Adding a variable is one line in the [`layout!`] list below: it makes the [`VarId`], the
//! [`LAYOUT`] entry and the bytes [`State`] keeps for it. The list is in address order, so a
//! variable is found by address with a binary search; the tests check the order, that entries do
//! not overlap and that byte runs are whole longs.
//!
//! The engine keeps every variable as the original's bytes at the original's address (as the
//! sound driver does, T11): the 68000 code overlays words and bytes and keeps pointers to its own
//! tables, and a byte-exact copy keeps every such quirk. A write to an address no entry covers is a
//! bug (a variable the port uses but the table does not map), so [`State::write`] panics on it.
//!
//! [`State`]: crate::State
//! [`State::transplant`]: crate::State::transplant
//! [`State::probe`]: crate::State::probe

/// Where the bootstrap places hunk 0 of the game executable `d1_c03` (docs/08, Memory map): the
/// original's `h0+$x` is at `H0 + x`.
pub const H0: u32 = 0xC0_0FA0;

/// `h0+$off` as an address.
pub const fn h0(off: u32) -> u32 {
    H0 + off
}

/// Hunk 1 (BSS, in slow RAM): the world's tiles and the level map (docs/08, Memory map).
pub const H1: u32 = 0xC4_92EC;
/// Hunk 2 (chip RAM): the copper lists and the panel and HUD graphics.
pub const H2: u32 = 0x400;
/// Hunk 3 (chip RAM): bob sheets, the play-field buffers, the panel, sprite frames.
pub const H3: u32 = 0x77FC;

pub const fn h1(off: u32) -> u32 {
    H1 + off
}

pub const fn h2(off: u32) -> u32 {
    H2 + off
}

pub const fn h3(off: u32) -> u32 {
    H3 + off
}

/// How many bytes a variable spans. Values are big-endian, as the 68000 stores them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Width {
    B,
    W,
    L,
    /// A run of `n` bytes, `n` a multiple of 4; probed as `n / 4` longs (see [`VarDef::pieces`]).
    Bytes(u32),
}

impl Width {
    pub const fn len(self) -> u32 {
        match self {
            Width::B => 1,
            Width::W => 2,
            Width::L => 4,
            Width::Bytes(n) => n,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VarDef {
    pub id: VarId,
    pub name: &'static str,
    pub addr: u32,
    pub width: Width,
}

impl VarDef {
    /// The units probe and comparison work in: the entry itself for `B`, `W` and `L`; for
    /// `Bytes(n)` one `L` per four bytes, each with the entry's id and name and its own address,
    /// so a divergence names the exact long that differs.
    pub fn pieces(&self) -> impl Iterator<Item = VarDef> + '_ {
        let (n, w) = match self.width {
            Width::Bytes(n) => (n / 4, Width::L),
            w => (1, w),
        };
        (0..n).map(move |k| VarDef {
            addr: self.addr + 4 * k,
            width: w,
            ..*self
        })
    }

    /// This piece's value in `ram` (a `B`, `W` or `L`; for `Bytes` use [`VarDef::pieces`]).
    pub fn read(&self, ram: &dyn Ram) -> u32 {
        (0..self.width.len().min(4)).fold(0, |v, k| v << 8 | ram.byte(self.addr + k) as u32)
    }
}

/// Map rows the original keeps, and cells per row (300 used, 4 more of padding).
pub const MAP_ROWS: u32 = 84;
pub const MAP_ROW_CELLS: u32 = 304;

/// Bytes in one display buffer: five planes of `$3b70`.
pub const PLAYFIELD_BUFFER_BYTES: u32 = 0x12930;


/// Declares the variables: `Id = "symbol name", address, width;`, in address order.
macro_rules! layout {
    ($( $(#[$doc:meta])* $id:ident = $name:literal, $addr:expr, $width:expr; )*) => {
        /// A modelled variable. Names are the ones in `re/symbols.toml`.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum VarId { $( $(#[$doc])* $id, )* }

        impl VarId {
            /// Every id, in [`LAYOUT`] order.
            pub const ALL: &[VarId] = &[ $( VarId::$id, )* ];
        }

        /// Every modelled variable of the original (DESIGN §3.3), in address order. T12 models
        /// the RNG, T13 what the level's picture is drawn from, T14 the level and the frog; each
        /// mechanic task adds its own.
        pub static LAYOUT: &[VarDef] = &[
            $( VarDef { id: VarId::$id, name: $name, addr: $addr, width: $width }, )*
        ];
    };
}

layout! {
    /// `level_copper` (h2+$6bd0): the level's copper list (docs/12 "The copper list").
    LevelCopper = "level_copper", h2(0x6bd0), Width::Bytes(0x400);
    /// `playfield_buffer_0` (h3+$10600) and `_1` (h3+$22f30): the two display buffers the play
    /// field is shown from, five planes of `$3b70` bytes each (docs/12 "The play field").
    PlayfieldBuffer0 = "playfield_buffer_0", h3(0x10600), Width::Bytes(PLAYFIELD_BUFFER_BYTES);
    PlayfieldBuffer1 = "playfield_buffer_1", h3(0x22f30), Width::Bytes(PLAYFIELD_BUFFER_BYTES);
    /// `panel_bitmap` (h3+$4b390): the status panel, five planes of 14 lines (docs/12 "Panel").
    PanelBitmap = "panel_bitmap", h3(0x4b390), Width::Bytes(5 * 0x2a0);
    /// `attract_mode` (h0+$18): -1 while the attract demo runs.
    AttractMode = "attract_mode", h0(0x18), Width::L;
    /// `final_level_flag` (h0+$1e): set for the final level.
    FinalLevelFlag = "final_level_flag", h0(0x1e), Width::W;
    /// `frog` (h0+$7d6): the frog's record (docs/09 "The frog"), one entry per field; the
    /// `frog_NN` names are fields not yet understood, by offset.
    FrogX = "frog_x", h0(0x7d6), Width::W;
    FrogY = "frog_y", h0(0x7d8), Width::W;
    FrogVx = "frog_vx", h0(0x7da), Width::W;
    FrogVy = "frog_vy", h0(0x7dc), Width::W;
    Frog08 = "frog_08", h0(0x7de), Width::W;
    Frog0A = "frog_0a", h0(0x7e0), Width::W;
    Frog0C = "frog_0c", h0(0x7e2), Width::W;
    Frog0E = "frog_0e", h0(0x7e4), Width::W;
    Frog10 = "frog_10", h0(0x7e6), Width::W;
    FrogFacing = "frog_facing", h0(0x7e8), Width::W;
    FrogClimb = "frog_climb", h0(0x7ea), Width::W;
    Frog16 = "frog_16", h0(0x7ec), Width::W;
    Frog18 = "frog_18", h0(0x7ee), Width::W;
    Frog1A = "frog_1a", h0(0x7f0), Width::W;
    Frog1C = "frog_1c", h0(0x7f2), Width::W;
    Frog1E = "frog_1e", h0(0x7f4), Width::W;
    Frog20 = "frog_20", h0(0x7f6), Width::W;
    Frog22 = "frog_22", h0(0x7f8), Width::W;
    Frog24 = "frog_24", h0(0x7fa), Width::W;
    Frog26 = "frog_26", h0(0x7fc), Width::W;
    Frog28 = "frog_28", h0(0x7fe), Width::W;
    Frog2A = "frog_2a", h0(0x800), Width::W;
    FrogAnim = "frog_anim", h0(0x802), Width::L;
    FrogAnimStep = "frog_anim_step", h0(0x806), Width::L;
    Frog34 = "frog_34", h0(0x80a), Width::W;
    Frog36 = "frog_36", h0(0x80c), Width::W;
    Frog38 = "frog_38", h0(0x80e), Width::W;
    Frog3A = "frog_3a", h0(0x810), Width::W;
    Frog3C = "frog_3c", h0(0x812), Width::W;
    Frog3E = "frog_3e", h0(0x814), Width::W;
    Frog40 = "frog_40", h0(0x816), Width::W;
    Frog42 = "frog_42", h0(0x818), Width::W;
    Frog44 = "frog_44", h0(0x81a), Width::W;
    Frog46 = "frog_46", h0(0x81c), Width::W;
    FrogCameraHeld = "frog_camera_held", h0(0x81e), Width::W;
    Frog4A = "frog_4a", h0(0x820), Width::W;
    Frog4C = "frog_4c", h0(0x822), Width::W;
    Frog4E = "frog_4e", h0(0x824), Width::W;
    Frog50 = "frog_50", h0(0x826), Width::W;
    Frog52 = "frog_52", h0(0x828), Width::W;
    Frog54 = "frog_54", h0(0x82a), Width::W;
    PlayerDied = "player_died", h0(0x82c), Width::W;
    Frog58 = "frog_58", h0(0x82e), Width::W;
    Frog5A = "frog_5a", h0(0x830), Width::W;
    CoinsLeftForExit = "coins_left_for_exit", h0(0x832), Width::W;
    Score = "score", h0(0x834), Width::L;
    Frog62 = "frog_62", h0(0x838), Width::W;
    Frog64 = "frog_64", h0(0x83a), Width::W;
    Frog66 = "frog_66", h0(0x83c), Width::W;
    Frog68 = "frog_68", h0(0x83e), Width::W;
    Frog6A = "frog_6a", h0(0x840), Width::W;
    Frog6C = "frog_6c", h0(0x842), Width::W;
    Frog6E = "frog_6e", h0(0x844), Width::W;
    Frog70 = "frog_70", h0(0x846), Width::W;
    Frog72 = "frog_72", h0(0x848), Width::W;
    Frog74 = "frog_74", h0(0x84a), Width::W;
    Frog76 = "frog_76", h0(0x84c), Width::W;
    Frog78 = "frog_78", h0(0x84e), Width::W;
    Frog7A = "frog_7a", h0(0x850), Width::W;
    Frog7C = "frog_7c", h0(0x852), Width::W;
    Frog7E = "frog_7e", h0(0x854), Width::W;
    Frog80 = "frog_80", h0(0x856), Width::W;
    Frog82 = "frog_82", h0(0x858), Width::W;
    Frog84 = "frog_84", h0(0x85a), Width::W;
    Frog86 = "frog_86", h0(0x85c), Width::W;
    FrogFrozen = "frog_frozen", h0(0x85e), Width::W;
    Frog8A = "frog_8a", h0(0x860), Width::W;
    Frog8C = "frog_8c", h0(0x862), Width::W;
    Frog8E = "frog_8e", h0(0x864), Width::W;
    Frog90 = "frog_90", h0(0x866), Width::W;
    Frog92 = "frog_92", h0(0x868), Width::W;
    Frog94 = "frog_94", h0(0x86a), Width::W;
    Frog96 = "frog_96", h0(0x86c), Width::W;
    Frog98 = "frog_98", h0(0x86e), Width::W;
    Frog9A = "frog_9a", h0(0x870), Width::W;
    Frog9C = "frog_9c", h0(0x872), Width::W;
    Frog9E = "frog_9e", h0(0x874), Width::W;
    LevelClock = "level_clock", h0(0x876), Width::L;
    FrogA4 = "frog_a4", h0(0x87a), Width::W;
    FrogA6 = "frog_a6", h0(0x87c), Width::W;
    FrogA8 = "frog_a8", h0(0x87e), Width::W;
    FrogAA = "frog_aa", h0(0x880), Width::W;
    FrogAC = "frog_ac", h0(0x882), Width::W;
    FrogAE = "frog_ae", h0(0x884), Width::W;
    FrogB0 = "frog_b0", h0(0x886), Width::W;
    FrogB2 = "frog_b2", h0(0x888), Width::W;
    /// `shooter_mode` (h0+$9dc): the bonus level's auto-scrolling mode.
    ShooterMode = "shooter_mode", h0(0x9dc), Width::W;
    /// `sprites_hidden` (h0+$9e2): the frog's sprites stay where they are.
    SpritesHidden = "sprites_hidden", h0(0x9e2), Width::W;
    /// `level_width`, `level_height` (h0+$a22, h0+$a24): the level's size in pixels.
    LevelWidth = "level_width", h0(0xa22), Width::W;
    LevelHeight = "level_height", h0(0xa24), Width::W;
    /// `world` (h0+$40da): 1..6, 7 the bonus level.
    World = "world", h0(0x40da), Width::L;
    /// `shooter_half_step` (h0+$de28): the bonus level's camera creeps on every second frame.
    ShooterHalfStep = "shooter_half_step", h0(0xde28), Width::W;
    /// `rng_state` (h0+$160d4): the 64-bit RNG state, `rng_next`'s low long then its high long.
    RngState = "rng_state", h0(0x160d4), Width::Bytes(8);
    /// `camera_y` (h0+$16660): the camera's level y (docs/12).
    CameraY = "camera_y", h0(0x16660), Width::L;
    /// `camera_x` (h0+$16664): the camera's level x.
    CameraX = "camera_x", h0(0x16664), Width::L;
    /// `scroll_dirs` (h0+$16682): which way the camera last moved (bits 1 right, 2 left, 3
    /// down, 4 up).
    ScrollDirs = "scroll_dirs", h0(0x16682), Width::B;
    /// `scroll_column` (h0+$172b0): the play-field column the vertical blank last scrolled to, as
    /// the address of its first word in the shown buffer (docs/12 "Scroll").
    ScrollColumn = "scroll_column", h0(0x172b0), Width::L;
    /// `map_rows` (h0+$17334): one row above the in-RAM map.
    MapRows = "map_rows", h0(0x17334), Width::L;
    /// `frog_sprite_left`, `_right` (h0+$1aff4, h0+$1b010): the frog's two hardware-sprite
    /// records (`$1c` bytes: screen x, y, height, attached, copper words, data, animation).
    FrogSpriteLeft = "frog_sprite_left", h0(0x1aff4), Width::Bytes(0x1c);
    FrogSpriteRight = "frog_sprite_right", h0(0x1b010), Width::Bytes(0x1c);
    /// `level_map` (h1+$218e0): the map, 84 rows of 304 cells, `tile << 6 | marker`.
    LevelMap = "level_map", h1(0x218e0), Width::Bytes(MAP_ROWS * MAP_ROW_CELLS * 2);
}

/// The entry for `id`.
pub fn def(id: VarId) -> &'static VarDef {
    &LAYOUT[id as usize]
}

/// The index in [`LAYOUT`] of the entry covering `addr`, if any.
pub fn index_of(addr: u32) -> Option<usize> {
    let i = LAYOUT.partition_point(|d| d.addr <= addr).checked_sub(1)?;
    (addr < LAYOUT[i].addr + LAYOUT[i].width.len()).then_some(i)
}

/// The original's memory as transplant reads it: one byte per CPU address.
pub trait Ram {
    fn byte(&self, addr: u32) -> u8;
}

/// Chip RAM at 0 and slow RAM at `$C00000`, as the A500 the original runs on has them. An address
/// outside both reads 0.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct AmigaMemory {
    pub chip: Vec<u8>,
    pub slow: Vec<u8>,
}

/// Where slow ("ranger") RAM sits on the A500.
pub const SLOW_BASE: u32 = 0xC0_0000;

impl Ram for AmigaMemory {
    fn byte(&self, addr: u32) -> u8 {
        if (addr as usize) < self.chip.len() {
            self.chip[addr as usize]
        } else {
            addr.checked_sub(SLOW_BASE)
                .and_then(|a| self.slow.get(a as usize).copied())
                .unwrap_or(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_id_has_exactly_one_entry_and_every_entry_a_known_id() {
        for id in VarId::ALL {
            assert_eq!(LAYOUT.iter().filter(|d| d.id == *id).count(), 1, "{id:?}");
        }
        assert_eq!(LAYOUT.len(), VarId::ALL.len());
        for (i, d) in LAYOUT.iter().enumerate() {
            assert_eq!(d.id as usize, i, "{}", d.name);
            assert_eq!(def(d.id).name, d.name);
        }
    }

    #[test]
    fn the_table_is_in_address_order_and_found_by_address() {
        for w in LAYOUT.windows(2) {
            assert!(w[0].addr < w[1].addr, "{} before {}", w[0].name, w[1].name);
        }
        for (i, d) in LAYOUT.iter().enumerate() {
            assert_eq!(index_of(d.addr), Some(i), "{}", d.name);
            assert_eq!(index_of(d.addr + d.width.len() - 1), Some(i), "{}", d.name);
        }
        assert_eq!(index_of(0), None);
        assert_eq!(index_of(0xF0_0000), None);
    }

    #[test]
    fn entries_do_not_overlap_and_byte_runs_are_whole_longs() {
        let mut spans: Vec<(u32, u32, &str)> = LAYOUT
            .iter()
            .map(|d| (d.addr, d.addr + d.width.len(), d.name))
            .collect();
        spans.sort();
        for w in spans.windows(2) {
            assert!(w[0].1 <= w[1].0, "{} overlaps {}", w[0].2, w[1].2);
        }
        for d in LAYOUT {
            if let Width::Bytes(n) = d.width {
                assert!(n > 0 && n.is_multiple_of(4), "{}: Bytes({n})", d.name);
            }
        }
    }

    #[test]
    fn pieces_split_byte_runs_into_longs_and_read_big_endian() {
        let d = VarDef {
            id: VarId::RngState,
            name: "x",
            addr: 0x10,
            width: Width::Bytes(8),
        };
        let p: Vec<VarDef> = d.pieces().collect();
        assert_eq!(p.len(), 2);
        assert_eq!((p[1].addr, p[1].width, p[1].name), (0x14, Width::L, "x"));
        let mut mem = AmigaMemory {
            chip: vec![0; 0x20],
            slow: vec![0; 4],
        };
        mem.chip[0x14..0x18].copy_from_slice(&[1, 2, 3, 4]);
        mem.slow[1] = 0xAB;
        assert_eq!(p[1].read(&mem), 0x0102_0304);
        let w = VarDef {
            addr: 0x15,
            width: Width::W,
            ..d
        };
        assert_eq!(w.read(&mem), 0x0203);
        assert_eq!(mem.byte(SLOW_BASE + 1), 0xAB);
        assert_eq!(mem.byte(0x100), 0);
        assert_eq!(mem.byte(SLOW_BASE + 4), 0);
    }
}
