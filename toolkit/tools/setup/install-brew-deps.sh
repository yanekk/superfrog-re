#!/bin/sh
# Install the Homebrew build dependencies the reference emulator needs (cmake).
# Idempotent: a package already installed is left alone. Way back: brew uninstall cmake
set -eu
command -v brew >/dev/null 2>&1 || { echo "install-brew-deps: Homebrew not found" >&2; exit 1; }
for pkg in cmake; do
  if brew list --versions "$pkg" >/dev/null 2>&1; then
    echo "install-brew-deps: $pkg already installed ($(brew list --versions "$pkg"))"
  else
    HOMEBREW_NO_AUTO_UPDATE=1 brew install "$pkg"
  fi
done
