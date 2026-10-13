//! Entering a level as the original does (T14): what [`crate::Start::Level`] runs.
//!
//! From power-on the original boots, shows the title, and a game's script then loads the world
//! (`world_load_wN`: its sound, tiles and bobs), loads the level (`level_load_wN_lM`: the clock,
//! the map and what is read off it), sets it up (`level_init_wN`: objects, the frog, the camera,
//! the first scroll) and starts it (`level_start`: one frog step, the copper list, the panel, the
//! fade in), then enters `level_frame_loop` (docs/08 "Scene flow"). The engine does all of that at
//! once, leaving the state at the frame `level_frame_loop` starts on: frame 0 of the level.
//!
//! The screens shown on the way (the world's intro picture, the progress map) leave nothing a
//! level reads; they are skipped.

use sf_assets::{LevelId, World};

use crate::frog::{self, FROG};
use crate::layout::{H0, H1, H2, H3, LAYOUT, h0, h1};
use crate::level::{Hw, LevelCtx, camera, collision, scroll, standard_dispatch};
use crate::mem::Bus;
use crate::render::{DisplayData, HERO_SPRITES};
use crate::state::State;

/// `logic_running` (h0+$9d6).
const LOGIC_RUNNING: u32 = h0(0x9d6);
/// `world`, `level` (h0+$40da, $40de).
const WORLD: u32 = h0(0x40da);
const LEVEL: u32 = h0(0x40de);
/// `level_clock` (h0+$876): four digits, minutes first.
const LEVEL_CLOCK: u32 = h0(0x876);
/// `level_state` (h0+$a06): negative while a level plays.
const LEVEL_STATE: u32 = h0(0xa06);
/// `level_started` (h0+$9f0).
const LEVEL_STARTED: u32 = h0(0x9f0);
/// `start_world`, `start_level` (h0+$a, $e): the level a game starts at.
const START_WORLD: u32 = h0(0xa);
const START_LEVEL: u32 = h0(0xe);
/// `obj_world_offset` (h0+$9fa).
const OBJ_WORLD_OFFSET: u32 = h0(0x9fa);
/// `level_start_x`, `level_start_y` (h0+$a1a, $a1e).
const START_X: u32 = h0(0xa1a);
const START_Y: u32 = h0(0xa1e);
/// `entry_fall` (h0+$6c92): the drop-in's frame count.
const ENTRY_FALL: u32 = h0(0x6c92);
/// `map_rows` (h0+$17334).
const MAP_ROWS: u32 = h0(0x17334);
/// `playfield_buffer_1` (h3+$22f30): where a level's map file is loaded.
const LOAD_BUFFER: u32 = crate::layout::h3(0x22f30);
/// The map's area, from five rows above `level_map` (h1+$20d00) to the tile pointers.
const MAP_AREA: u32 = h1(0x20d00);
const MAP_AREA_CELLS: u32 = 0x8170;
/// `level_map + $4c0`: the map's first row (h0+$1647a).
const MAP_FIRST_ROW: u32 = h1(0x218e0 + 0x4c0);
/// Bytes from one map row to the next: 304 cells.
const ROW: u32 = 0x260;

/// Leave `st` as the original's memory is on the frame `level_frame_loop` starts for `id`, from
/// a game started at that level's code.
pub fn enter(st: &mut State, display: &DisplayData, id: LevelId) {
    boot_image(st, display);
    world_sound(st, display, id);
    let mut paula = Vec::new();
    let mut c = LevelCtx {
        bus: Bus::new(st, display.rom()),
        dispatch: standard_dispatch(),
        hw: Hw {
            sdr: 0xff,
            ..Hw::default()
        },
        sound: display.sound(),
        paula: &mut paula,
    };
    new_game(&mut c, id);
    level_load(&mut c, display, id);
    level_init(&mut c, id);
    level_start(&mut c);
}

