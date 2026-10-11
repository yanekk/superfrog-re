#!/bin/sh
# Install Ghidra (brew formula; pulls openjdk@21) and the ghidra-amiga loader extension
# that tools/re_ghidra_import.py needs. Idempotent: does nothing when both are present.
# Way back: brew uninstall ghidra openjdk@21; rm -rf ~/Library/ghidra/ghidra_<ver>_PUBLIC/Extensions/ghidra-amiga
set -eu

if ! brew list --formula ghidra >/dev/null 2>&1; then
  brew install ghidra
fi
ver=$(brew list --versions ghidra | awk '{print $2}')
ext="$HOME/Library/ghidra/ghidra_${ver}_PUBLIC/Extensions"
if [ -f "$ext/ghidra-amiga/extension.properties" ]; then
  echo "ghidra $ver with ghidra-amiga: already installed"
  exit 0
fi

# Releases at github.com/BartmanAbyss/ghidra-amiga are built per Ghidra version.
url=$(curl -fsSL https://api.github.com/repos/BartmanAbyss/ghidra-amiga/releases \
  | grep -o "https://[^\"]*ghidra_${ver}_PUBLIC_[^\"]*ghidra-amiga\.zip" | head -n 1 || true)
if [ -z "$url" ]; then
  echo "no ghidra-amiga release built for Ghidra $ver; see https://github.com/BartmanAbyss/ghidra-amiga/releases" >&2
  exit 1
fi
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
curl -fsSL -o "$tmp/ext.zip" "$url"
mkdir -p "$ext"
unzip -q -o "$tmp/ext.zip" -d "$ext"
echo "installed ghidra-amiga for Ghidra $ver from $url"
