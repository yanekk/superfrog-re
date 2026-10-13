// sf-oracle's C API over the vAmiga core; see shim.h. Built by sf-oracle's build.rs against the core
// that tools/setup/oracle.sh builds with engine/crates/sf-oracle/patches applied:
//   0001 installs `vamiga_custom_write_hook`, called on every custom-register write (CPU and Copper);
//   0004 installs `vamiga_audio_sample_hook`, called whenever a channel's DAC takes a new value;
//   0005 installs `vamiga_irq_hook`, called as the CPU starts an interrupt's exception;
//   0002 makes `SfShimAccess` a friend of `Emulator`, so the emulator can be stepped without its thread.

#include "shim.h"

#include "vaconfig.h"
#include "VAmiga.h"
#include "Emulator.h"

#include <cstdio>
#include <cstring>
#include <exception>
#include <memory>
#include <mutex>
#include <stdexcept>
#include <vector>

namespace vamiga {

extern void (*vamiga_custom_write_hook)(u32 addr, u16 value, int accessor, isize vpos, isize hpos);
extern void (*vamiga_audio_sample_hook)(int channel, i8 sample, u16 volume, isize vpos, isize hpos);
extern void (*vamiga_irq_hook)(int level, isize vpos, isize hpos);

// Reaches Emulator's private stepping functions (patch 0002). vAmiga normally runs these on its own
// thread, paced by the wall clock; the oracle calls them directly, one frame per call.
struct SfShimAccess {

    static Amiga &main(Emulator &e) { return e.main; }

    static void powerOn(Emulator &e)
    {
        e.isReady();
        e.state = ExecState::PAUSED;
        e._powerOn();
        e.state = ExecState::RUNNING;
        e._run();
    }

    // ~VAmiga halts through the thread's state machine, which refuses an emulator that was never
    // launched; mark it halted so teardown goes straight to freeing.
    static void halted(Emulator &e)
    {
        e.state = ExecState::HALTED;
    }

    /* vAmiga snapshots leave out what the ports and keyboard hold (the joystick's axes and
     * button, the mouse buttons, the keyboard's key-down matrix): after a restore those are
     * whatever the instance last saw. Seat them to the input held when the snapshot was taken,
     * without sending any event the game would see. The mouse buttons debounce over 2 frames;
     * they are seated as stable, which is exact unless a button changed in the 2 frames before
     * the snapshot.
     */
    static void seatInput(Amiga &a, uint32_t joy, const uint8_t *keys)
    {
        auto &stick = a.controlPort2.joystick;
        stick.trigger(GamePadAction::RELEASE_XY);
        stick.trigger(GamePadAction::RELEASE_FIRE);
        if (joy & SF_JOY_UP) stick.trigger(GamePadAction::PULL_UP);
        if (joy & SF_JOY_DOWN) stick.trigger(GamePadAction::PULL_DOWN);
        if (joy & SF_JOY_LEFT) stick.trigger(GamePadAction::PULL_LEFT);
        if (joy & SF_JOY_RIGHT) stick.trigger(GamePadAction::PULL_RIGHT);
        if (joy & SF_JOY_FIRE) stick.trigger(GamePadAction::PRESS_FIRE);

        auto &mouse = a.controlPort1.mouse;
        mouse.leftButton.init();
        mouse.middleButton.init();
        mouse.rightButton.init();
        if (joy & SF_JOY_PORT1_BUTTON) mouse.leftButton.set(true, 0);

        for (int code = 0; code < 128; code++) {
            a.keyboard.keyDown[code] = keys[code / 8] >> (code % 8) & 1;
        }
    }

    static void step(Emulator &e)
    {
        e.update();
        e.computeFrame();
    }
};

}

using namespace vamiga;

