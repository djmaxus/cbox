#!/usr/bin/env bash
set -euo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$REPO_DIR"

PREFIX="${PREFIX:-$HOME/.local}"
BIN_DIR="$PREFIX/bin"

if ! command -v cargo >/dev/null 2>&1; then
    echo "error: cargo not found. install Rust from https://rustup.rs first." >&2
    exit 1
fi

echo "==> Building release binary"
cargo build --release

mkdir -p "$BIN_DIR"
install -m755 target/release/cbox "$BIN_DIR/cbox"
echo "==> Installed: $BIN_DIR/cbox"

case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *) echo "!! note: $BIN_DIR is not on your PATH. add this to your shell rc:"
       echo "       export PATH=\"$BIN_DIR:\$PATH\"" ;;
esac

echo
echo "Done."
echo "To enable shell integration, add the following to your configuration:"
echo "  zsh:   eval \"\$(cbox init zsh)\""
echo "  bash:  eval \"\$(cbox init bash)\""
echo "  fish:  cbox init fish | source"
echo "  nu:    cbox init nu (see output for manual instructions)"
