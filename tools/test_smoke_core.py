"""tools/smoke_core.py on small synthetic projects: every rule of smoke-tests DESIGN §2.3, the argv of §3.2,
and that the module stays pure (§3.1)."""
import ast
import os
import subprocess
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from smoke_core import Entry, Project, check, run_list  # noqa: E402

LIVE = '#[test]\nfn a() {}\n#[test]\nfn b() {}\n'
QUICK = 'crates/x/tests/quick.rs'
AREA = f'[[area]]\nname = "it works"\ncovers = ["{QUICK}"]\n'


def project(manifest, sources=None, extra_files=(), root='toolkit'):
    sources = {QUICK: LIVE} if sources is None else sources
    return Project(root, manifest, [*sources, *extra_files], sources)


def toolkit(tests='', python='', areas=AREA):
    return f'workspace = "."\n[tests]\n{tests}\n[python]\n{python}\n{areas}'


LABELLED = f'"{QUICK}" = "smoke"'


class Clean(unittest.TestCase):
    def test_whole_file_named_and_inline_entries(self):
        sources = {
            QUICK: LIVE,
            'crates/x/tests/mixed.rs': LIVE,
            'crates/x/tests/slow.rs': LIVE,
            'crates/x/src/lib.rs': LIVE,
        }
        manifest = toolkit(f'''{LABELLED}
"crates/x/tests/mixed.rs" = ["a", "b"]
"crates/x/tests/slow.rs" = "full"
"crates/x/src" = "smoke"
''')
        p = project(manifest, sources)
        self.assertEqual(check(p), [])
        self.assertEqual(run_list(p), [
            Entry('toolkit', 'rust', 'toolkit', ['cargo', 'test', '-q', '-p', 'x', '--test', 'quick']),
            Entry('toolkit', 'rust', 'toolkit',
                  ['cargo', 'test', '-q', '-p', 'x', '--test', 'mixed', '--', '--exact', 'a', 'b']),
            Entry('toolkit', 'rust', 'toolkit', ['cargo', 'test', '-q', '-p', 'x', '--lib']),
        ])

    def test_inline_flags_follow_the_targets_the_crate_has(self):
        for files, flags in (
            ({'crates/x/src/main.rs': LIVE}, ['--bins']),
            ({'crates/x/src/lib.rs': LIVE, 'crates/x/src/main.rs': ''}, ['--lib', '--bins']),
            ({'crates/x/src/lib.rs': '', 'crates/x/src/bin/tool.rs': LIVE}, ['--lib', '--bins']),
        ):
            manifest = 'workspace = "."\n[tests]\n"crates/x/src" = "smoke"\n' \
                       '[[area]]\nname = "n"\ncovers = ["crates/x/src"]\n'
            p = project(manifest, files)
            self.assertEqual(check(p), [], files)
            self.assertEqual(run_list(p)[0].argv, ['cargo', 'test', '-q', '-p', 'x', *flags])

    def test_a_game_runs_from_its_workspace_with_workspace_relative_keys(self):
        sources = {f'engine/{QUICK}': LIVE}
        manifest = f'workspace = "engine"\n[tests]\n{LABELLED}\n{AREA}'
        p = project(manifest, sources, root='games/g')
        self.assertEqual(check(p), [])
        self.assertEqual(run_list(p)[0].cwd, 'games/g/engine')

    def test_python_entries_keep_todays_working_directories(self):
        area = '[[area]]\nname = "n"\ncovers = ["tools/test_x.py"]\n'
        manifest = f'workspace = "."\n[python]\n"tools/test_x.py" = "smoke"\n"tools/test_y.py" = "full"\n{area}'
        files = ['tools/test_x.py', 'tools/test_y.py']
        self.assertEqual(check(project(manifest, {}, files)), [])
        self.assertEqual(run_list(project(manifest, {}, files)), [Entry(
            'toolkit', 'python', '.',
            ['venv/bin/python', '-m', 'unittest', 'discover', '-q', '-s', 'toolkit/tools', '-p', 'test_x.py'])])
        game = run_list(project(manifest, {}, files, root='games/g'))
        self.assertEqual(game, [Entry(
            'games/g', 'python', 'games/g',
            ['../../venv/bin/python', '-m', 'unittest', 'discover', '-q', '-s', 'tools', '-p', 'test_x.py'])])

    def test_tests_inside_a_module_are_named_by_their_path(self):
        src = '#[cfg(test)]\nmod outer {\n  mod inner {\n    #[test]\n    fn t() {}\n  }\n  #[test]\n  fn u() {}\n}\n#[test]\nfn top() {}\n'
        manifest = toolkit(f'"{QUICK}" = ["outer::inner::t", "outer::u", "top"]')
        self.assertEqual(check(project(manifest, {QUICK: src})), [])

    def test_an_inline_named_test_includes_its_files_module(self):
        sources = {'crates/x/src/lib.rs': '', 'crates/x/src/a/b.rs': 'mod tests {\n#[test]\nfn t() {}\n}\n'}
        manifest = 'workspace = "."\n[tests]\n"crates/x/src" = ["a::b::tests::t"]\n' \
                   '[[area]]\nname = "n"\ncovers = ["crates/x/src::a::b::tests::t"]\n'
        self.assertEqual(check(project(manifest, sources)), [])

    def test_a_manifest_with_no_tests_and_no_area_is_clean(self):
        self.assertEqual(check(project('workspace = "."\n', {})), [])