namespace {

// The DMA debugger's copy of a line's bus owners (private to it): reached by the explicit
// instantiation rule, which skips access checks. For timing studies only.
template <typename Tag, typename Tag::type M> struct Rob {
    friend typename Tag::type get(Tag) { return M; }
};
struct OwnersTag {
    using type = BusOwner (DmaDebugger::*)[HPOS_CNT];
    friend type get(OwnersTag);
};
template struct Rob<OwnersTag, &DmaDebugger::busOwner>;

// The registers kept from the write hook (DESIGN §2.1 (2), FINDINGS 2026-10-11).
bool isPaulaReg(u32 reg)
{
    return reg == 0x96 || reg == 0x9A || reg == 0x9C || reg == 0x9E || (reg >= 0xA0 && reg <= 0xDE);
}

// The capture the hook appends to: the one being stepped on this thread, if any.
thread_local std::vector<sf_custom_write> *capturing = nullptr;

// The blits started (BLTSIZE written) during the step being run on this thread, if any.
thread_local std::vector<sf_custom_write> *blitting = nullptr;

void hook(u32 addr, u16 value, int, isize vpos, isize hpos)
{
    if (capturing && isPaulaReg(addr)) {
        capturing->push_back(sf_custom_write { u16(addr), value, u16(vpos), u16(hpos) });
    }
    if (blitting && addr == 0x58) {
        blitting->push_back(sf_custom_write { u16(addr), value, u16(vpos), u16(hpos) });
    }
}

thread_local std::vector<sf_audio_tap> *tapping = nullptr;

void sampleHook(int channel, i8 sample, u16 volume, isize vpos, isize hpos)
{
    if (tapping) {
        tapping->push_back(sf_audio_tap { u8(channel), sample, volume, u16(vpos), u16(hpos) });
    }
}

thread_local std::vector<sf_irq> *interrupting = nullptr;
// The machine being stepped and the frame its step runs.
thread_local Amiga *interrupted = nullptr;
thread_local i64 stepFrame = 0;

void irqHook(int level, isize vpos, isize hpos)
{
    if (interrupting) {
        u8 next = interrupted && interrupted->agnus.pos.frame > stepFrame;
        interrupting->push_back(sf_irq { u8(level), next, u16(vpos), u16(hpos) });
    }
}

void fail(char *err, size_t len, const char *what)
{
    if (err && len) { std::snprintf(err, len, "%s", what); }
}

// Runs `f`, turning a C++ exception into a return value of 1 and a message.
template <typename F> int guarded(char *err, size_t len, const char *doing, F f)
{
    try {
        f();
        return 0;
    } catch (std::exception &e) {
        char msg[512];
        std::snprintf(msg, sizeof(msg), "%s: %s", doing, e.what());
        fail(err, len, msg);
    } catch (...) {
        char msg[512];
        std::snprintf(msg, sizeof(msg), "%s: unknown error", doing);
        fail(err, len, msg);
    }
    return 1;
}

}

struct sf_amiga {

    VAmiga va;
    std::vector<sf_custom_write> writes;
    uint32_t joy = 0;
    uint8_t keys[16] = { };
    std::vector<sf_audio_tap> taps;
    std::vector<sf_irq> irqs;
    std::vector<float> mixed;
    int lines = 0;
    bool profile = false;
    std::vector<sf_line_sample> samples;
    std::vector<sf_custom_write> blits;

    Emulator &emu() { return *va.emu; }
    Amiga &amiga() { return SfShimAccess::main(*va.emu); }

