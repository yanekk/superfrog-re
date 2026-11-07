//! The `objects` generators T38 keeps (plans/forest-dumb-level-1 T38, DESIGN §4.3), on the main
//! program from the owner's disks.
//!
//! As `gen_camera.rs`: the rig has no instruction trace, so each routine has a witness, a literal
//! model written from the flow listing that counts which way each conditional branch went. Every
//! witness must match the original on all 2 000 of its generator's states (registers, bytes changed,
//! custom-chip writes in order); on a state where they agree, the witness took the original's
//! branches. A branch is named by its instruction's address, `h0+$x` in the listing.
//!
//! Five branches are taken one way only, by the rig's design, and are named in [`ONE_WAY`].

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use fd_conform::generators::{self, Gen, h, objects::*};
use fd_conform::judge::{self, Cpu};
use fd_conform::routine::{Rig, RigReport};

const CASES: usize = 2000;

thread_local! {
    /// Per branch address: how many times it went each way, [not taken, taken].
    static COV: RefCell<BTreeMap<u32, [usize; 2]>> =
        const { RefCell::new(BTreeMap::new()) };
}

/// Record that the branch at `h0+$at` was `taken` or not, and return `taken`.
fn br(at: u32, taken: bool) -> bool {
    COV.with(|c| c.borrow_mut().entry(at).or_default()[taken as usize] += 1);
    taken
}

/// Branches no generated state can take one of the ways, with why.
const ONE_WAY: &[(u32, &str)] = &[
    // `cmp.b $dff006,d1; beq`: the rig's VHPOSR steps on every read, so two reads never match.
    (0x265e, "VHPOSR read twice never matches on the rig"),
    (0x291c, "VHPOSR read twice never matches on the rig"),
    (0x3af2, "VHPOSR read twice never matches on the rig"),
    (0x3da4, "VHPOSR read twice never matches on the rig"),
    // `btst #6,$dff002; bne`: BLTBUSY reads 0; a state setting it would wait for ever (a hang).
    (0x2a1c, "BLTBUSY set would never clear: the wait hangs"),
];

/// N after `cmp` of `b` from `a` (word).
fn n16(a: u16, b: u16) -> bool {
    a.wrapping_sub(b) & 0x8000 != 0
}

/// N after `cmpi.b #imm` on `x`.
fn n8(x: u8, imm: u8) -> bool {
    x.wrapping_sub(imm) & 0x80 != 0
}

/// `r` with its low word `w`.
fn lo_w(r: u32, w: u16) -> u32 {
    (r & 0xffff_0000) | w as u32
}

/// `r` with its low byte `b`.
fn lo_b(r: u32, b: u8) -> u32 {
    (r & 0xffff_ff00) | b as u32
}

/// ABCD on valid BCD: `dst + src + x`, and the carry.
fn abcd(src: u8, dst: u8, x: bool) -> (u8, bool) {
    let mut lo = (src & 0xf) as u16 + (dst & 0xf) as u16 + x as u16;
    if lo > 9 {
        lo += 6;
    }
    let mut r = (src & 0xf0) as u16 + (dst & 0xf0) as u16 + lo;
    let c = r > 0x99;
    if c {
        r += 0x60;
    }
    (r as u8, c)
}

/// SBCD on valid BCD: `dst - src - x`, and the borrow.
fn sbcd(src: u8, dst: u8, x: bool) -> (u8, bool) {
    let lo = (dst & 0xf) as i16 - (src & 0xf) as i16 - x as i16;
    let mut r = dst as i16 - src as i16 - x as i16;
    if lo < 0 {
        r -= 6;
    }
    let c = r < 0;
    if c {
        r -= 0x60;
    }
    (r as u8, c)
}

const VHPOSR: u32 = 0xdf_f006;
const DMACON: u32 = 0xdf_f096;

/// `moveq #n,d0`, then `n + 1` times: a VHPOSR byte into `d1`, wait while it reads the same.
fn beam_wait(cpu: &mut Cpu, n: u32, beq_at: u32, dbra_at: u32) {
    cpu.d[0] = n;
    loop {
        let b = cpu.mem.rb(VHPOSR);
        cpu.d[1] = lo_b(cpu.d[1], b);
        while br(beq_at, cpu.mem.rb(VHPOSR) == cpu.d[1] as u8) {}
        let w = (cpu.d[0] as u16).wrapping_sub(1);
        cpu.d[0] = lo_w(cpu.d[0], w);
        if !br(dbra_at, w != 0xffff) {
            return;
        }
    }
}

