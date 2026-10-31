#!/usr/bin/env python3
"""Classify and unpack Forest Dumb Forever's files (docs/08-packers.md).

    unpack.py <infile> <outfile>   unpack one packed file
    unpack.py --all                every packed file from build/files to build/unpacked
    unpack.py --table              the classification table docs/08-packers.md carries

Two packers occur. `ESL!` is PowerPacker's format under another magic; `S404` is StoneCracker 4
inside a self-unpacking executable. Neither has a permissive decoder (the PowerPacker crates are the
GPL format198x family and the LGPL newtua-amiga), so each is unpacked by the original's own 68k routine under Musashi: the
playbook's step 2 reference, and exact by construction. The ESL! routine is cut from the main
program, so unpacking needs build/files from `fd-tools extract`.
"""
import argparse
import collections
import hashlib
import math
import struct
import sys
from pathlib import Path

GAME = Path(__file__).resolve().parent.parent
BUILD = GAME / 'build'
MAIN_PROGRAM = BUILD / 'files' / 'disk1' / 'FOREST_DUMB_FOREVER'

HUNK_HEADER = b'\x00\x00\x03\xf3'
# Every packer's magic this checks. Matched at offset 0, and inside the first 4 KiB of an executable,
# where a self-unpacking stub keeps its payload. Searching whole files would hit short magics by chance.
SIGNATURES = {
    b'ESL!': 'ESL! (PowerPacker format)',
    b'PP20': 'PowerPacker', b'PX20': 'PowerPacker (encrypted)', b'PP11': 'PowerPacker 1.1',
    b'RNC\x01': 'RNC method 1', b'RNC\x02': 'RNC method 2',
    b'IMP!': 'Imploder', b'ATN!': 'Imploder clone', b'MADE': 'Imploder clone', b'FIMP': 'File Imploder',
    b'LZX': 'LZX', b'XPKF': 'XPK', b'CrM!': 'CrunchMania', b'CrM2': 'CrunchMania',
    b'S300': 'StoneCracker 3', b'S310': 'StoneCracker 3', b'S400': 'StoneCracker 4',
    b'S401': 'StoneCracker 4', b'S403': 'StoneCracker 4', b'S404': 'StoneCracker 4',
    b'TPWM': 'Turbo Packer', b'ICE!': 'Pack-Ice',
}
STUB_WINDOW = 4096

# The main program's ESL! routine: file offset = first hunk's data (0x30) + hunk offset 0x1d28, the
# instruction after its absolute-address setup, so D0 = packed size, A0 = source, A1 = destination
# can be set from here. It ends at its own RTS, and every branch in it is PC-relative.
ESL_ROUTINE = (0x30 + 0x1d28, 0x30 + 0x1e10)
ESL_ROUTINE_SHA1 = '6c8909e4b76ead6cc5a7e11fd729a11523d4b478'

# The S404 stub's second hunk: its unpacking loop ends at hunk offset 0xdc, where it moves on to
# allocating memory through exec. Stopping there leaves the unpacked image starting at A5, with A2 back at A5.
S404_STOP = 0xdc

CODE, SRC, DST, STACK, RETURN = 0x1000, 0x100000, 0x400000, 0x300000, 0x7f0000
STOP_INSN = bytes.fromhex('4e722700')  # STOP #$2700
RAM_KIB = 8192
MAX_CYCLES = 2_000_000_000  # a bad stream ends here instead of running for ever


def signature(data):
    """The magic found in `data`, or None. Where to look is the note on SIGNATURES."""
    for magic in SIGNATURES:
        if data.startswith(magic):
            return magic
    if data.startswith(HUNK_HEADER):
        window = data[:STUB_WINDOW]
        for magic in SIGNATURES:
            if magic in window:
                return magic
    return None


def entropy(data):
    """Bits per byte. Near 8 with no signature would point at an unknown packer."""
    if not data:
        return 0.0
    return -sum(c / len(data) * math.log2(c / len(data)) for c in collections.Counter(data).values())