    // State a snapshot does not carry or overwrites; set before power-on and after every restore.
    // The palette and texel format make the texture exact 12-bit colour (FINDINGS 2026-10-11). The
    // ports' device is otherwise "detected" on first use and not saved, so a restored run would
    // read port 1 differently from the run it was taken in: an A500 has a mouse in port 1 and the
    // joystick in port 2, from the start.
    void fixedSettings()
    {
        emu().set(Opt::MON_PALETTE, (i64)Palette::RGB);
        emu().set(Opt::HOST_TEX_FORMAT, (i64)TexFormat::ARGB);
        amiga().controlPort1.setDevice(ControlPortDevice::MOUSE);
        amiga().controlPort2.setDevice(ControlPortDevice::JOYSTICK);
        // The core's mix, kept for sf-paula's comparison (shim.h, SF_AUDIO_RATE): the hardware's
        // pairing with nothing added, at a fixed rate (the adaptive rate follows the wall clock).
        emu().set(Opt::HOST_SAMPLE_RATE, SF_AUDIO_RATE);
        emu().set(Opt::AUD_ASR, false);
        emu().set(Opt::AUD_FASTPATH, false);
        emu().set(Opt::AUD_SAMPLING_METHOD, (i64)SamplingMethod::NONE);
        emu().set(Opt::AUD_FILTER_TYPE, (i64)FilterType::NONE);
        for (auto [opt, pan] : { std::pair { Opt::AUD_PAN0, 300 }, { Opt::AUD_PAN1, 100 },
                                 { Opt::AUD_PAN2, 100 }, { Opt::AUD_PAN3, 300 } }) {
            emu().set(opt, pan); // 300: all left, 100: all right (AudioPort::setOption)
        }
        for (auto opt : { Opt::AUD_VOL0, Opt::AUD_VOL1, Opt::AUD_VOL2, Opt::AUD_VOL3 }) emu().set(opt, 100);
        emu().set(Opt::AUD_VOLL, 50);
        emu().set(Opt::AUD_VOLR, 50);
    }

    void configure()
    {
        // Nobody listens: without this the queue fills and the core warns on every message.
        amiga().msgQueue.disable();
        emu().set(ConfigScheme::A500_OCS_1MB);
        emu().set(Opt::AMIGA_VIDEO_FORMAT, (i64)TV::PAL);
        emu().set(Opt::AMIGA_WARP_MODE, (i64)Warp::NEVER);
        emu().set(Opt::AMIGA_WARP_BOOT, 0);
        emu().set(Opt::AMIGA_VSYNC, false);
        emu().set(Opt::AMIGA_RUN_AHEAD, 0);
        emu().set(Opt::RTC_MODEL, (i64)RTCRevision::NONE);
        emu().set(Opt::MEM_FAST_RAM, 0);
        emu().set(Opt::MEM_RAM_INIT_PATTERN, (i64)RamInitPattern::ALL_ZEROES);
        emu().set(Opt::DRIVE_CONNECT, true, { 0 });
        emu().set(Opt::DRIVE_CONNECT, false, { 1, 2, 3 });
        fixedSettings();
    }

    // One frame with the CPU paused at the end of every line to be sampled (sf_amiga_set_profile).
    // The end-of-line trap pauses the core between two instructions; it goes on from there. No trap
    // is set on the frame's last lines, so a pause never falls between the frame's end and the
    // vertical sync's flag; one that lands past the frame's end swallowed that flag (both raised
    // within one long instruction, a blit holding the bus) and ends the step there.
    void profiledFrame()
    {
        samples.clear();
        auto &m = amiga();
        bool first = true;
        isize last = -1;
        for (;;) {
            m.agnus.dmaDebugger.eolTrap = m.agnus.pos.v < 309;
            try {
                if (first) {
                    first = false;
                    SfShimAccess::step(emu());
                } else {
                    m.computeFrame();
                }
                break;
            } catch (StateChangeException &) {
                if (m.agnus.pos.v < last) break;
                last = m.agnus.pos.v;
                sf_line_sample s { u16(m.agnus.pos.v), u16(m.agnus.pos.h), m.cpu.getPC0(),
                                   u32(m.cpu.getSR()), m.cpu.getSP(), { } };
                for (int k = 0; k < SF_STACK_LONGS; k++) {
                    s.stack[k] = m.mem.spypeek32<Accessor::CPU>(s.sp + 4 * u32(k));
                }
                auto &owners = m.agnus.dmaDebugger.*get(OwnersTag());
                for (int h = 0; h < 228 && h < HPOS_CNT; h++) s.owners[h] = u8(owners[h]);
                samples.push_back(s);
            }
        }
        m.agnus.dmaDebugger.eolTrap = false;
    }