/// A sample started on an audio channel at `regs` (`$dff0a0`-style), as the inline sound code does.
fn sample(
    cpu: &mut Cpu,
    bit: u16,
    regs: u32,
    ptr: u32,
    len: u16,
    per: u16,
    waits: (u32, u32, u32),
) {
    cpu.mem.ww(DMACON, bit);
    beam_wait(cpu, waits.0, waits.1, waits.2);
    cpu.mem.wl(regs, ptr);
    cpu.mem.ww(regs + 4, len);
    cpu.mem.ww(regs + 6, per);
    cpu.mem.ww(regs + 8, 0x3f);
    cpu.mem.ww(DMACON, 0x8200 | bit);
}

/// `h0_2648`: channel 2's sample.
fn sound_2648(cpu: &mut Cpu) {
    sample(
        cpu,
        4,
        0xdf_f0c0,
        h(4, 0x1_2acc),
        0x7cb,
        0x165,
        (3, 0x265e, 0x2662),
    );
}

/// `h0_3adc`: channel 2's other sample.
fn sound_3adc(cpu: &mut Cpu) {
    sample(
        cpu,
        4,
        0xdf_f0c0,
        h(4, 0x1_0df4),
        0xe6c,
        0x165,
        (3, 0x3af2, 0x3af6),
    );
}

/// `h0_24ce`.
fn map_code(cpu: &mut Cpu) {
    if !br(0x24d4, cpu.mem.rb(h(0, FLAG_4ED0)) != 0) {
        return;
    }
    if !br(0x24e0, cpu.mem.rw(h(0, FLAG_4B62)) == 0) {
        return;
    }
    let (x, y) = (cpu.mem.rl(h(0, CHAR_X)), cpu.mem.rl(h(0, CHAR_Y)));
    cpu.d[0] = ((x.wrapping_add(0xe) >> 4) & 0xffff) * 0x60;
    cpu.d[1] = (y.wrapping_add(0x23) >> 4).wrapping_add(cpu.d[0]);
    cpu.a[0] = h(0, MAP);
    let b = cpu.mem.rb(cpu.a[0].wrapping_add(cpu.d[1]));
    cpu.d[0] = lo_b(cpu.d[0], b);
    // The code, the branch testing it, the x offset and the `h0+$21be` word it sets.
    for (code, at, dx, w) in [
        (0x10, 0x2516, 0xe, 0),
        (0x11, 0x251e, 0xffff_fffe, 0),
        (0x12, 0x2526, 0xe, 1),
        (0x13, 0x252e, 0xffff_fffe, 1),
    ] {
        if br(at, b == code) {
            cpu.d[0] = x.wrapping_add(dx);
            cpu.d[1] = y.wrapping_add(0x23);
            cpu.mem.wb(h(0, 0x4ecf), 1);
            cpu.mem.ww(h(0, 0x41b6), 1);
            cpu.mem.wl(h(0, 0x21b4), cpu.d[0]);
            cpu.mem.wl(h(0, 0x21b8), cpu.d[1]);
            cpu.mem.ww(h(0, 0x21be), w);
            cpu.mem.ww(h(0, 0x21bc), 1);
            return;
        }
    }
}

/// `items_left_minus_one`, entered with X clear.
fn items_left(cpu: &mut Cpu) {
    let at = h(0, ITEMS_LEFT);
    if !br(0x6a52, cpu.mem.rw(at) != 0) {
        return;
    }
    cpu.d[0] = 0;
    cpu.d[1] = lo_b(cpu.d[1], cpu.mem.rb(at + 1));
    cpu.d[2] = lo_b(cpu.d[2], cpu.mem.rb(at));
    cpu.d[4] = 1;
    cpu.d[5] = 0;
    let (r, x) = sbcd(1, cpu.d[1] as u8, false);
    cpu.d[1] = lo_b(cpu.d[1], r);
    let (r, _) = sbcd(0, cpu.d[2] as u8, x);
    cpu.d[2] = lo_b(cpu.d[2], r);
    cpu.mem.wb(at + 1, cpu.d[1] as u8);
    cpu.mem.wb(at, cpu.d[2] as u8);
}

/// `score_add_routine`: `score_add` added to the score, four BCD bytes, then cleared.
fn score_add(cpu: &mut Cpu) {
    cpu.a[2] = h(0, SCORE_ADD);
    cpu.a[3] = h(0, SCORE_ADD + 4);
    cpu.d[7] = lo_b(cpu.d[7], 0);
    let mut x = false;
    for _ in 0..4 {
        cpu.a[3] -= 1;
        cpu.a[2] -= 1;
        let (r, c) = abcd(cpu.mem.rb(cpu.a[3]), cpu.mem.rb(cpu.a[2]), x);
        cpu.mem.wb(cpu.a[2], r);
        x = c;
    }
    cpu.mem.wl(h(0, SCORE_ADD), 0);
}

