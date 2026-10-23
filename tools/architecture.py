#!/usr/bin/env python3
"""The architecture docs' front matter: who owns which engine file, and the line totals
(plans/engine-architecture DESIGN §2.4, §3.2).

Every chapter in docs/architecture/ opens with TOML front matter between `+++` lines. Its `owns`
lists path prefixes (a directory ending in `/`, or a file); each tracked file under
engine/crates/ and engine/conform/ belongs to the chapter with the longest matching prefix.

    python tools/architecture.py totals     # the overview's tables, in Markdown

The functions are pure over strings and path lists except `load_chapters`, `tracked_files`,
`line_counts` and `check_paths`, which read the tree. tools/test_architecture_docs.py is the
drift check built on them.
"""
import os, re, subprocess, sys, tomllib

DOCS = 'docs/architecture'
DOMAIN = ('engine/crates/', 'engine/conform/')
KEYS = ('id', 'title', 'tier', 'depth', 'kind', 'mix', 'reuse', 'owns')
TIERS = ('mechanism', 'subsystem', 'component')
DEPTHS = ('deep', 'light')
KINDS = ('amiga', 'game', 'data', 'shell')
RATINGS = ('as-is', 'small-changes', 'big-changes', 'superfrog-only')
TBD = 'tbd'
# A line is a line of code (DESIGN §2.2): recorded input scripts, `.toml`, patches and Markdown
# would otherwise make verification half the codebase.
CODE_EXTS = ('.rs', '.cpp', '.h', '.wgsl')
PATH_ROOTS = ('engine/', 'docs/', 'plans/', 'tools/', 're/')


def repo_root():
    return subprocess.run(['git', 'rev-parse', '--show-toplevel'], cwd=os.path.dirname(os.path.abspath(__file__)),
                          capture_output=True, text=True, check=True).stdout.strip()


def parse_front_matter(text):
    """The TOML between a leading `+++` line and the next one; None when the file has none."""
    lines = text.splitlines()
    if not lines or lines[0].strip() != '+++':
        return None
    for i in range(1, len(lines)):
        if lines[i].strip() == '+++':
            return tomllib.loads('\n'.join(lines[1:i]))
    raise ValueError('front matter opened with +++ is never closed')


def load_chapters(root):
    """id -> front matter plus `path` (repo-relative), for each docs/architecture/*.md that has
    front matter. The README, glossary and template have none and are not chapters."""
    chapters = {}
    folder = os.path.join(root, DOCS)
    for name in sorted(os.listdir(folder)):
        if not name.endswith('.md'):
            continue
        rel = f'{DOCS}/{name}'
        with open(os.path.join(root, rel), encoding='utf-8') as f:
            fm = parse_front_matter(f.read())
        if fm is None:
            continue
        cid = fm.get('id', name[:-3])
        chapters[cid] = dict(fm, path=rel)
    return chapters


def tracked_files(root):
    out = subprocess.run(['git', 'ls-files', '-z', '--', *DOMAIN], cwd=root,
                         capture_output=True, check=True).stdout
    return sorted(p.decode() for p in out.split(b'\0') if p)


def _matches(prefix, path):
    return path.startswith(prefix) if prefix.endswith('/') else path == prefix


def assign(files, chapters):
    """(file -> chapter id, problems). A file goes to the longest prefix that matches it; a file
    no prefix matches, a prefix two chapters list, and a prefix matching no file are problems."""
    problems = []
    owner = {}
    for cid, ch in sorted(chapters.items()):
        for prefix in ch.get('owns', []):
            if prefix in owner:
                problems.append(f'duplicate prefix {prefix!r}: in {owner[prefix]} and {cid}')
            else:
                owner[prefix] = cid
    by_length = sorted(owner, key=len, reverse=True)
    assignment, unowned = {}, []
    for path in files:
        best = next((p for p in by_length if _matches(p, path)), None)
        if best is None:
            unowned.append(path)
        else:
            assignment[path] = owner[best]
    for path in unowned:
        problems.append(f'unowned file {path}: add it to the right chapter\'s owns (DESIGN §2.4)')
    for prefix in sorted(owner):
        if not any(_matches(prefix, f) for f in files):
            problems.append(f'stale prefix {prefix!r} in {owner[prefix]}: matches no tracked file')
    return assignment, problems


def validate(chapters, allow_tbd):
    """Problems with the front matter's keys and values (DESIGN §2.4 rule 3)."""
    problems = []
    for cid, ch in sorted(chapters.items()):
        where = ch.get('path', cid)
        for key in KEYS:
            if key not in ch:
                problems.append(f'{where}: missing key {key!r}')
        if 'id' in ch and 'path' in ch and os.path.basename(ch['path']) != f"{ch['id']}.md":
            problems.append(f"{where}: id {ch['id']!r} does not match the file name")
        for key, allowed, tbd_ok in (('tier', TIERS, False), ('depth', DEPTHS, False),
                                     ('kind', KINDS, True), ('reuse', RATINGS, True)):
            if key not in ch:
                continue
            value = ch[key]
            if value == TBD and tbd_ok:
                if not allow_tbd:
                    problems.append(f'{where}: {key} is still "tbd"')
                continue
            if value not in allowed:
                problems.append(f'{where}: {key} {value!r} is not one of {", ".join(allowed)}')
        if 'mix' in ch:
            mix = ch['mix']
            if mix == TBD:
                if not allow_tbd:
                    problems.append(f'{where}: mix is still "tbd"')
            elif not isinstance(mix, dict) or set(mix) != set(KINDS):
                problems.append(f'{where}: mix must give exactly {", ".join(KINDS)}')
            elif not all(isinstance(v, int) and v >= 0 for v in mix.values()):
                problems.append(f'{where}: mix values must be whole percentages')
            elif sum(mix.values()) != 100:
                problems.append(f'{where}: mix sums to {sum(mix.values())}, not 100')
        if 'owns' in ch and not (isinstance(ch['owns'], list) and all(isinstance(p, str) for p in ch['owns'])):
            problems.append(f'{where}: owns must be a list of paths')
    return problems


