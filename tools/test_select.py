"""tools/select_scope.py's mapping on synthetic change lists, and that it stays pure (DESIGN §2.4, §3.1)."""
import ast
import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import select_scope  # noqa: E402

GAMES = ['forest-dumb', 'superfrog']


def scope(changed, env=None):
    return select_scope.scope(changed, env or {}, GAMES)


class Mapping(unittest.TestCase):
    def assert_everything(self, changed, env=None):
        self.assertTrue(scope(changed, env)['everything'], changed)

    def test_a_game_path_runs_that_game(self):
        s = scope(['games/superfrog/engine/crates/sf-core/src/lib.rs'])
        self.assertEqual((s['everything'], s['projects']), (False, ['games/superfrog']))

    def test_a_toolkit_path_runs_the_toolkit_and_every_game(self):
        s = scope(['toolkit/crates/amiga-clock/src/lib.rs'])
        self.assertEqual(s['projects'], ['games/forest-dumb', 'games/superfrog', 'toolkit'])
        self.assertFalse(s['everything'])

    def test_each_root_file_kind_runs_everything(self):
        for path in ('.pir/settings.json', 'tools/test.sh', 'knowledge/KNOWLEDGE.md', 'CLAUDE.md',
                     'README.md', 'requirements.txt', '.gitignore'):
            self.assert_everything([path])

    def test_plans_only_runs_nothing(self):
        s = scope(['plans/x/PLAN.md', 'plans/y/tasks/T01-a.md'])
        self.assertEqual((s['everything'], s['projects']), (False, []))
        self.assertEqual(len(s['reasons']), 1)

    def test_the_all_override_wins(self):
        self.assert_everything(['plans/x/PLAN.md'], {'PIR_TEST_SCOPE': 'all'})

    def test_an_empty_diff_runs_everything(self):
        self.assert_everything([])

    def test_scopes_merge_by_union(self):
        s = scope(['games/superfrog/x', 'plans/p/PLAN.md', 'games/forest-dumb/y'])
        self.assertEqual(s['projects'], ['games/forest-dumb', 'games/superfrog'])
        self.assertEqual(scope(['games/superfrog/x', 'toolkit/y'])['projects'],
                         ['games/forest-dumb', 'games/superfrog', 'toolkit'])
        self.assert_everything(['games/superfrog/x', 'README.md'])

    def test_unparseable_input_runs_everything(self):
        for bad in (None, 'games/superfrog/x', [None], [''], [3]):
            self.assert_everything(bad)
        self.assertIsNone(select_scope.lines_to_changed('DOUBT\n'))

    def test_an_unknown_game_and_a_bare_folder_name_are_doubts(self):
        self.assert_everything(['games/nosuch/x'])
        self.assert_everything(['plans'])
        self.assert_everything(['toolkit'])


class Purity(unittest.TestCase):
    def test_the_module_imports_no_world_and_opens_no_file(self):
        with open(select_scope.__file__, encoding='utf-8') as f:
            tree = ast.parse(f.read())
        banned = {'subprocess', 'os', 'socket'}
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                self.assertFalse(banned & {a.name.split('.')[0] for a in node.names})
            elif isinstance(node, ast.ImportFrom):
                self.assertNotIn((node.module or '').split('.')[0], banned)
            elif isinstance(node, ast.Call) and isinstance(node.func, ast.Name):
                self.assertNotIn(node.func.id, ('open', '__import__', 'eval', 'exec'))


if __name__ == '__main__':
    unittest.main()