/// `countdown_plus_100` with its `$899` limit as a parameter, entered with X clear.
fn countdown_to(cpu: &mut Cpu, limit: u16) {
    let at = h(0, COUNTDOWN);
    if !br(0x6a0c, n16(cpu.mem.rw(at), limit)) {
        cpu.mem.ww(at, 0x999);
        return;
    }
    cpu.d[0] = lo_b(cpu.d[0], cpu.mem.rb(at + 2));
    cpu.d[1] = lo_b(cpu.d[1], cpu.mem.rb(at + 1));
    cpu.d[2] = lo_b(cpu.d[2], cpu.mem.rb(at));
    cpu.d[3] = 0;
    cpu.d[4] = 0;
    cpu.d[5] = 1;
    let (r, x) = abcd(0, cpu.d[0] as u8, false);
    cpu.d[0] = lo_b(cpu.d[0], r);
    let (r, x) = abcd(0, cpu.d[1] as u8, x);
    cpu.d[1] = lo_b(cpu.d[1], r);
    let (r, _) = abcd(1, cpu.d[2] as u8, x);
    cpu.d[2] = lo_b(cpu.d[2], r);
    cpu.mem.wb(at + 2, cpu.d[0] as u8);
    cpu.mem.wb(at + 1, cpu.d[1] as u8);
    cpu.mem.wb(at, cpu.d[2] as u8);
}

fn countdown(cpu: &mut Cpu) {
    countdown_to(cpu, 0x899);
}

fn countdown_wrong(cpu: &mut Cpu) {
    countdown_to(cpu, 0x898);
}

/// `h0_29ca`: the item's 16x80 tile blitted from the tile set onto the screen at `d0`/`d1`; returns
/// `d0` from `h0+$29c6`.
fn item_blit(cpu: &mut Cpu) {
    let d = &mut cpu.d;
    d[3] = d[1];
    d[0] = lo_w(d[0], (d[0] as u16) >> 3);
    d[0] = lo_b(d[0], d[0] as u8 & 0xfe);
    d[2] = lo_w(d[2], d[0] as u16);
    d[1] = lo_w(d[1], d[1] as u16 & 0xf0);
    d[1] = (d[1] & 0xffff) * 0xe6;
    cpu.a[0] = h(3, 0x398).wrapping_add(d[0]).wrapping_add(d[1]);
    d[2] = (d[2] & 0xffff) * 0x60;
    d[3] = lo_w(d[3], (d[3] as u16) >> 3);
    d[3] = lo_b(d[3], d[3] as u8 & 0xfe);
    cpu.a[1] = h(0, TILES).wrapping_add(d[2]).wrapping_add(d[3]);
    let t = (cpu.mem.rw(cpu.a[1]) as u32) << 2;
    cpu.d[7] = cpu.mem.rl(h(0, TILE_TABLE).wrapping_add(t));
    cpu.a[5] = h(0, TILE_TABLE);
    cpu.a[2] = cpu.a[0];
    cpu.a[3] = h(3, 0x1_3540).wrapping_add(cpu.d[7]);
    br(0x2a1c, cpu.mem.rb(0xdf_f002) & 0x40 != 0);
    cpu.mem.wl(0xdf_f054, cpu.a[2]);
    cpu.mem.wl(0xdf_f050, cpu.a[3]);
    cpu.mem.ww(0xdf_f066, 0x2c);
    cpu.mem.ww(0xdf_f064, 0);
    cpu.mem.wl(0xdf_f044, 0xffff_ffff);
    cpu.mem.ww(0xdf_f040, 0x9f0);
    cpu.mem.ww(0xdf_f042, 0);
    cpu.mem.ww(0xdf_f058, 0x1401);
    cpu.d[0] = cpu.mem.rl(h(0, 0x29c6));
}

