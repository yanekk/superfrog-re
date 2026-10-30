#!/usr/bin/env python3
"""Rule 3's machine (monorepo-and-knowledge DESIGN §2.3): no game names above tier 3.

Scans toolkit/ and knowledge/ and prints one `path:line` per offence, exit 1 if any. The banned
list derives from games/*/, so a new game's folder is banned the moment it exists; nobody has to
remember to extend the check. A line carrying `learned on:` is a provenance line and is let through.
"""
import os
import re
import subprocess
import sys

SCANNED = ('toolkit', 'knowledge')
PROVENANCE = 'learned on:'
# The prefix of the first game's crates, and its studio, on top of the folder names.
FIXED_TOKENS = ('sf-', 'team17', 'team 17')
# The repository's own name stays until the repository is renamed; DESIGN §7 keeps it.
REPO_NAME = re.compile(r'superfrog-re(?![a-z0-9_-])', re.IGNORECASE)


def token_pattern(token):
    """A regex for one token, with any run of `-`, `_` or space matching any other."""
    # A trailing separator splits to an empty last part, so the join keeps it for `sf-`.
    body = r'[-_ ]+'.join(re.escape(p) for p in re.split(r'[-_ ]+', token))
    # `sf-` is a prefix: without the guard it would hit every word ending in those letters.
    guard = r'(?<![a-z0-9])' if token == 'sf-' else ''
    return re.compile(guard + body, re.IGNORECASE)


def banned_patterns(game_dirs):
    """The patterns for these games/*/ folder names plus the fixed tokens."""
    return [token_pattern(t) for t in (*game_dirs, *FIXED_TOKENS)]


def line_offends(line, patterns):
    """Whether this line names something banned; pure, so the rule is testable on strings."""
    if PROVENANCE in line:
        return False
    cleaned = REPO_NAME.sub('', line)
    return any(p.search(cleaned) for p in patterns)


def game_dirs(root):
    games = os.path.join(root, 'games')
    if not os.path.isdir(games):
        return []
    return sorted(d for d in os.listdir(games)
                  if not d.startswith('.') and os.path.isdir(os.path.join(games, d)))


def scanned_files(root):
    """The files git tracks or would track under the scanned folders, relative to root."""
    out = subprocess.run(
        ['git', 'ls-files', '--cached', '--others', '--exclude-standard', '-z', '--', *SCANNED],
        cwd=root, capture_output=True, check=True)
    return [f for f in out.stdout.decode().split('\0') if f]


def offences(root):
    """Every offence under root, as `path:line`."""
    patterns = banned_patterns(game_dirs(root))
    found = []
    for rel in scanned_files(root):
        try:
            with open(os.path.join(root, rel), 'rb') as f:
                text = f.read().decode('utf-8', 'replace')
        except FileNotFoundError:  # listed by git but deleted in the working tree
            continue
        for n, line in enumerate(text.splitlines(), 1):
            if line_offends(line, patterns):
                found.append(f'{rel}:{n}')
    return found


def main(argv):
    root = argv[1] if len(argv) > 1 else os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    found = offences(root)
    for f in found:
        print(f)
    return 1 if found else 0


if __name__ == '__main__':
    sys.exit(main(sys.argv))