/// Every modelled variable as the game's loader leaves memory: hunk 0 as loaded (with the boot's
/// tables), hunk 2's data; hunks 1 and 3 are cleared. Then what the boot sets (`init_and_script`):
/// the frog's sprite frames among it (`load_hero`).
fn boot_image(st: &mut State, display: &DisplayData) {
    let hero = display.hero();
    for d in LAYOUT {
        let bytes: Vec<u8> = (d.addr..d.addr + d.width.len())
            .map(|a| match a {
                _ if (H0..H1).contains(&a) => display.rom().byte(0, a).unwrap_or(0),
                _ if (H2..H3).contains(&a) => display.h2_bytes(a - H2, 1)[0],
                _ if (HERO_SPRITES..HERO_SPRITES + hero.len() as u32).contains(&a) => {
                    hero[(a - HERO_SPRITES) as usize]
                }
                4..8 => crate::state::EXEC_BASE.to_be_bytes()[a as usize - 4],
                _ => 0,
            })
            .collect();
        st.poke(d.addr, &bytes);
    }
    st.set_l(MAP_ROWS, h1(0x21680));
}

/// The sound driver as the boot leaves it, then the world's effects (`sfx_bank_init` on the bank
/// loaded at [`BANK_ADDR`]) and its module at `module_addr` (`world_load_wN`). The final level
/// plays W6's.
fn world_sound(st: &mut State, display: &DisplayData, id: LevelId) {
    use crate::audio::{AudioDriver, BANK_ADDR, BankId, File, TuneId};
    let d = display.sound();
    st.audio = AudioDriver::new(d);
    st.audio.set_low_chip(crate::front::title::KICKSTART_LOW_CHIP);
    if !d.is_complete() {
        return;
    }
    // The title's music, faded out and stopped as a game starts (`music_fade_wait`, h0+$11a4).
    st.audio.file_loaded(d, BANK_ADDR, File::Tune(TuneId::Title));
    st.audio.set_module_addr(BANK_ADDR);
    st.audio.stop_music(d, &mut Vec::new());
    let (bank, tune) = match id.world {
        World::W1 | World::W2 => (BankId::Worlds12, TuneId::World(numbers(id).0 as u8)),
        World::Bonus => (BankId::Bonus, TuneId::Bonus),
        _ => (BankId::Worlds36, TuneId::World(numbers(id).0 as u8)),
    };
    st.audio.file_loaded(d, BANK_ADDR, File::Bank(bank));
    st.audio.sfx_bank_init(d);
    let at = st.audio.module_addr();
    st.audio.file_loaded(d, at, File::Tune(tune));
}

/// What `title_screen` does as START GAME is chosen (h0+$11aa), then the level code's start.
fn new_game(c: &mut LevelCtx, id: LevelId) {
    let b = &mut c.bus;
    b.set_w(h0(0x9e8), b.w(h0(0x9e6)));
    b.set_l(h0(0x11a24), 1);
    b.set_w(h0(0x11a28), 0);
    b.set_w(h0(0x11a2a), 0);
    b.set_w(h0(0x11a2c), 2);
    b.set_l(h0(0x11a2e), b.l(h0(0x9f2)));
    frog::frog_new_game(b);
    b.set_w(h0(0x87c0), 0xffff);
    b.set_w(h0(0x9de), 0);
    b.set_w(h0(0x9e0), 0);
    b.set_w(camera::SHOOTER_MODE, 0);
    // The final level has no code: a game reaches it from W6-4's.
    let (world, level) = if id.world == World::Final { (6, 4) } else { numbers(id) };
    b.set_l(START_WORLD, world);
    b.set_l(START_LEVEL, level);
}

/// `world` and `level` for `id`.
fn numbers(id: LevelId) -> (u32, u32) {
    let world = match id.world {
        World::W1 => 1,
        World::W2 => 2,
        World::W3 => 3,
        World::W4 => 4,
        World::W5 => 5,
        World::W6 | World::Final => 6,
        World::Bonus => 7,
    };
    (world, id.level as u32)
}

