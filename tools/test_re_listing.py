"""Tests for tools/re_listing.py. Needs the player's disks (SUPERFROG_DISKS or the main checkout)."""
import bisect, os, re, subprocess, sys, tempfile, unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import re_listing as R

EXES = {  # chunk id -> hunk kinds, in order
    'd1_c01': ['CODE'],
    'd3_c01': ['CODE'],
    'd3_c02': ['CODE'],
    'd1_c03': ['CODE', 'BSS', 'ABSENT', 'ABSENT'],
    'd3_c04': ['CODE', 'BSS', 'ABSENT', 'ABSENT'],
}


class Disks:
    disks = None
    chunks = {}

    @classmethod
    def load(cls):
        if cls.disks is None:
            try:
                cls.disks = R.load_disks()
            except SystemExit as e:
                raise AssertionError(str(e))
        return cls.disks

    @classmethod
    def chunk(cls, cid):
        if cid not in cls.chunks:
            cls.chunks[cid] = R.load_chunk(cid, cls.load())
        return cls.chunks[cid]


class HunkParse(unittest.TestCase):
    def test_five_exes_parse_and_account_for_every_byte(self):
        for cid, kinds in EXES.items():
            with self.subTest(cid):
                data = Disks.chunk(cid)
                exe = R.parse_exe(data)
                self.assertEqual([h.kind for h in exe.hunks], kinds)
                self.assertEqual(sum(n for _, _, n in exe.blocks), len(data))
                self.assertEqual(exe.blocks[0][0], 'HUNK_HEADER')
                self.assertEqual(exe.blocks[-1][0], 'HUNK_END')
                for h in exe.hunks:
                    self.assertEqual(len(h.data), h.size)
                # back to back from 0, as the game's loader lays them out
                ends = [h.base + h.size for h in exe.hunks]
                self.assertEqual([h.base for h in exe.hunks], [0] + ends[:-1])

    def test_relocations_are_applied(self):
        exe = R.parse_exe(Disks.chunk('d1_c03'))
        self.assertGreater(len(exe.relocs), 1000)
        targets = {t for t, _ in exe.relocs.values()}
        self.assertEqual(targets, {0, 1, 2, 3})
        for (h, o), (t, delta) in exe.relocs.items():
            got = int.from_bytes(exe.hunks[h].data[o:o + 4], 'big')
            self.assertEqual(got, exe.hunks[t].base + delta, f"h{h}+${o:x}")

    def test_absolute_references_show_as_hunk_offset_labels(self):
        exe = R.parse_exe(Disks.chunk('d1_c03'))
        text = R.listing('d1_c03', exe, [])
        lines = {int(l[:8], 16): l for l in text.splitlines() if re.match(r'[0-9a-f]{8}  ', l)}
        starts = sorted(lines)
        checked = 0
        for (h, o), (t, delta) in sorted(exe.relocs.items())[:200]:
            line = lines[starts[bisect.bisect_right(starts, exe.hunks[h].base + o) - 1]]
            if 'dc.' in line:
                continue
            self.assertIn(f"h{t}+${delta:x}", line)
            checked += 1
        self.assertGreater(checked, 100)


SYMS = '''
[[sym]]
exe = "d1_c01"
hunk = 0
offset = 0x0
name = "entry_jump"
kind = "func"

[[sym]]
exe = "d1_c01"
hunk = 0
offset = {target}
name = "test_target"
kind = "func"
note = "what the entry jumps to"
'''


