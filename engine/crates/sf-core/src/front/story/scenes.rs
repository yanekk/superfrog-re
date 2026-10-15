//! The intro's scenes (docs/13 "Scenes"): `intro_main` (d3_c04 h0+$20) and each scene's routines,
//! step for step as the original calls them ([`Op`]). Addresses are the intro's.

use super::ops::Op::{self, *};
use super::{h0, h1, h2, h3};

/// Copper lists: the scenes' (h2+$db3e), the text pages' (h2+$da5a) and black (h2+$dc92).
pub const SCENE_LIST: u32 = h2(0xdb3e);
pub const TEXT_LIST: u32 = h2(0xda5a);
pub const BLACK_LIST: u32 = h2(0xdc92);
/// The colour moves the fades work on: the scenes' 32, the text pages' 8.
const SCENE_COLOURS: u32 = h2(0xdbe6);
const TEXT_COLOURS: u32 = h2(0xdb02);
/// A scene's palette (after its clean background) and a text page's (after its three planes).
const SCENE_PALETTE: u32 = h3(0x62170);
const TEXT_PALETTE: u32 = h3(0x5dc0);
/// A text page's three planes and where their pointer moves are.
const TEXT_PAGE: u32 = h3(0);
const TEXT_PLANES: u32 = h2(0xdb22);
/// The scenes' planes.
const SCENE_PLANES: u32 = h2(0xdc66);
const BUFFER_A: u32 = h3(0x41dd0);
const BANK: u32 = h3(0x5dd0);
const CLEAN: u32 = h3(0x57590);
/// The packed samples of the music modules start at hunk 2's start.
const SAMPLES: u32 = h2(0);

/// An object, by its address (h0+$8bb0, $8c60, ... $9600: sixteen, `$b0` bytes apart).
const fn obj(at: u32) -> u32 {
    h0(at)
}

/// A text page (h0+$4ca and its kind): unpacked, faded in, shown.
const fn text_in(packed: u32, ret: u32) -> [Op; 4] {
    [
        Unpack {
            from: h0(packed),
            to: TEXT_PAGE,
            ret: h0(ret),
        },
        FadeIn {
            list: TEXT_COLOURS,
            palette: TEXT_PALETTE,
            count: 8,
            speed: 1,
        },
        WaitVbl,
        Cop1lc(TEXT_LIST),
    ]
}

/// `text_out` (h0+$65a).
static TEXT_OUT: &[Op] = &[
    Cop1lc(TEXT_LIST),
    WaitFade,
    FadeOut {
        list: TEXT_COLOURS,
        count: 8,
        speed: 1,
    },
    WaitFade,
    Cop1lc(BLACK_LIST),
];

/// A scene's set-up (h0+$7ae and its kind): buffers cleared, planes, objects, faded in.
const fn scene_in(table: u32) -> [Op; 6] {
    [
        ClearBuffers,
        SetPlanes {
            moves: SCENE_PLANES,
            at: BUFFER_A,
            row_bytes: 0x2c,
            count: 5,
        },
        InitObjects(h0(table)),
        FadeIn {
            list: SCENE_COLOURS,
            palette: SCENE_PALETTE,
            count: 0x20,
            speed: 1,
        },
        WaitVbl,
        Cop1lc(SCENE_LIST),
    ]
}

/// A scene's fade-out at `speed`, without a wait (h0+$1ed2 and its kind).
const fn scene_out(speed: u16) -> [Op; 1] {
    [FadeOut {
        list: SCENE_COLOURS,
        count: 0x20,
        speed,
    }]
}

/// A scene's fade-out at `speed` with its wait and the black list.
const fn scene_out_wait(speed: u16) -> [Op; 3] {
    [
        FadeOut {
            list: SCENE_COLOURS,
            count: 0x20,
            speed,
        },
        WaitFade,
        Cop1lc(BLACK_LIST),
    ]
}