/// `world_load_wN` and `level_load_wN_lM` (h0+$94a0, $9552 …): what they leave that a level
/// reads: the world and level, the clock, the map and what is read off it.
fn level_load(c: &mut LevelCtx, display: &DisplayData, id: LevelId) {
    let (world, level) = numbers(id);
    let b = &mut c.bus;
    b.set_l(WORLD, world);
    b.set_l(LEVEL, level);
    if id.world == World::Final {
        b.set_w(camera::FINAL_LEVEL_FLAG, 1);
    }
    let clock: [u8; 4] = if matches!(id.world, World::Final | World::Bonus) {
        [1, 0, 0, 0]
    } else {
        [0, 4, 0, 0]
    };
    for (k, d) in clock.into_iter().enumerate() {
        b.set_b(LEVEL_CLOCK + k as u32, d);
    }
    // The map file, as load_file leaves it.
    let file = display.levels.get(&id).cloned().unwrap_or_default();
    for (k, &v) in file.iter().enumerate() {
        b.set_b(LOAD_BUFFER + k as u32, v);
    }
    palette_from_map(b);
    map_unpack(b);
    count_coins(b);
    find_marker_cells(b);
    find_start(b);
}

/// h0+$1622a: the 32 colours `$44` bytes past the map's `PALA` tag into `level_palette`.
fn palette_from_map(b: &mut Bus) {
    let Some(at) = (0..0x80).map(|k| LOAD_BUFFER + 4 * k).find(|&a| b.l(a) == 0x5041_4c41) else {
        return;
    };
    let from = at + 4 + 0x44;
    for k in 0..0x20 {
        b.set_w(h1(0x31de0) + 2 * k, b.w(from + 2 * k));
    }
}

/// h0+$1647a: the map area cleared, the `BODY` moved to the load buffer's start (h0+$16202), and
/// each of its 84 rows of 300 cells into `level_map`, each cell turned from `tile | marker << 10`
/// into `tile << 6 | marker`, four cells of padding after each row.
fn map_unpack(b: &mut Bus) {
    for k in 0..MAP_AREA_CELLS * 2 {
        let a = MAP_AREA + k;
        if crate::layout::index_of(a).is_some() {
            b.set_b(a, 0);
        }
    }
    let Some(body) = (0..0x800).map(|k| LOAD_BUFFER + 4 * k).find(|&a| b.l(a) == 0x424f_4459) else {
        return;
    };
    let len = b.l(body + 4) / 4;
    for k in 0..len {
        b.set_l(LOAD_BUFFER + 4 * k, b.l(body + 8 + 4 * k));
    }
    for row in 0..84 {
        for col in 0..300 {
            let v = b.w(LOAD_BUFFER + 2 * (row * 300 + col));
            b.set_w(MAP_FIRST_ROW + row * ROW + 2 * col, v.rotate_right(10));
        }
    }
}

/// The marker of the map cell at `a` in the map area (0 outside the modelled map).
fn marker(b: &Bus, a: u32) -> u16 {
    b.w(a) & 0x3f
}

/// h0+$16374: the coins (marker 8) and the coins and marker-9 cells counted; the exit wants all
/// the coins (at most 99).
fn count_coins(b: &mut Bus) {
    b.set_w(FROG + 0x7e, 0);
    b.set_l(h0(0x445e), 0);
    let (mut coins, mut items) = (0u16, 0u16);
    for k in 0..MAP_AREA_CELLS {
        match marker(b, MAP_AREA + 2 * k) {
            8 => {
                coins += 1;
                items += 1;
            }
            9 => items += 1,
            _ => {}
        }
    }
    b.set_w(h0(0x163d2), items);
    b.set_w(h0(0x163d0), coins);
    b.set_w(h0(0x832), coins.min(99));
}

/// h0+$1625e: the first cell of each marker `$30`..`$33` whose cells above and below are not of
/// that range, four in all.
fn find_marker_cells(b: &mut Bus) {
    for k in 0..4 {
        b.set_l(h0(0x162e4) + 4 * k, 0);
    }
    let bump = |m: u16| (0x30..0x34).contains(&m);
    let mut left = 4;
    for k in 0..MAP_AREA_CELLS {
        let a = MAP_AREA + 2 * k;
        let m = marker(b, a);
        if bump(m) && !bump(marker(b, a.wrapping_sub(ROW))) && !bump(marker(b, a + ROW)) {
            b.set_l(h0(0x162e4) + 4 * (m - 0x30) as u32, a);
            left -= 1;
            if left == 0 {
                return;
            }
        }
    }
}

