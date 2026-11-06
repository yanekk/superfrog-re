//! `level_init`'s first view (`fd_core::level::start`, plans/forest-dumb-level-1 T65) against the
//! original on the routine rig, 10 000 states of each cell routine's generator and 1 000 of each
//! whole-view loop's (`generators::level_start`), on the main program from the owner's disks.
//!
//! The engine lists each tile the original copies by CPU as a [`Blit::Tile`]; the adapter here makes
//! the original's CPU copy of it into memory (five words a line, 16 lines `$e6` bytes apart, from tile
//! `tile` at h3+$13540), so the rig compares every byte written. No register is compared: the loops
//! (`view_tiles`, `view_shapes`) set every register they use, the cell routines save and restore
//! theirs but `d4` and `d5`, which their loop never reads, and `level_init` calls `scena_screen`
//! next, which loads its own.

use fd_conform::generators::{self, h};
use fd_conform::judge::{self, Cpu};
use fd_conform::routine::{RegsCompared, Rig, RigReport};
use fd_core::PORTED;
use fd_core::level::redraw::{Blit, Io};
use fd_core::level::start;

const CASES: usize = 10_000;
/// States of the two whole-view loops: each runs its cell routine 368 times, so 1 000 states are
/// 368 000 cells, about 3 minutes; 10 000 took 32 (2026-11-06, all passed). The loops have no branch
/// but their counters, and every cell is the routine checked above over 10 000 states.
const WHOLE_CASES: usize = 1_000;

const SCREEN: u32 = h(3, 0x398);
const COLLISION: u32 = h(3, 0xfa50);
const TILES: u32 = h(3, 0x1_3540);

/// The first view's memory, seen through [`Io`]; a tile is the original's CPU copy.
struct CpuIo<'a>(&'a mut Cpu);

impl Io for CpuIo<'_> {
    fn rb(&self, a: u32) -> u8 {
        self.0.mem.rb(h(0, 0).wrapping_add(a))
    }
    fn clear_object(&mut self, _: u32) {
        unreachable!("the first view clears no object")
    }
    fn blit(&mut self, b: Blit) {
        let Blit::Tile { tile, dst } = b else {
            unreachable!("the first view cuts no bob")
        };
        let (mut src, mut d) = (
            TILES.wrapping_add(tile as u32 * 0xa0),
            SCREEN.wrapping_add(dst),
        );
        for _ in 0..16 {
            for p in 0..5 {
                let w = self.0.mem.rw(src);
                self.0.mem.ww(d.wrapping_add(p * 0x2e), w);
                src = src.wrapping_add(2);
            }
            d = d.wrapping_add(0xe6);
        }
    }
    fn collision_word(&mut self, at: u32, v: u16) {
        self.0.mem.ww(COLLISION.wrapping_add(at), v);
    }
}

fn view_tiles(cpu: &mut Cpu) {
    start::draw_tiles(&mut CpuIo(cpu));
}
fn view_tile(cpu: &mut Cpu) {
    let (d6, d7) = (cpu.d[6], cpu.d[7]);
    start::tile_cell(&mut CpuIo(cpu), d6, d7);
}
fn view_shapes(cpu: &mut Cpu) {
    start::draw_shapes(&mut CpuIo(cpu));
}
fn view_shape(cpu: &mut Cpu) {
    let (d6, d7) = (cpu.d[6], cpu.d[7]);
    start::shape_cell(&mut CpuIo(cpu), d6, d7);
}

/// Each routine: its name (`re/symbols.toml`, the generator's), its entry in hunk 0, the port.
type Routine = (&'static str, u32, fn(&mut Cpu));

fn routines() -> Vec<Routine> {
    vec![
        ("view_tiles", 0x62e, view_tiles),
        ("view_tile", 0x672, view_tile),
        ("view_shapes", 0x6c4, view_shapes),
        ("view_shape", 0x708, view_shape),
    ]
}

fn check(name: &str, off: u32, port: fn(&mut Cpu), cases: usize) -> RigReport {
    let mut r = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    let g = generators::generator(name).unwrap();
    judge::check_regs(&mut r, name, cases, h(0, off), &g, RegsCompared::NONE, port)
        .unwrap_or_else(|f| panic!("{f}"))
}

#[test]
fn every_t65_routine_is_registered() {
    for (name, off, _) in routines() {
        assert!(
            PORTED
                .iter()
                .any(|p| p.name == name && p.hunk == 0 && p.offset == off),
            "{name} is not in PORTED"
        );
        assert!(
            generators::generator(name).is_some(),
            "{name} has no generator"
        );
    }
}

#[test]
fn each_cell_routine_matches_the_original() {
    for (name, off, port) in routines().into_iter().filter(|r| r.0.ends_with("e")) {
        let rep = check(name, off, port, CASES);
        assert_eq!((rep.passed, rep.hangs, rep.faults), (CASES, 0, 0), "{name}");
        println!("{name}: {} paths", rep.paths);
    }
}

#[test]
fn each_whole_view_routine_matches_the_original() {
    for (name, off, port) in routines().into_iter().filter(|r| r.0.ends_with("s")) {
        let rep = check(name, off, port, WHOLE_CASES);
        assert_eq!(
            (rep.passed, rep.hangs, rep.faults),
            (WHOLE_CASES, 0, 0),
            "{name}"
        );
        println!("{name}: {} paths", rep.paths);
    }
}

/// `view_tile` with row and column swapped (the column clamped to a row): the rig must see a port
/// that reads the tile map down instead of across.
fn view_tile_with_the_columns_swapped(cpu: &mut Cpu) {
    let (d6, d7) = (cpu.d[6], cpu.d[7]);
    start::tile_cell(&mut CpuIo(cpu), d7.min(15), d6);
}

#[test]
fn a_tile_port_that_swaps_row_and_column_fails() {
    let mut r = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    let g = generators::generator("view_tile").unwrap();
    let f = judge::check_regs(
        &mut r,
        "view_tile",
        2000,
        h(0, 0x672),
        &g,
        RegsCompared::NONE,
        view_tile_with_the_columns_swapped,
    )
    .unwrap_err();
    assert!(f.failures > 0, "{f}");
}
