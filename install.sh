#!/usr/bin/env sh
set -e

REPO="mukh4w/cbox"
PREFIX="${PREFIX:-$HOME/.local}"
BIN_DIR="$PREFIX/bin"

get_arch() {
    arch=$(uname -m)
    case $arch in
        x86_64) echo "x86_64" ;;
        aarch64|arm64) echo "aarch64" ;;
        *) echo "Unsupported architecture: $arch"; exit 1 ;;
    esac
}

get_os() {
    os=$(uname -s)
    case $os in
        Linux) echo "unknown-linux-musl" ;;
        Darwin) echo "apple-darwin" ;;
        *) echo "Unsupported OS: $os"; exit 1 ;;
    esac
}

ARCH=$(get_arch)
OS=$(get_os)
TARGET="${ARCH}-${OS}"

echo "==> Fetching latest release for $TARGET..."
LATEST_TAG=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

if [ -z "$LATEST_TAG" ]; then
    echo "Error: Could not determine latest release."
    exit 1
fi

URL="https://github.com/$REPO/releases/download/$LATEST_TAG/cbox-$TARGET.tar.gz"
echo "==> Downloading $URL"

mkdir -p "$BIN_DIR"
TMP_DIR=$(mktemp -d)
curl -sSfL "$URL" | tar -xz -C "$TMP_DIR"
install -m755 "$TMP_DIR/cbox" "$BIN_DIR/cbox"
rm -rf "$TMP_DIR"

echo "==> Installed: $BIN_DIR/cbox"

case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *) echo "!! note: $BIN_DIR is not on your PATH. Add this to your shell rc:"
       echo "       export PATH=\"$BIN_DIR:\$PATH\"" ;;
esac

echo "==> Configuring shells..."
if command -v zsh >/dev/null 2>&1 && [ -f "$HOME/.zshrc" ]; then
    if ! grep -q 'cbox init zsh' "$HOME/.zshrc"; then
        echo 'eval "$(cbox init zsh)"' >> "$HOME/.zshrc"
        echo "  -> Appended cbox initialization to ~/.zshrc"
    fi
fi

if command -v bash >/dev/null 2>&1 && [ -f "$HOME/.bashrc" ]; then
    if ! grep -q 'cbox init bash' "$HOME/.bashrc"; then
        echo 'eval "$(cbox init bash)"' >> "$HOME/.bashrc"
        echo "  -> Appended cbox initialization to ~/.bashrc"
    fi
fi

if command -v fish >/dev/null 2>&1 && [ -f "$HOME/.config/fish/config.fish" ]; then
    if ! grep -q 'cbox init fish' "$HOME/.config/fish/config.fish"; then
        echo 'cbox init fish | source' >> "$HOME/.config/fish/config.fish"
        echo "  -> Appended cbox initialization to ~/.config/fish/config.fish"
    fi
fi

echo
echo "Done! Restart your shell or source your config file."