/// `h0_26ec`.
fn object_cell(cpu: &mut Cpu) {
    cpu.d[0] = ((cpu.d[0] >> 4) & 0xffff) * 0x60;
    cpu.d[1] = (cpu.d[1] >> 4).wrapping_add(cpu.d[0]);
    cpu.a[0] = h(0, MAP);
    let at = cpu.a[0].wrapping_add(cpu.d[1]);
    let b = cpu.mem.rb(at);
    cpu.d[0] = lo_b(cpu.d[0], b);
    if !br(0x2704, b != 0xff) {
        cpu.mem.ww(h(0, 0x2696), 1);
        return;
    }
    if br(0x2716, n8(b, 0x20)) {
        return;
    }
    let item = !br(0x271e, !n8(b, 0x28));
    if !item {
        if br(0x2764, n8(b, 0x40)) || br(0x276c, !n8(b, 0x60)) {
            return;
        }
        cpu.d[6] = cpu.mem.rl(h(0, CHAR_X)).wrapping_add(0xe);
        cpu.d[7] = cpu
            .mem
            .rl(h(0, CHAR_Y))
            .wrapping_add(cpu.mem.rl(h(0, ROW_OFFSET)));
    }
    cpu.mem.wb(at, 0);
    cpu.mem.wl(h(0, 0x29c6), cpu.d[0]);
    cpu.d[0] = cpu.mem.rl(h(0, CHAR_X)).wrapping_add(0xe);
    cpu.d[1] = cpu
        .mem
        .rl(h(0, CHAR_Y))
        .wrapping_add(cpu.mem.rl(h(0, ROW_OFFSET)));
    item_blit(cpu);
    let score = |cpu: &mut Cpu, v: u32| {
        cpu.mem.wl(h(0, SCORE_ADD), v);
        score_add(cpu);
        sound_2648(cpu);
    };
    if item {
        items_left(cpu);
        score(cpu, 0x100);
        return;
    }
    let b = cpu.d[0] as u8;
    for (code, at, v) in [
        (0x40, 0x27b2, 0x20),
        (0x41, 0x27ce, 0x30),
        (0x42, 0x27ea, 0x40),
        (0x43, 0x2806, 0x50),
        (0x44, 0x2822, 0x60),
        (0x45, 0x283e, 0x70),
        (0x46, 0x285a, 0x80),
        (0x47, 0x2876, 0x90),
        (0x48, 0x2892, 0x100),
        (0x49, 0x28ae, 0x1000),
        (0x4a, 0x28ca, 0x1500),
        (0x4b, 0x28e6, 0x2000),
    ] {
        if !br(at, b != code) {
            score(cpu, v);
            return;
        }
    }
    if !br(0x2902, b != 0x4c) {
        sample(
            cpu,
            8,
            0xdf_f0d0,
            h(4, 0x1_3a62),
            0x1c4f,
            0x1ff,
            (3, 0x291c, 0x2920),
        );
        countdown(cpu);
        return;
    }
    for (code, at, v) in [
        (0x4d, 0x2958, 0x2000),
        (0x4e, 0x2974, 0x3000),
        (0x4f, 0x2990, 0x4000),
        (0x50, 0x29ac, 0x5000),
    ] {
        if !br(at, b != code) {
            score(cpu, v);
            return;
        }
    }
}

/// `char_object_cells`.
fn object_cells(cpu: &mut Cpu) {
    cpu.mem.ww(h(0, 0x2696), 0);
    for off in [0x19, 9] {
        cpu.d[0] = cpu.mem.rl(h(0, CHAR_X)).wrapping_add(0xe);
        cpu.d[1] = cpu.mem.rl(h(0, CHAR_Y)).wrapping_add(off);
        cpu.mem.wl(h(0, ROW_OFFSET), off);
        object_cell(cpu);
    }
}

