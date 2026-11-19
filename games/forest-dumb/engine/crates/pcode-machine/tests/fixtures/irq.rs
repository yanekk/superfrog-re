// Generated from irq.json; never committed when made from a game.
// By pcode_rust.py. Do not edit: regenerate.
#[allow(unused_imports)]
use pcode_rt::{Bus, Image, Regs, Stop, ops::*, load, store, block_start, insn_start, fn_entry, PC, cycles as cy};

/// The bytes of every translated instruction, (hunk, start, end exclusive), sorted.
pub const CODE: &[(u32, u32, u32)] = &[
    (0, 0x0, 0xa),
    (0, 0x10, 0x20),
    (0, 0x40, 0x4e),
    (0, 0x60, 0x78),
    (0, 0x80, 0x9c),
    (0, 0xa0, 0xae),
    (0, 0xc0, 0xce),
    (0, 0xd0, 0xde),
    (0, 0xe0, 0xee),
];

/// Each hunk's length, as the export gives it.
pub const HUNK_SIZES: &[u32] = &[0x100];

#[allow(dead_code)]
pub type Routine = fn(&mut Regs, &mut dyn Bus, &Image) -> Result<(), Stop>;

/// The translated function entered at `addr` (`hN+$off`).
pub fn entry(addr: &str) -> Option<Routine> {
    match addr {
        "h0+$0" => Some(h0_0),
        "h0+$10" => Some(h0_10),
        "h0+$40" => Some(h0_40),
        "h0+$60" => Some(h0_60),
        "h0+$80" => Some(h0_80),
        "h0+$a0" => Some(h0_a0),
        "h0+$c0" => Some(h0_c0),
        "h0+$d0" => Some(h0_d0),
        "h0+$e0" => Some(h0_e0),
        _ => None,
    }
}

#[allow(dead_code)]
fn by_addr(k: (u32, u32)) -> Option<Routine> {
    match k {
        (0, 0x0) => Some(h0_0),
        (0, 0x10) => Some(h0_10),
        (0, 0x40) => Some(h0_40),
        (0, 0x60) => Some(h0_60),
        (0, 0x80) => Some(h0_80),
        (0, 0xa0) => Some(h0_a0),
        (0, 0xc0) => Some(h0_c0),
        (0, 0xd0) => Some(h0_d0),
        (0, 0xe0) => Some(h0_e0),
        _ => None,
    }
}

/// Control reached `k`, which is no block of the function it was in: enter the translated
/// function there, whose return our caller checks, or stop.
#[allow(dead_code)]
fn enter(k: (u32, u32), r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    match by_addr(k) {
        Some(f) => f(r, bus, img),
        None => Err(Stop::Untranslated(format!("h{}+${:x}", k.0, k.1))),
    }
}

#[allow(dead_code)]
pub type Resume = fn((u32, u32), &mut Regs, &mut dyn Bus, &Image) -> Result<(), Stop>;

