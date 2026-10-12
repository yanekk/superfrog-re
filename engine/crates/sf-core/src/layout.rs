//! The layout table: each original variable the engine models, with its address and width in the
//! original's memory (DESIGN §3.3). Transplant ([`State::transplant`]) and probe
//! ([`State::probe`]) are both driven from [`LAYOUT`], so a variable is either fully mapped or not
//! mapped at all; there is no third state in which a half-mapped variable could hide a divergence.
//!
//! Adding a variable: a [`VarId`], an entry here, a field in [`State`] and its arm in
//! `State::bytes`/`State::set_bytes` (the match is exhaustive, so the compiler asks for both).
//! The tests check that every id has exactly one entry and that entries do not overlap.
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
    /// A run of `n` bytes; probed as longs, then a word and a byte for a tail (see [`VarDef::pieces`]).
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

/// A modelled variable. Names are the ones in `re/symbols.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum VarId {
    /// `rng_state` (h0+$160d4): the 64-bit RNG state, `rng_next`'s low long then its high long.
    RngState,
    /// `camera_y` (h0+$16660): the camera's level y (docs/12).
    CameraY,
    /// `camera_x` (h0+$16664): the camera's level x.
    CameraX,
    /// `scroll_column` (h0+$172b0): the play-field column the vertical blank last scrolled to, as
    /// the address of its first word in the shown buffer (docs/12 "Scroll").
    ScrollColumn,
    /// `map_rows` (h0+$17334): one row above the in-RAM map.
    MapRows,
    /// `world` (h0+$40da): 1..6, 7 the bonus level.
    World,
    /// `level_map` (h1+$218e0): the map, 84 rows of 304 cells, `tile << 6 | marker`.
    LevelMap,
    /// `playfield_buffer_0` (h3+$10600) and `_1` (h3+$22f30): the two display buffers the play
    /// field is shown from, five planes of `$3b70` bytes each (docs/12 "The play field").
    PlayfieldBuffer0,
    PlayfieldBuffer1,
    /// `level_copper` (h2+$6bd0): the level's copper list (docs/12 "The copper list").
    LevelCopper,
    /// `panel_bitmap` (h3+$4b390): the status panel, five planes of 14 lines (docs/12 "Panel").
    PanelBitmap,
    // The front end (T20, docs/13): runs of the original's memory kept in its own form, read and
    // written at the original's addresses (`crate::front`).
    /// h0+$6..$26: `game_full_mode`, `loader_mode`, `start_world`, `start_level`, `attract_mode`,
    /// `final_level_flag` and the flags between them.
    FrontFlags,
    /// h0+$9d6..$9ec: `logic_running` .. the options (`opt_lives`, `opt_game_mode`).
    FrontGame,
    /// h0+$9f0..$a1a: `level_started` .. `ram_cache_base`, `demo_index`, `level_state`.
    FrontGame2,
    /// h0+$834: `score`.
    Score,
    /// h0+$2f90..$3028: `hiscore_table`: ten scores (a long and a long) and ten names.
    HiscoreTable,
    /// h0+$1368..$136c: the title's two eye-blink timers.
    TitleBlink,
    /// h0+$1686..$168c: the menu's flash counter and its colour pointer.
    TitleMenu,
    /// h0+$1958..$195c: `opt_cursor` and the hi-score-save option; h0+$1b16 the options' flash.
    OptCursor,
    OptFlash,
    /// h0+$22b8: `attract_abort`.
    AttractAbort,
    /// h0+$2424..$2434: the level-code entry's flash counter, its cursor and `password_buf`.
    PasswordBuf,
    /// h0+$3258: where the text writer (h0+$30fc) stopped.
    TextPtr,
    /// h0+$86e..$876: the game mode's tuning (`frog+$98`..`$9e`), set by the options screen.
    FrogMode,
    /// h0+$87c0: set to -1 as a game starts.
    GameStartFlag,
    /// h0+$11a24..$11a32: the frog's start values the title sets as a game starts.
    FrogStart,
    /// h0+$37e2..$3922: the palettes the title and its menu fade to (five of 32 colours).
    TitleTargets,
    /// h0+$3ba0..$3ca4: the title's two fade palettes and the VBL's every-other-frame toggle.
    TitlePalettes,
    /// h0+$3dc8: the title intro's blit source pointer.
    IntroBlit,
    /// h0+$b806..$b80e: the Accessing-disk overlay's palettes while it is up.
    OverlayUp,
    /// h0+$11b64..$11b6a: the mouse counters and the pointer's position.
    Mouse,
    /// h0+$123d4: `display_timer`.
    DisplayTimer,
    /// h0+$157d8..$15fa0: the four palette-fade records (`$1f2` bytes each).
    Fades,
    /// h0+$16076..$1607a: `kbd_toggle`, `key_down`, `key_up`.
    Keyboard,
    /// h0+$1b10c..$1b208: hardware-sprite records (pointer, title eyes, menu cursor).
    SpriteRecs,
    /// h2+$6fd0..$73fc: every copper list but the level's (title, menus, blank).
    FrontCopper,
    /// h2+$6110..$6270: the award pages' copper list.
    AwardCopper,
    /// h0+$32b4..$33ea: the hall of talent's ten lines of text, built from `hiscore_table`.
    HallText,
    /// h0+$161aa..$161be: the number-to-text routine's (h0+$160dc) flag and digits.
    NumberText,
    /// h3+$35860..$48190: the level's clean background buffer (docs/12 "Bobs"); outside a level
    /// the Accessing-disk overlay program is unpacked into it (at h3+$3a560) and run from there.
    CleanBuffer,
    /// The sound driver's variables (`crate::audio::vars`).
    AudioSfx,
    AudioMusicOn,
    AudioP41,
    AudioShadow,
    AudioStream,
    AudioSfxTables,
    AudioModuleAddr,
    AudioHeaderBackup,
}

