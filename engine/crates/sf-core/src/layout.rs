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

/// Bytes of `level_map`: from h1+$218e0 to the boot's tile pointers at h1+$30fe0.
pub const LEVEL_MAP_BYTES: u32 = 0x30fe0 - 0x218e0;

/// Bytes in one display buffer: five planes of `$3b70`.
pub const PLAYFIELD_BUFFER_BYTES: u32 = 0x12930;

/// Declares the variables: `Id = "symbol name", address, width;`, in address order.
macro_rules! layout {
    ($( $(#[$doc:meta])* $id:ident = $name:literal, $addr:expr, $width:expr; )*
     @outside { $( $(#[$xdoc:meta])* $xid:ident, )* }) => {
        /// A modelled variable. Names are the ones in `re/symbols.toml`.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum VarId { $( $(#[$doc])* $id, )* $( $(#[$xdoc])* $xid, )* }

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
    /// Chip RAM's first two longs: 0 and `ExecBase` (the reset vectors as Kickstart leaves them:
    /// `$00C00276` under Kickstart 1.3 with slow RAM, measured). The game reads them through a null
    /// pointer: a bob never drawn has no animation step yet, so `obj_despawn_if_offscreen` takes
    /// the long at 0 for its frame record and its width and height from address 4.
    LowVectors = "low_vectors", 0, Width::Bytes(8);
    /// h2+$5efc..$6058: the end sequence's copper list (five planes, its palette at h2+$5fa4).
    EndCopper = "end_copper", h2(0x5efc), Width::Bytes(0x15c);
    /// h2+$6110..$6270: the award pages' copper list.
    AwardCopper = "award_copper", h2(0x6110), Width::Bytes(0x160);
    /// h2+$6270..$63cc: the rest of the tally's copper list (h2+$6268, T38; its first long is the
    /// award list's last): its window, its palette at h2+$630c, its plane pointers at h2+$638c and
    /// the two choices' colours.
    TallyCopper = "tally_copper", h2(0x6270), Width::Bytes(0x15c);
    /// `slot_copper` (h2+$65e4..$6750): the slot machine's copper list (T71); its fade list at
    /// h2+$6688, its plane pointers at h2+$6708.
    SlotCopper = "slot_copper", h2(0x65e4), Width::Bytes(0x16c);
    /// `exit_sprite_copper` (h2+$68a8): the sprite moves the second pair of frog sprite records
    /// writes (`exit_sprite_left`, `_right`), two attached pairs of `$10` bytes each.
    ExitSpriteCopper = "exit_sprite_copper", h2(0x68a8), Width::Bytes(0x40);
    /// `level_copper` (h2+$6bd0): the level's copper list (docs/12 "The copper list").
    LevelCopper = "level_copper", h2(0x6bd0), Width::Bytes(0x400);
    /// h2+$6fd0..$73fc: every copper list but the level's (title, menus, blank).
    FrontCopper = "front_copper", h2(0x6fd0), Width::Bytes(0x42c);
    /// h3+$0..$10600: in a level a world's bobs (`LnBO`); the end sequence unpacks `END1` and
    /// `END2` from here on, over the display buffers (T35).
    BobSheet = "bob_sheet", h3(0), Width::Bytes(0x10600);
    /// `playfield_buffer_0` (h3+$10600) and `_1` (h3+$22f30): the two display buffers the play
    /// field is shown from, five planes of `$3b70` bytes each (docs/12 "The play field").
    PlayfieldBuffer0 = "playfield_buffer_0", h3(0x10600), Width::Bytes(PLAYFIELD_BUFFER_BYTES);
    PlayfieldBuffer1 = "playfield_buffer_1", h3(0x22f30), Width::Bytes(PLAYFIELD_BUFFER_BYTES);
    /// `playfield_clean` (h3+$35860): the clean background, tiles only; bobs are erased from it.
    PlayfieldClean = "playfield_clean", h3(0x35860), Width::Bytes(PLAYFIELD_BUFFER_BYTES);
    /// h3+$48190..$4b390: between the clean buffer and the panel; the end sequence's second
    /// ANIM buffer starts in it (h3+$49890).
    Hunk3Gap = "hunk3_gap", h3(0x48190), Width::Bytes(0x3200);
    /// `panel_bitmap` (h3+$4b390): the status panel, five planes of 14 lines (docs/12 "Panel").
    PanelBitmap = "panel_bitmap", h3(0x4b390), Width::Bytes(5 * 0x2a0);
    /// h3+$4c0b0..$64b70: the frog's attached-sprite frames (`hero_sprite_frames` h0+$19ce0).
    HeroFrames = "hero_frames", h3(0x4c0b0), Width::Bytes(0x18ac0);
    /// The front end's flags (T20, docs/13), h0+$6..$26: `game_full_mode` (h0+$6), `loader_mode`,
    /// `start_world`, `start_level` (from a level code), the words between them, `attract_mode`,
    /// `final_level_flag`.
    GameFullMode = "game_full_mode", h0(0x6), Width::W;
    LoaderMode = "loader_mode", h0(0x8), Width::W;
    StartWorld = "start_world", h0(0xa), Width::L;
    StartLevel = "start_level", h0(0xe), Width::L;
    Front12 = "front_12", h0(0x12), Width::W;
    /// `no_death` (h0+$14): non-zero, hazards do nothing (inferred: a cheat).
    NoDeath = "no_death", h0(0x14), Width::W;
    Front16 = "front_16", h0(0x16), Width::W;
    /// `attract_mode` (h0+$18): -1 while the attract demo runs.
    AttractMode = "attract_mode", h0(0x18), Width::L;
    /// h0+$1c: `demo_record` (h0+$11a32) records the joystick while it is set (a developer's switch).
    DemoRecordOn = "demo_record_on", h0(0x1c), Width::W;
    /// `final_level_flag` (h0+$1e): set for the final level.
    FinalLevelFlag = "final_level_flag", h0(0x1e), Width::W;
    Front20 = "front_20", h0(0x20), Width::Bytes(6);
    /// `frog` (h0+$7d6): the frog's record (docs/09 "The frog"), one entry per field; the
    /// `frog_NN` names are fields not yet understood, by offset.
    FrogX = "frog_x", h0(0x7d6), Width::W;
    FrogY = "frog_y", h0(0x7d8), Width::W;
    FrogVx = "frog_vx", h0(0x7da), Width::W;
    FrogVy = "frog_vy", h0(0x7dc), Width::W;
    FrogSpeed = "frog_speed", h0(0x7de), Width::W;
    FrogAir = "frog_air", h0(0x7e0), Width::W;
    FrogGravityTick = "frog_gravity_tick", h0(0x7e2), Width::W;
    FrogWallRight = "frog_wall_right", h0(0x7e4), Width::W;
    FrogWallLeft = "frog_wall_left", h0(0x7e6), Width::W;
    FrogFacing = "frog_facing", h0(0x7e8), Width::W;
    FrogClimb = "frog_climb", h0(0x7ea), Width::W;
    Frog16 = "frog_16", h0(0x7ec), Width::W;
    FrogPrevX = "frog_prev_x", h0(0x7ee), Width::W;
    FrogPrevY = "frog_prev_y", h0(0x7f0), Width::W;
    FrogAirStep = "frog_air_step", h0(0x7f2), Width::W;
    FrogHeadBump = "frog_head_bump", h0(0x7f4), Width::W;
    Frog20 = "frog_20", h0(0x7f6), Width::W;
    FrogSpeedTick = "frog_speed_tick", h0(0x7f8), Width::W;
    FrogOnGround = "frog_on_ground", h0(0x7fa), Width::W;
    Frog26 = "frog_26", h0(0x7fc), Width::W;
    FrogSlopeSide = "frog_slope_side", h0(0x7fe), Width::W;
    FrogLaunched = "frog_launched", h0(0x800), Width::W;
    FrogAnim = "frog_anim", h0(0x802), Width::L;
    FrogAnimStep = "frog_anim_step", h0(0x806), Width::L;
    FrogAnimTime = "frog_anim_time", h0(0x80a), Width::W;
    FrogSpriteDx = "frog_sprite_dx", h0(0x80c), Width::W;
    FrogSpriteDy = "frog_sprite_dy", h0(0x80e), Width::W;
    FrogHurtLock = "frog_hurt_lock", h0(0x810), Width::W;
    FrogLanded = "frog_landed", h0(0x812), Width::W;
    FrogDying = "frog_dying", h0(0x814), Width::W;
    FrogFrictionToggle = "frog_friction_toggle", h0(0x816), Width::W;
    FrogRightToggle = "frog_right_toggle", h0(0x818), Width::W;
    FrogLeftToggle = "frog_left_toggle", h0(0x81a), Width::W;
    FrogHurtFriction = "frog_hurt_friction", h0(0x81c), Width::W;
    FrogCameraHeld = "frog_camera_held", h0(0x81e), Width::W;
    FrogSlope = "frog_slope", h0(0x820), Width::W;
    FrogSlopeWalkAnim = "frog_slope_walk_anim", h0(0x822), Width::L;
    FrogGroundCell = "frog_ground_cell", h0(0x826), Width::L;
    FrogOnPlatform = "frog_on_platform", h0(0x82a), Width::W;
    PlayerDied = "player_died", h0(0x82c), Width::W;
    FrogLives = "frog_lives", h0(0x82e), Width::W;
    FrogHearts = "frog_hearts", h0(0x830), Width::W;
    CoinsLeftForExit = "coins_left_for_exit", h0(0x832), Width::W;
    Score = "score", h0(0x834), Width::L;
    Frog62 = "frog_62", h0(0x838), Width::W;
    FrogFireHeld = "frog_fire_held", h0(0x83a), Width::W;
    FrogCrouch = "frog_crouch", h0(0x83c), Width::W;
    Frog68 = "frog_68", h0(0x83e), Width::W;
    FrogSliding = "frog_sliding", h0(0x840), Width::W;
    FrogSlideTick = "frog_slide_tick", h0(0x842), Width::W;
    FrogPushed = "frog_pushed", h0(0x844), Width::W;
    FrogCrouchTime = "frog_crouch_time", h0(0x846), Width::W;
    FrogSpudFlying = "frog_spud_flying", h0(0x848), Width::W;
    FrogSpudX = "frog_spud_x", h0(0x84a), Width::W;
    FrogSpudY = "frog_spud_y", h0(0x84c), Width::W;
    FrogSpudVx = "frog_spud_vx", h0(0x84e), Width::W;
    FrogSpudVy = "frog_spud_vy", h0(0x850), Width::W;
    FrogSpudOn = "frog_spud_on", h0(0x852), Width::W;
    FrogTileCoins = "frog_tile_coins", h0(0x854), Width::W;
    FrogSpecialAnim = "frog_special_anim", h0(0x856), Width::W;
    FrogMaxSpeed = "frog_max_speed", h0(0x858), Width::W;
    FrogSpring = "frog_spring", h0(0x85a), Width::W;
    FrogSitTime = "frog_sit_time", h0(0x85c), Width::W;
    FrogFrozen = "frog_frozen", h0(0x85e), Width::W;
    FrogInPassage = "frog_in_passage", h0(0x860), Width::W;
    FrogBlink = "frog_blink", h0(0x862), Width::W;
    FrogLaunchHold = "frog_launch_hold", h0(0x864), Width::W;
    FrogFlight = "frog_flight", h0(0x866), Width::W;
    FrogShield = "frog_shield", h0(0x868), Width::W;
    FrogAnimLock = "frog_anim_lock", h0(0x86a), Width::W;
    FrogIdleSound = "frog_idle_sound", h0(0x86c), Width::W;
    FrogBounceVy = "frog_bounce_vy", h0(0x86e), Width::W;
    FrogGravityDelay = "frog_gravity_delay", h0(0x870), Width::W;
    FrogAirStepDelay = "frog_air_step_delay", h0(0x872), Width::W;
    FrogAirSpeed = "frog_air_speed", h0(0x874), Width::W;
    LevelClock = "level_clock", h0(0x876), Width::L;
    FrogFloatToggle = "frog_float_toggle", h0(0x87a), Width::W;
    FrogJumpPresses = "frog_jump_presses", h0(0x87c), Width::W;
    FrogSpeedUp = "frog_speed_up", h0(0x87e), Width::W;
    /// The score at which the next extra life comes (every 200 000, h0+$d9e).
    FrogNextLife = "frog_next_life", h0(0x880), Width::L;
    FrogSpudWait = "frog_spud_wait", h0(0x884), Width::W;
    FrogBaseSpeed = "frog_base_speed", h0(0x886), Width::W;
    FrogB2 = "frog_b2", h0(0x888), Width::W;
    /// The rest of the frog's `$200`-byte record, to h0+$9d6, which `frog_new_game` clears.
    FrogRest = "frog_rest", h0(0x88a), Width::Bytes(0x14c);
    /// `logic_running`, `vbl_count` (h0+$9d6, h0+$9d8): the level's main loop is running, and the
    /// vertical blanks since its last pass.
    LogicRunning = "logic_running", h0(0x9d6), Width::W;
    VblCount = "vbl_count", h0(0x9d8), Width::W;
    /// `vbl_pass_flag` (h0+$9da): set by the vertical blank, cleared by the buffer flip.
    VblPassFlag = "vbl_pass_flag", h0(0x9da), Width::W;
    /// `shooter_mode` (h0+$9dc): the bonus level's auto-scrolling mode.
    ShooterMode = "shooter_mode", h0(0x9dc), Width::W;
    AbortFlag = "abort_flag", h0(0x9de), Width::W;
    /// `screen_fades_off` (h0+$9e0): outside a level, fades step every second frame.
    ScreenFadesOff = "screen_fades_off", h0(0x9e0), Width::W;
    /// `sprites_hidden` (h0+$9e2): the frog's sprites stay where they are.
    SpritesHidden = "sprites_hidden", h0(0x9e2), Width::W;
    /// `opt_lives`, `opt_game_mode` (h0+$9e4, h0+$9e6): the options screen's lives (0..2 for
    /// 3/5/7) and game mode (0 easy, 1 normal).
    OptLives = "opt_lives", h0(0x9e4), Width::W;
    OptGameMode = "opt_game_mode", h0(0x9e6), Width::W;
    Front9e8 = "front_9e8", h0(0x9e8), Width::L;
    /// The sound driver's variables (`crate::audio::vars`), kept by the driver in [`crate::State`]'s
    /// `audio`: `module_addr` (h0+$9ec) here, the rest further on.
    AudioModuleAddr = "module_addr", h0(crate::audio::vars::MODULE_ADDR.0), Width::Bytes(crate::audio::vars::MODULE_ADDR.1 as u32);
    LevelStarted = "level_started", h0(0x9f0), Width::W;
    Front9f2 = "front_9f2", h0(0x9f2), Width::L;
    DemoIndex = "demo_index", h0(0x9f6), Width::W;
    /// `frog_exit_sprites` (h0+$9f8): the frog shows in the second pair of sprite records.
    FrogExitSprites = "frog_exit_sprites", h0(0x9f8), Width::W;
    /// `obj_world_offset` (h0+$9fa): the world's row of the object tables, `$100` a world.
    ObjWorldOffset = "obj_world_offset", h0(0x9fa), Width::L;
    Front9fe = "front_9fe", h0(0x9fe), Width::Bytes(8);
    /// `level_state` (h0+$a06): negative while playing; 1 exit, 2 bonus end, $64 boss, $190 death.
    LevelState = "level_state", h0(0xa06), Width::L;
    FrontA0a = "front_a0a", h0(0xa0a), Width::Bytes(8);
    RamCacheBase = "ram_cache_base", h0(0xa12), Width::L;
    FrontA16 = "front_a16", h0(0xa16), Width::L;
    /// `level_start_x`, `level_start_y` (h0+$a1a, $a1e): the camera's start, from the map's start
    /// marker (`$35`, h0+$162f4); the frog starts `$90`, `$70` from it (h0+$164fa).
    LevelStartX = "level_start_x", h0(0xa1a), Width::L;
    LevelStartY = "level_start_y", h0(0xa1e), Width::L;
    /// `level_width`, `level_height` (h0+$a22, h0+$a24): the level's size in pixels.
    LevelWidth = "level_width", h0(0xa22), Width::W;
    LevelHeight = "level_height", h0(0xa24), Width::W;
    ExtraLifeShown = "extra_life_shown", h0(0xdd4), Width::W;
    /// h0+$1368..$136c: the title's two eye-blink timers.
    TitleBlink = "title_blink", h0(0x1368), Width::Bytes(4);
    /// h0+$1686..$168c: the menu's flash counter and its colour pointer.
    TitleMenu = "title_menu", h0(0x1686), Width::Bytes(6);
    /// h0+$1958..$195c: `opt_cursor` and the hi-score-save option.
    OptCursor = "opt_cursor", h0(0x1958), Width::Bytes(4);
    /// h0+$1b16: the options' flash.
    OptFlash = "opt_flash", h0(0x1b16), Width::B;
    AttractAbort = "attract_abort", h0(0x22b8), Width::W;
    /// h0+$2424..$2434: the level-code entry's flash counter, its cursor and `password_buf`.
    PasswordBuf = "password_buf", h0(0x2424), Width::Bytes(0x10);
    /// `password_shown` (h0+$24cc): the password the slot machine's prize shows
    /// (`password_for_level`).
    PasswordShown = "password_shown", h0(0x24cc), Width::Bytes(0xc);
    /// h0+$2f90..$3028: `hiscore_table`: ten scores (a long and a long) and ten names.
    HiscoreTable = "hiscore_table", h0(0x2f90), Width::Bytes(0x98);
    /// h0+$3258: where the text writer (h0+$30fc) stopped.
    TextPtr = "text_ptr", h0(0x3258), Width::L;
    /// h0+$32b4..$33ea: the hall of talent's ten lines of text, built from `hiscore_table`.
    HallText = "hall_text", h0(0x32b4), Width::Bytes(0x136);
    /// h0+$37e2..$3922: the palettes the title and its menu fade to (five of 32 colours).
    TitleTargets = "title_targets", h0(0x37e2), Width::Bytes(0x140);
    /// h0+$3ba0..$3ca0: the title's two fade palettes (the VBL's toggle after them is
    /// `screen_fade_toggle`).
    TitlePalettes = "title_palettes", h0(0x3ba0), Width::Bytes(0x100);
    /// `screen_fade_toggle` (h0+$3ca0).
    ScreenFadeToggle = "screen_fade_toggle", h0(0x3ca0), Width::W;
    /// h0+$3dc8: the title intro's blit source pointer.
    IntroBlit = "intro_blit", h0(0x3dc8), Width::L;
    /// `world` (h0+$40da): 1..6, 7 the bonus level.
    World = "world", h0(0x40da), Width::L;
    /// `level` (h0+$40de): 1..4, the level of the world.
    Level = "level", h0(0x40de), Width::L;
    /// `coins_collected` (h0+$445e): coins taken since the last tally (`touch_coin` counts them).
    CoinsCollected = "coins_collected", h0(0x445e), Width::L;
    /// `level_tally`'s pages (h0+$4796: the credits earned, 5 000 points each) and its choice
    /// (h0+$4798: 1 the slot machine), and the text it prints the level by (h0+$49be, `0-0`
    /// with the world's and level's digits; T38).
    TallyPages = "tally_pages", h0(0x4796), Width::W;
    TallyChoice = "tally_choice", h0(0x4798), Width::W;
    TallyLevelText = "tally_level_text", h0(0x49be), Width::Bytes(3);
    /// The slot machine's variables (T71; `slot_machine` h0+$49dc, docs/13): `slot_state`
    /// (h0+$49c6..$49dc: the coins put in, then the three reels' positions and more), `slot_count`
    /// (h0+$515a), `slot_prize` (h0+$5526: the ladder's rung), `slot_win` (h0+$57b2), `slot_hilo`
    /// (h0+$5988), `slot_hilo_start` (h0+$5a04), `slot_spin` (h0+$5dc6), `slot_flash` (h0+$603e),
    /// `slot_cursor` (h0+$6064: the pointer's stop), `slot_menu` (h0+$6152), `slot_attract`
    /// (h0+$62ec: the attract pattern's row, its frame count and its pass count), `slot_lamps`
    /// (h0+$6842: 26 lamps of 8 bytes) and `slot_palette` (h0+$6912: the fade's target, 32
    /// colours).
    SlotState = "slot_state", h0(0x49c6), Width::Bytes(0x16);
    SlotCount = "slot_count", h0(0x515a), Width::Bytes(0x12);
    SlotPrize = "slot_prize", h0(0x5526), Width::W;
    SlotWin = "slot_win", h0(0x57b2), Width::L;
    SlotHilo = "slot_hilo", h0(0x5988), Width::Bytes(6);
    SlotHiloStart = "slot_hilo_start", h0(0x5a04), Width::W;
    SlotSpin = "slot_spin", h0(0x5dc6), Width::Bytes(0x1c);
    SlotFlash = "slot_flash", h0(0x603e), Width::W;
    SlotCursor = "slot_cursor", h0(0x6064), Width::L;
    SlotMenu = "slot_menu", h0(0x6152), Width::Bytes(0xe6);
    SlotAttract = "slot_attract", h0(0x62ec), Width::Bytes(8);
    SlotLamps = "slot_lamps", h0(0x6842), Width::Bytes(0xd0);
    SlotPalette = "slot_palette", h0(0x6912), Width::Bytes(0x40);
    /// `entry_fall` (h0+$6c92): how fast the frog drops in at a level's start.
    EntryFall = "entry_fall", h0(0x6c92), Width::W;
    /// `text_plane` (h0+$706e): which plane the text printer (h0+$6f3a) writes, 1 or 2.
    TextPlane = "text_plane", h0(0x706e), Width::W;
    /// `passage_cell` (h0+$776c): the fake-wall cell the frog entered (marker 38).
    PassageCell = "passage_cell", h0(0x776c), Width::L;
    /// `passage_toggle` (h0+$7770).
    PassageToggle = "passage_toggle", h0(0x7770), Width::W;
    /// The frog's shots in the bonus level (`bonus::shots`): where they were spawned
    /// (`shot_spawn_x`, `shot_spawn_y`, h0+$7f0a, $7f0e), `shot_power` (h0+$7f12), `shot_delay`
    /// (h0+$80c6), `shot_cycle` and its table (h0+$80c8, $80cc) and the five `shots` (h0+$80dc).
    ShotSpawnX = "shot_spawn_x", h0(0x7f0a), Width::L;
    ShotSpawnY = "shot_spawn_y", h0(0x7f0e), Width::L;
    ShotPower = "shot_power", h0(0x7f12), Width::W;
    ShotDelay = "shot_delay", h0(0x80c6), Width::W;
    ShotCycle = "shot_cycle", h0(0x80c8), Width::L;
    ShotCycleTable = "shot_cycle_table", h0(0x80cc), Width::Bytes(0x10);
    Shots = "shots", h0(0x80dc), Width::Bytes(0x14);
    /// The panel's drawing (`level::panel`): `lives_icon_shown` (h0+$87c0), `score_digit`
    /// (h0+$8a74), `score_text` (h0+$8ab4..$8ac0), `clock_digit` and `clock_shown` (h0+$8be0,
    /// h0+$8be2), `extra_life_shown` (h0+$dd4), `number_started` and `number_text` (h0+$161aa,
    /// h0+$161b2).
    LivesIconShown = "lives_icon_shown", h0(0x87c0), Width::W;
    ScoreDigit = "score_digit", h0(0x8a74), Width::W;
    ScoreText = "score_text", h0(0x8ab4), Width::Bytes(0xc);
    ClockDigit = "clock_digit", h0(0x8be0), Width::W;
    ClockShown = "clock_shown", h0(0x8be2), Width::Bytes(4);
    /// `bonus_visit` (h0+$8cd6): a word inside the code `level_init_lb` reads for the bonus
    /// level's start row (2 lower, 3 lower still, cleared from 3 on); nothing in the game sets it.
    BonusVisit = "bonus_visit", h0(0x8cd6), Width::W;
    /// h0+$aa40..$aac0: the end sequence's two palettes, white (the last picture's flash) and the
    /// pictures' own, which the fades read.
    EndPalettes = "end_palettes", h0(0xaa40), Width::Bytes(0x80);
    /// h0+$b806..$b80e: the Accessing-disk overlay's palettes while it is up.
    OverlayUp = "overlay_up", h0(0xb806), Width::Bytes(8);
    /// `hitboxes` (h0+$d350): the objects' hitboxes, 12 bytes each, rebuilt each pass.
    Hitboxes = "hitboxes", h0(0xd350), Width::Bytes(0x600);
    /// `shooter_half_step` (h0+$de28): the bonus level's camera creeps on every second frame.
    ShooterHalfStep = "shooter_half_step", h0(0xde28), Width::W;
    /// `ice_tick`, `push_tick` (h0+$eb28, h0+$ebf0): frames on an icy or a pushing surface.
    IceTick = "ice_tick", h0(0xeb28), Width::W;
    PushTick = "push_tick", h0(0xebf0), Width::W;
    /// `tile_anim_queue` (h0+$fa68): cells waiting to show their next tile.
    TileAnimQueue = "tile_anim_queue", h0(0xfa68), Width::Bytes(16);
    /// `fall_history` (h0+$10182): a hurt frog's y on each of the last eight frames.
    FallHistory = "fall_history", h0(0x10182), Width::Bytes(16);
    /// `slope_frame_right`, `slope_frame_left` (h0+$1050e, h0+$10ce6): the frame of one step in
    /// each walking animation, which the slope under the frog rewrites (h0+$de2a).
    SlopeFrameRight = "slope_frame_right", h0(0x1050e), Width::L;
    SlopeFrameLeft = "slope_frame_left", h0(0x10ce6), Width::L;
    /// `frog_idle_timer` (h0+$11350).
    FrogIdleTimer = "frog_idle_timer", h0(0x11350), Width::W;
    /// `shooter_decay` (h0+$1194e): toggles every frame in the bonus level.
    ShooterDecay = "shooter_decay", h0(0x1194e), Width::W;
    /// `demo_running`, `demo_fire`, `demo_left`, `demo_at` (h0+$11a24..$11a2e): the attract
    /// demo's playback (`demo_joy` is between them).
    DemoRunning = "demo_running", h0(0x11a24), Width::L;
    DemoFire = "demo_fire", h0(0x11a28), Width::W;
    /// `demo_joy` (h0+$11a2a): the attract demo's joystick, read instead of JOY1DAT.
    DemoJoy = "demo_joy", h0(0x11a2a), Width::W;
    DemoLeft = "demo_left", h0(0x11a2c), Width::W;
    DemoAt = "demo_at", h0(0x11a2e), Width::L;
    /// `demo_record_at` (h0+$11aa2): where `demo_record` writes; `level_start` points it at the
    /// world's demo (h0+$9f2).
    DemoRecordAt = "demo_record_at", h0(0x11aa2), Width::L;
    /// h0+$11b64..$11b6a: the mouse counters and the pointer's position.
    Mouse = "mouse", h0(0x11b64), Width::Bytes(6);
    /// `clock_ticks` (h0+$11dfc): vertical blanks into the level clock's current second.
    ClockTicks = "clock_ticks", h0(0x11dfc), Width::W;
    /// `display_timer` (h0+$123d4): counted down each vertical blank.
    DisplayTimer = "display_timer", h0(0x123d4), Width::W;
    AudioSfx = "sfx_chan0", h0(crate::audio::vars::SFX.0), Width::Bytes(crate::audio::vars::SFX.1 as u32);
    AudioMusicOn = "music_on", h0(crate::audio::vars::MUSIC_ON.0), Width::Bytes(crate::audio::vars::MUSIC_ON.1 as u32);
    AudioP41 = "p41_channels", h0(crate::audio::vars::P41.0), Width::Bytes(crate::audio::vars::P41.1 as u32);
    AudioShadow = "music_fade", h0(crate::audio::vars::SHADOW.0), Width::Bytes(crate::audio::vars::SHADOW.1 as u32);
    AudioStream = "stream_ptr", h0(crate::audio::vars::STREAM.0), Width::Bytes(crate::audio::vars::STREAM.1 as u32);
    AudioSfxTables = "sfx_samples", h0(crate::audio::vars::SFX_TABLES.0), Width::Bytes(crate::audio::vars::SFX_TABLES.1 as u32);
    /// `fades` (h0+$157d8): the four palette fade records, `$1f2` bytes each (`level::fade`).
    Fades = "fades", h0(0x157d8), Width::Bytes(0x7c8);
    /// `kbd_toggle`, `key_down`, `key_up`, and the byte after them (h0+$16076..$16079).
    Keyboard = "kbd_toggle", h0(0x16076), Width::Bytes(4);
    /// `rng_state` (h0+$160d4): the 64-bit RNG state, `rng_next`'s low long then its high long.
    RngState = "rng_state", h0(0x160d4), Width::Bytes(8);
    NumberStarted = "number_started", h0(0x161aa), Width::L;
    Number161ae = "number_161ae", h0(0x161ae), Width::L;
    NumberText = "number_text", h0(0x161b2), Width::Bytes(0xc);
    /// The four bytes after `number_text`: `level_tally`'s padding (h0+$47de) turns sixteen bytes
    /// from the text's start that are 0 into spaces (T38).
    NumberPad = "number_pad", h0(0x161be), Width::Bytes(4);
    /// `marker_cells` (h0+$162e4): the cells of the map's first marker `$30`..`$33` each
    /// (h0+$1625e), one long a marker.
    MarkerCells = "marker_cells", h0(0x162e4), Width::Bytes(0x10);
    /// `coins_total` (h0+$163d0), `items_total` (h0+$163d2): the map's coins (marker 8), and its
    /// coins and marker-9 cells (h0+$16374).
    CoinsTotal = "coins_total", h0(0x163d0), Width::W;
    ItemsTotal = "items_total", h0(0x163d2), Width::W;
    /// The scroller (h0+$16604..$1668c): rows and columns drawn by the last pass (with a word
    /// either side, which the comparisons read), its row and column shifts, the frame toggle,
    /// tiles a line, the pending-blits flag and the count drawn.
    ScrollerRowsDrawn = "rows_drawn", h0(0x16604), Width::Bytes(0x28);
    ScrollerColsDrawn = "cols_drawn", h0(0x1662c), Width::Bytes(0x30);
    /// `shown_buffer` (h0+$1665e): 1 while buffer 0 is shown, 2 while buffer 1 is.
    ShownBuffer = "shown_buffer", h0(0x1665e), Width::W;
    /// `camera_y` (h0+$16660): the camera's level y (docs/12).
    CameraY = "camera_y", h0(0x16660), Width::L;
    /// `camera_x` (h0+$16664): the camera's level x.
    CameraX = "camera_x", h0(0x16664), Width::L;
    ScrollerRowShift = "scroller_row_shift", h0(0x16668), Width::L;
    ScrollerColShiftX = "scroller_col_shift_x", h0(0x1666c), Width::L;
    ScrollerToggle = "scroller_toggle", h0(0x16674), Width::W;
    ScrollerTilesPerLine = "scroller_tiles_per_line", h0(0x16676), Width::L;
    ScrollerRowShiftX = "scroller_row_shift_x", h0(0x1667a), Width::L;
    ScrollerColShiftY = "scroller_col_shift_y", h0(0x1667e), Width::L;
    /// `scroll_dirs` (h0+$16682): which way the camera last moved (bits 1 right, 2 left, 3
    /// down, 4 up).
    ScrollDirs = "scroll_dirs", h0(0x16682), Width::W;
    /// h0+$16684: cleared with the scroller's state (h0+$164c2).
    Scroller16684 = "scroller_16684", h0(0x16684), Width::L;
    BlitsPending = "blits_pending", h0(0x16688), Width::W;
    ScrollerDrawn = "scroller_drawn", h0(0x1668a), Width::W;
    /// `shooter_column` (h0+$16752), `blit_next` (h0+$16756), `blit_queue` (h0+$1675a, 23 × 16
    /// bytes; its end mark follows).
    ShooterColumn = "shooter_column", h0(0x16752), Width::L;
    BlitNext = "blit_next", h0(0x16756), Width::L;
    BlitQueue = "blit_queue", h0(0x1675a), Width::Bytes(0x170);
    /// `scroll_column` (h0+$172b0): the play-field column the vertical blank last scrolled to, as
    /// the address of its first word in the shown buffer (docs/12 "Scroll").
    ScrollColumn = "scroll_column", h0(0x172b0), Width::L;
    /// `map_rows` (h0+$17334): one row above the in-RAM map; a front-end write (the title sets it),
    /// shadowing the boot's row-pointer table there.
    MapRows = "map_rows", h0(0x17334), Width::L;
    /// `bob_draw_vars` (h0+$18bc8..$18be8): the bob drawer's camera, seam lines, column, seam
    /// pass flag and `bob_buffer` (h0+$18be4, the display buffer it drew into).
    BobDrawVars = "bob_draw_vars", h0(0x18bc8), Width::Bytes(0x20);
    /// `line_offsets` (h0+$18be8): each play-field line's byte offset (`$30` a line), built by
    /// the level's set-up.
    LineOffsets = "line_offsets", h0(0x18be8), Width::Bytes(0x27c);
    /// h0+$19ddc..$1a6dc: their table, 96 pairs of 12-byte records (frames, offset, height),
    /// the frog's own then its mirror image's.
    HeroTable = "hero_table", h0(0x19ddc), Width::Bytes(0x900);
    /// `frog_sprite_left`, `_right` (h0+$1aff4, h0+$1b010): the frog's two hardware-sprite
    /// records (`$1c` bytes: screen x, y, height, attached, copper words, data, animation).
    FrogSpriteLeft = "frog_sprite_left", h0(0x1aff4), Width::Bytes(0x1c);
    FrogSpriteRight = "frog_sprite_right", h0(0x1b010), Width::Bytes(0x1c);
    /// `exit_sprite_left`, `_right` (h0+$1b02c, h0+$1b048): the second pair of frog sprites.
    ExitSpriteLeft = "exit_sprite_left", h0(0x1b02c), Width::Bytes(0x1c);
    ExitSpriteRight = "exit_sprite_right", h0(0x1b048), Width::Bytes(0x1c);
    /// h0+$1b10c..$1b208: hardware-sprite records (pointer, title eyes, menu cursor).
    SpriteRecs = "sprite_recs", h0(0x1b10c), Width::Bytes(0xfc);
    /// `bob_free_list` (h0+$1ba16), `obj_count` (h0+$1ba6a), `obj_slots` (h0+$1ba6c).
    BobFreeList = "bob_free_list", h0(0x1ba16), Width::Bytes(0x54);
    ObjCount = "obj_count", h0(0x1ba6a), Width::W;
    ObjSlots = "obj_slots", h0(0x1ba6c), Width::Bytes(0x9b0);
    /// `obj_templates` (h0+$1c6e2): 160 templates of `$ae` bytes; a spawner writes the position
    /// into one before copying its head into a slot (docs/09 "The slot record").
    ObjTemplates = "obj_templates", h0(0x1c6e2), Width::Bytes(160 * 0xae);
    /// The bonus level's reward counters (`bonus::death`): `reward_life_count` (h0+$23de4),
    /// `reward_power_count` (h0+$23e20, 9 in the executable).
    RewardLifeCount = "reward_life_count", h0(0x23de4), Width::W;
    RewardPowerCount = "reward_power_count", h0(0x23e20), Width::W;
    /// W6's enemies' variables (`enemies::w6`), inside the code: `chain_link_count` (h0+$262ba),
    /// the mace's links counted as marker 17 spawns them; `thrower_vx` (h0+$281d6);
    /// `flier_facing_right` (h0+$28322); `cannon_aim` (h0+$28454), the next shot's vx and vy.
    ChainLinkCount = "chain_link_count", h0(0x262ba), Width::L;
    ThrowerVx = "thrower_vx", h0(0x281d6), Width::W;
    FlierFacingRight = "flier_facing_right", h0(0x28322), Width::W;
    CannonAim = "cannon_aim", h0(0x28454), Width::L;
    /// The bonus level's animation toggles (`bonus::enemies`), words inside the code that the
    /// listing reads as instructions: `bonus_toggle_114` (h0+$28ad8) and `bonus_toggle_113`
    /// (h0+$28b4a), flipped by each spawn of those types to pick one of two animations.
    BonusToggle114 = "bonus_toggle_114", h0(0x28ad8), Width::W;
    BonusToggle113 = "bonus_toggle_113", h0(0x28b4a), Width::W;
    /// `bonus_fly_off` (h0+$293ce): a word inside the code, set by the bonus level's marker 11
    /// and cleared by the inits of 127 and 128; while it is set those fly off left.
    BonusFlyOff = "bonus_fly_off", h0(0x293ce), Width::W;
    /// `bob_draw_list` (h0+$2970a..): a 0, the bobs to draw, a 0.
    BobDrawList = "bob_draw_list", h0(0x2970a), Width::Bytes(0xa8);
    /// `bob_records` (h0+$297b2..$2aee2): the spud's bob and the 20 objects' (`$11c` bytes,
    /// four sub-records each), each with its level position in the long before it.
    BobRecords = "bob_records", h0(0x297b2), Width::Bytes(0x1730);
    /// `bob_frames` (h0+$2aee2..$2ed32): the bobs' animation lists and frame records, which the
    /// level's set-up builds for its world (h0+$1995a).
    BobFrames = "bob_frames", h0(0x2aee2), Width::Bytes(0x3e50);
    /// h0+$3b18c..$3b1ae: the ANIM player's record (`crate::front::anim::RECORD`).
    AnimPlayer = "anim_player", h0(0x3b18c), Width::Bytes(0x22);
    AudioHeaderBackup = "module_header_backup", h0(crate::audio::vars::HEADER_BACKUP.0), Width::Bytes(crate::audio::vars::HEADER_BACKUP.1 as u32);
    /// `level_map` (h1+$218e0): the map, 84 rows of 304 cells, `tile << 6 | marker`, and the
    /// rows the loader adds under it (a floor of `$3d` cells) up to the tile pointers.
    LevelMap = "level_map", h1(0x218e0), Width::Bytes(LEVEL_MAP_BYTES);
    /// `level_palette` (h1+$31de0): the level's 32 colours, from its map's `PALA` (h0+$1622a);
    /// the fades fade to it.
    LevelPalette = "level_palette", h1(0x31de0), Width::Bytes(0x40);
    // Ids with no entry here (not in `VarId::ALL`): a program with its own memory map keeps its
    // own table of them.
    @outside {
        /// The story intro's (T32, `front::story`), compared only while it runs: disk 3's boot
        /// program's fade record (`d3_c02` h0+$2be18).
        StoryBootFade,
        /// Its two field copper lists and the black one (h0+$2c00c..$2c2b4).
        StoryBootCopper,
        /// The intro's (`d3_c04`): `intro_skip`, `vbl_flag`, `vbl_count`, the frame table pointer.
        StoryVars,
        /// `buffer_toggle` and the row table.
        StoryRows,
        /// The object list and the sixteen objects.
        StoryObjects,
        /// The frame descriptors.
        StoryFrames,
        /// `draw_buffer`, `shown_buffer`, `planes_frozen`.
        StoryShown,
        /// The four fade records.
        StoryFades,
        /// The copper lists (text pages, scenes, black).
        StoryCopper,
        /// The text page's three planes and palette.
        StoryText,
        /// The two display buffers.
        StoryBuffers,
        /// The clean background and the scene's palette after it.
        StoryClean,
        /// The intro's sound driver (the game's at other addresses): the effect records,
        StorySfx,
        /// `music_on`,
        StoryMusicOn,
        /// the replayer's channels and globals,
        StoryReplayer,
        /// the fade and the shadow registers,
        StoryShadow,
        /// the streamer,
        StoryStream,
        /// and the intro's timeline of sound events.
        StoryTimeline,
    }
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
        assert_eq!(index_of(0), Some(0), "low_vectors");
        assert_eq!(index_of(8), None);
        assert_eq!(index_of(0xF0_0000), None);
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
