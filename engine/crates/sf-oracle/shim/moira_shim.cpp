// sf-oracle's routine runner over Moira (see moira_shim.h). Compiled by build.rs together with a copy
// of Moira's sources whose `vamiga` namespace is renamed by a -D flag, so this copy and the one inside
// libVACore never collide at link time. Moira is built with MOIRA_VIRTUAL_API false, so its memory
// and delegate functions are plain members defined here, as vAmiga's CPU.cpp defines its own.

#include "moira_shim.h"

#include "Moira.h"

#include <algorithm>
#include <cstring>
#include <map>
#include <new>
#include <vector>

namespace vamiga::moira {

namespace {

constexpr u32 kMemMask = SF_CPU_MEM_SIZE - 1;

// $DFF000-$DFF1FF (the custom chips) and $BF0000-$BFFFFF (the CIAs).
inline bool is_hardware(u32 a) { return (a & 0xFFFE00) == 0xDFF000 || (a & 0xFF0000) == 0xBF0000; }

}  // namespace

class FlatCpu : public Moira {

public:

    std::vector<u8> mem;
    // One bit per address: already in the journal.
    std::vector<u64> dirty;
    std::vector<u32> jaddr;
    std::vector<u8> jold;

    // The reset vectors handed to Moira's reset instead of memory's first eight bytes.
    u32 resetSsp = 0, resetPc = 0;
    bool overrideVectors = false;

    sf_call_status status {};

    // Hardware mode (sf_cpu_set_hw): custom-chip writes are recorded instead of refused, VHPOSR
    // counts up on every read so raster waits end, and the CIAs read 0 and ignore writes.
    bool hw = false;
    u16 beam = 0;
    std::vector<sf_hw_write> hwWrites;
    // Bytes a hardware read returns (sf_cpu_set_hw_read), by address; any other reads 0.
    std::map<u32, u8> hwReads;
    // The blitter's registers ($DFF040-$DFF074, by word): a write to BLTSIZE runs the blit at
    // once, in area mode, into this memory (journalled like the CPU's writes).
    u16 blt[0x1c] = {};

    u16 mem16(u32 a) { a &= kMemMask & ~1u; return u16(mem[a] << 8 | mem[a + 1]); }
    void put16(u32 a, u16 v) { a &= kMemMask & ~1u; wr8(a, u8(v >> 8)); wr8(a + 1, u8(v)); }

    void blit(u16 size) {
        auto r = [&](u32 off) { return blt[(off - 0x40) / 2]; };
        auto ptr = [&](u32 off) { return (u32(r(off)) << 16 | r(off + 2)) & 0x7FFFE; };
        u16 con0 = r(0x40), con1 = r(0x42);
        if (con1 & 1) return;  // line mode: not used by the routines tested
        bool desc = con1 & 2;
        int ash = con0 >> 12, bsh = con1 >> 12;
        bool useA = con0 & 0x800, useB = con0 & 0x400, useC = con0 & 0x200, useD = con0 & 0x100;
        u8 mt = u8(con0);
        u32 h = size >> 6 ? size >> 6 : 1024, w = size & 63 ? size & 63 : 64;
        u32 apt = ptr(0x50), bpt = ptr(0x4c), cpt = ptr(0x48), dpt = ptr(0x54);
        i32 step = desc ? -2 : 2;
        i32 amod = i16(r(0x64)), bmod = i16(r(0x62)), cmod = i16(r(0x60)), dmod = i16(r(0x66));
        if (desc) { amod = -amod; bmod = -bmod; cmod = -cmod; dmod = -dmod; }
        u16 adat = r(0x74), bdat = r(0x72), cdat = r(0x70);
        u16 aold = 0, bold = 0;
        for (u32 y = 0; y < h; y++) {
            for (u32 x = 0; x < w; x++) {
                u16 a = useA ? mem16(apt) : adat;
                if (useA) apt += step;
                if (x == 0) a &= r(0x44);
                if (x == w - 1) a &= r(0x46);
                u16 b = useB ? mem16(bpt) : bdat;
                if (useB) bpt += step;
                u16 c = useC ? mem16(cpt) : cdat;
                if (useC) cpt += step;
                u16 as, bs;
                if (!desc) {
                    as = u16((u32(aold) << 16 | a) >> ash);
                    bs = u16((u32(bold) << 16 | b) >> bsh);
                } else {
                    as = u16(((u32(a) << 16 | aold) << ash) >> 16);
                    bs = u16(((u32(b) << 16 | bold) << bsh) >> 16);
                }
                aold = a;
                bold = b;
                u16 d = 0;
                if (mt & 0x80) d |= as & bs & c;
                if (mt & 0x40) d |= as & bs & ~c;
                if (mt & 0x20) d |= as & ~bs & c;
                if (mt & 0x10) d |= as & ~bs & ~c;
                if (mt & 0x08) d |= ~as & bs & c;
                if (mt & 0x04) d |= ~as & bs & ~c;
                if (mt & 0x02) d |= ~as & ~bs & c;
                if (mt & 0x01) d |= ~as & ~bs & ~c;
                if (useD) { put16(dpt, d); dpt += step; }
            }
            if (useA) apt += amod;
            if (useB) bpt += bmod;
            if (useC) cpt += cmod;
            if (useD) dpt += dmod;
        }
    }

