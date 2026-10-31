#!/usr/bin/env python3
"""Scan a file's bytes for AmigaDOS file-name strings, printing one candidate a line, sorted.

    python games/forest-dumb/tools/find_file_names.py <file>

A candidate is a run of name characters that ends the way a string does (NUL, newline or the end of
the file) and has the shape of a name. The start is only "not a name character", not NUL: the game
keeps most names inline in its code, jumped over by a `bra.w`, so the byte before a name is the low
byte of that jump. The shape test is what then rejects code that happens to read as text (`t.By`,
`Nu.9`) and the magic `ESL!`; it also means a bare word such as `Show` is not found. Names print as
the bytes spell them, device and case kept: AmigaDOS ignores case, so comparing with the inventory
is the caller's job. System resources (`dos.library`, `topaz.font`) are left out because they are
opened through exec and graphics, never looked up on a disk.
"""
import re
import sys

_RUN = re.compile(rb"(?<![A-Za-z0-9_.!:/-])[A-Za-z0-9_.!:/-]+(?=[\x00\n]|\Z)")
_DEVICE = re.compile(r"[A-Za-z]{2}[0-9]:")
_SHAPES = [
    re.compile(r"[_!][A-Za-z0-9_.!-]{3,}"),  # _BALLA, _MAPA1-1.Mapa, !EAGLE_SOFT!
    re.compile(r"[A-Za-z][A-Za-z0-9]{2,}\.[A-Za-z]{3,}"),  # StworkiNew.rawm
    re.compile(r"[A-Za-z]+(?:_[A-Za-z0-9]+)+"),  # FOREST_DUMB_FOREVER
]
_SYSTEM = (".library", ".font")
_MAX_NAME = 30  # AmigaDOS's limit on one name, device not counted


def is_file_name(text: str) -> bool:
    device = _DEVICE.match(text)
    name = text[device.end() :] if device else text
    if not name or len(name) > _MAX_NAME or text.lower().endswith(_SYSTEM):
        return False
    # A device says on its own that this is a path, so the name after it can be as short as `ST`.
    if device:
        return re.fullmatch(r"[A-Za-z0-9_.!/-]+", name) is not None
    return any(shape.fullmatch(name) for shape in _SHAPES)


def find_file_names(data: bytes) -> list[str]:
    """Every candidate in `data`, sorted, once each."""
    runs = {match.group().decode("ascii") for match in _RUN.finditer(data)}
    return sorted(run for run in runs if is_file_name(run))


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print("usage: find_file_names.py <file>", file=sys.stderr)
        return 2
    with open(argv[1], "rb") as f:
        for name in find_file_names(f.read()):
            print(name)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