/// h0+$808, $11b8 and $5264: at speed 1.
static SCENE_OUT_WAIT: &[Op] = &scene_out_wait(1);
/// h0+$7ee4, the last scene's: at speed 2.
static SCENE_OUT_WAIT_LAST: &[Op] = &scene_out_wait(2);

const fn music(module: u32) -> Op {
    Music {
        module: h2(module),
        samples: SAMPLES,
    }
}

// --- Scene 1: the prince and princess (h0+$4ca .. $808). ---

static TEXT_1: &[Op] = &{
    let t = text_in(0x320b8, 0x4ec);
    [
        SetPlanes {
            moves: TEXT_PLANES,
            at: TEXT_PAGE,
            row_bytes: 0x28,
            count: 3,
        },
        t[0],
        t[1],
        t[2],
        t[3],
    ]
};
static LOAD_1: &[Op] = &[
    Load {
        file: 0,
        to: BANK,
        ret: h0(0x79a),
    },
    UnpackCopy {
        from: h0(0xd0be),
        to: CLEAN,
        ret: h0(0x7ac),
    },
];
static IN_1: &[Op] = &scene_in(0x97c);
static PLAY_1: &[Op] = &[
    Object {
        obj: obj(0x8bb0),
        x: 0x18,
        y: 0x8c,
        anim: h0(0x844),
    },
    Object {
        obj: obj(0x8c60),
        x: 0x88,
        y: 0x8c,
        anim: h0(0x844),
    },
    Object {
        obj: obj(0x8d10),
        x: 0xf8,
        y: 0x8c,
        anim: h0(0x844),
    },
    Object {
        obj: obj(0x8dc0),
        x: 0x5c,
        y: 0x9c,
        anim: h0(0x888),
    },
    Object {
        obj: obj(0x8e70),
        x: 0xc4,
        y: 0x9c,
        anim: h0(0x888),
    },
    Object {
        obj: obj(0x8f20),
        x: 0xb2,
        y: 0x0e,
        anim: h0(0x8cc),
    },
    Object {
        obj: obj(0x8fd0),
        x: 0x7c,
        y: 0x06,
        anim: h0(0x8f0),
    },
    Draw(0x15e),
    MusicStop,
];

// --- Scene 2: the witch (h0+$522 .. $11b8). ---

static TEXT_2: &[Op] = &text_in(0x33578, 0x534);
static LOAD_2: &[Op] = &[
    Load {
        file: 1,
        to: BANK,
        ret: h0(0x114a),
    },
    UnpackCopy {
        from: h0(0x10e78),
        to: CLEAN,
        ret: h0(0x115c),
    },
];
static IN_2: &[Op] = &scene_in(0x1490);
static PLAY_2: &[Op] = &[
    Object {
        obj: obj(0x8bb0),
        x: 0,
        y: 0,
        anim: h0(0x135c),
    },
    Object {
        obj: obj(0x8c60),
        x: 0x6c,
        y: 0x5b,
        anim: h0(0x12d8),
    },
    Object {
        obj: obj(0x8d10),
        x: 0xdf,
        y: 0x73,
        anim: h0(0x11f4),
    },
    Draw(0xfa),
    MusicStop,
];

// --- Scene 3: the witch flies off (h0+$56a .. $1ed2). ---

static TEXT_3: &[Op] = &text_in(0x34a96, 0x57c);
static LOAD_3: &[Op] = &[
    LoadRaw {
        file: 2,
        to: h1(0),
        ret: h0(0x1df6),
    },
    UnpackCopy {
        from: h1(0),
        to: BANK,
        ret: h0(0x1e64),
    },
    UnpackCopy {
        from: h0(0x16c1e),
        to: CLEAN,
        ret: h0(0x1e76),
    },
];
static IN_3: &[Op] = &scene_in(0x1f4c);
static PLAY_3: &[Op] = &[
    Object {
        obj: obj(0x8bb0),
        x: 0x150,
        y: 0x20,
        anim: h0(0x1ef8),
    },
    Timeline(h0(0xb1fa)),
    MoveLeftUntil {
        obj: obj(0x8bb0),
        by: 3,
        until: 0x78,
        once: true,
    },
    SetW(h0(0x8bca), 0),
    Draw(0x1e),
];
static OUT_3: &[Op] = &scene_out(2);