/// `h0_2e0e`: slot `d0` off the camera window is written into the map as its kind plus one, on its
/// cell or a neighbour, and its flag at the index long cleared.
fn slot_off_screen(cpu: &mut Cpu) {
    cpu.d[0] <<= 3;
    cpu.a[0] = h(0, SLOTS).wrapping_add(cpu.d[0]);
    let a0 = cpu.a[0];
    cpu.d[0] = lo_w(cpu.d[0], cpu.mem.rw(a0));
    cpu.d[1] = lo_w(cpu.d[1], cpu.mem.rw(a0.wrapping_add(2)));
    cpu.d[2] = lo_w(cpu.d[2], cpu.mem.rw(h(0, CAMERA_X)).wrapping_sub(0x10));
    cpu.d[3] = lo_w(cpu.d[3], cpu.mem.rw(h(0, CAMERA_Y)));
    let (x, y) = (cpu.d[0] as u16, cpu.d[1] as u16);
    // Which edge: (y + 1, first offset, its branch, second step, its branch, third step).
    let edge: (bool, i32, u32, i32, u32, i32);
    if br(0x2e30, !n16(cpu.d[2] as u16, x)) {
        edge = (false, 0, 0x2e7c, -0x60, 0x2e88, 0xc0);
    } else {
        cpu.d[2] = lo_w(cpu.d[2], (cpu.d[2] as u16).wrapping_add(0x140));
        if br(0x2e3a, n16(cpu.d[2] as u16, x)) {
            edge = (false, 0x60, 0x2ed4, 0x60, 0x2ee0, -0xc0);
        } else {
            cpu.d[3] = lo_w(cpu.d[3], (cpu.d[3] as u16).wrapping_sub(0x20));
            if br(0x2e44, !n16(cpu.d[3] as u16, y)) {
                edge = (true, 0, 0x2f2a, 0x60, 0x2f36, -0x60);
            } else {
                cpu.d[3] = lo_w(cpu.d[3], (cpu.d[3] as u16).wrapping_add(0xe5));
                if br(0x2e4e, n16(cpu.d[3] as u16, y)) {
                    edge = (true, 0, 0x2f80, 0x60, 0x2f8c, -0xc0);
                } else {
                    return;
                }
            }
        }
    }
    let (plus_one, first, b1, second, b2, third) = edge;
    cpu.d[0] = ((cpu.d[0] & 0xffff) >> 4) * 0x60;
    if plus_one {
        cpu.d[1] = lo_w(cpu.d[1], y.wrapping_add(1));
    }
    cpu.d[1] = (cpu.d[1] & 0xffff) >> 4;
    cpu.d[2] = lo_w(cpu.d[2], cpu.mem.rw(a0.wrapping_add(4)).wrapping_add(1));
    cpu.d[1] = cpu.d[1].wrapping_add(cpu.d[0]);
    let mut a1 = h(0, MAP).wrapping_add(cpu.d[1]).wrapping_add(first as u32);
    if !br(b1, cpu.mem.rb(a1) == 0) {
        a1 = a1.wrapping_add(second as u32);
        if !br(b2, cpu.mem.rb(a1) == 0) {
            a1 = a1.wrapping_add(third as u32);
        }
    }
    cpu.a[1] = a1;
    cpu.mem.wb(a1, cpu.d[2] as u8);
    cpu.d[0] = cpu.mem.rl(h(0, SLOT_INDEX));
    cpu.a[0] = h(0, SLOT_FLAGS).wrapping_add(cpu.d[0]);
    cpu.mem.wb(cpu.a[0], 0);
}

/// `h0_2dd6`.
fn slots_off_screen(cpu: &mut Cpu) {
    let at = h(0, SLOT_INDEX);
    cpu.mem.wl(at, 0);
    loop {
        cpu.d[0] = cpu.mem.rl(at);
        cpu.a[0] = h(0, SLOT_FLAGS).wrapping_add(cpu.d[0]);
        if !br(0x2dec, cpu.mem.rb(cpu.a[0]) == 0) {
            slot_off_screen(cpu);
        }
        let n = cpu.mem.rl(at).wrapping_add(1);
        cpu.mem.wl(at, n);
        if !br(0x2e08, n != 5) {
            return;
        }
    }
}

/// `h0_3dd8`: `d7` 1 (its low byte) when `x < d0 <= x + $30` and `y < d1 <= y + $20`, else 0.
fn box_test(cpu: &mut Cpu) {
    let a0 = cpu.a[0];
    let (x, y) = (cpu.mem.rw(a0), cpu.mem.rw(a0.wrapping_add(2)));
    cpu.d[4] = lo_w(cpu.d[4], x);
    cpu.d[5] = lo_w(cpu.d[5], y);
    cpu.d[6] = lo_w(cpu.d[6], x.wrapping_add(0x30));
    cpu.d[7] = lo_w(cpu.d[7], y.wrapping_add(0x20));
    let (px, py) = (cpu.d[0] as u16, cpu.d[1] as u16);
    if br(0x3dec, !n16(x, px))
        || br(0x3df2, n16(x.wrapping_add(0x30), px))
        || br(0x3df8, !n16(y, py))
        || br(0x3dfe, n16(y.wrapping_add(0x20), py))
    {
        cpu.d[7] = 0;
        return;
    }
    cpu.d[7] = lo_b(cpu.d[7], 1);
}

/// `lives_minus_one`, entered with X clear.
fn lives(cpu: &mut Cpu) {
    let at = h(0, LIVES);
    if !br(0x69dc, cpu.mem.rw(at) != 0) {
        return;
    }
    cpu.d[0] = lo_b(cpu.d[0], cpu.mem.rb(at));
    cpu.d[1] = lo_b(cpu.d[1], cpu.mem.rb(at + 1));
    cpu.d[2] = 0;
    cpu.d[3] = 1;
    let (r, x) = sbcd(1, cpu.d[1] as u8, false);
    cpu.d[1] = lo_b(cpu.d[1], r);
    let (r, _) = sbcd(0, cpu.d[0] as u8, x);
    cpu.d[0] = lo_b(cpu.d[0], r);
    cpu.mem.wb(at, cpu.d[0] as u8);
    cpu.mem.wb(at + 1, cpu.d[1] as u8);
}

