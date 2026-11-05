"""call_tree.py: the scan on hand-assembled 68k, the rules on re/translate.toml, and the committed toml
against the real program when `fd-tools extract` has put it in build/ (plan forest-dumb-level-1, T34)."""
import os
import subprocess
import sys
import tempfile
import tomllib
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import call_tree as ct
import hunk_listing  # call_tree put toolkit/tools on the path

TOOL = os.path.join(HERE, "call_tree.py")


class Asm:
    """A few 68000 encodings, enough for call graphs, with labels resolved on build()."""

    def __init__(self):
        self.parts, self.labels = [], {}

    def at(self):
        return sum(len(p) if isinstance(p, bytes) else p[2] for p in self.parts)

    def label(self, name):
        self.labels[name] = self.at()
        return self

    def raw(self, hexs):
        self.parts.append(bytes.fromhex(hexs))
        return self

    def nops(self, n):
        return self.raw("4e71" * n)

    def rts(self):
        return self.raw("4e75")

    def _rel_w(self, opcode, name):
        self.parts.append((opcode, name, 4, self.at()))
        return self

    def bsr(self, name):
        return self._rel_w(0x6100, name)

    def beq(self, name):
        return self._rel_w(0x6700, name)

    def bra(self, name):
        return self._rel_w(0x6000, name)

    def build(self):
        out = b""
        for p in self.parts:
            if isinstance(p, bytes):
                out += p
            else:
                opcode, name, _, at = p
                disp = (self.labels[name] - (at + 2)) & 0xFFFF
                out += opcode.to_bytes(2, "big") + disp.to_bytes(2, "big")
        return out


def program(asm):
    return ct.Program(hunk_listing.load_exe(asm.build()))


def scan(asm, root="root", names=None):
    roots = [root] if isinstance(root, str) else root
    return ct.scan(program(asm), [asm.labels[r] for r in roots], names or {})


def by_off(entries):
    return {e["offset"]: e for e in entries}


class Scan(unittest.TestCase):
    def test_calls_branches_and_tail_jumps_are_followed(self):
        a = Asm()
        a.label("root").bsr("a").beq("skip").bsr("b").label("skip").bra("tail")
        a.label("a").nops(2).rts()
        a.label("b").rts()
        a.label("tail").nops(1).rts()
        e = by_off(scan(a)[0])
        self.assertEqual(sorted(e), [0, a.labels["a"], a.labels["b"]])
        root = e[0]
        # bsr, beq, bsr, bra, then the tail's nop and rts: the tail jump is own flow, not a call
        self.assertEqual(root["own"], 6)
        self.assertEqual(root["calls"], [f"h0+${a.labels['a']:x}", f"h0+${a.labels['b']:x}"])
        self.assertEqual(root["instructions"], 6 + 3 + 1)

    def test_a_shared_callee_is_counted_once_with_callees(self):
        a = Asm()
        a.label("root").bsr("x").bsr("y").rts()
        a.label("x").bsr("z").rts()
        a.label("y").bsr("z").rts()
        a.label("z").nops(9).rts()
        root = by_off(scan(a)[0])[0]
        self.assertEqual(root["instructions"], 3 + 2 + 2 + 10)

    def test_unfollowed_edges_are_named_by_kind(self):
        a = Asm()
        a.label("root")
        a.raw("4e90")       # jsr (a0)
        a.raw("4eaefe68")   # jsr -$198(a6)
        a.raw("4efb0002")   # jmp 2(pc,d0.w): its operand names the table, not where it goes
        a.nops(4).rts()     # the table's room: decoded only if the jump were wrongly followed
        entries, unf = scan(a)
        self.assertEqual([u["kind"] for u in unf], ["indirect-call", "kickstart", "indirect-jump"])
        self.assertEqual([u["at"] for u in unf], ["h0+$0", "h0+$2", "h0+$6"])
        self.assertEqual(entries[0]["own"], 3)

    def test_running_into_data_is_unfollowed(self):
        a = Asm()
        a.label("root").raw("ffff")
        _, unf = scan(a)
        self.assertEqual(unf[0]["kind"], "undecodable")

    def test_every_root_and_its_callees_are_listed_once(self):
        a = Asm()
        a.label("one").bsr("shared").rts()
        a.label("two").bsr("shared").rts()
        a.label("shared").rts()
        entries, _ = scan(a, root=["one", "two"])
        self.assertEqual([e["offset"] for e in entries], [0, a.labels["two"], a.labels["shared"]])

    def test_named_routines_take_their_symbol(self):
        a = Asm()
        a.label("root").bsr("a").rts().label("a").rts()
        entries, _ = scan(a, names={(0, 0): "game_frame"})
        self.assertEqual([e["name"] for e in entries], ["game_frame", f"h0_{a.labels['a']:x}"])


class Split(unittest.TestCase):
    def test_a_unit_over_the_limit_is_split_at_its_largest_callee(self):
        a = Asm()
        a.label("root").bsr("small").bsr("big").rts()
        a.label("small").nops(20).rts()
        a.label("big").nops(170).rts()
        root = by_off(scan(a)[0])[0]
        self.assertEqual(root["instructions"], 3 + 21 + 171)
        self.assertEqual(root["split_at"], [f"h0+${a.labels['big']:x}"])
        self.assertEqual(root["unit"], 3 + 21)
        self.assertFalse(root["by_hand"])

    def test_a_split_callee_takes_what_only_it_reaches(self):
        a = Asm()
        a.label("root").bsr("mid").bsr("shared").rts()
        a.label("mid").bsr("deep").bsr("shared").rts()
        a.label("deep").nops(150).rts()
        a.label("shared").nops(30).rts()
        e = by_off(scan(a)[0])
        root = e[0]
        self.assertEqual(root["split_at"], [f"h0+${a.labels['mid']:x}"])
        # shared is still reached directly, so it stays in the unit
        self.assertEqual(root["unit"], 3 + 31)

    def test_own_flow_over_the_limit_is_by_hand(self):
        a = Asm()
        a.label("root").nops(ct.LIMIT).rts()
        root = scan(a)[0][0]
        self.assertEqual(root["unit"], ct.LIMIT + 1)
        self.assertTrue(root["by_hand"])
        self.assertEqual(root["split_at"], [])

    def test_exactly_the_limit_fits(self):
        a = Asm()
        a.label("root").nops(ct.LIMIT - 1).rts()
        root = scan(a)[0][0]
        self.assertEqual((root["unit"], root["by_hand"], root["split_at"]), (ct.LIMIT, False, []))