class Macros(unittest.TestCase):
    # The two shapes that exist: sf-conform's cases.rs (one test a call, attributes passed in) and
    # chain.rs (many tests a call, #[ignore in the macro).
    CASE = '''
macro_rules! case {
    ($(#[$m:meta])* $f:ident, $name:literal) => {
        $(#[$m])*
        #[test]
        fn $f() { green($name); }
    };
}
case!(#[ignore = "slow: about 4 min"] attract_demo_3, "attract-demo-3");
case!(front_noskip, "front-noskip");
'''
    CODES = '''
macro_rules! code_cases {
    ($($test:ident: $name:literal => ($world:literal, $level:literal),)*) => {$(
        #[test]
        #[ignore = "about 90 s"]
        fn $test() { run_to($name, $world, $level); }
    )*};
}
code_cases! {
    code_w1_1: "code-w1-1" => (1, 1),
    code_w1_2: "code-w1-2" => (1, 2),
}
'''
    REPEAT_LIVE = CODES.replace('#[ignore = "about 90 s"]', '')

    def test_a_named_test_made_by_an_invocation_is_found(self):
        manifest = toolkit(f'"{QUICK}" = ["front_noskip"]')
        p = project(manifest, {QUICK: self.CASE})
        self.assertEqual(check(p), [])
        self.assertEqual(run_list(p)[0].argv[-3:], ['--', '--exact', 'front_noskip'])

    def test_an_invocation_that_is_ignored_is_reported(self):
        problems = check(project(toolkit(f'"{QUICK}" = ["attract_demo_3"]'), {QUICK: self.CASE}))
        self.assertEqual(len(problems), 1)
        self.assertIn('attract_demo_3', problems[0])
        self.assertIn('#[ignore]', problems[0])

    def test_an_argument_is_not_taken_for_a_test(self):
        src = self.CASE + 'case!(third, other_ident);\n'
        problems = check(project(toolkit(f'"{QUICK}" = ["other_ident"]'), {QUICK: src}))
        self.assertIn('not in that file', problems[0])

    def test_an_ignore_in_the_macro_holds_for_every_test_it_makes(self):
        problems = check(project(toolkit(f'"{QUICK}" = ["code_w1_2"]'), {QUICK: self.CODES}))
        self.assertIn('#[ignore]', problems[0])

    def test_a_repeating_macro_makes_one_name_an_item_with_its_own_ignore(self):
        src = self.REPEAT_LIVE.replace('code_w1_2:', '#[ignore] code_w1_2:').replace(
            '$($test:ident', '$($(#[$m:meta])* $test:ident')
        self.assertEqual(check(project(toolkit(f'"{QUICK}" = ["code_w1_1"]'), {QUICK: src})), [])
        problems = check(project(toolkit(f'"{QUICK}" = ["code_w1_2"]'), {QUICK: src}))
        self.assertIn('#[ignore]', problems[0])

    def test_a_macro_that_makes_no_test_makes_no_names(self):
        src = 'macro_rules! helper { ($f:ident) => { fn $f() {} }; }\nhelper!(not_a_test);\n' + LIVE
        problems = check(project(toolkit(f'"{QUICK}" = ["not_a_test"]'), {QUICK: src}))
        self.assertIn('not in that file', problems[0])

    def test_the_real_macros_in_this_repo_are_read(self):
        # The shapes above are copies; this holds the parser to the files themselves.
        crate = 'games/superfrog/engine/crates/sf-conform/tests'
        root = os.path.dirname(HERE)
        with open(f'{root}/{crate}/cases.rs') as f:
            cases = f.read()
        with open(f'{root}/{crate}/chain.rs') as f:
            chain = f.read()
        ok = check(project(toolkit(f'"{QUICK}" = ["front_noskip"]'), {QUICK: cases}))
        self.assertEqual(ok, [])
        for src, name in ((cases, 'attract_demo_3'), (chain, 'code_w1_1')):
            problems = check(project(toolkit(f'"{QUICK}" = ["{name}"]'), {QUICK: src}))
            self.assertIn('#[ignore]', ''.join(problems), name)