/// `char_hit_check`.
fn hit_check(cpu: &mut Cpu) {
    let clx = cpu.mem.rw(CLXDAT);
    cpu.mem.ww(h(0, 0x1fe6), clx);
    cpu.d[0] = lo_w(cpu.d[0], clx & 6);
    if br(0x3b3e, clx & 6 == 0) {
        cpu.mem.ww(h(0, HITS_TAKEN), 0);
        return;
    }
    if !br(0x3b52, cpu.mem.rw(h(0, FLAG_4B62)) == 0) {
        return;
    }
    if br(0x3b5e, cpu.mem.rb(h(0, FLAG_4ED0)) != 0) {
        cpu.d[0] = cpu.mem.rl(h(0, CHAR_X)).wrapping_add(0x10);
        cpu.d[1] = cpu.mem.rl(h(0, CHAR_Y)).wrapping_add(0x24);
        for (i, [flag_at, hit_at, kind_at]) in [
            [0x3b84, 0x3b94, 0x3b9e],
            [0x3bdc, 0x3bec, 0x3bf6],
            [0x3c34, 0x3c44, 0x3c4e],
            [0x3c8c, 0x3c9c, 0x3ca6],
            [0x3ce4, 0x3cf4, 0x3cfe],
        ]
        .into_iter()
        .enumerate()
        {
            if br(flag_at, cpu.mem.rb(h(0, SLOT_FLAGS) + i as u32) == 0) {
                continue;
            }
            cpu.a[0] = h(0, SLOTS) + 8 * i as u32;
            box_test(cpu);
            if br(hit_at, cpu.d[7] as u8 == 0) {
                continue;
            }
            let a0 = cpu.a[0];
            if br(kind_at, cpu.mem.rw(a0 + 4) == 7) {
                continue;
            }
            cpu.mem.ww(a0 + 4, 7);
            cpu.mem.wb(a0 + 7, 0);
            cpu.mem.ww(h(0, HITS_TAKEN), 0);
            cpu.mem.ww(h(0, 0x3b28), 1);
            cpu.mem.wb(h(0, 0x4ecf), 1);
            cpu.mem.wl(h(0, SCORE_ADD), 0x250);
            score_add(cpu);
            sound_3adc(cpu);
            return;
        }
    }
    if !br(0x3d3c, cpu.mem.rb(h(0, FLAG_9B04)) == 0) {
        return;
    }
    let hits = cpu.mem.rw(h(0, HITS_TAKEN)).wrapping_add(1);
    cpu.mem.ww(h(0, HITS_TAKEN), hits);
    // `addi.w` sets X on a carry, only from $ffff, whose 0 then returns here: lives_minus_one is
    // entered with X clear.
    if !br(0x3d52, !n16(hits, 3)) {
        return;
    }
    if !br(0x3d5e, cpu.mem.rw(h(0, CHEAT_FLAG)) == 0) {
        return;
    }
    cpu.mem.ww(h(0, FLAG_4B62), 1);
    lives(cpu);
    cpu.mem.wl(h(0, 0x4710), 0);
    cpu.mem.wl(h(0, 0x4714), 0);
    cpu.mem.wb(h(0, 0x4ecf), 0);
    cpu.mem.wb(h(0, 0x4109), 0);
    cpu.mem.wb(h(0, 0x410a), 0);
    sample(
        cpu,
        2,
        0xdf_f0b0,
        h(4, 0xc8d0),
        0x15a4,
        0xff,
        (2, 0x3da4, 0x3da8),
    );
}

/// The branches of `h0_26ec`'s flow listing, its callees' included.
fn object_cell_branches() -> Vec<u32> {
    vec![
        0x2704, 0x2716, 0x271e, 0x2764, 0x276c, 0x27b2, 0x27ce, 0x27ea, 0x2806, 0x2822, 0x283e,
        0x285a, 0x2876, 0x2892, 0x28ae, 0x28ca, 0x28e6, 0x2902, 0x291c, 0x2920, 0x2958, 0x2974,
        0x2990, 0x29ac, 0x2a1c, 0x265e, 0x2662, 0x6a52, 0x6a0c,
    ]
}