class Symbols(unittest.TestCase):
    def test_symbol_is_a_label_and_named_in_xrefs(self):
        exe = R.parse_exe(Disks.chunk('d1_c01'))
        (_, _), (t, delta) = min(exe.relocs.items())
        syms = R.load_symbols('test.toml', SYMS.format(target=hex(delta)))
        text = R.listing('d1_c01', exe, syms, xrefs=True)
        lines = text.splitlines()
        i = lines.index('test_target:')
        self.assertIn('what the entry jumps to', lines[i - 1])
        self.assertIn('xref:', lines[i + 1])
        self.assertIn('entry_jump', lines[i + 1])
        self.assertTrue(lines[i + 2].startswith(f"{delta:08x}"))
        self.assertTrue(any('test_target' in l and l.startswith('00000000') for l in lines))

    def test_duplicate_name_is_rejected_with_line(self):
        text = '''[[sym]]
exe = "d1_c01"
hunk = 0
offset = 0x10
name = "a"
kind = "func"

[[sym]]
exe = "d1_c01"
hunk = 0
offset = 0x20
name = "a"
kind = "func"
'''
        with self.assertRaisesRegex(R.SymbolError, r'test\.toml:8: duplicate name a'):
            R.load_symbols('test.toml', text)

    def test_same_name_in_two_exes_is_fine(self):
        text = '[[sym]]\nexe = "x"\nhunk = 0\noffset = 0\nname = "a"\nkind = "func"\n' \
               '[[sym]]\nexe = "y"\nhunk = 0\noffset = 0\nname = "a"\nkind = "func"\n'
        self.assertEqual(len(R.load_symbols('t', text)), 2)

    def test_overlapping_data_is_rejected_with_line(self):
        text = '''# comment
[[sym]]
exe = "d1_c01"
hunk = 0
offset = 0x10
name = "tab"
kind = "table"
size = 8

[[sym]]
exe = "d1_c01"
hunk = 0
offset = 0x14
name = "v"
kind = "var"
size = 4
'''
        with self.assertRaisesRegex(R.SymbolError, r'test\.toml:10: v overlaps tab \(line 2\)'):
            R.load_symbols('test.toml', text)

    def test_data_needs_a_size_and_kind_is_checked(self):
        base = '[[sym]]\nexe = "x"\nhunk = 0\noffset = 0\nname = "a"\nkind = "{}"\n'
        with self.assertRaisesRegex(R.SymbolError, r't:1: data symbol a needs a size'):
            R.load_symbols('t', base.format('data'))
        with self.assertRaisesRegex(R.SymbolError, r't:1: kind .label. is not one of'):
            R.load_symbols('t', base.format('label'))

    def test_committed_symbols_load_and_point_at_the_atn_routine(self):
        syms = R.load_symbols()
        atn = [s for s in syms if s.name == 'atn_decompress']
        self.assertEqual({s.exe for s in atn}, {'d1_stage15', *EXES})
        for s in atn:
            with self.subTest(s.exe):
                code = R.load_exe(Disks.chunk(s.exe)).hunks[s.hunk].data
                # movem.l d2-d5/a2-a4,-(a7) then, ten bytes in, cmpi.l #'ATN!',(a0)+
                self.assertEqual(code[s.offset:s.offset + 4].hex(), '48e73c38')
                self.assertEqual(code[s.offset + 10:s.offset + 16], b'\x0c\x98ATN!')


class Run(unittest.TestCase):
    def test_listing_goes_outside_the_repo_and_leaves_the_tree_unchanged(self):
        def status():
            return subprocess.run(['git', 'status', '--porcelain', '--untracked-files=all'], cwd=R.ROOT,
                                  capture_output=True, text=True, check=True).stdout
        before = status()
        with tempfile.TemporaryDirectory() as home:
            env = dict(os.environ, HOME=home, SUPERFROG_DISKS=R.disks_dir())
            out = subprocess.run([sys.executable, os.path.join(R.ROOT, 'tools', 're_listing.py'),
                                  'd1_stage15', '--xrefs'], env=env, cwd=R.ROOT,
                                 capture_output=True, text=True)
            self.assertEqual(out.returncode, 0, out.stderr)
            path = os.path.join(home, '.cache', 'superfrog-re', 're', 'd1_stage15.lst')
            self.assertEqual(out.stdout.strip(), path)
            with open(path) as f:
                text = f.read()
            self.assertIn('\natn_decompress:\n', text)
            self.assertIn('\nhunk_load:\n', text)
        self.assertEqual(status(), before)


if __name__ == '__main__':
    unittest.main()
