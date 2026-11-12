"""The whole-tree gate on the game (translator T16, its DESIGN §2.2, §2.3, §2.12): `translator_trial.py
--whole` exports the whole program, translates it, builds fd-judge optimised and judges every routine
with a registered generator on 2 000 states. Fails, never skips, without Ghidra, the disks or the
extracted program (DESIGN §2.14): the tool names the path it looked in. About 1 to 2 minutes warm, a
cold Ghidra import more; full set."""
import json
import os
import subprocess
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
TRIAL = os.path.join(HERE, 'translator_trial.py')
OUT = os.path.join(os.path.dirname(HERE), 'build', 'translator')
# Bounds the run: Ghidra's own import timeout is 1 800 s (DESIGN §5.2), the judging a few minutes.
SECONDS = 2700


class WholeTree(unittest.TestCase):
    def test_the_whole_tree_passes_with_no_hand_edit(self):
        p = subprocess.run([sys.executable, '-I', TRIAL, '--whole'], capture_output=True, text=True,
                           timeout=SECONDS)
        self.assertEqual(p.returncode, 0, p.stderr[-3000:])
        s = json.loads(p.stdout.strip().splitlines()[-1])
        with open(os.path.join(OUT, 'trial.json'), encoding='utf-8') as f:
            whole = json.load(f)['whole']
        # Stage 1 and 2: every instruction translated, every computed jump's targets found without a
        # hint; library calls are the machine's (DESIGN §2.8).
        self.assertEqual(s['unsupported_insns'], 0)
        self.assertEqual(s['unresolved_computed'], 0, whole['reading']['unresolved'])
        self.assertEqual(s['hints'], 0, whole['reading']['hints'])
        # Every judgeable routine passes on 2 000 states with no hint. 100 judgeable at T16: a drop
        # means a generator or the survey broke, not a pass.
        self.assertGreaterEqual(s['judgeable'], 100)
        with open(os.path.join(OUT, 'table.md'), encoding='utf-8') as f:
            table = f.read()
        self.assertEqual(s['pass'], s['judgeable'], table)
        self.assertEqual((s['untranslated'], s['write_into_code'], s['unsupported']), (0, 0, 0))
        self.assertTrue(s['gate'], whole['why'])


