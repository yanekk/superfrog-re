"""Tests for tools/check_tier_names.py: the banned set, its two exemptions and what is scanned."""
import contextlib, io, os, subprocess, sys, tempfile, unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import check_tier_names as C

PATTERNS = C.banned_patterns(['superfrog', 'forest-dumb'])


def offends(line):
    return C.line_offends(line, PATTERNS)


class Tokens(unittest.TestCase):
    def test_a_bare_slug(self):
        self.assertTrue(offends('see superfrog here'))

    def test_an_uppercase_name(self):
        self.assertTrue(offends('SUPERFROG and FOREST-DUMB'))
        self.assertTrue(offends('Superfrog'))

    def test_hyphen_underscore_and_space_are_interchangeable(self):
        for line in ('forest-dumb', 'forest_dumb', 'forest dumb', 'forest - dumb', 'Forest  Dumb'):
            self.assertTrue(offends(line), line)

    def test_the_full_title_matches_the_slug(self):
        self.assertTrue(offends('Forest Dumb Forever'))
        self.assertTrue(offends('forest dumb forever'))

    def test_the_crate_prefix(self):
        for line in ('sf-core', 'use sf_core::Game;', 'SF-PAULA', '`sf-plan`'):
            self.assertTrue(offends(line), line)

    def test_the_prefix_is_not_a_word_ending(self):
        for line in ('transf-x', 'pdsf_y', 'sfcore'):
            self.assertFalse(offends(line), line)

    def test_the_studio(self):
        for line in ('team17', 'Team 17', 'TEAM-17', 'team_17'):
            self.assertTrue(offends(line), line)
        self.assertFalse(offends('team 18'))

    def test_a_clean_line(self):
        self.assertFalse(offends('use amiga_chips::Picture;'))


class Exemptions(unittest.TestCase):
    def test_a_learned_on_line_anywhere_in_it(self):
        self.assertFalse(offends('const X: u32 = 3; // value learned on: Superfrog'))
        self.assertFalse(offends('learned on: Team 17 and sf-core'))

    def test_a_learned_on_line_behind_any_comment_leader(self):
        for leader in ('///', '//', '#', ';', '*'):
            self.assertFalse(offends(f'{leader} learned on: Superfrog'), leader)

    def test_the_marker_exempts_its_own_line_only(self):
        self.assertTrue(offends('learned on Superfrog'))
        self.assertTrue(offends('Learned On: Superfrog'))

    def test_the_repo_name_is_exempt(self):
        self.assertFalse(offends('~/src/superfrog-re/x'))
        self.assertFalse(offends('Application Support/Superfrog-RE/settings.toml'))

    def test_the_repo_name_does_not_shield_the_game_name(self):
        self.assertTrue(offends('superfrog-re holds superfrog'))
        self.assertTrue(offends('superfrog-render'))
        self.assertTrue(offends('superfrog-re-env'))


class Scan(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = self.tmp.name
        subprocess.run(['git', 'init', '-q'], cwd=self.root, check=True)
        self.write('games/superfrog/docs/x.md', 'superfrog sf-core team 17\n')

    def write(self, rel, text):
        path = os.path.join(self.root, rel)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, 'w') as f:
            f.write(text)

    def test_toolkit_is_scanned_with_path_and_line(self):
        self.write('toolkit/docs/a.md', 'clean\nas superfrog does\n')
        self.assertEqual(C.offences(self.root), ['toolkit/docs/a.md:2'])

    def test_knowledge_is_scanned(self):
        self.write('knowledge/a.md', 'sf-plan\n')
        self.assertEqual(C.offences(self.root), ['knowledge/a.md:1'])

    def test_games_plans_and_root_files_are_not_scanned(self):
        self.write('plans/p/DESIGN.md', 'superfrog\n')
        self.write('CLAUDE.md', 'superfrog\n')
        self.write('tools/a.py', 'superfrog\n')
        self.assertEqual(C.offences(self.root), [])

    def test_ignored_files_are_not_scanned(self):
        self.write('.gitignore', 'target/\n')
        self.write('toolkit/target/a.txt', 'superfrog\n')
        self.assertEqual(C.offences(self.root), [])

    def test_a_new_game_folder_bans_its_slug(self):
        self.write('toolkit/a.md', 'as newname-two does\n')
        self.assertEqual(C.offences(self.root), [])
        self.write('games/newname-two/x.md', 'x\n')
        self.assertEqual(C.offences(self.root), ['toolkit/a.md:1'])

    def test_a_file_that_is_not_text_does_not_stop_the_scan(self):
        path = os.path.join(self.root, 'toolkit', 'a.bin')
        os.makedirs(os.path.dirname(path))
        with open(path, 'wb') as f:
            f.write(b'\xff\xfe\nsuperfrog\n')
        self.assertEqual(C.offences(self.root), ['toolkit/a.bin:2'])

    def run_main(self):
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = C.main(['check', self.root])
        return code, out.getvalue()

    def test_exit_codes(self):
        self.write('toolkit/a.md', 'clean\n')
        self.assertEqual(self.run_main(), (0, ''))
        self.write('toolkit/a.md', 'superfrog\n')
        self.assertEqual(self.run_main(), (1, 'toolkit/a.md:1\n'))


class ThisRepo(unittest.TestCase):
    def test_the_repo_passes(self):
        root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        self.assertEqual(C.offences(root), [])


if __name__ == '__main__':
    unittest.main()