/// The translated function holding the instruction at `k`, entered at it.
#[allow(dead_code)]
pub fn resume(k: (u32, u32)) -> Option<Resume> {
    match k {
        (0, 0x0)
        | (0, 0x4)
        | (0, 0x6)
        | (0, 0x8) => Some(h0_0_from),
        (0, 0x10)
        | (0, 0x18)
        | (0, 0x1a)
        | (0, 0x1c)
        | (0, 0x1e) => Some(h0_10_from),
        (0, 0x40)
        | (0, 0x44)
        | (0, 0x4c) => Some(h0_40_from),
        (0, 0x60)
        | (0, 0x68)
        | (0, 0x6a)
        | (0, 0x6e)
        | (0, 0x76) => Some(h0_60_from),
        (0, 0x80)
        | (0, 0x88)
        | (0, 0x8a)
        | (0, 0x8e)
        | (0, 0x92)
        | (0, 0x9a) => Some(h0_80_from),
        (0, 0xa0)
        | (0, 0xa4)
        | (0, 0xac) => Some(h0_a0_from),
        (0, 0xc0)
        | (0, 0xc8)
        | (0, 0xcc) => Some(h0_c0_from),
        (0, 0xd0)
        | (0, 0xd2)
        | (0, 0xd4)
        | (0, 0xd6)
        | (0, 0xd8)
        | (0, 0xda)
        | (0, 0xdc) => Some(h0_d0_from),
        (0, 0xe0)
        | (0, 0xe4)
        | (0, 0xec) => Some(h0_e0_from),
        _ => None,
    }
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
#[inline]
pub fn h0_0(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    if let Some(f) = bus.mounted((0, 0x0)) {
        return f(r, bus, img);
    }
    h0_0_from((0, 0x0), r, bus, img)
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn h0_0_from(start: (u32, u32), r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    fn_entry(r)?;
    let mut pc: (u32, u32) = start;
    'b: loop {
        block_start(bus, r, img)?;
        match pc {
            (0, 0x0) | (0, 0x4) | (0, 0x6) => {
                let entered = pc.1;
                if entered <= 0x0 {
                    // h0+$0 move.w ($100).w,d0
                    {
                        bus.sync(r);
                        let v = load(bus, 0x100u64 as u32, 2);
                        r.set(0x2, 2, v);
                    }
                }
                if entered <= 0x4 {
                    insn_start(bus, r, img)?;
                    // h0+$4 cmp.w d2,d0
                    {
                        let v = int_equal(r.get(0x2, 2), r.get(0xa, 2), 2);
                        r.set(0x45, 1, v);
                    }
                }
                if entered <= 0x6 {
                    insn_start(bus, r, img)?;
                    // h0+$6 bne.b h0+$0
                    {
                        let mut u_100: u64 = 0;
                        let v = bool_negate(r.get(0x45, 1));
                        u_100 = v & 0xff;
                        if u_100 != 0 { r.tick(const { cy::bcc(true, cy::Size::B) }); r.off_bus(const { cy::bcc_off(true) }); r.idle_first(const { cy::bcc_off(true) }); pc = (0, 0x0); continue 'b; }
                        r.tick(const { cy::bcc(false, cy::Size::B) }); r.off_bus(const { cy::bcc_off(false) }); r.idle_first(const { cy::bcc_off(false) });
                    }
                }
                pc = (0, 0x8);
            }
            (0, 0x8) => {
                // h0+$8 rts
                {
                    bus.sync(r);
                    let v = load(bus, r.get(0x3c, 4) as u32, 4);
                    r.set(0x50, 4, v);
                    let v = int_add(r.get(0x3c, 4), 0x4u64, 4);
                    r.set(0x3c, 4, v);
                    let v = r.get(0x50, 4);
                    r.set(PC, 4, v);
                    r.depth -= 1;
                    return Ok(());
                }
                pc = (0, 0xa);
            }
            k => {
                let res = enter(k, r, bus, img);
                r.depth -= 1;
                return res;
            }
        }
    }
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
#[inline]
pub fn h0_10(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    if let Some(f) = bus.mounted((0, 0x10)) {
        return f(r, bus, img);
    }
    h0_10_from((0, 0x10), r, bus, img)
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn h0_10_from(start: (u32, u32), r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    fn_entry(r)?;
    let mut pc: (u32, u32) = start;
    'b: loop {
        block_start(bus, r, img)?;
        match pc {
            (0, 0x0) | (0, 0x4) | (0, 0x6) => {
                let entered = pc.1;
                if entered <= 0x0 {
                    // h0+$0 move.w ($100).w,d0
                    {
                        bus.sync(r);
                        let v = load(bus, 0x100u64 as u32, 2);
                        r.set(0x2, 2, v);
                    }
                }
                if entered <= 0x4 {
                    insn_start(bus, r, img)?;
                    // h0+$4 cmp.w d2,d0
                    {
                        let v = int_equal(r.get(0x2, 2), r.get(0xa, 2), 2);
                        r.set(0x45, 1, v);
                    }
                }
                if entered <= 0x6 {
                    insn_start(bus, r, img)?;
                    // h0+$6 bne.b h0+$0
                    {
                        let mut u_100: u64 = 0;
                        let v = bool_negate(r.get(0x45, 1));
                        u_100 = v & 0xff;
                        if u_100 != 0 { r.tick(const { cy::bcc(true, cy::Size::B) }); r.off_bus(const { cy::bcc_off(true) }); r.idle_first(const { cy::bcc_off(true) }); pc = (0, 0x0); continue 'b; }
                        r.tick(const { cy::bcc(false, cy::Size::B) }); r.off_bus(const { cy::bcc_off(false) }); r.idle_first(const { cy::bcc_off(false) });
                    }
                }
                pc = (0, 0x8);
            }
            (0, 0x8) => {
                // h0+$8 rts
                {
                    bus.sync(r);
                    let v = load(bus, r.get(0x3c, 4) as u32, 4);
                    r.set(0x50, 4, v);
                    let v = int_add(r.get(0x3c, 4), 0x4u64, 4);
                    r.set(0x3c, 4, v);
                    let v = r.get(0x50, 4);
                    r.set(PC, 4, v);
                    r.depth -= 1;
                    return Ok(());
                }
                pc = (0, 0xa);
            }
            (0, 0x10) | (0, 0x18) | (0, 0x1a) | (0, 0x1c) => {
                let entered = pc.1;
                if entered <= 0x10 {
                    // h0+$10 move.w #$8020,($dff09c).l
                    {
                        bus.sync(r);
                        let v = 0x8020u64;
                        store(bus, img, 0xdff09cu64 as u32, 2, v)?;
                    }
                }
                if entered <= 0x18 {
                    insn_start(bus, r, img)?;
                    // h0+$18 addq.l #1,d1
                    {
                        r.tick(const { cy::of(cy::Op::Quick, cy::Size::L, cy::Ea::None, cy::Ea::None) }); r.off_bus(const { cy::off_of(cy::Op::Quick, cy::Size::L, cy::Ea::None, cy::Ea::None) });
                        let v = int_add(r.get(0x4, 4), 0x1u64, 4);
                        r.set(0x4, 4, v);
                    }
                }
                if entered <= 0x1a {
                    insn_start(bus, r, img)?;
                    // h0+$1a cmp.l d2,d1
                    {
                        let v = int_equal(r.get(0x4, 4), r.get(0x8, 4), 4);
                        r.set(0x45, 1, v);
                    }
                }
                if entered <= 0x1c {
                    insn_start(bus, r, img)?;
                    // h0+$1c bne.b h0+$10
                    {
                        let mut u_100: u64 = 0;
                        let v = bool_negate(r.get(0x45, 1));
                        u_100 = v & 0xff;
                        if u_100 != 0 { r.tick(const { cy::bcc(true, cy::Size::B) }); r.off_bus(const { cy::bcc_off(true) }); r.idle_first(const { cy::bcc_off(true) }); pc = (0, 0x10); continue 'b; }
                        r.tick(const { cy::bcc(false, cy::Size::B) }); r.off_bus(const { cy::bcc_off(false) }); r.idle_first(const { cy::bcc_off(false) });
                    }
                }
                pc = (0, 0x1e);
            }
            (0, 0x1e) => {
                // h0+$1e bra.b h0+$0
                {
                    pc = (0, 0x0); continue 'b;
                }
                pc = (0, 0x20);
            }
            k => {
                let res = enter(k, r, bus, img);
                r.depth -= 1;
                return res;
            }
        }
    }
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
#[inline]
pub fn h0_40(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    if let Some(f) = bus.mounted((0, 0x40)) {
        return f(r, bus, img);
    }
    h0_40_from((0, 0x40), r, bus, img)
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn h0_40_from(start: (u32, u32), r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    fn_entry(r)?;
    let mut pc: (u32, u32) = start;
    'b: loop {
        block_start(bus, r, img)?;
        match pc {
            (0, 0x40) | (0, 0x44) | (0, 0x4c) => {
                let entered = pc.1;
                if entered <= 0x40 {
                    // h0+$40 addq.w #1,($100).w
                    {
                        bus.sync(r);
                        let mut u_200: u64 = 0;
                        let mut u_210: u64 = 0;
                        let v = load(bus, 0x100u64 as u32, 2);
                        u_200 = v & 0xffff;
                        let v = int_add(u_200, 0x1u64, 2);
                        u_210 = v & 0xffff;
                        let v = u_210;
                        store(bus, img, 0x100u64 as u32, 2, v)?;
                    }
                }
                if entered <= 0x44 {
                    insn_start(bus, r, img)?;
                    // h0+$44 move.w #$20,($dff09c).l
                    {
                        bus.sync(r);
                        let v = 0x20u64;
                        store(bus, img, 0xdff09cu64 as u32, 2, v)?;
                    }
                }
                if entered <= 0x4c {
                    insn_start(bus, r, img)?;
                    // h0+$4c rte
                    {
                        r.tick(const { cy::of(cy::Op::Rte, cy::Size::L, cy::Ea::None, cy::Ea::None) }); r.off_bus(const { cy::off_of(cy::Op::Rte, cy::Size::L, cy::Ea::None, cy::Ea::None) });
                        bus.sync(r);
                        let mut u_fff000: u64 = 0;
                        let v = load(bus, r.get(0x3c, 4) as u32, 2);
                        u_fff000 = v & 0xffff;
                        let v = int_add(r.get(0x3c, 4), 0x2u64, 4);
                        r.set(0x3c, 4, v);
                        let v = load(bus, r.get(0x3c, 4) as u32, 4);
                        r.set(0x50, 4, v);
                        let v = int_add(r.get(0x3c, 4), 0x4u64, 4);
                        r.set(0x3c, 4, v);
                        pcode_rt::ops::sr_set(r, u_fff000);
                        let v = r.get(0x50, 4);
                        r.set(PC, 4, v);
                        r.depth -= 1;
                        return Ok(());
                    }
                }
                pc = (0, 0x4e);
            }
            k => {
                let res = enter(k, r, bus, img);
                r.depth -= 1;
                return res;
            }
        }
    }
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
#[inline]
pub fn h0_60(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    if let Some(f) = bus.mounted((0, 0x60)) {
        return f(r, bus, img);
    }
    h0_60_from((0, 0x60), r, bus, img)
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn h0_60_from(start: (u32, u32), r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    fn_entry(r)?;
    let mut pc: (u32, u32) = start;
    'b: loop {
        block_start(bus, r, img)?;
        match pc {
            (0, 0x60) | (0, 0x68) => {
                let entered = pc.1;
                if entered <= 0x60 {
                    // h0+$60 move.w #$8020,($dff09c).l
                    {
                        bus.sync(r);
                        let v = 0x8020u64;
                        store(bus, img, 0xdff09cu64 as u32, 2, v)?;
                    }
                }
                if entered <= 0x68 {
                    insn_start(bus, r, img)?;
                    // h0+$68 bra.b h0+$6a
                    {
                        pc = (0, 0x6a); continue 'b;
                    }
                }
                pc = (0, 0x6a);
            }
            (0, 0x6a) | (0, 0x6e) | (0, 0x76) => {
                let entered = pc.1;
                if entered <= 0x6a {
                    // h0+$6a move.w ($100).w,d3
                    {
                        bus.sync(r);
                        let v = load(bus, 0x100u64 as u32, 2);
                        r.set(0xe, 2, v);
                    }
                }
                if entered <= 0x6e {
                    insn_start(bus, r, img)?;
                    // h0+$6e move.w #$8,($dff09c).l
                    {
                        bus.sync(r);
                        let v = 0x8u64;
                        store(bus, img, 0xdff09cu64 as u32, 2, v)?;
                    }
                }
                if entered <= 0x76 {
                    insn_start(bus, r, img)?;
                    // h0+$76 rte
                    {
                        r.tick(const { cy::of(cy::Op::Rte, cy::Size::L, cy::Ea::None, cy::Ea::None) }); r.off_bus(const { cy::off_of(cy::Op::Rte, cy::Size::L, cy::Ea::None, cy::Ea::None) });
                        bus.sync(r);
                        let mut u_fff000: u64 = 0;
                        let v = load(bus, r.get(0x3c, 4) as u32, 2);
                        u_fff000 = v & 0xffff;
                        let v = int_add(r.get(0x3c, 4), 0x2u64, 4);
                        r.set(0x3c, 4, v);
                        let v = load(bus, r.get(0x3c, 4) as u32, 4);
                        r.set(0x50, 4, v);
                        let v = int_add(r.get(0x3c, 4), 0x4u64, 4);
                        r.set(0x3c, 4, v);
                        pcode_rt::ops::sr_set(r, u_fff000);
                        let v = r.get(0x50, 4);
                        r.set(PC, 4, v);
                        r.depth -= 1;
                        return Ok(());
                    }
                }
                pc = (0, 0x78);
            }
            k => {
                let res = enter(k, r, bus, img);
                r.depth -= 1;
                return res;
            }
        }
    }
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
#[inline]
pub fn h0_80(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    if let Some(f) = bus.mounted((0, 0x80)) {
        return f(r, bus, img);
    }
    h0_80_from((0, 0x80), r, bus, img)
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn h0_80_from(start: (u32, u32), r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    fn_entry(r)?;
    let mut pc: (u32, u32) = start;
    'b: loop {
        block_start(bus, r, img)?;
        match pc {
            (0, 0x80) | (0, 0x88) => {
                let entered = pc.1;
                if entered <= 0x80 {
                    // h0+$80 move.w #$8008,($dff09c).l
                    {
                        bus.sync(r);
                        let v = 0x8008u64;
                        store(bus, img, 0xdff09cu64 as u32, 2, v)?;
                    }
                }
                if entered <= 0x88 {
                    insn_start(bus, r, img)?;
                    // h0+$88 bra.b h0+$8a
                    {
                        pc = (0, 0x8a); continue 'b;
                    }
                }
                pc = (0, 0x8a);
            }
            (0, 0x8a) | (0, 0x8e) | (0, 0x92) | (0, 0x9a) => {
                let entered = pc.1;
                if entered <= 0x8a {
                    // h0+$8a move.w ($102).w,d4
                    {
                        bus.sync(r);
                        let v = load(bus, 0x102u64 as u32, 2);
                        r.set(0x12, 2, v);
                    }
                }
                if entered <= 0x8e {
                    insn_start(bus, r, img)?;
                    // h0+$8e addq.w #1,($100).w
                    {
                        bus.sync(r);
                        let mut u_300: u64 = 0;
                        let mut u_310: u64 = 0;
                        let v = load(bus, 0x100u64 as u32, 2);
                        u_300 = v & 0xffff;
                        let v = int_add(u_300, 0x1u64, 2);
                        u_310 = v & 0xffff;
                        let v = u_310;
                        store(bus, img, 0x100u64 as u32, 2, v)?;
                    }
                }
                if entered <= 0x92 {
                    insn_start(bus, r, img)?;
                    // h0+$92 move.w #$20,($dff09c).l
                    {
                        bus.sync(r);
                        let v = 0x20u64;
                        store(bus, img, 0xdff09cu64 as u32, 2, v)?;
                    }
                }
                if entered <= 0x9a {
                    insn_start(bus, r, img)?;
                    // h0+$9a rte
                    {
                        r.tick(const { cy::of(cy::Op::Rte, cy::Size::L, cy::Ea::None, cy::Ea::None) }); r.off_bus(const { cy::off_of(cy::Op::Rte, cy::Size::L, cy::Ea::None, cy::Ea::None) });
                        bus.sync(r);
                        let mut u_fff000: u64 = 0;
                        let v = load(bus, r.get(0x3c, 4) as u32, 2);
                        u_fff000 = v & 0xffff;
                        let v = int_add(r.get(0x3c, 4), 0x2u64, 4);
                        r.set(0x3c, 4, v);
                        let v = load(bus, r.get(0x3c, 4) as u32, 4);
                        r.set(0x50, 4, v);
                        let v = int_add(r.get(0x3c, 4), 0x4u64, 4);
                        r.set(0x3c, 4, v);
                        pcode_rt::ops::sr_set(r, u_fff000);
                        let v = r.get(0x50, 4);
                        r.set(PC, 4, v);
                        r.depth -= 1;
                        return Ok(());
                    }
                }
                pc = (0, 0x9c);
            }
            k => {
                let res = enter(k, r, bus, img);
                r.depth -= 1;
                return res;
            }
        }
    }
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
#[inline]
pub fn h0_a0(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    if let Some(f) = bus.mounted((0, 0xa0)) {
        return f(r, bus, img);
    }
    h0_a0_from((0, 0xa0), r, bus, img)
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn h0_a0_from(start: (u32, u32), r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    fn_entry(r)?;
    let mut pc: (u32, u32) = start;
    'b: loop {
        block_start(bus, r, img)?;
        match pc {
            (0, 0xa0) | (0, 0xa4) | (0, 0xac) => {
                let entered = pc.1;
                if entered <= 0xa0 {
                    // h0+$a0 addq.w #1,($102).w
                    {
                        bus.sync(r);
                        let mut u_400: u64 = 0;
                        let mut u_410: u64 = 0;
                        let v = load(bus, 0x102u64 as u32, 2);
                        u_400 = v & 0xffff;
                        let v = int_add(u_400, 0x1u64, 2);
                        u_410 = v & 0xffff;
                        let v = u_410;
                        store(bus, img, 0x102u64 as u32, 2, v)?;
                    }
                }
                if entered <= 0xa4 {
                    insn_start(bus, r, img)?;
                    // h0+$a4 move.w #$8,($dff09c).l
                    {
                        bus.sync(r);
                        let v = 0x8u64;
                        store(bus, img, 0xdff09cu64 as u32, 2, v)?;
                    }
                }
                if entered <= 0xac {
                    insn_start(bus, r, img)?;
                    // h0+$ac rte
                    {
                        r.tick(const { cy::of(cy::Op::Rte, cy::Size::L, cy::Ea::None, cy::Ea::None) }); r.off_bus(const { cy::off_of(cy::Op::Rte, cy::Size::L, cy::Ea::None, cy::Ea::None) });
                        bus.sync(r);
                        let mut u_fff000: u64 = 0;
                        let v = load(bus, r.get(0x3c, 4) as u32, 2);
                        u_fff000 = v & 0xffff;
                        let v = int_add(r.get(0x3c, 4), 0x2u64, 4);
                        r.set(0x3c, 4, v);
                        let v = load(bus, r.get(0x3c, 4) as u32, 4);
                        r.set(0x50, 4, v);
                        let v = int_add(r.get(0x3c, 4), 0x4u64, 4);
                        r.set(0x3c, 4, v);
                        pcode_rt::ops::sr_set(r, u_fff000);
                        let v = r.get(0x50, 4);
                        r.set(PC, 4, v);
                        r.depth -= 1;
                        return Ok(());
                    }
                }
                pc = (0, 0xae);
            }
            k => {
                let res = enter(k, r, bus, img);
                r.depth -= 1;
                return res;
            }
        }
    }
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
#[inline]
pub fn h0_c0(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    if let Some(f) = bus.mounted((0, 0xc0)) {
        return f(r, bus, img);
    }
    h0_c0_from((0, 0xc0), r, bus, img)
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn h0_c0_from(start: (u32, u32), r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    fn_entry(r)?;
    let mut pc: (u32, u32) = start;
    'b: loop {
        block_start(bus, r, img)?;
        match pc {
            (0, 0xc0) | (0, 0xc8) | (0, 0xcc) => {
                let entered = pc.1;
                if entered <= 0xc0 {
                    // h0+$c0 move.w #$8020,($dff09c).l
                    {
                        bus.sync(r);
                        let v = 0x8020u64;
                        store(bus, img, 0xdff09cu64 as u32, 2, v)?;
                    }
                }
                if entered <= 0xc8 {
                    insn_start(bus, r, img)?;
                    // h0+$c8 move.w ($100).w,d5
                    {
                        bus.sync(r);
                        let v = load(bus, 0x100u64 as u32, 2);
                        r.set(0x16, 2, v);
                    }
                }
                if entered <= 0xcc {
                    insn_start(bus, r, img)?;
                    // h0+$cc rts
                    {
                        bus.sync(r);
                        let v = load(bus, r.get(0x3c, 4) as u32, 4);
                        r.set(0x50, 4, v);
                        let v = int_add(r.get(0x3c, 4), 0x4u64, 4);
                        r.set(0x3c, 4, v);
                        let v = r.get(0x50, 4);
                        r.set(PC, 4, v);
                        r.depth -= 1;
                        return Ok(());
                    }
                }
                pc = (0, 0xce);
            }
            k => {
                let res = enter(k, r, bus, img);
                r.depth -= 1;
                return res;
            }
        }
    }
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
#[inline]
pub fn h0_d0(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    if let Some(f) = bus.mounted((0, 0xd0)) {
        return f(r, bus, img);
    }
    h0_d0_from((0, 0xd0), r, bus, img)
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn h0_d0_from(start: (u32, u32), r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    fn_entry(r)?;
    let mut pc: (u32, u32) = start;
    'b: loop {
        block_start(bus, r, img)?;
        match pc {
            (0, 0xd0) | (0, 0xd2) | (0, 0xd4) | (0, 0xd6) | (0, 0xd8) | (0, 0xda) | (0, 0xdc) => {
                let entered = pc.1;
                if entered <= 0xd0 {
                    // h0+$d0 addq.l #1,d1
                    {
                        r.tick(const { cy::of(cy::Op::Quick, cy::Size::L, cy::Ea::None, cy::Ea::None) }); r.off_bus(const { cy::off_of(cy::Op::Quick, cy::Size::L, cy::Ea::None, cy::Ea::None) });
                        let v = int_add(r.get(0x4, 4), 0x1u64, 4);
                        r.set(0x4, 4, v);
                    }
                }
                if entered <= 0xd2 {
                    insn_start(bus, r, img)?;
                    // h0+$d2 addq.l #1,d1
                    {
                        r.tick(const { cy::of(cy::Op::Quick, cy::Size::L, cy::Ea::None, cy::Ea::None) }); r.off_bus(const { cy::off_of(cy::Op::Quick, cy::Size::L, cy::Ea::None, cy::Ea::None) });
                        let v = int_add(r.get(0x4, 4), 0x1u64, 4);
                        r.set(0x4, 4, v);
                    }
                }
                if entered <= 0xd4 {
                    insn_start(bus, r, img)?;
                    // h0+$d4 addq.l #1,d1
                    {
                        r.tick(const { cy::of(cy::Op::Quick, cy::Size::L, cy::Ea::None, cy::Ea::None) }); r.off_bus(const { cy::off_of(cy::Op::Quick, cy::Size::L, cy::Ea::None, cy::Ea::None) });
                        let v = int_add(r.get(0x4, 4), 0x1u64, 4);
                        r.set(0x4, 4, v);
                    }
                }
                if entered <= 0xd6 {
                    insn_start(bus, r, img)?;
                    // h0+$d6 addq.l #1,d1
                    {
                        r.tick(const { cy::of(cy::Op::Quick, cy::Size::L, cy::Ea::None, cy::Ea::None) }); r.off_bus(const { cy::off_of(cy::Op::Quick, cy::Size::L, cy::Ea::None, cy::Ea::None) });
                        let v = int_add(r.get(0x4, 4), 0x1u64, 4);
                        r.set(0x4, 4, v);
                    }
                }
                if entered <= 0xd8 {
                    insn_start(bus, r, img)?;
                    // h0+$d8 addq.l #1,d1
                    {
                        r.tick(const { cy::of(cy::Op::Quick, cy::Size::L, cy::Ea::None, cy::Ea::None) }); r.off_bus(const { cy::off_of(cy::Op::Quick, cy::Size::L, cy::Ea::None, cy::Ea::None) });
                        let v = int_add(r.get(0x4, 4), 0x1u64, 4);
                        r.set(0x4, 4, v);
                    }
                }
                if entered <= 0xda {
                    insn_start(bus, r, img)?;
                    // h0+$da addq.l #1,d1
                    {
                        r.tick(const { cy::of(cy::Op::Quick, cy::Size::L, cy::Ea::None, cy::Ea::None) }); r.off_bus(const { cy::off_of(cy::Op::Quick, cy::Size::L, cy::Ea::None, cy::Ea::None) });
                        let v = int_add(r.get(0x4, 4), 0x1u64, 4);
                        r.set(0x4, 4, v);
                    }
                }
                if entered <= 0xdc {
                    insn_start(bus, r, img)?;
                    // h0+$dc rts
                    {
                        bus.sync(r);
                        let v = load(bus, r.get(0x3c, 4) as u32, 4);
                        r.set(0x50, 4, v);
                        let v = int_add(r.get(0x3c, 4), 0x4u64, 4);
                        r.set(0x3c, 4, v);
                        let v = r.get(0x50, 4);
                        r.set(PC, 4, v);
                        r.depth -= 1;
                        return Ok(());
                    }
                }
                pc = (0, 0xde);
            }
            k => {
                let res = enter(k, r, bus, img);
                r.depth -= 1;
                return res;
            }
        }
    }
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
#[inline]
pub fn h0_e0(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    if let Some(f) = bus.mounted((0, 0xe0)) {
        return f(r, bus, img);
    }
    h0_e0_from((0, 0xe0), r, bus, img)
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn h0_e0_from(start: (u32, u32), r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    fn_entry(r)?;
    let mut pc: (u32, u32) = start;
    'b: loop {
        block_start(bus, r, img)?;
        match pc {
            (0, 0xe0) | (0, 0xe4) | (0, 0xec) => {
                let entered = pc.1;
                if entered <= 0xe0 {
                    // h0+$e0 move.l d1,($104).w
                    {
                        bus.sync(r);
                        let v = r.get(0x4, 4);
                        store(bus, img, 0x104u64 as u32, 4, v)?;
                    }
                }
                if entered <= 0xe4 {
                    insn_start(bus, r, img)?;
                    // h0+$e4 move.w #$20,($dff09c).l
                    {
                        bus.sync(r);
                        let v = 0x20u64;
                        store(bus, img, 0xdff09cu64 as u32, 2, v)?;
                    }
                }
                if entered <= 0xec {
                    insn_start(bus, r, img)?;
                    // h0+$ec rte
                    {
                        r.tick(const { cy::of(cy::Op::Rte, cy::Size::L, cy::Ea::None, cy::Ea::None) }); r.off_bus(const { cy::off_of(cy::Op::Rte, cy::Size::L, cy::Ea::None, cy::Ea::None) });
                        bus.sync(r);
                        let mut u_fff000: u64 = 0;
                        let v = load(bus, r.get(0x3c, 4) as u32, 2);
                        u_fff000 = v & 0xffff;
                        let v = int_add(r.get(0x3c, 4), 0x2u64, 4);
                        r.set(0x3c, 4, v);
                        let v = load(bus, r.get(0x3c, 4) as u32, 4);
                        r.set(0x50, 4, v);
                        let v = int_add(r.get(0x3c, 4), 0x4u64, 4);
                        r.set(0x3c, 4, v);
                        pcode_rt::ops::sr_set(r, u_fff000);
                        let v = r.get(0x50, 4);
                        r.set(PC, 4, v);
                        r.depth -= 1;
                        return Ok(());
                    }
                }
                pc = (0, 0xee);
            }
            k => {
                let res = enter(k, r, bus, img);
                r.depth -= 1;
                return res;
            }
        }
    }
}