    // Queue the commands that turn the held input into `j`/`k`; update() applies them.
    void input(uint32_t j, const uint8_t *k)
    {
        auto stick = [&](GamePadAction a) { emu().put(Cmd::JOY_EVENT, GamePadCmd { .port = 1, .action = a }); };
        auto changed = [&](uint32_t bit) { return ((joy ^ j) & bit) != 0; };

        if (changed(SF_JOY_UP | SF_JOY_DOWN)) {
            stick(GamePadAction::RELEASE_Y);
            if (j & SF_JOY_UP) stick(GamePadAction::PULL_UP);
            if (j & SF_JOY_DOWN) stick(GamePadAction::PULL_DOWN);
        }
        if (changed(SF_JOY_LEFT | SF_JOY_RIGHT)) {
            stick(GamePadAction::RELEASE_X);
            if (j & SF_JOY_LEFT) stick(GamePadAction::PULL_LEFT);
            if (j & SF_JOY_RIGHT) stick(GamePadAction::PULL_RIGHT);
        }
        if (changed(SF_JOY_FIRE)) {
            stick(j & SF_JOY_FIRE ? GamePadAction::PRESS_FIRE : GamePadAction::RELEASE_FIRE);
        }
        if (changed(SF_JOY_PORT1_BUTTON)) {
            auto a = j & SF_JOY_PORT1_BUTTON ? GamePadAction::PRESS_LEFT : GamePadAction::RELEASE_LEFT;
            emu().put(Cmd::MOUSE_BUTTON, GamePadCmd { .port = 0, .action = a });
        }
        for (int code = 0; code < 128; code++) {
            bool was = keys[code / 8] >> (code % 8) & 1, now = k[code / 8] >> (code % 8) & 1;
            if (was != now) {
                emu().put(now ? Cmd::KEY_PRESS : Cmd::KEY_RELEASE, KeyCmd { .keycode = KeyCode(code), .delay = 0.0 });
            }
        }
        joy = j;
        std::memcpy(keys, k, sizeof(keys));
    }
};