/// The scenes that unpack a bank from the raw file in hunk 1 and a background from hunk 0.
const fn unpack2(bank: u32, bank_ret: u32, clean: u32, clean_ret: u32) -> [Op; 2] {
    [
        Unpack {
            from: h1(bank),
            to: BANK,
            ret: h0(bank_ret),
        },
        UnpackCopy {
            from: h0(clean),
            to: CLEAN,
            ret: h0(clean_ret),
        },
    ]
}

// --- Scene 4 (h0+$25ec .. $266c). ---

static LOAD_4: &[Op] = &unpack2(0x1bbc, 0x25fe, 0x18cca, 0x2610);
static IN_4: &[Op] = &scene_in(0x26fc);
static PLAY_4: &[Op] = &[
    Object {
        obj: obj(0x8bb0),
        x: 0x90,
        y: 0x40,
        anim: h0(0x2698),
    },
    Timeline(h0(0xb20a)),
    Draw(0x32),
];
static OUT_4: &[Op] = &[FadeAcross {
    from: h0(0x844a),
    to: h0(0x848a),
    list: SCENE_COLOURS,
    count: 0x20,
    speed: 3,
}];

// --- Scene 5 (h0+$2dea .. $2e6a). ---

static LOAD_5: &[Op] = &unpack2(0x32c8, 0x2dfc, 0x1cc10, 0x2e0e);
static IN_5: &[Op] = &scene_in(0x2ef8);
static PLAY_5: &[Op] = &[
    Object {
        obj: obj(0x8bb0),
        x: 0x40,
        y: 0x20,
        anim: h0(0x2e90),
    },
    Object {
        obj: obj(0x8c60),
        x: 0x50,
        y: 0x28,
        anim: h0(0x2eec),
    },
    Timeline(h0(0xb232)),
    Draw(0x32),
    Swoop { obj: obj(0x8c60) },
];
static OUT_2: &[Op] = &scene_out(2);

// --- Scene 6 (h0+$35dc .. $365c). ---

static LOAD_6: &[Op] = &unpack2(0xbb1c, 0x35ee, 0x1ef98, 0x3600);
static IN_6: &[Op] = &scene_in(0x374e);
static PLAY_6: &[Op] = &[
    Object {
        obj: obj(0x8bb0),
        x: 0x50,
        y: 0,
        anim: h0(0x3682),
    },
    Object {
        obj: obj(0x8c60),
        x: 0xc0,
        y: 0,
        anim: h0(0x36ce),
    },
    Object {
        obj: obj(0x8d10),
        x: 0xa0,
        y: 0x18,
        anim: h0(0x36fa),
    },
    Timeline(h0(0xb25a)),
    Draw(0x96),
];

// --- Scene 7 (h0+$3eb6 .. $3f36). ---

static LOAD_7: &[Op] = &unpack2(0x140b4, 0x3ec8, 0x16c1e, 0x3eda);
static IN_7: &[Op] = &scene_in(0x4148);
static PLAY_7: &[Op] = &[
    Object {
        obj: obj(0x8d10),
        x: 0x64,
        y: 0x40,
        anim: h0(0x3f5c),
    },
    Object {
        obj: obj(0x8c60),
        x: 0x90,
        y: 0x1a,
        anim: h0(0x3f68),
    },
    Object {
        obj: obj(0x8bb0),
        x: 0,
        y: 0x64,
        anim: h0(0x3fa4),
    },
    Timeline(h0(0xb28e)),
    Draw(0x78),
];

// --- Scene 8 (h0+$48b4 .. $4922): the same bank, another background. ---

static LOAD_8: &[Op] = &[UnpackCopy {
    from: h0(0x1ef98),
    to: CLEAN,
    ret: h0(0x48c6),
}];
static IN_8: &[Op] = &scene_in(0x4a5c);
static PLAY_8: &[Op] = &[
    Object {
        obj: obj(0x8bb0),
        x: 0x20,
        y: 0x10,
        anim: h0(0x4948),
    },
    Timeline(h0(0xb29e)),
    Draw(0x82),
];