const SLOT_BRANCHES: [u32; 12] = [
    0x2e30, 0x2e3a, 0x2e44, 0x2e4e, 0x2e7c, 0x2e88, 0x2ed4, 0x2ee0, 0x2f2a, 0x2f36, 0x2f80, 0x2f8c,
];
const BOX_BRANCHES: [u32; 4] = [0x3dec, 0x3df2, 0x3df8, 0x3dfe];

type Witness = fn(&mut Cpu);

/// Each routine: its generator's name, entry offset in hunk 0, witness, and the branches of its
/// flow listing (its own and its callees').
fn routines() -> Vec<(&'static str, u32, Witness, Vec<u32>)> {
    let hit = [
        vec![0x3b3e, 0x3b52, 0x3b5e],
        vec![
            0x3b84, 0x3b94, 0x3b9e, 0x3bdc, 0x3bec, 0x3bf6, 0x3c34, 0x3c44, 0x3c4e, 0x3c8c, 0x3c9c,
            0x3ca6, 0x3ce4, 0x3cf4, 0x3cfe,
        ],
        vec![
            0x3d3c, 0x3d52, 0x3d5e, 0x3da4, 0x3da8, 0x3af2, 0x3af6, 0x69dc,
        ],
        BOX_BRANCHES.to_vec(),
    ]
    .concat();
    vec![
        (
            "h0_24ce",
            0x24ce,
            map_code as Witness,
            vec![0x24d4, 0x24e0, 0x2516, 0x251e, 0x2526, 0x252e],
        ),
        (
            "char_object_cells",
            0x2698,
            object_cells,
            object_cell_branches(),
        ),
        ("h0_26ec", 0x26ec, object_cell, object_cell_branches()),
        ("score_add_routine", 0x2a6e, score_add, vec![]),
        (
            "h0_2dd6",
            0x2dd6,
            slots_off_screen,
            [vec![0x2dec, 0x2e08], SLOT_BRANCHES.to_vec()].concat(),
        ),
        ("h0_2e0e", 0x2e0e, slot_off_screen, SLOT_BRANCHES.to_vec()),
        ("char_hit_check", 0x3b2a, hit_check, hit),
        ("h0_3dd8", 0x3dd8, box_test, BOX_BRANCHES.to_vec()),
        ("countdown_plus_100", 0x6a04, countdown, vec![0x6a0c]),
        ("items_left_minus_one", 0x6a4c, items_left, vec![0x6a52]),
    ]
}

/// Every `objects` entry of `re/translate.toml` T38 keeps, by name.
fn t38_entries() -> BTreeSet<String> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../re/translate.toml");
    // Each `[[routine]]` table's `name`, `subsystem` and `gen_task` lines, read by hand: fd-conform
    // takes no toml crate.
    let text = std::fs::read_to_string(path).unwrap();
    let field = |block: &str, key: &str| {
        block.lines().find_map(|l| {
            let (k, v) = l.split_once('=')?;
            (k.trim() == key).then(|| v.trim().trim_matches('"').to_string())
        })
    };
    text.split("[[routine]]")
        .skip(1)
        .filter(|b| field(b, "gen_task").as_deref() == Some("T38"))
        .inspect(|b| assert_eq!(field(b, "subsystem").as_deref(), Some("objects")))
        .map(|b| field(b, "name").unwrap())
        .collect()
}

#[test]
fn every_t38_entry_has_a_generator_and_a_witness() {
    let names: BTreeSet<String> = routines().iter().map(|r| r.0.to_string()).collect();
    assert_eq!(names, t38_entries());
    for n in &names {
        assert!(generators::generator(n).is_some(), "{n} is not registered");
    }
}

#[test]
fn every_generator_takes_every_branch_both_ways_on_the_original() {
    let mut r = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    let one_way: BTreeSet<u32> = ONE_WAY.iter().map(|o| o.0).collect();
    let mut lines = Vec::new();
    for (name, off, witness, branches) in routines() {
        COV.with(|c| c.borrow_mut().clear());
        let generator = generators::generator(name).unwrap();
        let rep: RigReport = judge::check(&mut r, name, CASES, h(0, off), &generator, witness)
            .unwrap_or_else(|f| panic!("the witness of {name} differs from the original: {f}"));
        assert_eq!(
            (rep.passed, rep.hangs, rep.faults, rep.unmodelled_blits),
            (CASES, 0, 0, 0),
            "{name}"
        );
        let cov = COV.with(|c| c.borrow().clone());
        let seen: BTreeSet<u32> = cov.keys().copied().collect();
        assert_eq!(
            seen,
            branches.iter().copied().collect(),
            "{name}: the witness's branches are not the listing's"
        );
        for b in &branches {
            let [not, taken] = cov[b];
            if one_way.contains(b) {
                assert!(not > 0 && taken == 0, "{name}: h0+${b:x}");
                continue;
            }
            assert!(
                not > 0 && taken > 0,
                "{name}: the branch at h0+${b:x} went one way only ({not} not taken, {taken} taken)"
            );
        }
        let least = branches
            .iter()
            .filter(|b| !one_way.contains(b))
            .map(|b| cov[b][0].min(cov[b][1]))
            .min();
        lines.push(format!(
            "{name}: {} paths, {} branches, each way at least {} times",
            rep.paths,
            branches.len(),
            least.map_or("-".into(), |l| l.to_string())
        ));
    }
    // What docs/11-translation.md records; `--nocapture` prints it.
    println!("{}", lines.join("\n"));
}

