/* sf-oracle's C API over the vAmiga core (DESIGN §3.2). Test tooling only; never linked into sf-app.
 *
 * One `sf_amiga` owns one emulator, driven synchronously on the caller's thread: no emulator thread
 * is launched, and `sf_amiga_step` runs exactly one PAL frame (to the next vertical sync). Functions
 * that can fail return 0 on success and write a message into `err` (always NUL-terminated).
 *
 * An `sf_amiga` may be used from any one thread at a time; several may run on several threads.
 */
#ifndef SF_ORACLE_SHIM_H
#define SF_ORACLE_SHIM_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct sf_amiga sf_amiga;

/* One custom-chip register write that the capture keeps: the register's offset from 0xDFF000, the
 * value, and the beam position when it was made. */
typedef struct {
    uint16_t reg;
    uint16_t value;
    uint16_t vpos;
    uint16_t hpos;
} sf_custom_write;

/* One new value on a channel's DAC (Paula's penhi/penlo): the channel 0..3, the sample byte, the
 * volume 0..64 it is played at, and the beam position. The DAC holds it until the next one. */
typedef struct {
    uint8_t channel;
    int8_t sample;
    uint16_t volume;
    uint16_t vpos;
    uint16_t hpos;
} sf_audio_tap;

/* An interrupt the CPU took: its level, the beam position as its exception began, and whether that
 * was in the next frame (the step runs on into its start), not the one the step ran. */
typedef struct {
    uint8_t level;
    uint8_t next;
    uint16_t vpos;
    uint16_t hpos;
} sf_irq;

/* The host rate the core's own mixer runs at: stereo, channels 0/3 hard left and 1/2 hard right, no
 * filter, no adaptive rate, nearest-sample resampling. */
#define SF_AUDIO_RATE 48000

/* Input bits for sf_amiga_step. Directions, fire: the joystick in port 2. PORT1_BUTTON: the left
 * mouse button in port 1 (the same pin as a port-1 joystick's fire). */
enum {
    SF_JOY_UP = 1,
    SF_JOY_DOWN = 2,
    SF_JOY_LEFT = 4,
    SF_JOY_RIGHT = 8,
    SF_JOY_FIRE = 16,
    SF_JOY_PORT1_BUTTON = 32
};

/* A powered-off A500 (OCS, 512K chip + 512K slow, no RTC, RAM zeroed, never warping, PAL) with
 * `rom` as its Kickstart. NULL on failure. */
sf_amiga *sf_amiga_new(const uint8_t *rom, size_t rom_len, char *err, size_t err_len);
void sf_amiga_free(sf_amiga *a);

/* Put an ADF image (copied) into drive 0..3; the image file itself is never touched. */
int sf_amiga_insert_disk(sf_amiga *a, int drive, const uint8_t *adf, size_t len, char *err, size_t err_len);
int sf_amiga_eject_disk(sf_amiga *a, int drive, char *err, size_t err_len);

/* Write `len` bytes at `addr` between frames, as a debugger would. Refused unless every byte is in
 * chip or slow RAM as the CPU sees it now (the Kickstart's overlay maps ROM at 0 after a reset). */
int sf_amiga_write(sf_amiga *a, uint32_t addr, const uint8_t *bytes, size_t len, char *err, size_t err_len);

/* Connect (or disconnect) drive 1..3 before power-on; a new machine has drive 0 only. */
int sf_amiga_connect_drive(sf_amiga *a, int drive, int connected, char *err, size_t err_len);

int sf_amiga_power_on(sf_amiga *a, char *err, size_t err_len);

/* Hold `joy` (SF_JOY_* bits) and the keys whose raw Amiga key codes are set in `keys` (bit k of
 * byte k/8), then run one frame. */
int sf_amiga_step(sf_amiga *a, uint32_t joy, const uint8_t keys[16], char *err, size_t err_len);

/* The writes to the Paula-related registers made during the last step, in order: DMACON (0x96),
 * INTENA (0x9A), INTREQ (0x9C), ADKCON (0x9E) and AUD0..3 (0xA0..0xDE). Valid until the next step. */
size_t sf_amiga_paula_writes(const sf_amiga *a, const sf_custom_write **out);

/* The DAC values the four channels took during the last step, in order. Valid until the next step. */
size_t sf_amiga_audio_taps(const sf_amiga *a, const sf_audio_tap **out);

/* The interrupts the CPU took during the last step, in order. Valid until the next step. */
size_t sf_amiga_irqs(const sf_amiga *a, const sf_irq **out);

/* The core's own mix of the last step at SF_AUDIO_RATE: `2 * n` floats, left then right, for the
 * returned n. Valid until the next step. */
size_t sf_amiga_audio_out(const sf_amiga *a, const float **out);

/* How many lines the frame the last step ran has: 313 for a long frame, 312 for a short one. Agnus
 * alternates them while BPLCON0's LACE bit is set and keeps the last one when it is cleared. */
int sf_amiga_frame_lines(const sf_amiga *a);

/* The finished frame: `*stride` 32-bit entries per row (one per super-hires pixel, 0xAARRGGBB with
 * each channel the OCS 4-bit value times 17), `*rows` rows, row = vertical beam position, entry
 * 4 * h = lores horizontal position h as DIWSTRT counts it. Valid until the next step. */
const uint32_t *sf_amiga_texture(const sf_amiga *a, size_t *stride, size_t *rows);

/* Denise's DIWSTRT and DIWSTOP as they stand after the last step. */
void sf_amiga_diw(const sf_amiga *a, uint16_t *diwstrt, uint16_t *diwstop);

/* Chip and slow RAM: the address of the emulator's own buffers and their sizes in bytes. */
const uint8_t *sf_amiga_chip(const sf_amiga *a, size_t *len);
const uint8_t *sf_amiga_slow(const sf_amiga *a, size_t *len);
/* Write `len` bytes into chip or slow RAM from `addr` (between frames): test tooling's patches. */
void sf_amiga_poke(sf_amiga *a, uint32_t addr, const uint8_t *bytes, size_t len);

/* The 68000's registers as they stand after the last step (at the frame boundary): d0-d7, a0-a7
 * (a7 the active stack pointer), the address of the instruction being executed, SR, USP and SSP. */
typedef struct {
    uint32_t d[8];
    uint32_t a[8];
    uint32_t pc;
    uint32_t sr;
    uint32_t usp;
    uint32_t ssp;
} sf_cpu_regs;
void sf_amiga_cpu(const sf_amiga *a, sf_cpu_regs *out);

/* The whole machine state as a vAmiga snapshot. The returned buffer is freed with sf_buffer_free. */
uint8_t *sf_amiga_snapshot(sf_amiga *a, size_t *len, char *err, size_t err_len);
void sf_buffer_free(uint8_t *buf);

/* Restore a snapshot made by sf_amiga_snapshot. `joy`/`keys` are the inputs held when it was made. */
int sf_amiga_restore(sf_amiga *a, const uint8_t *snap, size_t len, uint32_t joy, const uint8_t keys[16],
                     char *err, size_t err_len);

#ifdef __cplusplus
}
#endif

#endif