impl VarId {
    /// Every id, in declaration order.
    pub const ALL: &[VarId] = &[
        VarId::RngState,
        VarId::CameraY,
        VarId::CameraX,
        VarId::ScrollColumn,
        VarId::MapRows,
        VarId::World,
        VarId::LevelMap,
        VarId::PlayfieldBuffer0,
        VarId::PlayfieldBuffer1,
        VarId::LevelCopper,
        VarId::PanelBitmap,
        VarId::FrontFlags,
        VarId::FrontGame,
        VarId::FrontGame2,
        VarId::Score,
        VarId::HiscoreTable,
        VarId::TitleBlink,
        VarId::TitleMenu,
        VarId::OptCursor,
        VarId::OptFlash,
        VarId::AttractAbort,
        VarId::PasswordBuf,
        VarId::TextPtr,
        VarId::FrogMode,
        VarId::GameStartFlag,
        VarId::FrogStart,
        VarId::TitleTargets,
        VarId::TitlePalettes,
        VarId::IntroBlit,
        VarId::OverlayUp,
        VarId::Mouse,
        VarId::DisplayTimer,
        VarId::Fades,
        VarId::Keyboard,
        VarId::SpriteRecs,
        VarId::FrontCopper,
        VarId::AwardCopper,
        VarId::HallText,
        VarId::NumberText,
        VarId::CleanBuffer,
        VarId::AudioSfx,
        VarId::AudioMusicOn,
        VarId::AudioP41,
        VarId::AudioShadow,
        VarId::AudioStream,
        VarId::AudioSfxTables,
        VarId::AudioModuleAddr,
        VarId::AudioHeaderBackup,
    ];