class Unpacker:
    def __init__(self, main_program=MAIN_PROGRAM):
        # Imported here so classifying needs no emulator.
        from machine68k import Machine, CPUType
        self.machine = Machine(CPUType.M68000, RAM_KIB)
        self.main_program = Path(main_program)
        self.routine = None

    def _esl_routine(self):
        if self.routine is None:
            if not self.main_program.exists():
                raise SystemExit(f'{self.main_program} is missing: run `cargo run -p fd-tools -- extract` first')
            routine = self.main_program.read_bytes()[slice(*ESL_ROUTINE)]
            if hashlib.sha1(routine).hexdigest() != ESL_ROUTINE_SHA1:
                raise ValueError(f'{self.main_program}: the ESL! routine is not at its known offset')
            self.routine = routine
        return self.routine

    def esl(self, data):
        # The routine trusts its input and has no failure path, so a damaged file would otherwise
        # come back as plausible bytes of the size its tail claims.
        size = int.from_bytes(data[-4:-1], 'big')
        if len(data) < 16 or len(data) % 4 or not 0 < size <= RETURN - DST or data[-1] > 32:
            raise ValueError(f'not a well-formed ESL! file ({len(data)} bytes, claims {size} unpacked)')
        mem, cpu = self.machine.mem, self.machine.cpu
        mem.w_block(CODE, self._esl_routine())
        mem.w_block(RETURN, STOP_INSN)
        mem.clear_block(DST, size + 16, 0)
        mem.w_block(SRC, data)
        cpu.pulse_reset()
        cpu.w_reg(0, len(data))
        cpu.w_reg(8, SRC)
        cpu.w_reg(9, DST)
        cpu.w_sp(STACK)
        mem.w32(STACK, RETURN)
        cpu.w_pc(CODE)
        self.machine.execute(MAX_CYCLES)
        if cpu.r_pc() != RETURN + len(STOP_INSN):
            raise ValueError(f'the ESL! routine did not return (pc {cpu.r_pc():#x})')
        # A0 back at the stream's first long, D5 holding its end marker and at most one unread bit.
        if cpu.r_reg(8) != SRC + 8 or cpu.r_reg(5) > 3:
            raise ValueError(f'the ESL! stream did not end where it starts (a0 {cpu.r_reg(8):#x}, d5 {cpu.r_reg(5):#x})')
        return bytes(mem.r_block(DST, size))

    def s404(self, data):
        if not data.startswith(HUNK_HEADER):
            raise ValueError('S404 outside a self-unpacking executable: no routine for that')
        # Hunk header: magic, no names, table size, first, last, one size per hunk, then each
        # hunk's HUNK_CODE, length in longs and data. The payload's hunk is the last one.
        count = struct.unpack_from('>I', data, 8)[0]
        sizes = struct.unpack_from(f'>{count}I', data, 20)
        at = 20 + 4 * count
        for _ in range(count):
            longs = struct.unpack_from('>I', data, at + 4)[0]
            hunk, at = data[at + 8:at + 8 + 4 * longs], at + 8 + 4 * longs
        room = (sizes[-1] & 0x3fffffff) * 4
        magic = hunk.find(b'S404')
        if magic < 0:
            raise ValueError('no S404 payload in the last hunk')
        size = struct.unpack_from('>I', hunk, magic + 8)[0]
        mem, cpu = self.machine.mem, self.machine.cpu
        mem.clear_block(SRC, room, 0)
        mem.w_block(SRC, hunk)
        mem.w_block(SRC + S404_STOP, STOP_INSN)
        cpu.pulse_reset()
        cpu.w_sp(STACK)
        cpu.w_pc(SRC)
        self.machine.execute(MAX_CYCLES)
        start = cpu.r_reg(13)  # A5, the output's start
        if cpu.r_pc() != SRC + S404_STOP + len(STOP_INSN) or cpu.r_reg(10) != start:
            raise ValueError(f'the S404 stub did not finish (pc {cpu.r_pc():#x})')
        return bytes(mem.r_block(start, size))

    def unpack(self, data):
        magic = signature(data)
        if magic == b'ESL!':
            return self.esl(data)
        if magic == b'S404':
            return self.s404(data)
        if magic is None:
            raise ValueError('not packed')
        raise ValueError(f'{SIGNATURES[magic]}: no unpacker for this packer')


def disk_files():
    files = sorted(p for p in (BUILD / 'files').rglob('*') if p.is_file())
    if not files:
        raise SystemExit(f'nothing in {BUILD / "files"}: run `cargo run -p fd-tools -- extract` first')
    return files


def table():
    """The classification table, one row per file, in disk and name order."""
    unpacker = Unpacker()
    rows = ['| File | Bytes | First bytes | Entropy | Packer | Unpacked bytes | Unpacked SHA-1 |',
            '|---|---|---|---|---|---|---|']
    for path in disk_files():
        data = path.read_bytes()
        magic = signature(data)
        packer, size, sha = 'none', '', ''
        if magic:
            out = unpacker.unpack(data)
            packer, size, sha = SIGNATURES[magic], str(len(out)), hashlib.sha1(out).hexdigest()
        name = path.relative_to(BUILD / 'files').as_posix()
        rows.append(f'| `{name}` | {len(data)} | `{data[:8].hex()}` | {entropy(data):.2f} | {packer} | {size} | {sha} |')
    return '\n'.join(rows) + '\n'


def unpack_all():
    unpacker = Unpacker()
    written = 0
    for path in disk_files():
        data = path.read_bytes()
        if signature(data):
            out = BUILD / 'unpacked' / path.relative_to(BUILD / 'files')
            out.parent.mkdir(parents=True, exist_ok=True)
            out.write_bytes(unpacker.unpack(data))
            written += 1
    print(f'{written} files unpacked to {BUILD / "unpacked"}')


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--all', action='store_true')
    parser.add_argument('--table', action='store_true')
    parser.add_argument('infile', nargs='?')
    parser.add_argument('outfile', nargs='?')
    args = parser.parse_args()
    if args.table:
        sys.stdout.write(table())
    elif args.all:
        unpack_all()
    elif args.infile and args.outfile:
        Path(args.outfile).write_bytes(Unpacker().unpack(Path(args.infile).read_bytes()))
    else:
        parser.error('give <infile> <outfile>, --all or --table')


if __name__ == '__main__':
    main()
