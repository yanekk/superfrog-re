//! The intro's main program as the steps it is written in (docs/13 "Scenes"): `intro_main`
//! (h0+$20) calls each scene's four routines in turn (unpack its graphics, set it up and fade it
//! in, play it, fade it out) and, after each scene, looks at `intro_skip`. The routines are short
//! runs of a few library calls; each [`Op`] is one such call, named after it, so a scene here
//! reads as its routine does in the original.

/// One step of the intro's main program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Op {
    /// A routine: its steps, then back.
    Call(&'static [Op]),
    /// `intro_skip` set (fire or the mouse button, seen by the vertical blank): on to the end.
    SkipCheck,
    /// `intro_main`'s end (h0+$2f6): music off, the insert-disk-1 screen and its wait.
    End,

    /// A word or long written, added to or inverted at an address.
    SetW(u32, u16),
    AddW(u32, i16),
    NotW(u32),

    /// COP1LC written: the next frame starts this copper list.
    Cop1lc(u32),
    /// `set_planes`: the bitplane pointer moves at `moves`, `count` planes from `at`, a plane
    /// `row_bytes` × 200 bytes.
    SetPlanes {
        moves: u32,
        at: u32,
        row_bytes: u32,
        count: u32,
    },

    /// `atn_decompress` (h0+$cf5c) from `from` to `to`; `ret` is the call's return address,
    /// which names it (its CPU time is in [`super::scenes::UNPACK_LINES`]).
    Unpack {
        from: u32,
        to: u32,
        ret: u32,
    },
    /// `unpack_copy` (h0+$cf3e): the packed block at `from` copied to `to`, then unpacked there.
    UnpackCopy {
        from: u32,
        to: u32,
        ret: u32,
    },
    /// `load_unpack` (h0+$c3e8): intro file `file` (0..3) loaded and unpacked to `to`.
    Load {
        file: u8,
        to: u32,
        ret: u32,
    },
    /// `load_file` (h0+$c3fa): intro file `file` loaded as it is to `to`.
    LoadRaw {
        file: u8,
        to: u32,
        ret: u32,
    },
    /// `n` longs copied by the CPU.
    CopyLongs {
        from: u32,
        to: u32,
        n: u32,
    },

    /// `buffers_clear` (h0+$8314).
    ClearBuffers,
    /// `h0+$1c` set to a scene's frame table, then `objects_init` (h0+$96d4).
    InitObjects(u32),
    /// An object placed and given an animation (`anim_set` h0+$8680).
    Object {
        obj: u32,
        x: u16,
        y: u16,
        anim: u32,
    },
    /// An object given an animation alone.
    Anim {
        obj: u32,
        anim: u32,
    },
    /// `draw_frames` (h0+$841e): `n` draws, each on the second vertical blank after the last.
    Draw(u16),
    /// `draw_next` (h0+$84ca): one draw on the second vertical blank.
    DrawOnce,

    /// `fade_in` (h0+$b6d8) on record 0 with `speed`: the copper's colours at `list` towards the
    /// `count` colours at `palette`.
    FadeIn {
        list: u32,
        palette: u32,
        count: u16,
        speed: u16,
    },
    /// `fade_out` (h0+$b906) on record 0.
    FadeOut {
        list: u32,
        count: u16,
        speed: u16,
    },
    /// `fade_across` (h0+$b4b4) on record 0.
    FadeAcross {
        from: u32,
        to: u32,
        list: u32,
        count: u16,
        speed: u16,
    },
    /// `fade_wait` (h0+$b4a0) on record 0.
    WaitFade,

    /// `wait_vbl` (h0+$c37e).
    WaitVbl,
    /// `wait_vbl` `n` times.
    WaitVbls(u16),
    /// `wait_fire` (h0+$3b4): up to `n` vertical blanks, or until fire is down.
    WaitOrFire(u16),

    /// `timeline_start` (h0+$b17c): the vertical blank plays this list of sound events.
    Timeline(u32),
    /// `music_start` (h0+$390c2): the P41A module at `module`, samples from `samples`.
    Music {
        module: u32,
        samples: u32,
    },
    /// `music_stop` (h0+$391fc).
    MusicStop,
    /// `music_fade_stop` (h0+$391d6): fade out over up to 100 vertical blanks, then stop.
    MusicFadeStop,
    /// `sfx_play` (h0+$38a16): effect `d0` on channel `d1`.
    Sfx(u16, u16),

    /// Scene specials, each a loop of the scene's own routine (named by its address).
    /// h0+$1db8: object at `obj` moves left 3 a draw until its x is below `until`.
    MoveLeftUntil {
        obj: u32,
        by: u16,
        until: i16,
        once: bool,
    },
    /// h0+$2daa: object `obj` climbs and turns back (x by a growing step, y by one) until x is
    /// `$150` or more.
    Swoop {
        obj: u32,
    },
    /// h0+$5ad2: object `obj` moves left 2 a draw, down 1 every other one, until x < `$cc`.
    Glide {
        obj: u32,
        flag: u32,
    },
}