#[test]
fn a_witness_with_the_wrong_countdown_limit_fails() {
    // The states reach the `$899` compare's edge exactly: one off is caught.
    let mut r = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    let generator = generators::generator("countdown_plus_100").unwrap();
    let f = judge::check(
        &mut r,
        "countdown_plus_100",
        CASES,
        h(0, 0x6a04),
        &generator,
        countdown_wrong,
    )
    .unwrap_err();
    assert!(f.failures > 0, "{f}");
}

/// The addresses each routine reads, as `h0` offsets and sizes, and the hardware bytes.
fn reads(name: &str) -> (Vec<(u32, u32)>, Vec<u32>) {
    let map = (MAP, MAP_END - MAP);
    let tiles = (TILES, TILES_END - TILES);
    let table = (TILE_TABLE, 4 * TILE_COUNT);
    let chr = [(CHAR_X, 4), (CHAR_Y, 4)];
    let cam = [(CAMERA_X, 2), (CAMERA_Y, 2)];
    // x, y and kind of each slot; its last two bytes are only written.
    let slots = |n: u32| (0..5).map(move |i| (SLOTS + 8 * i, n));
    let counters = [(ITEMS_LEFT, 2), (SCORE, 4), (COUNTDOWN, 3)];
    let mut hw = vec![];
    let v: Vec<(u32, u32)> = match name {
        "h0_24ce" => [vec![(FLAG_4ED0, 1), (FLAG_4B62, 2), map], chr.to_vec()].concat(),
        "char_object_cells" => [vec![map, tiles, table], chr.to_vec(), counters.to_vec()].concat(),
        "h0_26ec" => [
            vec![map, tiles, table, (ROW_OFFSET, 4)],
            chr.to_vec(),
            counters.to_vec(),
        ]
        .concat(),
        "score_add_routine" => vec![(SCORE, 8)],
        "h0_2dd6" => [vec![map, (SLOT_FLAGS, 5)], cam.to_vec(), slots(6).collect()].concat(),
        "h0_2e0e" => [vec![map, (SLOT_INDEX, 4)], cam.to_vec(), slots(6).collect()].concat(),
        "char_hit_check" => {
            hw = vec![CLXDAT, CLXDAT + 1];
            [
                vec![
                    (FLAG_4ED0, 1),
                    (FLAG_4B62, 2),
                    (FLAG_9B04, 1),
                    (SLOT_FLAGS, 5),
                    (HITS_TAKEN, 2),
                    (CHEAT_FLAG, 2),
                    (LIVES, 2),
                    (SCORE, 4),
                ],
                chr.to_vec(),
                slots(6).collect(),
            ]
            .concat()
        }
        "h0_3dd8" => slots(4).collect(),
        "countdown_plus_100" => vec![(COUNTDOWN, 3)],
        "items_left_minus_one" => vec![(ITEMS_LEFT, 2)],
        _ => panic!("{name}"),
    };
    (v, hw)
}

#[test]
fn no_generator_writes_outside_what_its_routine_reads() {
    for (name, _, _, _) in routines() {
        let generator = generators::generator(name).unwrap();
        let (ranges, hw) = reads(name);
        let allowed = |a: u32| {
            ranges
                .iter()
                .any(|&(o, n)| (h(0, o)..h(0, o) + n).contains(&a))
        };
        let mut g = Gen::new(0x7e57);
        for _ in 0..CASES {
            let s = (generator.case)(&mut g);
            for (a, _) in &s.hw_reads {
                assert!(hw.contains(a), "{name} sets hardware read ${a:06x}");
            }
            for (a, bytes) in &s.pokes {
                for i in 0..bytes.len() as u32 {
                    assert!(
                        allowed(a + i),
                        "{name} writes ${:06x}, which its routine does not read",
                        a + i
                    );
                }
            }
        }
    }
}