# The routines that differ on pcode-machine (translator T27), each because the routine oracle's
# hardware mode is not a real Amiga, never the machine (docs/14-translator.md, the plan's FINDINGS).
# routine -> (the oracle's limit, the evidence of the first failing state, a line of that state's
# first difference as fd-judge prints it). The states are seeded by the generator's name, so the
# line is the same every run; a different one means something else changed.
#
# beam: the oracle steps VHPOSR's byte by one a read, the machine from counted cycles. Each `d1` one
# is the line-change wait `move.b $dff006,d1; cmp.b $dff006,d1; beq` four times: from line b the
# oracle reads b, b+1 .. b+7 and leaves d1 = b+6; the machine waits a line each time.
# blit: a generated blit runs past the top of chip RAM; the oracle's blitter writes on into its flat
# 16 MB, the machine's wraps at 512 KB as the A500's does.
MACHINE_PINNED = {
    'h0+$1fec': ('beam', 'case 2, line $03: the original stores VHPOSR as the word $0304 (two reads, '
                 'one step); the machine, reaching the read 59 lines on, $3e86',
                 'byte h0+$8dfa: original wrote $03, port wrote $3e'),
    'h0+$2698': ('beam', 'case 7, line $18: two waits; the original reads $18..$27, d1 $26; the machine '
                 '$1b x12, $1c x17, $1d x18, $1e x17, $1f, then $22..$26, d1 $25',
                 'd1: original $00008126, port $00008125'),
    'h0+$26ec': ('beam', 'case 2, line $03: the original reads $03..$0a, d1 $09; the machine $05 x7, '
                 '$06 x17, $07 x18, $08 x17, $09, d1 $08',
                 'd1: original $00008f09, port $00008f08'),
    'h0+$3adc': ('beam', 'case 0, line $01: the original reads $01..$08, d1 $07; the machine $01 x17, '
                 '$02 x18, $03 x17, $04 x18, $05, d1 $04',
                 'd1: original $0a9a9f07, port $0a9a9f04'),
    'h0+$3b2a': ('beam', 'case 1, line $02: the original reads $02..$09, d1 $08; the machine $03 x6, '
                 '$04 x18, $05 x17, $06 x18, $07, d1 $06',
                 'd1: original $00000508, port $00000506'),
    'h0+$3e0c': ('beam', 'case 1067, line $34: the original reads $34..$3b, d1 $3a; the machine, reaching '
                 'the wait 33 lines on, $55 x9, $56 x18, $57 x17, $58 x17, $59, d1 $58',
                 'd1: original $00002a3a, port $00002a58'),
    'h0+$3e30': ('beam', 'case 584, line $49: the original reads $49..$50, d1 $4f; the machine $4a x6, '
                 '$4b x17, $4c x18, $4d x17, $4e, d1 $4d',
                 'd1: original $7ea8374f, port $7ea8374d'),
    'h0+$3ea8': ('beam', 'case 5, line $06: the original reads $06..$0d, d1 $0c; the machine $07 x18, '
                 '$08 x17, $09 x17, $0a x18, $0b, d1 $0a',
                 'd1: original $2db8bd0c, port $2db8bd0a'),
    'h0+$a06': ('blit', 'case 71: the original writes 23 bytes at $080004..$0805c4, the machine the same '
                '23 at $000004..$0005c4',
                'byte $000004: original left it $00, port wrote $1f'),
    'h0+$ce4': ('blit', 'case 16: the original writes 103 bytes at $08001e..$080b9f, the machine the '
                'same 103 at $00001e..$000b9f',
                'byte $00001e: original left it $00, port wrote $37'),
    'h0+$21c0': ('blit', 'case 285: the original writes 167 bytes at $080010..$080771, the machine the '
                 'same 167 at $000010..$000771',
                 'byte $000010: original left it $00, port wrote $18'),
    'h0+$29ca': ('blit', 'case 178: the original writes 86 bytes at $080028..$0807b5, the machine the '
                 'same 86 at $000028..$0007b5',
                 'byte $000028: original left it $00, port wrote $f4'),
}


class WholeTreeOnTheMachine(unittest.TestCase):
    """translator T27: `--whole --machine` judges every routine on pcode-machine. Done when is met
    with named exceptions (owner, 2026-11-12): every judgeable routine passes but the pinned ones,
    and each pinned one fails with its pinned first difference."""

    def test_every_routine_passes_on_the_machine_but_the_pinned_oracle_limits(self):
        p = subprocess.run([sys.executable, '-I', TRIAL, '--whole', '--machine'], capture_output=True,
                           text=True, timeout=SECONDS)
        self.assertEqual(p.returncode, 0, p.stderr[-3000:])
        s = json.loads(p.stdout.strip().splitlines()[-1])
        with open(os.path.join(OUT, 'trial-machine.json'), encoding='utf-8') as f:
            doc = json.load(f)
        self.assertEqual(doc['hardware'], 'machine')
        self.assertGreaterEqual(s['judgeable'], 100)
        failing = {r['routine']: r['plain']['first'] for r in doc['rows']
                   if not r['unjudgeable'] and r['plain']['verdict'] != 'pass'}
        self.assertEqual(sorted(failing), sorted(MACHINE_PINNED), 'a failure not pinned, or a pin now passing')
        for routine, (_, evidence, line) in MACHINE_PINNED.items():
            self.assertIn(line, failing[routine], f'{routine}: {evidence}')
        self.assertEqual(s['pass'], s['judgeable'] - len(MACHINE_PINNED))
        self.assertEqual((s['untranslated'], s['write_into_code'], s['unsupported']), (0, 0, 0))


if __name__ == '__main__':
    unittest.main()