/// h0+$162f4: the camera's start from the first start marker (`$35`): its cell's pixel less
/// (`$9b`, `$ae`), at least 0, 64 pixels higher in the final level; `$10`, `$160` without one.
fn find_start(b: &mut Bus) {
    let found = (0..MAP_AREA_CELLS).find(|&k| marker(b, MAP_AREA + 2 * k) == 0x35);
    let Some(k) = found else {
        b.set_l(START_X, 0x10);
        b.set_l(START_Y, 0x160);
        return;
    };
    let x = ((k % 304) * 16) as i16 - 0x9b;
    let y = ((k / 304) * 16) as i16 - 0xae;
    b.set_l(START_X, x.max(0) as i32 as u32);
    b.set_l(START_Y, y.max(0) as i32 as u32);
    if b.w(camera::FINAL_LEVEL_FLAG) != 0 {
        b.set_l(START_Y, b.l(START_Y).wrapping_sub(0x40));
    }
}

/// `level_init_wN` (h0+$8cd8 …): the objects, the frog, the panel, the camera and the first scroll.
fn level_init(c: &mut LevelCtx, id: LevelId) {
    let (world, _) = numbers(id);
    let (demo, a02, offset): (u32, u32, u32) = match (id.world, world) {
        (World::Bonus, _) => (0, h0(0x30a30), 0x600),
        (World::Final, _) => (h0(0x3ad8c), h0(0x309e8), 0x500),
        (_, w) => (
            h0(0x3ad8c),
            [h0(0x2ed2c), h0(0x2f164), h0(0x2f710), h0(0x2fbfc), h0(0x300a0), h0(0x30598)]
                [w as usize - 1],
            (w - 1) * 0x100,
        ),
    };
    let bonus = id.world == World::Bonus;
    let b = &mut c.bus;
    if bonus {
        b.set_w(camera::SHOOTER_MODE, 1);
    } else {
        b.set_l(h0(0x9f2), demo);
    }
    b.set_l(h0(0xa02), a02);
    b.set_l(OBJ_WORLD_OFFSET, offset);
    frog::frog_reset(b);
    bobs_and_sprites(b);
    panel_clear(b);
    if bonus {
        // level_init_lb (h0+$8c3a): the start at the top, lower on the second and third visits
        // (h0+$8cd6 counts them; a game started at the bonus level's code is a first visit).
        b.set_l(START_X, 0);
        b.set_l(START_Y, 0);
    }
    start_point(c);
    let b = &mut c.bus;
    if bonus {
        b.set_l(h0(0x16752), 4);
    }
    b.set_l(LEVEL_STATE, 0xffff_ffff);
    if bonus {
        crate::bonus::shots::spawn_shots(c);
    }
}

/// What `h0+$1995a` sets up that the engine models: each free bob record's first two words
/// marked `$c180` (h0+$84f0), the frog's two sprite records pointed into the copper list, the
/// sprites' colours (`level_copper+$390`, another set in the final level and shooter mode) and
/// the two panel-digit sprites. (Its bob sheets are the objects', T15–T18.)
fn bobs_and_sprites(b: &mut Bus) {
    let mut at = h0(0x1ba16);
    loop {
        let rec = b.l(at);
        if rec == 0 {
            break;
        }
        b.set_w(rec.wrapping_sub(4), 0xc180);
        b.set_w(rec.wrapping_sub(2), 0xc180);
        at += 4;
    }
    sprite_attach(b, camera::FROG_SPRITE_LEFT);
    sprite_attach(b, camera::FROG_SPRITE_RIGHT);
    let alt = b.w(camera::FINAL_LEVEL_FLAG) != 0 || b.w(camera::SHOOTER_MODE) != 0;
    let from = h0(0x19ac2) + if alt { 0x40 } else { 0 };
    for k in 0..0x40 {
        b.set_b(crate::layout::h2(0x6bd0 + 0x390) + k, b.b(from + k));
    }
    for rec in [h0(0x1b128), h0(0x1b144)] {
        sprite_attach(b, rec);
    }
    for rec in [h0(0x1b128), h0(0x1b144)] {
        frog::anim::sprite_to_copper(b, rec);
    }
}