class Missing(unittest.TestCase):
    def test_no_manifest_is_a_problem_with_or_without_test_files(self):
        for sources in ({}, {QUICK: LIVE}):
            problems = check(project(None, sources))
            self.assertEqual(len(problems), 1)
            self.assertIn('toolkit/smoke.toml', problems[0])

    def test_a_malformed_manifest_reports_and_does_not_raise(self):
        for text in ('this is = = not toml', 'workspace = 3', 'workspace = "."\ntests = 3\n',
                     'workspace = "."\narea = 3\n', 'workspace = "."\narea = [3]\n'):
            self.assertTrue(check(project(text, {})), text)

    def test_a_bad_label_value_is_reported(self):
        for label in ('"quick"', '[]', '[1]', '3'):
            problems = check(project(toolkit(f'"{QUICK}" = {label}', areas=''), {QUICK: LIVE}))
            self.assertTrue(any('a label is' in p for p in problems), label)
        area = '[[area]]\nname = "n"\ncovers = ["tools/test_x.py"]\n'
        problems = check(project(f'workspace = "."\n[python]\n"tools/test_x.py" = ["a"]\n{area}', {},
                                 ['tools/test_x.py']))
        self.assertTrue(any('Python label' in p for p in problems))


class Labels(unittest.TestCase):
    def test_an_unlabelled_test_file(self):
        problems = check(project(toolkit('', areas=''), {QUICK: LIVE}))
        self.assertTrue(any(QUICK in p and 'no label' in p for p in problems))

    def test_an_unlabelled_crate_with_inline_tests(self):
        problems = check(project(toolkit(LABELLED), {QUICK: LIVE, 'crates/y/src/lib.rs': LIVE}))
        self.assertEqual(len(problems), 1)
        self.assertIn('crates/y/src', problems[0])

    def test_a_crate_with_no_inline_tests_needs_no_label(self):
        self.assertEqual(check(project(toolkit(LABELLED), {QUICK: LIVE, 'crates/y/src/lib.rs': 'fn a() {}'})), [])

    def test_a_label_for_a_missing_file(self):
        problems = check(project(toolkit(f'{LABELLED}\n"crates/x/tests/gone.rs" = "smoke"')))
        self.assertEqual(len(problems), 1)
        self.assertIn('gone.rs', problems[0])

    def test_a_label_for_a_crate_with_no_inline_tests(self):
        problems = check(project(toolkit(f'{LABELLED}\n"crates/y/src" = "smoke"'),
                                 {QUICK: LIVE, 'crates/y/src/lib.rs': 'fn a() {}'}))
        self.assertIn('no inline tests', problems[0])

    def test_a_helper_needs_no_label_and_a_folder_test_does(self):
        sources = {QUICK: LIVE, 'crates/x/tests/common/mod.rs': '', 'crates/x/tests/big/main.rs': LIVE,
                   'crates/x/tests/big/part.rs': ''}
        problems = check(project(toolkit(LABELLED), sources))
        self.assertEqual(len(problems), 1)
        self.assertIn('crates/x/tests/big/main.rs', problems[0])
        p = project(toolkit(f'{LABELLED}\n"crates/x/tests/big/main.rs" = "smoke"'), sources)
        self.assertEqual(check(p), [])
        self.assertEqual(run_list(p)[1].argv[-2:], ['--test', 'big'])

    def test_a_label_for_a_helper_names_no_test_file(self):
        sources = {QUICK: LIVE, 'crates/x/tests/common/mod.rs': ''}
        problems = check(project(toolkit(f'{LABELLED}\n"crates/x/tests/common/mod.rs" = "smoke"'), sources))
        self.assertIn('no such test file', problems[0])

    def test_a_python_test_file_without_a_label(self):
        problems = check(project(toolkit(LABELLED), {QUICK: LIVE}, ['tools/test_x.py', 'tools/helper.py']))
        self.assertEqual(len(problems), 1)
        self.assertIn('tools/test_x.py', problems[0])

    def test_a_label_for_a_missing_python_file(self):
        problems = check(project(toolkit(LABELLED, '"tools/test_gone.py" = "smoke"')))
        self.assertIn('test_gone.py', problems[0])