    void custom_write(u32 a, u16 v) {
        u32 reg = a & 0x1FE;
        hwWrites.push_back({ u16(reg), v });
        if (reg >= 0x40 && reg < 0x78) {
            blt[(reg - 0x40) / 2] = v;
            if (reg == 0x58) blit(v);
        }
    }

    static bool is_custom(u32 a) { return (a & 0xFFFE00) == 0xDFF000; }

    explicit FlatCpu(Amiga &ref) : Moira(ref), mem(SF_CPU_MEM_SIZE, 0), dirty(SF_CPU_MEM_SIZE / 64, 0) { }

    void fault(u32 a) {
        if (status.status == SF_CALL_RETURNED) {
            status.status = SF_CALL_CUSTOM;
            status.pc = reg.pc0;
            status.addr = a;
        }
    }

    u8 rd8(u32 a) {
        a &= kMemMask;
        if (is_hardware(a)) {
            if (!hw) { fault(a); return 0; }
            if (a == 0xDFF006 || a == 0xDFF007) return u8(beam++);
            auto it = hwReads.find(a);
            return it == hwReads.end() ? 0 : it->second;
        }
        return mem[a];
    }

    void wr8(u32 a, u8 v) {
        a &= kMemMask;
        if (is_hardware(a)) {
            if (!hw) { fault(a); return; }
            // A byte written to a custom register reaches both halves of the data bus.
            if (is_custom(a)) custom_write(a, u16(v << 8 | v));
            return;
        }
        u64 bit = u64(1) << (a & 63);
        if (!(dirty[a >> 6] & bit)) {
            dirty[a >> 6] |= bit;
            jaddr.push_back(a);
            jold.push_back(mem[a]);
        }
        mem[a] = v;
    }

    // The last word on the data bus (vAmiga's `Memory::dataBus`): what a CPU read of a
    // write-only custom register writes into it (vAmiga `peekCustomFaulty16`, after UAE).
    u16 bus = 0;

    static bool custom_readable(u32 reg) {
        return (reg >= 0x002 && reg <= 0x01E) || reg == 0x07C || (reg >= 0x180 && reg <= 0x1BE);
    }

    u16 rd16(u32 a) {
        u32 m = a & kMemMask;
        if (hw && is_custom(m) && !custom_readable(m & 0x1FE)) {
            custom_write(m & ~1u, bus);
            return bus;
        }
        u16 v = u16(rd8(a) << 8 | rd8(a + 1));
        if (!is_hardware(m)) bus = v;
        return v;
    }
    void wr16(u32 a, u16 v) {
        if (hw && is_hardware(a & kMemMask)) {
            if (is_custom(a & kMemMask)) custom_write(a & kMemMask, v);
            return;
        }
        wr8(a, u8(v >> 8));
        wr8(a + 1, u8(v));
    }

    void rollback() {
        for (size_t i = jaddr.size(); i-- > 0;) {
            mem[jaddr[i]] = jold[i];
            dirty[jaddr[i] >> 6] = 0;
        }
        jaddr.clear();
        jold.clear();
    }