/// `$1b208`: point a sprite record's copper MOVEs at its sprite (its animation's first frame,
/// if it has one), its CTL bits ORed in (`front::sprite::attach`, over the executable's tables).
fn sprite_attach(b: &mut Bus, rec: u32) {
    let anim = b.l(rec + 0x10);
    let data = if anim != 0 {
        b.set_l(rec + 0x14, anim);
        b.set_w(rec + 0x18, b.w(anim + 6));
        b.l(anim)
    } else {
        b.l(rec + 0xc)
    };
    let mv = b.l(rec + 8);
    b.set_w(mv + 0xe, b.w(mv + 0xe) | b.w(rec + 6));
    b.set_w(mv + 6, data as u16);
    b.set_w(mv + 2, (data >> 16) as u16);
}

/// h0+$8bf6: the panel's planes into the copper list (`level_copper+$f0`), and cleared.
fn panel_clear(b: &mut Bus) {
    let panel = crate::layout::h3(0x4b390);
    let list = crate::layout::h2(0x6bd0 + 0xf0);
    for k in 0..5 {
        let p = panel + k * 0x2a0;
        b.set_w(list + 8 * k + 6, p as u16);
        b.set_w(list + 8 * k + 2, (p >> 16) as u16);
    }
    for k in 0..0xd20 {
        b.set_b(panel + k, 0);
    }
}

/// h0+$164fa: the frog `$90`, `$70` from the start, the camera at it, kept inside the level; the
/// scroller's state reset (h0+$164c2), the drawn rows cleared and the drawn columns set to the
/// camera's, one scroll and its blits, the scroll latched.
fn start_point(c: &mut LevelCtx) {
    let b = &mut c.bus;
    let shooter = b.w(camera::SHOOTER_MODE);
    b.set_w(camera::SHOOTER_MODE, 0);
    let (x, y) = (b.l(START_X), b.l(START_Y));
    b.set_w(FROG, (x as u16).wrapping_add(0x90));
    b.set_w(FROG + 2, (y as u16).wrapping_add(0x70));
    b.set_l(camera::CAMERA_X, x);
    b.set_l(camera::CAMERA_Y, y);
    let right = b.w(camera::LEVEL_WIDTH).wrapping_sub(0x168);
    if !crate::level::n_clear(right.wrapping_sub(b.w(camera::CAMERA_X + 2))) {
        b.set_w(camera::CAMERA_X + 2, right);
    }
    if !crate::level::n_clear(b.w(camera::CAMERA_X + 2).wrapping_sub(0x10)) {
        b.set_l(camera::CAMERA_X, 0x10);
    }
    let bottom = b.w(camera::LEVEL_HEIGHT).wrapping_sub(0xf0);
    if !crate::level::n_clear(bottom.wrapping_sub(b.w(camera::CAMERA_Y + 2))) {
        b.set_w(camera::CAMERA_Y + 2, bottom);
    }
    if !crate::level::n_clear(b.w(camera::CAMERA_Y + 2).wrapping_sub(0x20)) {
        b.set_l(camera::CAMERA_Y, 0x20);
    }
    scroller_reset(b);
    for k in 0..0x11 {
        b.set_w(h0(0x16606) + 2 * k, 0);
    }
    let mut col = (b.l(camera::CAMERA_X) as u16) & 0xfff0;
    for k in 0..0x17 {
        b.set_w(h0(0x1662c) + 2 * k, col);
        col = col.wrapping_add(0x10);
    }
    b.set_b(h0(0x16674), 0);
    scroll::scroller(c);
    scroll::copper_irq_blits(c);
    c.bus.set_b(h0(0x16674), 0);
    scroll::scroll_latch(c);
    c.bus.set_w(camera::SHOOTER_MODE, shooter);
}