// --- Scene 9 (h0+$51e4 .. $5264). ---

static LOAD_9: &[Op] = &unpack2(0x1cb60, 0x51f6, 0x16c1e, 0x5208);
static IN_9: &[Op] = &scene_in(0x5360);
static PLAY_9: &[Op] = &[
    Object {
        obj: obj(0x8bb0),
        x: 0x80,
        y: 0x20,
        anim: h0(0x52a0),
    },
    Object {
        obj: obj(0x8c60),
        x: 0x10,
        y: 0x7e,
        anim: h0(0x5324),
    },
    Timeline(h0(0xb2c6)),
    Draw(0x6c),
];

// --- Scene 10: the frog's spell (h0+$5b2 .. $5d90). ---

static TEXT_4: &[Op] = &text_in(0x35fa2, 0x5c4);
static LOAD_10: &[Op] = &[
    LoadRaw {
        file: 3,
        to: h1(0),
        ret: h0(0x5cd8),
    },
    UnpackCopy {
        from: h1(0),
        to: BANK,
        ret: h0(0x5d22),
    },
    UnpackCopy {
        from: h0(0x2197c),
        to: CLEAN,
        ret: h0(0x5d34),
    },
];
static IN_10: &[Op] = &scene_in(0x5ff2);
static PLAY_10: &[Op] = &[
    Object {
        obj: obj(0x8bb0),
        x: 0x53,
        y: 0x66,
        anim: h0(0x5efa),
    },
    Object {
        obj: obj(0x8c60),
        x: 0x48,
        y: 0x6e,
        anim: h0(0x5dd6),
    },
    Object {
        obj: obj(0x9600),
        x: 0x150,
        y: 0x80,
        anim: h0(0x5d92),
    },
    Timeline(h0(0xb2e2)),
    Draw(0x3c),
    Glide {
        obj: obj(0x9600),
        flag: h0(0x5cbe),
    },
    SetW(obj(0x9600), 0x190),
    Draw(0x1e),
    SetW(h0(0xb436), 1),
    FadeOut {
        list: SCENE_COLOURS,
        count: 0x20,
        speed: 2,
    },
    UnpackCopy {
        from: h0(0x2e57c),
        to: CLEAN,
        ret: h0(0x5b4c),
    },
    WaitFade,
    SetPlanes {
        moves: SCENE_PLANES,
        at: CLEAN + 2,
        row_bytes: 0x2c,
        count: 5,
    },
    FadeIn {
        list: SCENE_COLOURS,
        palette: SCENE_PALETTE,
        count: 0x20,
        speed: 1,
    },
    WaitFade,
    Timeline(h0(0xb2fe)),
    WaitVbls(0x96),
    FadeOut {
        list: SCENE_COLOURS,
        count: 0x20,
        speed: 2,
    },
    WaitFade,
    UnpackCopy {
        from: h0(0x2197c),
        to: CLEAN,
        ret: h0(0x5bf6),
    },
    SetPlanes {
        moves: SCENE_PLANES,
        at: BUFFER_A + 2,
        row_bytes: 0x2c,
        count: 5,
    },
    FadeIn {
        list: SCENE_COLOURS,
        palette: SCENE_PALETTE,
        count: 0x20,
        speed: 1,
    },
    SetW(h0(0xb436), 0),
    WaitFade,
    Timeline(h0(0xb30e)),
    Draw(0x48),
    Object {
        obj: obj(0x8bb0),
        x: 0x53,
        y: 0x66,
        anim: h0(0x5f3e),
    },
    Draw(0x78),
    Object {
        obj: obj(0x8bb0),
        x: 0,
        y: 0x30,
        anim: h0(0x5f7a),
    },
    Object {
        obj: obj(0x8c60),
        x: 0,
        y: 0x20,
        anim: h0(0x5fbe),
    },
    Draw(0x32),
    // (h0+$5d90, called after this, is an empty routine.)
];