_FENCE = re.compile(r'^(```|~~~).*?^\1', re.M | re.S)
_CODE = re.compile(r'`([^`\n]+)`')
_LINK = re.compile(r'\]\(([^)\s]+)(?:\s+"[^"]*")?\)')


def cited_paths(text):
    """(path, from_link) for each repo path the Markdown cites: a code span or a link target that
    starts with one of PATH_ROOTS, or a relative link target (`../x.md`). A `:line` or `::item`
    suffix and a `#anchor` are stripped; a path with `*` or `{` is a pattern and skipped. A code
    span with a space in it is a command, not a path. Fenced blocks are skipped."""
    text = _FENCE.sub('', text)
    out = []
    for span in _CODE.findall(text):
        if any(c.isspace() for c in span):
            continue
        out.append((span, False))
    for target in _LINK.findall(text):
        if '://' in target or target.startswith(('#', 'mailto:')):
            continue
        out.append((target.split('#', 1)[0], True))
    paths = []
    for raw, from_link in out:
        if not raw or '*' in raw or '{' in raw:
            continue
        path = raw.split('::', 1)[0]
        path = re.sub(r':\d+(-\d+)?$', '', path)
        if path.startswith(PATH_ROOTS) or (from_link and path.startswith(('./', '../'))):
            paths.append((path, from_link))
    return paths


def check_paths(root, md_files):
    """Problems for each cited path that does not exist. A path starting with a PATH_ROOT is from
    the repo root; a relative link target is from the citing file's folder."""
    problems = []
    for rel in md_files:
        with open(os.path.join(root, rel), encoding='utf-8') as f:
            text = f.read()
        for path, _ in cited_paths(text):
            base = root if path.startswith(PATH_ROOTS) else os.path.join(root, os.path.dirname(rel))
            if not os.path.exists(os.path.normpath(os.path.join(base, path))):
                problems.append(f'{rel}: cites {path}, which does not exist')
    return problems


def _lines(path):
    with open(path, 'rb') as f:
        data = f.read()
    return data.count(b'\n') + (1 if data and not data.endswith(b'\n') else 0)


def line_counts(root, assignment):
    """(chapter id -> lines of code, uncounted lines). Code is CODE_EXTS (DESIGN §2.2); every
    other owned file's lines go into the one uncounted total."""
    counts = {cid: 0 for cid in set(assignment.values())}
    uncounted = 0
    for path, cid in assignment.items():
        n = _lines(os.path.join(root, path))
        if path.endswith(CODE_EXTS):
            counts[cid] += n
        else:
            uncounted += n
    return counts, uncounted


def totals(chapters, lines):
    """The overview's tables (DESIGN §2.6): (per chapter, per kind, per rating). A chapter row is
    (id, tier, kind, mix, reuse, lines); a kind's lines are each chapter's lines times its share;
    a "tbd" kind or mix counts under "tbd"."""
    chapter_rows = []
    by_kind = {k: 0.0 for k in KINDS}
    by_rating = {r: 0 for r in RATINGS}
    for cid in sorted(chapters):
        ch = chapters[cid]
        n = lines.get(cid, 0)
        chapter_rows.append((cid, ch.get('tier'), ch.get('kind'), ch.get('mix'), ch.get('reuse'), n))
        mix = ch.get('mix')
        if isinstance(mix, dict):
            for k in KINDS:
                by_kind[k] += n * mix.get(k, 0) / 100
        else:
            by_kind[TBD] = by_kind.get(TBD, 0) + n
        rating = ch.get('reuse') if ch.get('reuse') in RATINGS else TBD
        by_rating[rating] = by_rating.get(rating, 0) + n
    kind_rows = [(k, round(v)) for k, v in by_kind.items()]
    rating_rows = list(by_rating.items())
    return chapter_rows, kind_rows, rating_rows


def _mix_text(mix):
    if not isinstance(mix, dict):
        return str(mix)
    return ' '.join(f'{k} {mix.get(k, 0)}' for k in KINDS)


def render_totals(chapter_rows, kind_rows, rating_rows, uncounted):
    out = ['| Chapter | Tier | Kind | Mix | Reuse | Lines |', '|---|---|---|---|---|---:|']
    for cid, tier, kind, mix, reuse, n in chapter_rows:
        out.append(f'| `{cid}` | {tier} | {kind} | {_mix_text(mix)} | {reuse} | {n} |')
    out.append(f'| (uncounted: scripts, `.toml`, patches, Markdown) | | | | | {uncounted} |')
    total = sum(r[-1] for r in chapter_rows)
    out += ['', '| Kind | Lines |', '|---|---:|']
    out += [f'| {k} | {n} |' for k, n in kind_rows]
    out += ['', '| Reuse | Lines |', '|---|---:|']
    out += [f'| {r} | {n} |' for r, n in rating_rows]
    out += ['', f'Lines of code: {total}.']
    return '\n'.join(out)


def main(argv):
    if argv != ['totals']:
        print('usage: python tools/architecture.py totals', file=sys.stderr)
        return 2
    root = repo_root()
    chapters = load_chapters(root)
    assignment, _ = assign(tracked_files(root), chapters)
    lines, uncounted = line_counts(root, assignment)
    print(render_totals(*totals(chapters, lines), uncounted))
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