    void call(u32 entry, sf_regs *r, u32 sentinel, u64 max_instr, sf_call_status *out) {
        status = {};
        hwWrites.clear();
        std::fill(std::begin(blt), std::end(blt), u16(0));
        bus = 0;
        resetSsp = r->a[7];
        resetPc = entry;
        overrideVectors = true;
        reset();
        overrideVectors = false;
        setClock(0);
        setSR(r->sr);
        for (int n = 0; n < 8; n++) setD(n, r->d[n]);
        for (int n = 0; n < 8; n++) setA(n, r->a[n]);
        setPC0(getPC());

        u64 n = 0;
        while (status.status == SF_CALL_RETURNED) {
            if (getPC() == sentinel) break;
            if (n == max_instr) {
                status.status = SF_CALL_TIMEOUT;
                status.pc = getPC();
                break;
            }
            execute();
            n++;
            if (isHalted() && status.status == SF_CALL_RETURNED) {
                status.status = SF_CALL_HALTED;
                status.pc = getPC0();
            }
        }
        status.instr = n;
        status.cycles = u64(getClock());

        for (int i = 0; i < 8; i++) r->d[i] = getD(i);
        for (int i = 0; i < 8; i++) r->a[i] = getA(i);
        r->sr = getSR();
        *out = status;
    }
};

namespace { inline FlatCpu &self(const Moira *m) { return *const_cast<FlatCpu *>(static_cast<const FlatCpu *>(m)); } }

void Moira::sync(int cycles) { clock += cycles; }
u8 Moira::read8(u32 addr) const { return self(this).rd8(addr); }
u16 Moira::read16(u32 addr) const { return self(this).rd16(addr); }
u32 Moira::read32(u32 addr) const { return u32(read16(addr)) << 16 | read16(addr + 2); }
u16 Moira::read16Dasm(u32 addr) const { return self(this).mem[addr & kMemMask] << 8 | self(this).mem[(addr + 1) & kMemMask]; }

u16 Moira::read16OnReset(u32 addr) const
{
    FlatCpu &c = self(this);
    if (c.overrideVectors && addr < 8) {
        u32 v = addr < 4 ? c.resetSsp : c.resetPc;
        return addr & 2 ? u16(v) : u16(v >> 16);
    }
    return read16(addr);
}

void Moira::write8(u32 addr, u8 val) const { self(this).wr8(addr, val); }
void Moira::write16(u32 addr, u16 val) const { self(this).wr16(addr, val); }
void Moira::write32(u32 addr, u32 val) const { write16(addr, u16(val >> 16)); write16(addr + 2, u16(val)); }
u8 Moira::dsack(u32) const { return DSACK_16; }
u16 Moira::readIrqUserVector(u8) const { return 0; }

void Moira::cpuDidReset() { }
void Moira::cpuDidHalt() { }
void Moira::willExecute(const char *, Instr, Mode, Size, u16) { }
void Moira::didExecute(const char *, Instr, Mode, Size, u16) { }

void Moira::willExecute(M68kException, u16 vector)
{
    FlatCpu &c = self(this);
    if (c.status.status == SF_CALL_RETURNED) {
        c.status.status = SF_CALL_EXCEPTION;
        c.status.pc = reg.pc0;
        c.status.vector = vector;
    }
}

void Moira::didExecute(M68kException, u16) { }
void Moira::willInterrupt(u8) { }
void Moira::didJumpToVector(int, u32) { }
void Moira::didChangeCACR(u32) { }
void Moira::didChangeCAAR(u32) { }
void Moira::didReachSoftstop(u32) { }
void Moira::didReachBreakpoint(u32) { }
void Moira::didReachWatchpoint(u32) { }
void Moira::didReachCatchpoint(u8) { }
void Moira::didReachSoftwareTrap(u32) { }
void Moira::didLogInstruction(const Registers &) { }

}  // namespace vamiga::moira

using vamiga::moira::FlatCpu;

// Moira's base class keeps the reference and never uses it; this is all it is given.
static char g_noAmiga;

struct sf_cpu {
    FlatCpu cpu { *reinterpret_cast<vamiga::Amiga *>(&g_noAmiga) };
};

extern "C" {

sf_cpu *sf_cpu_new(void) { return new (std::nothrow) sf_cpu; }
void sf_cpu_free(sf_cpu *c) { delete c; }
uint8_t *sf_cpu_mem(sf_cpu *c) { return c->cpu.mem.data(); }

void sf_cpu_call(sf_cpu *c, uint32_t entry, sf_regs *regs, uint32_t sentinel, uint64_t max_instr,
                 sf_call_status *out)
{
    c->cpu.call(entry, regs, sentinel, max_instr, out);
}

size_t sf_cpu_journal(const sf_cpu *c, const uint32_t **addrs, const uint8_t **olds)
{
    *addrs = c->cpu.jaddr.data();
    *olds = c->cpu.jold.data();
    return c->cpu.jaddr.size();
}

void sf_cpu_rollback(sf_cpu *c) { c->cpu.rollback(); }

void sf_cpu_set_hw(sf_cpu *c, int on) { c->cpu.hw = on != 0; }

void sf_cpu_set_hw_read(sf_cpu *c, uint32_t addr, uint8_t value)
{
    c->cpu.hwReads[addr & 0xFFFFFF] = value;
}

size_t sf_cpu_hw_writes(const sf_cpu *c, const sf_hw_write **out)
{
    *out = c->cpu.hwWrites.data();
    return c->cpu.hwWrites.size();
}

}