def entry(**kw):
    e = {"name": "r", "hunk": 0, "offset": 0, "own": 10, "instructions": 10, "unit": 10,
         "subsystem": "character", "gen_task": "T35", "port_task": "T08", "calls": [], "split_at": []}
    e.update(kw)
    return e


class Rules(unittest.TestCase):
    def test_a_good_entry_passes(self):
        self.assertEqual(ct.problems({"routine": [entry()]}), [])

    def test_sound_takes_no_gen_task(self):
        errs = ct.problems({"routine": [entry(subsystem="sound")]})
        self.assertTrue(any("sound is out of scope" in e for e in errs), errs)
        e = entry(subsystem="sound")
        del e["gen_task"], e["port_task"]
        self.assertEqual(ct.problems({"routine": [e]}), [])

    def test_each_subsystem_goes_to_its_generator_task(self):
        errs = ct.problems({"routine": [entry(subsystem="camera", gen_task="T35")]})
        self.assertTrue(any("goes to T36" in e for e in errs), errs)

    def test_unassigned_is_refused(self):
        errs = ct.problems({"routine": [entry(subsystem="unassigned")]})
        self.assertTrue(errs)

    def test_over_the_limit_needs_split_or_by_hand(self):
        errs = ct.problems({"routine": [entry(unit=181)]})
        self.assertTrue(any("not split and not by_hand" in e for e in errs), errs)
        self.assertEqual(ct.problems({"routine": [entry(unit=181, by_hand=True)]}), [])
        errs = ct.problems({"routine": [entry(unit=180, by_hand=True)]})
        self.assertTrue(any("fits" in e for e in errs), errs)

    def test_every_call_has_an_entry(self):
        errs = ct.problems({"routine": [entry(calls=["h0+$40"])]})
        self.assertTrue(any("h0+$40, which has no entry" in e for e in errs), errs)

    def test_an_unfollowed_edge_needs_a_reason(self):
        doc = {"routine": [entry()], "unfollowed": [{"routine": "r", "at": "h0+$2", "kind": "kickstart"}]}
        self.assertTrue(any("no reason" in e for e in ct.problems(doc)))
        doc["unfollowed"][0]["reason"] = "dos.library Read"
        self.assertEqual(ct.problems(doc), [])

    def test_a_fresh_scan_finds_what_the_toml_lacks(self):
        a = Asm()
        a.label("root").bsr("a").raw("4e90").rts().label("a").rts()
        entries, unf = scan(a)
        doc = {"routine": [entry(own=3, instructions=4, unit=4, calls=[f"h0+${a.labels['a']:x}"])]}
        errs = ct.problems(doc, entries, unf)
        self.assertTrue(any("has no entry" in e for e in errs), errs)
        self.assertTrue(any("indirect-call at h0+$4" in e and "not listed" in e for e in errs), errs)

    def test_a_rewrite_keeps_the_classification(self):
        a = Asm()
        a.label("root").bsr("a").rts().label("a").rts()
        entries, unf = scan(a)
        old = {"routine": [entry(subsystem="camera", gen_task="T36", port_task="T09", note="kept")]}
        ct.merge(entries, unf, old)
        doc = tomllib.loads(ct.render(entries, unf))
        r0, r1 = doc["routine"]
        self.assertEqual((r0["subsystem"], r0["gen_task"], r0["port_task"], r0["note"]),
                         ("camera", "T36", "T09", "kept"))
        self.assertEqual(r1["subsystem"], "unassigned")


class Shares(unittest.TestCase):
    def test_shares_sum_own_instructions_and_flag_the_limits(self):
        rs = [entry(name=f"r{i}", offset=i, own=10) for i in range(16)]
        rs.append(entry(name="s", offset=99, subsystem="sound", gen_task=None, port_task=None, own=500))
        port, gen = ct.shares({"routine": rs})
        self.assertEqual(port, {"T08": [16, 160, []]})
        self.assertIn("| T08 | 16 | 160 |  | yes |", ct.summary_text({"routine": rs}))


class RealProgram(unittest.TestCase):
    """The committed toml against game_frame's tree in the real program (needs fd-tools extract)."""

    def test_the_committed_toml_matches_the_program(self):
        if not os.path.exists(ct.EXE):
            self.skipTest(f"{ct.EXE} not extracted")
        out = subprocess.run([sys.executable, TOOL, "--check"], capture_output=True, text=True)
        self.assertEqual(out.returncode, 0, out.stdout + out.stderr)

    def test_a_missing_program_says_how_to_get_it(self):
        with tempfile.TemporaryDirectory() as d:
            out = subprocess.run([sys.executable, TOOL, "--check", "--exe", os.path.join(d, "nope")],
                                 capture_output=True, text=True)
        self.assertNotEqual(out.returncode, 0)
        self.assertIn("fd-tools -- extract", out.stderr)


if __name__ == "__main__":
    unittest.main()
