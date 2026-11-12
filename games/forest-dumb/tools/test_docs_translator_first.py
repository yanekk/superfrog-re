"""Translator DESIGN §2.12, T20: the translator is the first pass a session meets in every doc, and no doc
tells a session to call Muse. Muse, the model level 1's sessions translated with, may be named only
under a heading containing "History", in `spikes/`, in a plan (a plan is the record of its own build) or
in the dated records listed below. Standard library and git only: no disks."""
import os
import re
import subprocess
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(os.path.dirname(HERE)))
GAME = 'games/forest-dumb'

# Dated records that report what happened then, not what a session does now.
RECORDS = {
    f'{GAME}/docs/level-1-brief.md': "level 1's brief, which chose Muse (2026-11-05)",
    f'{GAME}/docs/translator-brief.md': "the translator plan's brief, which retires Muse (2026-11-11)",
    f'{GAME}/docs/13-translator-trial.md': "the trial's reading, with Muse's T40 results as a column (2026-11-07)",
}
MUSE = re.compile(r'\bMuse\b')
HEADING = re.compile(r'^(#{1,6}) (.*)')


def docs():
    out = subprocess.run(['git', 'ls-files', '*.md'], cwd=REPO, capture_output=True, text=True, check=True).stdout
    return [p for p in out.splitlines()
            if not p.startswith('plans/') and '/spikes/' not in p and p not in RECORDS]


def offences(text):
    """Line numbers naming Muse outside a History section; fenced code is not a heading."""
    stack, fenced, bad = [], False, []
    for n, line in enumerate(text.splitlines(), 1):
        if line.lstrip().startswith('```'):
            fenced = not fenced
        m = None if fenced else HEADING.match(line)
        if m:
            level = len(m.group(1))
            stack = [(lv, h) for lv, h in stack if lv < level] + [(level, m.group(2))]
            continue
        if MUSE.search(line) and not any('History' in h for _, h in stack):
            bad.append(n)
    return bad


class Offences(unittest.TestCase):
    def test_a_history_section_and_its_subsections_may_name_muse(self):
        text = '# Doc\n\nTranslate first.\n\n## History\n\n### Old\n\nMuse wrote ports.\n\n## Now\n\nAsk Muse.\n'
        self.assertEqual(offences(text), [13])

    def test_a_hash_inside_fenced_code_is_not_a_heading(self):
        text = '## History\n\n```\n# not a heading\n```\n\nMuse.\n'
        self.assertEqual(offences(text), [])

    def test_muse_spark_paths_are_not_the_name(self):
        self.assertEqual(offences('See spikes/muse-spark/README.md.\n'), [])


class Docs(unittest.TestCase):
    def test_no_doc_outside_history_tells_a_session_to_call_muse(self):
        found = []
        for path in docs():
            with open(os.path.join(REPO, path), encoding='utf-8') as f:
                found += [f'{path}:{n}' for n in offences(f.read())]
        self.assertEqual(found, [], 'Muse named outside a "History" section; the translator comes first')

    def test_the_records_still_exist(self):
        for path in RECORDS:
            self.assertTrue(os.path.exists(os.path.join(REPO, path)), path)

    def test_the_places_a_session_starts_say_translate_first(self):
        for path, phrase in ((f'{GAME}/CLAUDE.md', 'tools/translate.py'),
                             (f'{GAME}/docs/11-translation.md', 'translate with the translator first'),
                             (f'{GAME}/tools/README.md', 'Translate a routine first'),
                             ('toolkit/PLAYBOOK.md', 'translate first, port by choice')):
            with open(os.path.join(REPO, path), encoding='utf-8') as f:
                self.assertIn(phrase, f.read(), path)


if __name__ == '__main__':
    unittest.main()