// --- Scene 11 (h0+$697a .. $69a0): the same buffers, new objects. ---

static LOAD_11: &[Op] = &[Unpack {
    from: h1(0x701c),
    to: BANK,
    ret: h0(0x698c),
}];
static IN_11: &[Op] = &[InitObjects(h0(0x6ace))];
static PLAY_11: &[Op] = &[
    Object {
        obj: obj(0x8bb0),
        x: 0x60,
        y: 0x66,
        anim: h0(0x6a16),
    },
    Object {
        obj: obj(0x8c60),
        x: 0x60,
        y: 0x66,
        anim: h0(0x69c6),
    },
    Object {
        obj: obj(0x9600),
        x: 0x60,
        y: 0x66,
        anim: h0(0x69ea),
    },
    Timeline(h0(0xb342)),
    MusicStop,
    music(0xcb98),
    Sfx(9, 0),
    Draw(0x46),
    SetW(obj(0x8bb0), 0x190),
    SetW(obj(0x9600), 0x190),
    Object {
        obj: obj(0x8c60),
        x: 0x60,
        y: 0x66,
        anim: h0(0x6a4a),
    },
    Draw(6),
    Object {
        obj: obj(0x8bb0),
        x: 0x60,
        y: 0x66,
        anim: h0(0x6a6e),
    },
    Draw(0x64),
    Object {
        obj: obj(0x8bb0),
        x: 0x60,
        y: 0x66,
        anim: h0(0x6aa2),
    },
    SetW(obj(0x9600), 0x190),
    SetW(obj(0x8c60), 0x190),
    Draw(0x3c),
];

// --- Scene 12 (h0+$736e .. $73dc). ---

static LOAD_12: &[Op] = &[UnpackCopy {
    from: h0(0x25950),
    to: CLEAN,
    ret: h0(0x7380),
}];
static IN_12: &[Op] = &scene_in(0x6ace);
static PLAY_12: &[Op] = &[
    Object {
        obj: obj(0x9600),
        x: 0xb8,
        y: 0x3a,
        anim: h0(0x7402),
    },
    Object {
        obj: obj(0x9550),
        x: 0x9a,
        y: 0x56,
        anim: h0(0x7446),
    },
    Object {
        obj: obj(0x94a0),
        x: 0x76,
        y: 0x10,
        anim: h0(0x7492),
    },
    Timeline(h0(0xb352)),
    Draw(0xf0),
];

// --- Scene 13 (h0+$7a00 .. $7a80). ---

static LOAD_13: &[Op] = &unpack2(0x1048c, 0x7a12, 0x2197c, 0x7a24);
static IN_13: &[Op] = &scene_in(0x7b46);
static PLAY_13: &[Op] = &[
    Object {
        obj: obj(0x8bb0),
        x: 0x78,
        y: 0xfffa,
        anim: h0(0x7ad2),
    },
    Object {
        obj: obj(0x8c60),
        x: 0x3e,
        y: 0x56,
        anim: h0(0x7aa6),
    },
    Timeline(h0(0xb386)),
    Draw(0x3f),
    SetW(obj(0x8c60), 0x4b),
    SetW(obj(0x8c60) + 2, 0x56),
    Draw(1),
    SetW(obj(0x8c60), 0x5c),
    SetW(obj(0x8c60) + 2, 0x56),
    Draw(0x28),
    Call(PLAY_13_END),
];
static PLAY_13_END: &[Op] = &[SetW(obj(0x8c60), 0x190), Draw(0x13)];

// --- Scene 14 (h0+$7c82 .. $7cf0). ---

static LOAD_14: &[Op] = &[UnpackCopy {
    from: h0(0x2aaca),
    to: CLEAN,
    ret: h0(0x7c94),
}];
static IN_14: &[Op] = &scene_in(0x7b46);
static PLAY_14: &[Op] = &[
    Object {
        obj: obj(0x8bb0),
        x: 0x60,
        y: 1,
        anim: h0(0x7d16),
    },
    Timeline(h0(0xb3a2)),
    Draw(0x16),
];