class Named(unittest.TestCase):
    def test_a_named_test_absent_from_its_file(self):
        problems = check(project(toolkit(f'"{QUICK}" = ["nope"]')))
        self.assertEqual(len(problems), 1)
        self.assertIn('nope', problems[0])

    def test_a_named_test_ignored_with_the_attribute_before_or_after_test(self):
        for src in ('#[ignore]\n#[test]\nfn a() {}\n', '#[test]\n#[ignore = "slow"]\nfn a() {}\n',
                    '#[test]\n#[should_panic]\n#[ignore]\npub fn a() {}\n'):
            problems = check(project(toolkit(f'"{QUICK}" = ["a"]'), {QUICK: src}))
            self.assertEqual(len(problems), 1, src)
            self.assertIn('#[ignore]', problems[0])

    def test_masked_text_is_not_read_as_code(self):
        # Fixture strings and comments, as the purity tests hold, must not make or ignore a test.
        src = r'''
// #[ignore]
/* outer /* #[test] fn in_comment() {} */ #[test] fn in_comment2() {} */
const S: &str = "#[test] fn in_string() {}";
const R: &str = r#"#[test] fn in_raw() { " }"#;
const C: char = '{';
fn life<'a>(x: &'a str) -> &'a str { x }
#[test]
fn a() {}
'''
        self.assertEqual(check(project(toolkit(f'"{QUICK}" = ["a"]'), {QUICK: src})), [])
        for name in ('in_comment', 'in_comment2', 'in_string', 'in_raw'):
            problems = check(project(toolkit(f'"{QUICK}" = ["{name}"]'), {QUICK: src}))
            self.assertIn('not in that file', problems[0], name)


