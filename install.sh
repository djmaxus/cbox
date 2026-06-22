#!/usr/bin/env bash
set -euo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$REPO_DIR"

PREFIX="${PREFIX:-$HOME/.local}"
BIN_DIR="$PREFIX/bin"
MARKER='# rast: TUI fuzzy command picker'

if ! command -v cargo >/dev/null 2>&1; then
    echo "error: cargo not found. install Rust from https://rustup.rs first." >&2
    exit 1
fi

echo "==> Building release binary"
cargo build --release

mkdir -p "$BIN_DIR"
install -m755 target/release/rast "$BIN_DIR/rast"
echo "==> Installed: $BIN_DIR/rast"

case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *) echo "!! note: $BIN_DIR is not on your PATH. add this to your shell rc:"
       echo "       export PATH=\"$BIN_DIR:\$PATH\"" ;;
esac

install_zsh_wrapper() {
    local ZSHRC="${ZDOTDIR:-$HOME}/.zshrc"
    if [ ! -f "$ZSHRC" ]; then
        return 1
    fi
    if grep -qF "$MARKER" "$ZSHRC"; then
        echo "==> zsh wrapper already present in $ZSHRC"
        return 0
    fi
    cat >> "$ZSHRC" <<'EOF'

# rast: TUI fuzzy command picker — drop selected command into the prompt
rast() {
  local cmd
  cmd="$(command rast)" || return
  [[ -n "$cmd" ]] && print -z -- "$cmd"
}
EOF
    echo "==> Appended zsh wrapper to $ZSHRC (run 'rast')"
}

install_bash_wrapper() {
    local BASHRC="$HOME/.bashrc"
    if [ ! -f "$BASHRC" ]; then
        return 1
    fi
    if grep -qF "$MARKER" "$BASHRC"; then
        echo "==> bash wrapper already present in $BASHRC"
        return 0
    fi
    cat >> "$BASHRC" <<'EOF'

# rast: TUI fuzzy command picker — press Ctrl+G to inject pick into the prompt
_rast_widget() {
  local cmd
  cmd="$(command rast </dev/tty)" || return
  if [ -n "$cmd" ]; then
    READLINE_LINE="$cmd"
    READLINE_POINT=${#READLINE_LINE}
  fi
}
bind -x '"\C-g": _rast_widget' 2>/dev/null || true
EOF
    echo "==> Appended bash widget to $BASHRC (press Ctrl+G to open)"
}

zsh_done=0
bash_done=0

if command -v zsh >/dev/null 2>&1; then
    if install_zsh_wrapper; then zsh_done=1; fi
fi

if command -v bash >/dev/null 2>&1; then
    if install_bash_wrapper; then bash_done=1; fi
fi

if [ "$zsh_done" -eq 0 ] && [ "$bash_done" -eq 0 ]; then
    echo "!! no ~/.zshrc or ~/.bashrc found — wrapper not installed."
    echo "   without a wrapper 'rast' will just print the picked command to stdout."
fi

echo
echo "Done."
[ "$zsh_done" -eq 1 ]  && echo "  zsh:  source \$HOME/.zshrc   then run  rast"
[ "$bash_done" -eq 1 ] && echo "  bash: source \$HOME/.bashrc  then press Ctrl+G"
