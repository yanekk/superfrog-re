/* sf-oracle's routine runner (T06): the Moira 68000 core alone, over a flat 24-bit memory, with no
 * chipset. Test tooling only; never linked into sf-app.
 *
 * Moira is compiled from the vAmiga source tree that tools/setup/oracle.sh fetched, as a private copy
 * in its own namespace (build.rs), because vAmiga's copy has its memory interface bound to the
 * emulated Amiga. One `sf_cpu` may be used from one thread at a time; several may run on several.
 */
#ifndef SF_ORACLE_MOIRA_SHIM_H
#define SF_ORACLE_MOIRA_SHIM_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct sf_cpu sf_cpu;

/* The flat memory: 16 MiB, every 24-bit address, zeroed. */
#define SF_CPU_MEM_SIZE 0x1000000u

typedef struct {
    uint32_t d[8];
    uint32_t a[8];
    uint16_t sr;
} sf_regs;

/* How a call ended. */
enum {
    SF_CALL_RETURNED = 0,   /* the routine returned to the sentinel */
    SF_CALL_TIMEOUT = 1,    /* max_instr instructions ran without returning */
    SF_CALL_CUSTOM = 2,     /* an access to $DFF000-$DFF1FF or $BF0000-$BFFFFF (pc, addr) */
    SF_CALL_EXCEPTION = 3,  /* the CPU took an exception (pc, vector) */
    SF_CALL_HALTED = 4      /* the CPU halted (a double fault) (pc) */
};

typedef struct {
    uint32_t status;
    uint32_t pc;      /* the instruction that caused it */
    uint32_t addr;    /* SF_CALL_CUSTOM: the address accessed */
    uint16_t vector;  /* SF_CALL_EXCEPTION: the exception vector number */
    uint64_t instr;   /* instructions executed */
} sf_call_status;

sf_cpu *sf_cpu_new(void);
void sf_cpu_free(sf_cpu *c);

/* The flat memory, SF_CPU_MEM_SIZE bytes, for the caller to fill between calls. */
uint8_t *sf_cpu_mem(sf_cpu *c);

/* Reset the CPU, start it at `entry` with `regs` (SR first, then D0-D7/A0-A7, A7 being the stack
 * pointer SR selects), and run until the PC reaches `sentinel`, an error, or `max_instr` instructions.
 * The caller has already pushed `sentinel` as the return address. Every byte written from here on is
 * journalled; custom-chip writes are not performed. On return `regs` holds the final registers. */
void sf_cpu_call(sf_cpu *c, uint32_t entry, sf_regs *regs, uint32_t sentinel, uint64_t max_instr,
                 sf_call_status *out);

/* The bytes written since the last rollback: each address once, in first-write order, with the value
 * it held before. Valid until the next call or rollback. */
size_t sf_cpu_journal(const sf_cpu *c, const uint32_t **addrs, const uint8_t **olds);

/* Put every journalled byte back and empty the journal. */
void sf_cpu_rollback(sf_cpu *c);

#ifdef __cplusplus
}
#endif

#endif
