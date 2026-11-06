//! The port register (plans/forest-dumb-level-1 DESIGN §4.1; `toolkit/docs/porting-routines.md` §4):
//! every routine of the original ported into this crate, by hunk and offset, with the Rust function
//! that ports it. Its name is the routine's `kind = "func"` entry in `re/symbols.toml`, at the same
//! hunk and offset; `fd-conform/tests/ported.rs` fails on a mismatch and on two entries for one
//! address, because parallel ports of one routine were the first game's most repeated silent gap.
//!
//! Porting tasks only append to [`PORTED`]. A routine another task registered is called, never
//! ported a second time. The coverage report (T06) counts how often the original ran each entry.

/// One ported routine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ported {
    /// Its name in `re/symbols.toml`.
    pub name: &'static str,
    pub hunk: u8,
    /// Byte offset within the hunk, as `hN+$off` in the listing.
    pub offset: u32,
    /// The Rust function that ports it, as a path from this crate's root (`level::char_step`).
    pub port: &'static str,
}

/// Every ported routine. Append only.
pub const PORTED: &[Ported] = &[
    // T51: the animation frame pickers and the joystick read.
    port("char_step_4be6", 0x4be6, "character::Anim::walk_left"),
    port("h0_4c28", 0x4c28, "character::Anim::walk_right"),
    port("h0_4c6a", 0x4c6a, "character::Anim::frame_from_probes"),
    port("h0_4d32", 0x4d32, "character::Anim::frame_4d32"),
    port("h0_4d54", 0x4d54, "character::Anim::frame_4d54"),
    port(
        "read_joystick",
        0x8dfc,
        "character::JoyFlags::read_joystick",
    ),
    // T08: the movement. `game_frame` is T63's, once every routine it calls is ported.
    port("char_update", 0x3e0c, "character::Character::update"),
    port("char_step", 0x3e30, "character::Character::step"),
    port("h0_3ea8", 0x3ea8, "character::Character::idle"),
    port("h0_410c", 0x410c, "character::Character::land_or_fall"),
    port("h0_416c", 0x416c, "character::Character::fall_step"),
    port("h0_4274", 0x4274, "character::Character::rise_step"),
    port("h0_4696", 0x4696, "character::Character::move_x"),
    port("h0_4718", 0x4718, "character::Character::left_pressed"),
    port("h0_47e2", 0x47e2, "character::Character::run_left_step"),
    port("h0_48be", 0x48be, "character::Character::right_pressed"),
    port("h0_497c", 0x497c, "character::Character::run_right_step"),
    port("h0_4b0e", 0x4b0e, "character::Character::up_pressed"),
    port("char_probe", 0x4f84, "character::Character::probe"),
    port(
        "char_probe_flags",
        0x4f90,
        "character::Character::probe_flags",
    ),
    // T09, the camera (level/camera.rs).
    port("camera_update", 0x4d7e, "level::camera::Camera::update"),
    port("h0_4e9a", 0x4e9a, "level::camera::Camera::follow"),
    port("h0_4ea6", 0x4ea6, "level::camera::Camera::follow"),
    port("h0_4eb4", 0x4eb4, "level::camera::Camera::follow"),
    port("h0_4ec2", 0x4ec2, "level::camera::Camera::follow"),
    port("h0_513c", 0x513c, "level::camera::Camera::clamp"),
    port("h0_51fc", 0x51fc, "level::camera::Camera::scroll"),
    port("h0_526a", 0x526a, "level::camera::Camera::scroll"),
    port("h0_52d8", 0x52d8, "level::camera::Camera::scroll"),
    port("h0_5346", 0x5346, "level::camera::Camera::scroll"),
    // T55, the edge redraws (level/redraw.rs).
    port("h0_774", 0x774, "level::redraw::tile_blit"),
    port("h0_7cc", 0x7cc, "level::redraw::shape_copy"),
    port("redraw_edge_a", 0x844, "level::redraw::Edges::redraw_below"),
    port("h0_a06", 0xa06, "level::redraw::bob_below"),
    port("redraw_edge_b", 0xb20, "level::redraw::Edges::redraw_above"),
    port("h0_ce4", 0xce4, "level::redraw::bob_above"),
    port("redraw_edge_c", 0xdf6, "level::redraw::Edges::redraw_left"),
    port("h0_104c", 0x104c, "level::redraw::bob_left"),
    port(
        "redraw_edge_d",
        0x114a,
        "level::redraw::Edges::redraw_right",
    ),
    port("h0_1390", 0x1390, "level::redraw::bob_right"),
    port("h0_3ac2", 0x3ac2, "level::redraw::blitter_wait"),
    port("h0_681c", 0x681c, "level::redraw::CreatureSlots::start"),
    port("h0_6852", 0x6852, "level::redraw::CreatureSlots::fill"),
    port("h0_6882", 0x6882, "level::redraw::AnimatedCells::add"),
    // T65, level_init's first view (level/start.rs).
    port("view_tiles", 0x62e, "level::start::draw_tiles"),
    port("view_tile", 0x672, "level::start::tile_cell"),
    port("view_shapes", 0x6c4, "level::start::draw_shapes"),
    port("view_shape", 0x708, "level::start::shape_cell"),
    // T10, the renderer (render/): the drawing routines of re/translate.toml, then the status bar's
    // and the start-up's copper list pointers, which are outside it.
    port("h0_3ab4", 0x3ab4, "render::playfield::blit_done"),
    port(
        "char_sprite_position",
        0x5462,
        "render::objects::CharSprite::position",
    ),
    port("h0_94ba", 0x94ba, "render::palette::Fade::step"),
    port("h0_94fe", 0x94fe, "render::palette::Fade::all"),
    port("h0_9550", 0x9550, "render::palette::colour"),
    port("h0_9754", 0x9754, "render::playfield::Scroll::from_camera"),
    port(
        "char_sprite_copy",
        0x9b06,
        "render::objects::CharSprite::copy",
    ),
    port("h0_6a96", 0x6a96, "render::status::Status::update"),
    port("h0_6ae8", 0x6ae8, "render::status::Status::score"),
    port("h0_6b70", 0x6b70, "render::status::Status::score_digit"),
    port("h0_6b78", 0x6b78, "render::status::Status::digit"),
    port("h0_6bc0", 0x6bc0, "render::status::Status::countdown"),
    port("h0_6c20", 0x6c20, "render::status::Status::items"),
    port("h0_6c80", 0x6c80, "render::status::Status::lives"),
    port("h0_9a34", 0x9a34, "render::sprite_pointers"),
    port("h0_988a", 0x988a, "render::status_pointers"),
    // T22, taking objects and the creatures' write-back (objects/collectible.rs).
    port(
        "char_object_cells",
        0x2698,
        "objects::MapObjects::object_cells",
    ),
    port("h0_26ec", 0x26ec, "objects::MapObjects::take"),
    port("h0_29ca", 0x29ca, "objects::MapObjects::redraw_cell"),
    port(
        "score_add_routine",
        0x2a6e,
        "objects::MapObjects::add_score",
    ),
    port("h0_2dd6", 0x2dd6, "objects::MapObjects::write_back"),
    port("h0_2e0e", 0x2e0e, "objects::MapObjects::write_back_slot"),
    port(
        "countdown_plus_100",
        0x6a04,
        "objects::MapObjects::add_time",
    ),
    port(
        "items_left_minus_one",
        0x6a4c,
        "objects::MapObjects::count_item",
    ),
];

/// A hunk 0 routine's entry.
const fn port(name: &'static str, offset: u32, port: &'static str) -> Ported {
    Ported {
        name,
        hunk: 0,
        offset,
        port,
    }
}
