"""Tests for tools/template_sweep.py. The sweep's own test needs the player's disks (SUPERFROG_DISKS
or the main checkout), as test_re_listing.py does."""
import os, sys, unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import re_listing as R
import template_sweep as S


def lea_template(t):
    """`lea obj_templates+t*$ae, a0`, its long relocated."""
    return bytes.fromhex('41f9') + (S.TEMPLATES + t * S.SIZE).to_bytes(4, 'big')


class Walk(unittest.TestCase):
    def code(self, data, relocs):
        size = S.TEMPLATES + S.SIZE * S.COUNT
        data = data + bytes(size - len(data))
        exe = R.Exe(raw=False, hunks=[R.Hunk(0, 'CODE', 'any', size, 0, data)])
        exe.relocs = {(0, at): (0, int.from_bytes(data[at:at + 4], 'big')) for at in relocs}
        return S.Code(exe)

    def test_a_walk_follows_calls_and_branches_and_stops_at_returns(self):
        # lea t2; jsr $20; bra.b over a lea of t7 to an rts; a lea of t8 after it; at $20 lea t5.
        code = bytearray()
        code += lea_template(2)                       # 0
        code += bytes.fromhex('4eb9') + (0x20).to_bytes(4, 'big')   # 6: jsr $20
        code += bytes.fromhex('6006')                 # c: bra.b $14
        code += lea_template(7)                       # e: never reached
        code += bytes.fromhex('4e75')                 # 14: rts
        code += lea_template(8)                       # 16: after the rts, never reached
        code += bytes(0x20 - len(code))
        code += lea_template(5)                       # 20
        code += bytes.fromhex('4e75')                 # 26: rts
        c = self.code(bytes(code), [2, 8, 0x10, 0x18, 0x22])
        got, _ = c.walk([0])
        self.assertEqual(sorted(got), [2, 5])

    def test_a_routine_set_into_a_slot_is_found(self):
        # move.l #$30, $4(a0); rts — and at $30 a lea of template 9.
        code = bytearray(bytes.fromhex('217c') + (0x30).to_bytes(4, 'big') + bytes.fromhex('0004'))
        code += bytes.fromhex('4e75')
        code += bytes(0x30 - len(code))
        code += lea_template(9) + bytes.fromhex('4e75')
        c = self.code(bytes(code), [2, 0x32])
        got, routines = c.walk([0])
        self.assertEqual((got, routines), ({}, {0x30}))
        self.assertEqual(sorted(c.walk(sorted(routines))[0]), [9])


class Sweep(unittest.TestCase):
    def test_the_maps_spawn_129_templates_and_three_more_sit_in_unreferenced_code(self):
        try:
            disks = R.load_disks()
        except SystemExit as e:
            raise AssertionError(str(e))
        code = S.Code(R.load_exe(R.load_chunk('d1_c03', disks)))
        sources = S.sweep(code, S.level_markers(disks))
        self.assertEqual(len(sources), 129)
        self.assertIn('marker 27 (table 2: W3-1, W3-2, W3-3, W3-4)', sources[66])
        self.assertEqual(sources[159], ['template 151'])
        unreached = {t for t in code.references() if t not in sources}
        self.assertTrue({3, 41, 73, 86} <= unreached, unreached)


if __name__ == '__main__':
    unittest.main()
