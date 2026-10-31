import os
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from find_file_names import find_file_names

TOOL = os.path.join(os.path.dirname(os.path.abspath(__file__)), "find_file_names.py")


class Scan(unittest.TestCase):
    def test_a_name_held_inline_after_a_jump_is_found(self):
        # bra.w over the string, as the game stores most of its names.
        self.assertEqual(
            find_file_names(b"\x60\x00\x00\x0c_BALLA\x00\x00\x00\x00\x4e\xb9"), ["_BALLA"]
        )

    def test_each_shape_of_name(self):
        data = b"\0".join(
            [b"", b"_MAPA1-1.Mapa", b"df0:!EAGLE_SOFT!", b"StworkiNew.rawm", b"HD_ID", b"DF0:ST", b""]
        )
        self.assertEqual(
            find_file_names(data),
            ["DF0:ST", "HD_ID", "StworkiNew.rawm", "_MAPA1-1.Mapa", "df0:!EAGLE_SOFT!"],
        )

    def test_script_lines_and_the_end_of_the_file_end_a_name(self):
        self.assertEqual(find_file_names(b"forest_dumb_forever\n"), ["forest_dumb_forever"])
        self.assertEqual(find_file_names(b"\0_END"), ["_END"])

    def test_sorted_and_once_each(self):
        self.assertEqual(find_file_names(b"_ZZZZ\0_AAAA\0_ZZZZ\0"), ["_AAAA", "_ZZZZ"])

    def test_binary_noise_gives_nothing(self):
        # Each of these sat in the real executable reading as text.
        noise = [b"t.By", b"Nu.9", b"g6B.", b":NuJ9", b"/!OP", b"ESL!", b"_B", b"!z", b"Hello", b"12.5"]
        self.assertEqual(find_file_names(b"\xff\0".join(noise) + b"\0"), [])
        self.assertEqual(find_file_names(bytes(range(256)) * 8), [])

    def test_a_run_that_does_not_end_like_a_string_is_ignored(self):
        self.assertEqual(find_file_names(b"\0_BALLA\xff\0"), [])
        self.assertEqual(find_file_names(b"w df0:     \0"), [])

    def test_system_resources_are_left_out(self):
        self.assertEqual(
            find_file_names(b"\0dos.library\0BNugraphics.library\0topaz.font\0"), []
        )

    def test_a_name_too_long_for_amigados_is_left_out(self):
        self.assertEqual(find_file_names(b"\0_" + b"a" * 30 + b"\0"), [])
        self.assertEqual(find_file_names(b"\0_" + b"a" * 29 + b"\0"), ["_" + "a" * 29])


class Command(unittest.TestCase):
    def test_prints_one_name_a_line(self):
        with tempfile.NamedTemporaryFile() as f:
            f.write(b"\0_NAPIS\0_END\0")
            f.flush()
            out = subprocess.run(
                [sys.executable, TOOL, f.name], capture_output=True, text=True, check=True
            )
        self.assertEqual(out.stdout, "_END\n_NAPIS\n")

    def test_wrong_arguments_say_how_to_use_it(self):
        out = subprocess.run([sys.executable, TOOL], capture_output=True, text=True)
        self.assertEqual(out.returncode, 2)
        self.assertIn("usage", out.stderr)


if __name__ == "__main__":
    unittest.main()