    /// Whether the variable is a run of memory kept as the original's bytes, in [`crate::State`]'s
    /// `ram`, rather than a typed field.
    pub fn is_raw_region(self) -> bool {
        self >= VarId::FrontFlags
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
    /// `Bytes(n)` one `L` per four bytes (then a `W` and a `B` for a tail), each with the entry's
    /// id and name and its own address, so a divergence names the exact long that differs.
    pub fn pieces(&self) -> impl Iterator<Item = VarDef> + '_ {
        let widths: Vec<Width> = match self.width {
            Width::Bytes(n) => {
                let mut v = vec![Width::L; (n / 4) as usize];
                if n % 4 >= 2 {
                    v.push(Width::W);
                }
                if n % 2 == 1 {
                    v.push(Width::B);
                }
                v
            }
            w => vec![w],
        };
        let mut at = self.addr;
        widths.into_iter().map(move |w| {
            let d = VarDef {
                addr: at,
                width: w,
                ..*self
            };
            at += w.len();
            d
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

/// Every modelled variable of the original (DESIGN §3.3). T12 models the RNG and T13 what the
/// level's picture is drawn from; each mechanic task adds its own.
pub static LAYOUT: &[VarDef] = &[
    VarDef {
        id: VarId::RngState,
        name: "rng_state",
        addr: h0(0x160d4),
        width: Width::Bytes(8),
    },
    VarDef {
        id: VarId::CameraY,
        name: "camera_y",
        addr: h0(0x16660),
        width: Width::L,
    },
    VarDef {
        id: VarId::CameraX,
        name: "camera_x",
        addr: h0(0x16664),
        width: Width::L,
    },
    VarDef {
        id: VarId::ScrollColumn,
        name: "scroll_column",
        addr: h0(0x172b0),
        width: Width::L,
    },
    VarDef {
        id: VarId::MapRows,
        name: "map_rows",
        addr: h0(0x17334),
        width: Width::L,
    },
    VarDef {
        id: VarId::World,
        name: "world",
        addr: h0(0x40da),
        width: Width::L,
    },
    VarDef {
        id: VarId::LevelMap,
        name: "level_map",
        addr: h1(0x218e0),
        width: Width::Bytes(MAP_ROWS * MAP_ROW_CELLS * 2),
    },
    VarDef {
        id: VarId::PlayfieldBuffer0,
        name: "playfield_buffer_0",
        addr: h3(0x10600),
        width: Width::Bytes(PLAYFIELD_BUFFER_BYTES),
    },
    VarDef {
        id: VarId::PlayfieldBuffer1,
        name: "playfield_buffer_1",
        addr: h3(0x22f30),
        width: Width::Bytes(PLAYFIELD_BUFFER_BYTES),
    },
    VarDef {
        id: VarId::LevelCopper,
        name: "level_copper",
        addr: h2(0x6bd0),
        width: Width::Bytes(0x400),
    },
    VarDef {
        id: VarId::PanelBitmap,
        name: "panel_bitmap",
        addr: h3(0x4b390),
        width: Width::Bytes(5 * 0x2a0),
    },
    VarDef {
        id: VarId::FrontFlags,
        name: "front_flags",
        addr: h0(0x6),
        width: Width::Bytes(0x20),
    },
    VarDef {
        id: VarId::FrontGame,
        name: "front_game",
        addr: h0(0x9d6),
        width: Width::Bytes(0x16),
    },
    VarDef {
        id: VarId::FrontGame2,
        name: "front_game2",
        addr: h0(0x9f0),
        width: Width::Bytes(0x2a),
    },
    VarDef {
        id: VarId::Score,
        name: "score",
        addr: h0(0x834),
        width: Width::L,
    },
    VarDef {
        id: VarId::HiscoreTable,
        name: "hiscore_table",
        addr: h0(0x2f90),
        width: Width::Bytes(0x98),
    },
    VarDef {
        id: VarId::TitleBlink,
        name: "title_blink",
        addr: h0(0x1368),
        width: Width::Bytes(4),
    },
    VarDef {
        id: VarId::TitleMenu,
        name: "title_menu",
        addr: h0(0x1686),
        width: Width::Bytes(6),
    },
    VarDef {
        id: VarId::OptCursor,
        name: "opt_cursor",
        addr: h0(0x1958),
        width: Width::Bytes(4),
    },
    VarDef {
        id: VarId::OptFlash,
        name: "opt_flash",
        addr: h0(0x1b16),
        width: Width::B,
    },
    VarDef {
        id: VarId::AttractAbort,
        name: "attract_abort",
        addr: h0(0x22b8),
        width: Width::W,
    },
    VarDef {
        id: VarId::PasswordBuf,
        name: "password_buf",
        addr: h0(0x2424),
        width: Width::Bytes(0x10),
    },
    VarDef {
        id: VarId::TextPtr,
        name: "text_ptr",
        addr: h0(0x3258),
        width: Width::L,
    },
    VarDef {
        id: VarId::FrogMode,
        name: "frog_mode",
        addr: h0(0x86e),
        width: Width::Bytes(8),
    },
    VarDef {
        id: VarId::GameStartFlag,
        name: "game_start_flag",
        addr: h0(0x87c0),
        width: Width::W,
    },
    VarDef {
        id: VarId::FrogStart,
        name: "frog_start",
        addr: h0(0x11a24),
        width: Width::Bytes(0xe),
    },
    VarDef {
        id: VarId::TitleTargets,
        name: "title_targets",
        addr: h0(0x37e2),
        width: Width::Bytes(0x140),
    },
    VarDef {
        id: VarId::TitlePalettes,
        name: "title_palettes",
        addr: h0(0x3ba0),
        width: Width::Bytes(0x102),
    },
    VarDef {
        id: VarId::IntroBlit,
        name: "intro_blit",
        addr: h0(0x3dc8),
        width: Width::L,
    },
    VarDef {
        id: VarId::OverlayUp,
        name: "overlay_up",
        addr: h0(0xb806),
        width: Width::Bytes(8),
    },
    VarDef {
        id: VarId::Mouse,
        name: "mouse",
        addr: h0(0x11b64),
        width: Width::Bytes(6),
    },
    VarDef {
        id: VarId::DisplayTimer,
        name: "display_timer",
        addr: h0(0x123d4),
        width: Width::W,
    },
    VarDef {
        id: VarId::Fades,
        name: "fades",
        addr: h0(0x157d8),
        width: Width::Bytes(4 * 0x1f2),
    },
    VarDef {
        id: VarId::Keyboard,
        name: "keyboard",
        addr: h0(0x16076),
        width: Width::Bytes(4),
    },
    VarDef {
        id: VarId::SpriteRecs,
        name: "sprite_recs",
        addr: h0(0x1b10c),
        width: Width::Bytes(0xfc),
    },
    VarDef {
        id: VarId::FrontCopper,
        name: "front_copper",
        addr: h2(0x6fd0),
        width: Width::Bytes(0x42c),
    },
    VarDef {
        id: VarId::AwardCopper,
        name: "award_copper",
        addr: h2(0x6110),
        width: Width::Bytes(0x160),
    },
    VarDef {
        id: VarId::HallText,
        name: "hall_text",
        addr: h0(0x32b4),
        width: Width::Bytes(0x136),
    },
    VarDef {
        id: VarId::NumberText,
        name: "number_text",
        addr: h0(0x161aa),
        width: Width::Bytes(0x14),
    },
    VarDef {
        id: VarId::CleanBuffer,
        name: "clean_buffer",
        addr: h3(0x35860),
        width: Width::Bytes(PLAYFIELD_BUFFER_BYTES),
    },
    VarDef {
        id: VarId::AudioSfx,
        name: "sfx_chan0",
        addr: h0(crate::audio::vars::SFX.0),
        width: Width::Bytes(crate::audio::vars::SFX.1 as u32),
    },
    VarDef {
        id: VarId::AudioMusicOn,
        name: "music_on",
        addr: h0(crate::audio::vars::MUSIC_ON.0),
        width: Width::Bytes(crate::audio::vars::MUSIC_ON.1 as u32),
    },
    VarDef {
        id: VarId::AudioP41,
        name: "p41_channels",
        addr: h0(crate::audio::vars::P41.0),
        width: Width::Bytes(crate::audio::vars::P41.1 as u32),
    },
    VarDef {
        id: VarId::AudioShadow,
        name: "music_fade",
        addr: h0(crate::audio::vars::SHADOW.0),
        width: Width::Bytes(crate::audio::vars::SHADOW.1 as u32),
    },
    VarDef {
        id: VarId::AudioStream,
        name: "stream",
        addr: h0(crate::audio::vars::STREAM.0),
        width: Width::Bytes(crate::audio::vars::STREAM.1 as u32),
    },
    VarDef {
        id: VarId::AudioSfxTables,
        name: "sfx_samples",
        addr: h0(crate::audio::vars::SFX_TABLES.0),
        width: Width::Bytes(crate::audio::vars::SFX_TABLES.1 as u32),
    },
    VarDef {
        id: VarId::AudioModuleAddr,
        name: "module_addr",
        addr: h0(crate::audio::vars::MODULE_ADDR.0),
        width: Width::Bytes(crate::audio::vars::MODULE_ADDR.1 as u32),
    },
    VarDef {
        id: VarId::AudioHeaderBackup,
        name: "module_header_backup",
        addr: h0(crate::audio::vars::HEADER_BACKUP.0),
        width: Width::Bytes(crate::audio::vars::HEADER_BACKUP.1 as u32),
    },
];

/// The entry for `id`.
pub fn def(id: VarId) -> &'static VarDef {
    LAYOUT
        .iter()
        .find(|d| d.id == id)
        .expect("every VarId has a LAYOUT entry (tested)")
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
    }

    #[test]
    fn entries_do_not_overlap_and_byte_runs_are_not_empty() {
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
                assert!(n > 0, "{}: Bytes({n})", d.name);
                let total: u32 = d.pieces().map(|p| p.width.len()).sum();
                assert_eq!(total, n, "{}: pieces cover the run", d.name);
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