// --- Scene 15 (h0+$7e64 .. $7ee4). ---

static LOAD_15: &[Op] = &unpack2(0x1a0cc, 0x7e76, 0x2197c, 0x7e88);
static IN_15: &[Op] = &scene_in(0x8188);
static PLAY_15: &[Op] = &[
    Object {
        obj: obj(0x8bb0),
        x: 0xf4,
        y: 0x6d,
        anim: h0(0x7f20),
    },
    Object {
        obj: obj(0x8c60),
        x: 0x48,
        y: 1,
        anim: h0(0x7f2c),
    },
    Timeline(h0(0xb3b2)),
    Draw(0x64),
    MusicStop,
    music(0xcf10),
    Sfx(9, 0),
    Draw(0x44),
    MoveLeftUntil {
        obj: obj(0x8c60),
        by: 3,
        until: -0x60,
        once: false,
    },
    Object {
        obj: obj(0x8c60),
        x: 0x150,
        y: 0x56,
        anim: h0(0x80f8),
    },
    MoveLeftUntil {
        obj: obj(0x8c60),
        by: 3,
        until: 0x8c,
        once: false,
    },
    Anim {
        obj: obj(0x8c60),
        anim: h0(0x812c),
    },
    Draw(0x3c),
    Anim {
        obj: obj(0x8c60),
        anim: h0(0x80f8),
    },
    MoveLeftUntil {
        obj: obj(0x8c60),
        by: 3,
        until: -0x40,
        once: false,
    },
];

/// `intro_main` (h0+$20) from its fourth step: each scene, `intro_skip` looked at after each.
pub static MAIN: &[Op] = &[
    Call(TEXT_1),
    Call(LOAD_1),
    WaitOrFire(400),
    Call(TEXT_OUT),
    Call(IN_1),
    music(0xc51e),
    Call(PLAY_1),
    Call(SCENE_OUT_WAIT),
    SkipCheck,
    Call(TEXT_2),
    Call(LOAD_2),
    WaitOrFire(400),
    Call(TEXT_OUT),
    Call(IN_2),
    MusicStop,
    music(0xc898),
    Call(PLAY_2),
    Call(SCENE_OUT_WAIT),
    SkipCheck,
    Call(TEXT_3),
    music(0xd0c6),
    Call(LOAD_3),
    WaitOrFire(400),
    Call(TEXT_OUT),
    MusicFadeStop,
    Call(IN_3),
    Call(PLAY_3),
    Call(OUT_3),
    SkipCheck,
    Call(LOAD_4),
    Call(IN_4),
    Call(PLAY_4),
    Call(OUT_4),
    SkipCheck,
    Call(LOAD_5),
    Call(IN_5),
    Call(PLAY_5),
    Call(OUT_2),
    SkipCheck,
    Call(LOAD_6),
    Call(IN_6),
    Call(PLAY_6),
    Call(OUT_2),
    SkipCheck,
    Call(LOAD_7),
    Call(IN_7),
    Call(PLAY_7),
    Call(OUT_2),
    SkipCheck,
    Call(LOAD_8),
    Call(IN_8),
    Call(PLAY_8),
    Call(OUT_2),
    SkipCheck,
    Call(LOAD_9),
    Call(IN_9),
    Call(PLAY_9),
    Call(SCENE_OUT_WAIT),
    SkipCheck,
    Call(TEXT_4),
    music(0xd590),
    Call(LOAD_10),
    Call(TEXT_OUT),
    MusicFadeStop,
    Call(IN_10),
    MusicStop,
    music(0xca26),
    Sfx(9, 0),
    Call(PLAY_10),
    SkipCheck,
    Call(LOAD_11),
    Call(IN_11),
    Call(PLAY_11),
    Call(OUT_2),
    SkipCheck,
    Call(LOAD_12),
    Call(IN_12),
    Call(PLAY_12),
    Call(OUT_2),
    SkipCheck,
    Call(LOAD_13),
    MusicStop,
    music(0xcd6a),
    Sfx(9, 0),
    WaitVbls(0x14),
    Call(IN_13),
    Call(PLAY_13),
    Call(OUT_2),
    SkipCheck,
    Call(LOAD_14),
    Call(IN_14),
    Call(PLAY_14),
    Call(OUT_2),
    SkipCheck,
    Call(LOAD_15),
    Call(IN_15),
    Call(PLAY_15),
    Call(SCENE_OUT_WAIT_LAST),
    // h0+$2ec: `intro_skip` set or not (h0+$3a6 shows the black list again), the same end.
    Call(END),
];