/// h0+$164c2: the scroller's toggles, shifts and blit queue cleared.
fn scroller_reset(b: &mut Bus) {
    b.set_w(h0(0x16674), 0);
    b.set_l(h0(0x16676), 0);
    b.set_l(h0(0x1667a), 0);
    b.set_l(h0(0x1667e), 0);
    b.set_w(h0(0x16682), 0);
    b.set_l(h0(0x16684), 0);
    for k in 0..0x170 {
        b.set_b(h0(0x1675a) + k, 0);
    }
}

/// `level_start` (h0+$6952): the frog's first step, its sprites, then the level's display.
fn level_start(c: &mut LevelCtx) {
    let b = &mut c.bus;
    b.set_w(LEVEL_STARTED, 1);
    let demo = b.l(h0(0x9f2));
    b.set_l(h0(0x11a2e), demo);
    b.set_l(h0(0x11aa2), demo);
    collision::frog_tiles(c);
    frog::anim::frog_anim_tick(c);
    frog::physics::frog_walk_input(c);
    frog::anim::frog_input_anim(c);
    frog::anim::place_frog_sprites(c);
    frog::physics::frog_move(c);
    camera::camera_follow(c);
    frog::anim::frog_anim_tick(c);
    scroll::scroll_latch(c);
    c.bus.set_w(camera::FROG_SPRITE_LEFT + 2, 0xffe0);
    c.bus.set_w(camera::FROG_SPRITE_RIGHT + 2, 0xffe0);
    frog::anim::place_frog_sprites(c);
    fade_in(&mut c.bus);
    crate::level::panel::draw_at_start(c);
    drop_in(c);
    if c.bus.l(crate::level::ATTRACT_MODE) == 0 && !c.bus.st.audio.music_on() {
        c.bus.st.audio.music_start(c.sound, c.paula);
    }
}

/// `level_start`'s fade in (h0+$153a4 on the first record, at speed 1): the level's colours from
/// `level_copper+$2d4` to the world's palette (h0+$9fe). The vertical blank steps it while the
/// panel slides in; it is over by frame 0.
fn fade_in(b: &mut Bus) {
    use crate::front::fade;
    let rec = fade::RECORDS[0];
    b.set_w(rec.at + fade::SPEED, 1);
    let target = b.l(h0(0x9fe));
    fade::fade_in(b.st, rec, crate::layout::h2(0x6bd0 + 0x2d4), target, 0x20);
    while !fade::done(b.st, rec) {
        fade::step_in_rec(b.st, rec);
    }
}

/// h0+$6ad2: the frog's drop-in (`frog_level_entry` plays it while the sprites are hidden): its
/// entry animation, its sprites two pixels left and six up; in shooter mode it is placed on the
/// screen instead. Then the level's logic runs.
fn drop_in(c: &mut LevelCtx) {
    let shooter = c.bus.w(camera::SHOOTER_MODE) != 0;
    frog::anim::frog_set_anim(c, if shooter { h0(0x10b7e) } else { h0(0x10a1a) });
    let b = &mut c.bus;
    b.set_w(ENTRY_FALL, 0);
    let (left, right) = (camera::FROG_SPRITE_LEFT, camera::FROG_SPRITE_RIGHT);
    if shooter {
        b.set_w(FROG, b.w(camera::CAMERA_X + 2).wrapping_add(0x80));
        b.set_w(FROG + 2, b.w(camera::CAMERA_Y + 2).wrapping_add(0x5c));
        b.set_w(left, 0xffe0);
        b.set_w(right, 0xfff0);
        b.set_w(left + 2, 0x5c);
        b.set_w(right + 2, 0x5c);
    } else {
        b.set_w(left, b.w(left).wrapping_sub(2));
        b.set_w(right, b.w(right).wrapping_sub(2));
        b.set_w(left + 2, 0xfffa);
        b.set_w(right + 2, 0xfffa);
    }
    b.set_w(camera::SPRITES_HIDDEN, 1);
    b.set_w(LOGIC_RUNNING, 1);
}