extern "C" {

sf_amiga *sf_amiga_new(const uint8_t *rom, size_t rom_len, char *err, size_t err_len)
{
    static std::once_flag installed;
    std::call_once(installed, [] {
        vamiga_custom_write_hook = hook;
        vamiga_audio_sample_hook = sampleHook;
        vamiga_irq_hook = irqHook;
    });
    // vAmiga fills its static table of option defaults on first use without a lock; a machine set
    // up on another thread meanwhile can read it half-filled ("Invalid key: ROM_PATH").
    static std::mutex setting_up;
    std::lock_guard<std::mutex> one_at_a_time(setting_up);
    auto a = std::make_unique<sf_amiga>();
    auto rc = guarded(err, err_len, "setting up the emulator", [&] {
        a->emu().initialize();
        a->configure();
        a->va.mem.loadRom(rom, isize(rom_len));
    });
    if (rc) {
        SfShimAccess::halted(a->emu());
        return nullptr;
    }
    return a.release();
}

void sf_amiga_free(sf_amiga *a)
{
    if (!a) return;
    SfShimAccess::halted(a->emu());
    delete a;
}

int sf_amiga_insert_disk(sf_amiga *a, int drive, const uint8_t *adf, size_t len, char *err, size_t err_len)
{
    if (drive < 0 || drive > 3) { fail(err, err_len, "no such drive"); return 1; }
    return guarded(err, err_len, "inserting a disk", [&] {
        a->va.df[drive]->insert(std::span<const u8>(adf, len), ImageFormat::ADF, false);
    });
}

int sf_amiga_eject_disk(sf_amiga *a, int drive, char *err, size_t err_len)
{
    if (drive < 0 || drive > 3) { fail(err, err_len, "no such drive"); return 1; }
    return guarded(err, err_len, "ejecting a disk", [&] { a->va.df[drive]->ejectDisk(); });
}

int sf_amiga_write(sf_amiga *a, uint32_t addr, const uint8_t *bytes, size_t len, char *err, size_t err_len)
{
    auto &mem = a->amiga().mem;
    for (size_t i = 0; i < len; i++) {
        auto src = mem.cpuMemSrc[((addr + i) & 0xFFFFFF) >> 16];
        if (src != MemSrc::CHIP && src != MemSrc::SLOW) {
            fail(err, err_len, "the address is not chip or slow RAM as the CPU sees it now");
            return 1;
        }
    }
    return guarded(err, err_len, "writing memory", [&] { mem.patch(addr, bytes, isize(len)); });
}

int sf_amiga_connect_drive(sf_amiga *a, int drive, int connected, char *err, size_t err_len)
{
    if (drive < 1 || drive > 3) { fail(err, err_len, "drive 0 is always connected; 1..3 can be"); return 1; }
    return guarded(err, err_len, "connecting a drive", [&] {
        a->emu().set(Opt::DRIVE_CONNECT, connected != 0, { drive });
    });
}

int sf_amiga_power_on(sf_amiga *a, char *err, size_t err_len)
{
    return guarded(err, err_len, "powering on", [&] { SfShimAccess::powerOn(a->emu()); });
}

int sf_amiga_step(sf_amiga *a, uint32_t joy, const uint8_t keys[16], char *err, size_t err_len)
{
    a->writes.clear();
    a->taps.clear();
    a->irqs.clear();
    a->input(joy, keys);
    a->blits.clear();
    capturing = &a->writes;
    blitting = &a->blits;
    tapping = &a->taps;
    interrupting = &a->irqs;
    interrupted = &a->amiga();
    stepFrame = a->amiga().agnus.pos.frame;
    // A step runs from just after one frame's start to just after the next one's (the CPU finishes
    // its instruction), so the frame it runs is the one starting now.
    a->lines = int(a->amiga().agnus.pos.vCnt());
    auto rc = guarded(err, err_len, "running a frame", [&] {
        if (a->profile) {
            a->profiledFrame();
        } else {
            try {
                SfShimAccess::step(a->emu());
            } catch (StateChangeException &) {
                throw std::runtime_error("the emulator stopped mid-frame (a CPU halt or a breakpoint)");
            }
        }
        // Keep the frame's mix and empty the stream, so it never overflows. Exactly what is there
        // is taken: asking the port for more counts as an underflow, which pads the stream.
        auto &stream = a->amiga().audioPort.stream;
        auto n = stream.count();
        a->mixed.resize(size_t(2 * n));
        stream.copyInterleaved(a->mixed.data(), n);
    });
    capturing = nullptr;
    blitting = nullptr;
    tapping = nullptr;
    interrupting = nullptr;
    interrupted = nullptr;
    return rc;
}

void sf_amiga_set_profile(sf_amiga *a, int on)
{
    a->profile = on != 0;
    a->emu().set(Opt::XRAY_MODE, (i64)(on > 1 ? XRayMode::XRAY_DMA : XRayMode::XRAY_NONE));
    a->samples.clear();
}

size_t sf_amiga_line_samples(const sf_amiga *a, const sf_line_sample **out)
{
    *out = a->samples.data();
    return a->samples.size();
}

size_t sf_amiga_blit_starts(const sf_amiga *a, const sf_custom_write **out)
{
    *out = a->blits.data();
    return a->blits.size();
}

size_t sf_amiga_paula_writes(const sf_amiga *a, const sf_custom_write **out)
{
    *out = a->writes.data();
    return a->writes.size();
}

size_t sf_amiga_audio_taps(const sf_amiga *a, const sf_audio_tap **out)
{
    *out = a->taps.data();
    return a->taps.size();
}

size_t sf_amiga_irqs(const sf_amiga *a, const sf_irq **out)
{
    *out = a->irqs.data();
    return a->irqs.size();
}

int sf_amiga_frame_lines(const sf_amiga *a)
{
    return a->lines;
}

size_t sf_amiga_audio_out(const sf_amiga *a, const float **out)
{
    *out = a->mixed.data();
    return a->mixed.size() / 2;
}

const uint32_t *sf_amiga_texture(const sf_amiga *a, size_t *stride, size_t *rows)
{
    auto &t = const_cast<sf_amiga *>(a)->emu().getTexture();
    *stride = 2 * HPIXELS;
    *rows = VPIXELS;
    return reinterpret_cast<const uint32_t *>(t.pixels.ptr);
}

void sf_amiga_diw(const sf_amiga *a, uint16_t *diwstrt, uint16_t *diwstop)
{
    auto &info = const_cast<sf_amiga *>(a)->va.denise.getInfo();
    *diwstrt = info.diwstrt;
    *diwstop = info.diwstop;
}

const uint8_t *sf_amiga_chip(const sf_amiga *a, size_t *len)
{
    auto &mem = const_cast<sf_amiga *>(a)->amiga().mem;
    *len = size_t(mem.getConfig().chipSize);
    return mem.chip;
}

const uint8_t *sf_amiga_slow(const sf_amiga *a, size_t *len)
{
    auto &mem = const_cast<sf_amiga *>(a)->amiga().mem;
    *len = size_t(mem.getConfig().slowSize);
    return mem.slow;
}

void sf_amiga_poke(sf_amiga *a, uint32_t addr, const uint8_t *bytes, size_t len)
{
    auto &mem = a->amiga().mem;
    size_t chip = size_t(mem.getConfig().chipSize), slow = size_t(mem.getConfig().slowSize);
    for (size_t k = 0; k < len; k++) {
        uint32_t at = uint32_t(addr + k) & 0xFFFFFF;
        if (at < chip) mem.chip[at] = bytes[k];
        else if (at >= 0xC00000 && at - 0xC00000 < slow) mem.slow[at - 0xC00000] = bytes[k];
    }
}

void sf_amiga_cpu(const sf_amiga *a, sf_cpu_regs *out)
{
    auto &cpu = const_cast<sf_amiga *>(a)->amiga().cpu;
    for (int i = 0; i < 8; i++) {
        out->d[i] = cpu.getD(i);
        out->a[i] = cpu.getA(i);
    }
    out->pc = cpu.getPC0();
    out->sr = cpu.getSR();
    out->usp = cpu.getUSP();
    out->ssp = cpu.getISP();
}

uint8_t *sf_amiga_snapshot(sf_amiga *a, size_t *len, char *err, size_t err_len)
{
    uint8_t *out = nullptr;
    guarded(err, err_len, "taking a snapshot", [&] {
        Snapshot snap(a->amiga());
        *len = size_t(snap.data.size);
        out = static_cast<uint8_t *>(std::malloc(*len));
        if (!out) throw std::bad_alloc();
        std::memcpy(out, snap.data.ptr, *len);
    });
    return out;
}

void sf_buffer_free(uint8_t *buf)
{
    std::free(buf);
}

int sf_amiga_restore(sf_amiga *a, const uint8_t *snap, size_t len, uint32_t joy, const uint8_t keys[16],
                     char *err, size_t err_len)
{
    return guarded(err, err_len, "restoring a snapshot", [&] {
        a->va.amiga.loadSnapshot(Snapshot(snap, isize(len)));
        SfShimAccess::seatInput(a->amiga(), joy, keys);
        a->fixedSettings();
        a->joy = joy;
        std::memcpy(a->keys, keys, sizeof(a->keys));
    });
}

}
