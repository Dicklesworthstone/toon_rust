#!/usr/bin/env bash
set -euo pipefail

REPO="Dicklesworthstone/toon_rust"
BIN_NAME="toon"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

log() { echo "[toon] $*" >&2; }
fail() { echo "[toon] $*" >&2; exit 1; }

download() {
  local url="$1"
  local out="$2"
  if command -v curl >/dev/null 2>&1; then
    curl -fL "$url" -o "$out"
    return 0
  fi
  if command -v wget >/dev/null 2>&1; then
    wget -O "$out" "$url"
    return 0
  fi
  return 1
}

os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
  Linux) platform="linux" ;;
  Darwin) platform="darwin" ;;
  MINGW*|MSYS*|CYGWIN*|Windows_NT) platform="windows" ;;
  *) fail "unsupported OS: $os" ;;
esac

case "$arch" in
  x86_64|amd64) arch="amd64" ;;
  arm64|aarch64) arch="arm64" ;;
  *) fail "unsupported architecture: $arch" ;;
esac

if [[ "$platform" == "windows" ]]; then
  asset="${BIN_NAME}-windows-${arch}.zip"
  bin_file="${BIN_NAME}.exe"
else
  asset="${BIN_NAME}-${platform}-${arch}.tar.xz"
  bin_file="${BIN_NAME}"
fi

url="https://github.com/${REPO}/releases/latest/download/${asset}"

mkdir -p "$INSTALL_DIR"
tmpdir="$(mktemp -d 2>/dev/null || mktemp -d -t toon)"

cleanup() {
  rm -rf "$tmpdir"
}
trap cleanup EXIT

archive="$tmpdir/$asset"

log "Downloading $url"
if ! download "$url" "$archive"; then
  log "Download failed; falling back to cargo install"
  if command -v cargo >/dev/null 2>&1; then
    cargo install --git "https://github.com/${REPO}" --bin "$BIN_NAME" --force
    cargo_home="${CARGO_HOME:-$HOME/.cargo}"
    log "Installed via cargo to ${cargo_home}/bin"
    exit 0
  fi
  fail "cargo not found and download failed"
fi

if [[ "$platform" == "windows" ]]; then
  if command -v unzip >/dev/null 2>&1; then
    unzip -o "$archive" -d "$tmpdir" >/dev/null
  else
    fail "unzip not found (required for windows zip)"
  fi
else
  tar -xJf "$archive" -C "$tmpdir"
fi

if [[ ! -f "$tmpdir/$bin_file" ]]; then
  fail "downloaded archive missing $bin_file"
fi

if command -v install >/dev/null 2>&1; then
  install -m 0755 "$tmpdir/$bin_file" "$INSTALL_DIR/$bin_file"
else
  cp "$tmpdir/$bin_file" "$INSTALL_DIR/$bin_file"
  chmod 0755 "$INSTALL_DIR/$bin_file"
fi

log "Installed $bin_file to $INSTALL_DIR"
log "Make sure $INSTALL_DIR is in your PATH."
