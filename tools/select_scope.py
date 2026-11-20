#!/usr/bin/env python3
"""Which suites a change needs (monorepo-and-knowledge DESIGN §2.4, §3.1). scope() is pure: the
changed list and the environment arrive as arguments, so the rule is testable on strings; every
doubt widens the selection. tools/test.sh does the world's part.

Not named select.py: that would shadow the stdlib's `select` for every script in this folder, and
subprocess imports it.
"""
import sys


def scope(changed, env, games=()):
    """{"everything": bool, "projects": [...], "reasons": [...]} for these changed paths.

    `games` is the games/*/ directory names; a game path outside them is a doubt.
    """
    if env.get('PIR_TEST_SCOPE') == 'all':
        return _everything('PIR_TEST_SCOPE=all')
    if not isinstance(changed, list) or not all(isinstance(p, str) and p.strip() for p in changed):
        return _everything('the changed list could not be read')
    if not changed:
        return _everything('an empty diff is a doubt, not a reason to run less')
    projects, reasons = set(), []
    for path in changed:
        parts = path.split('/')
        if parts[0] == 'plans' and len(parts) > 1:
            continue
        if parts[0] == 'toolkit' and len(parts) > 1:
            projects.update(['toolkit', *(f'games/{g}' for g in games)])
            reasons.append(f'{path}: the toolkit and every game')
        elif parts[0] == 'games' and len(parts) > 2 and parts[1] in games:
            projects.add(f'games/{parts[1]}')
            reasons.append(f'{path}: {parts[1]}')
        else:
            return _everything(f'{path}: a root file')
    if not projects:
        return {'everything': False, 'projects': [], 'reasons': ['plans only: nothing to run']}
    return {'everything': False, 'projects': sorted(projects), 'reasons': reasons}


def _everything(reason):
    return {'everything': True, 'projects': [], 'reasons': [reason]}


def lines_to_changed(text):
    """stdin's text to scope()'s list: one path a line; the word DOUBT when the diff failed."""
    return None if text.strip() == 'DOUBT' else [l for l in text.splitlines() if l]


if __name__ == '__main__':
    # argv: the PIR_TEST_SCOPE value (possibly empty), then the games/*/ names; test.sh has the world.
    selection = scope(lines_to_changed(sys.stdin.read()), {'PIR_TEST_SCOPE': sys.argv[1]}, sys.argv[2:])
    print('EVERYTHING' if selection['everything'] else ' '.join(selection['projects']) or 'NOTHING')
    for reason in selection['reasons'][:5]:
        print(f'test.sh: {reason}', file=sys.stderr)