/// How many frames after the one it starts in each unpack ends, by its call's return address,
/// and the line it ends on in that frame (measured on the oracle, untouched run; the line is the
/// one after the last the original's CPU was seen inside it, T56).
pub static UNPACK_FRAMES: &[(u32, u32, u64)] = &[
    (h0(0x4ec), 13, 105),
    (h0(0x534), 12, 279),
    (h0(0x57c), 12, 267),
    (h0(0x5c4), 13, 146),
    (h0(0x624), 13, 18),
    (h0(0x7ac), 37, 192),
    (h0(0x115c), 53, 276),
    (h0(0x1e64), 58, 310),
    (h0(0x1e76), 26, 197),
    (h0(0x25fe), 62, 250),
    (h0(0x2610), 42, 169),
    (h0(0x2dfc), 99, 310),
    (h0(0x2e0e), 29, 173),
    (h0(0x35ee), 118, 305),
    (h0(0x3600), 22, 88),
    (h0(0x3ec8), 122, 166),
    (h0(0x3eda), 29, 55),
    (h0(0x48c6), 22, 217),
    (h0(0x51f6), 85, 293),
    (h0(0x5208), 29, 190),
    (h0(0x5b4c), 42, 147),
    (h0(0x5bf6), 38, 51),
    (h0(0x5d22), 103, 79),
    (h0(0x5d34), 35, 101),
    (h0(0x698c), 125, 19),
    (h0(0x7380), 52, 128),
    (h0(0x7a12), 123, 21),
    (h0(0x7a24), 38, 85),
    (h0(0x7c94), 43, 37),
    (h0(0x7e76), 117, 80),
    (h0(0x7e88), 38, 164),
];

/// The line the unpack whose call returns to `ret` ends on in its last frame.
pub fn unpack_end_line(ret: u32) -> u64 {
    UNPACK_FRAMES
        .iter()
        .find(|(r, ..)| *r == ret)
        .map_or(0, |(.., l)| *l)
}

/// The frames the unpack whose call returns to `ret` takes.
pub fn unpack_frames(ret: u32) -> u64 {
    UNPACK_FRAMES
        .iter()
        .find(|(r, ..)| *r == ret)
        .map_or(0, |(_, n, _)| *n as u64)
}

/// `intro_main`'s end (h0+$2f6): the music stopped, the insert-disk-1 page (h0+$5fa) shown, then
/// `intro_wait_disk1` (h0+$c38e: fire down, disk 1's id sector read, fire up), `text_out`, and
/// disk 1's boot loaded and started (h0+$314: the game starts, [`Op::End`]).
pub static END: &[Op] = &[
    MusicStop,
    CopyLongs {
        from: h0(0x37644),
        to: TEXT_PAGE,
        n: 0x5dc,
    },
    Unpack {
        from: TEXT_PAGE,
        to: TEXT_PAGE,
        ret: h0(0x624),
    },
    FadeIn {
        list: TEXT_COLOURS,
        palette: TEXT_PALETTE,
        count: 8,
        speed: 1,
    },
    WaitVbl,
    Cop1lc(TEXT_LIST),
    WaitFire(true),
    ReadDiskId { ret: h0(0xc3c2) },
    WaitFire(false),
    Call(TEXT_OUT),
    End,
];