class Areas(unittest.TestCase):
    def test_a_project_with_test_files_and_no_area(self):
        problems = check(project(toolkit(LABELLED, areas='')))
        self.assertEqual(len(problems), 1)
        self.assertIn('[[area]]', problems[0])
        self.assertTrue(check(project(f'workspace = "."\n[python]\n"tools/test_x.py" = "full"\n', {},
                                      ['tools/test_x.py'])))

    def test_an_area_with_empty_covers(self):
        for area in ('[[area]]\nname = "empty"\ncovers = []\n', '[[area]]\nname = "empty"\n'):
            problems = check(project(toolkit(LABELLED, areas=area)))
            self.assertEqual(len(problems), 1, area)
            self.assertIn('empty', problems[0])

    def test_a_cover_labelled_full(self):
        problems = check(project(toolkit(f'"{QUICK}" = "full"')))
        self.assertEqual(len(problems), 1)
        self.assertIn('not labelled smoke', problems[0])

    def test_a_cover_with_no_label(self):
        area = '[[area]]\nname = "n"\ncovers = ["crates/x/tests/other.rs"]\n'
        problems = check(project(toolkit(LABELLED, areas=area)))
        self.assertIn('not labelled smoke', problems[0])

    def test_a_python_cover_labelled_full(self):
        area = '[[area]]\nname = "n"\ncovers = ["tools/test_x.py"]\n'
        problems = check(project(f'workspace = "."\n[python]\n"tools/test_x.py" = "full"\n{area}', {},
                                 ['tools/test_x.py']))
        self.assertIn('not labelled smoke', problems[0])

    def test_a_cover_naming_a_test_not_in_the_labels_list(self):
        area = f'[[area]]\nname = "n"\ncovers = ["{QUICK}::b"]\n'
        problems = check(project(toolkit(f'"{QUICK}" = ["a"]', areas=area)))
        self.assertEqual(len(problems), 1)
        self.assertIn('list', problems[0])

    def test_a_cover_naming_a_test_in_a_whole_smoke_file(self):
        def cover(name):
            return toolkit(LABELLED, areas=f'[[area]]\nname = "n"\ncovers = ["{QUICK}::{name}"]\n')
        src = '#[test]\nfn live() {}\n#[test]\n#[ignore]\nfn off() {}\n'
        self.assertEqual(check(project(cover('live'), {QUICK: src})), [])
        self.assertIn('no such test', check(project(cover('gone'), {QUICK: src}))[0])
        self.assertIn('#[ignore]', check(project(cover('off'), {QUICK: src}))[0])

    def test_a_whole_file_cover_with_only_ignored_tests(self):
        src = '#[test]\n#[ignore]\nfn a() {}\n'
        problems = check(project(toolkit(LABELLED), {QUICK: src}))
        self.assertEqual(len(problems), 1)
        self.assertIn('no live test', problems[0])


class Purity(unittest.TestCase):
    def test_smoke_core_imports_nothing_else(self):
        with open(os.path.join(HERE, 'smoke_core.py')) as f:
            tree = ast.parse(f.read())
        imported = set()
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                imported.update(alias.name.split('.')[0] for alias in node.names)
            elif isinstance(node, ast.ImportFrom):
                imported.add((node.module or '').split('.')[0])
        # If this fails, move the code to tools/smoke.py; never relax the test (DESIGN §3.1).
        self.assertLessEqual(imported, {'re', 'tomllib', 'dataclasses'})


class ThisRepo(unittest.TestCase):
    def test_every_tracked_test_file_parses(self):
        # The parser meets real Rust here, not only fixtures: no crash, and it finds tests in the toolkit.
        root = os.path.dirname(HERE)
        tracked = subprocess.run(['git', 'ls-files', 'toolkit'], cwd=root, capture_output=True, text=True,
                                 check=True).stdout.split()
        files = [p[len('toolkit/'):] for p in tracked]
        sources = {}
        for path in files:
            if path.endswith('.rs') and ('/tests/' in path or '/src/' in path):
                with open(f'{root}/toolkit/{path}', encoding='utf-8') as f:
                    sources[path] = f.read()
        problems = check(Project('toolkit', 'workspace = "."\n', files, sources))
        self.assertTrue(any('crates/amiga-disk/tests/ofs.rs' in p for p in problems), problems[:3])
        self.assertFalse(any('tests/common/' in p for p in problems))


if __name__ == '__main__':
    unittest.main()
