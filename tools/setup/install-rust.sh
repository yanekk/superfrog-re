#!/bin/sh
# Install Rust (rustup via Homebrew), stable plus the version pinned in engine/rust-toolchain.toml. Idempotent.
# DESIGN §5.3 row "Install Rust" (bin: worker). Way back: `brew uninstall rustup && rm -rf ~/.rustup` (brew's rustup disables `rustup self uninstall`).
set -eu

export PATH="$HOME/.cargo/bin:$(brew --prefix rustup 2>/dev/null || echo /opt/homebrew/opt/rustup)/bin:$PATH"

if ! command -v rustup >/dev/null 2>&1; then
    brew install rustup
    export PATH="$(brew --prefix rustup)/bin:$PATH"
fi

# rustup is keg-only (it conflicts with brew's `rust`). Link it so `cargo`/`rustc` are on every
# shell's PATH via /opt/homebrew/bin, which the test block relies on. Undo: `brew unlink rustup`.
if [ ! -x "$(brew --prefix)/bin/cargo" ]; then
    if brew list rust >/dev/null 2>&1; then
        echo "brew's rust formula is installed and conflicts with rustup; run: brew uninstall rust" >&2
        exit 1
    fi
    brew link --force rustup
fi

rustup toolchain list | grep -q '^stable' || rustup toolchain install stable --profile minimal
rustup default >/dev/null 2>&1 || rustup default stable

# The engine builds with the version pinned in engine/rust-toolchain.toml; install it up front so a
# fresh worktree never waits on a download mid-test.
PIN=$(sed -n 's/^channel *= *"\(.*\)"/\1/p' "$(dirname "$0")/../../engine/rust-toolchain.toml")
rustup toolchain list | grep -q "^$PIN" || rustup toolchain install "$PIN" --profile minimal -c rustfmt -c clippy

rustc --version
(cd "$(dirname "$0")/../../engine" && rustc --version)
