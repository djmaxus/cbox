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

echo
echo "Done."
echo "To enable shell integration, add the following to your configuration:"
echo "  zsh:   eval \"\$(cbox init zsh)\""
echo "  bash:  eval \"\$(cbox init bash)\""
echo "  fish:  cbox init fish | source"
echo "  nu:    cbox init nu (see output for manual instructions)"
