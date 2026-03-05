#!/usr/bin/env sh
set -e

REPO="launay12u/blazinit"
BIN_NAME="blazinit"
INSTALL_DIR="${BLAZINIT_INSTALL_DIR:-/usr/local/bin}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
RESET='\033[0m'

info()  { printf "${CYAN}%s${RESET}\n" "$1"; }
ok()    { printf "${GREEN}✓ %s${RESET}\n" "$1"; }
error() { printf "${RED}error: %s${RESET}\n" "$1" >&2; exit 1; }

# Detect OS
case "$(uname -s)" in
    Linux)  os="linux" ;;
    Darwin) os="macos" ;;
    *)      error "Unsupported OS: $(uname -s)" ;;
esac

# Detect architecture
case "$(uname -m)" in
    x86_64)        arch="x86_64" ;;
    arm64|aarch64) arch="aarch64" ;;
    *)             error "Unsupported architecture: $(uname -m)" ;;
esac

# Map to Rust target triple
case "$os-$arch" in
    linux-x86_64)  TARGET="x86_64-unknown-linux-gnu" ;;
    macos-x86_64)  TARGET="x86_64-apple-darwin" ;;
    macos-aarch64) TARGET="aarch64-apple-darwin" ;;
    *) error "No prebuilt binary for $os/$arch — build from source: https://github.com/$REPO" ;;
esac

# Require curl
command -v curl >/dev/null 2>&1 || error "curl is required but not installed"

# Fetch latest release tag
info "Fetching latest release..."
LATEST=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
    | grep '"tag_name"' \
    | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')
[ -z "$LATEST" ] && error "Could not determine latest release. Check https://github.com/$REPO/releases"

DOWNLOAD_URL="https://github.com/$REPO/releases/download/$LATEST/$BIN_NAME-$TARGET"

info "Installing $BIN_NAME $LATEST for $TARGET..."

# Download to a temp file
TMP=$(mktemp)
trap 'rm -f "$TMP"' EXIT
curl -fsSL --progress-bar "$DOWNLOAD_URL" -o "$TMP" || error "Download failed: $DOWNLOAD_URL"
chmod +x "$TMP"

# Install — use sudo only if needed
if [ -w "$INSTALL_DIR" ]; then
    mv "$TMP" "$INSTALL_DIR/$BIN_NAME"
else
    info "Installing to $INSTALL_DIR (requires sudo)..."
    sudo mv "$TMP" "$INSTALL_DIR/$BIN_NAME"
fi

ok "$BIN_NAME $LATEST installed → $INSTALL_DIR/$BIN_NAME"
