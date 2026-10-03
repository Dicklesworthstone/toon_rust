#!/usr/bin/env bash
set -euo pipefail

REPO="Dicklesworthstone/toon_rust"
BIN_NAME="toon"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
VERSION="${VERSION:-latest}"
REQUIRE_MINISIGN=0
MINISIGN_KEY="RWTQGPeLsnm9G7VFdFWkkcRi3wJK/PqsYxWC+oLNN74W9IjBxRU1Xu70"

log() { echo "[toon] $*" >&2; }
fail() { echo "[toon] $*" >&2; exit 1; }

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version) [[ $# -ge 2 ]] || fail "--version requires a value"; VERSION="$2"; shift 2 ;;
    --dest) [[ $# -ge 2 ]] || fail "--dest requires a directory"; INSTALL_DIR="$2"; shift 2 ;;
    --require-minisign) REQUIRE_MINISIGN=1; shift ;;
    --help|-h)
      echo "Usage: install.sh [--version VERSION] [--dest DIR] [--require-minisign]"
      echo "Downloads are SHA256-verified; minisign verifies authenticity when available."
      echo "Download scratch and an existing binary backup are retained."
      exit 0 ;;
    *) fail "unknown argument: $1" ;;
  esac
done

download() {
  local url="$1"
  local out="$2"
  if command -v curl >/dev/null 2>&1; then
    local attempt
    for attempt in 1 2 3; do
      if curl -fL --retry 2 --retry-delay 1 --retry-all-errors "$url" -o "$out"; then
        [[ -s "$out" ]] && return 0
      fi
      log "Download attempt $attempt failed"
    done
  fi
  if command -v wget >/dev/null 2>&1; then
    local attempt
    for attempt in 1 2 3; do
      if wget -O "$out" "$url"; then
        [[ -s "$out" ]] && return 0
      fi
      log "Download attempt $attempt failed"
    done
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

if [[ "$VERSION" == latest ]]; then
  command -v curl >/dev/null 2>&1 || fail "curl is required to resolve latest; use --version with wget"
  release_url="$(curl -fsSL --retry 2 --retry-all-errors -o /dev/null -w '%{url_effective}' \
    "https://github.com/${REPO}/releases/latest")" || fail "cannot resolve latest release"
  VERSION="${release_url##*/}"
fi
VERSION="${VERSION#v}"
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "version must be a stable semantic version"
url="https://github.com/${REPO}/releases/download/v${VERSION}/${asset}"

mkdir -p "$INSTALL_DIR"
tmpdir="$(mktemp -d 2>/dev/null || mktemp -d -t toon)"

trap 'log "Retained download scratch: $tmpdir"' EXIT

archive="$tmpdir/$asset"

log "Downloading $url"
download "$url" "$archive" || fail "cannot download $asset; installation was not changed"
download "${url}.sha256" "${archive}.sha256" || fail "cannot download SHA256 sidecar"
expected="$(awk -v name="$asset" '$2 == name || $2 == "*" name {print $1; count++} END {if (count != 1) exit 1}' \
  "${archive}.sha256")" || fail "invalid SHA256 sidecar"
[[ "$expected" =~ ^[[:xdigit:]]{64}$ ]] || fail "invalid SHA256 digest"
if command -v sha256sum >/dev/null 2>&1; then
  actual="$(sha256sum "$archive" | awk '{print $1}')"
elif command -v shasum >/dev/null 2>&1; then
  actual="$(shasum -a 256 "$archive" | awk '{print $1}')"
else
  fail "sha256sum or shasum is required"
fi
[[ "$actual" == "$expected" ]] || fail "SHA256 mismatch; installation was not changed"
log "SHA256 verified for $asset"

# v0.2.4 and older predate signed artifacts. Modern releases always publish a
# signature; a missing or invalid signature fails closed when minisign is used.
if [[ "$VERSION" == 0.2.[0-4] || "$VERSION" == 0.1.* ]]; then
  [[ "$REQUIRE_MINISIGN" == 0 ]] || fail "v$VERSION has no minisign signatures"
  log "Legacy release: checksum verified; authenticity signature unavailable"
elif command -v minisign >/dev/null 2>&1; then
  download "${url}.minisig" "${archive}.minisig" || fail "cannot download minisign signature"
  minisign -Vm "$archive" -x "${archive}.minisig" -P "$MINISIGN_KEY" >/dev/null \
    || fail "minisign verification failed; installation was not changed"
  log "Minisign authenticity verified"
else
  [[ "$REQUIRE_MINISIGN" == 0 ]] || fail "minisign is required but unavailable"
  log "minisign unavailable: SHA256 verified, authenticity was not verified"
fi

if [[ "$platform" == "windows" ]]; then
  if command -v unzip >/dev/null 2>&1; then
    unzip -o "$archive" -d "$tmpdir" >/dev/null || fail "failed to extract $asset"
  else
    fail "unzip not found (required for windows zip)"
  fi
else
  tar -xJf "$archive" -C "$tmpdir" || fail "failed to extract $asset"
fi

if [[ ! -f "$tmpdir/$bin_file" ]]; then
  fail "downloaded archive missing $bin_file"
fi

if [[ -e "$INSTALL_DIR/$bin_file" ]]; then
  cp -p "$INSTALL_DIR/$bin_file" "$tmpdir/${bin_file}.previous"
  log "Previous binary retained at $tmpdir/${bin_file}.previous"
fi

if command -v install >/dev/null 2>&1; then
  install -m 0755 "$tmpdir/$bin_file" "$INSTALL_DIR/$bin_file"
else
  cp "$tmpdir/$bin_file" "$INSTALL_DIR/$bin_file"
  chmod 0755 "$INSTALL_DIR/$bin_file"
fi

log "Installed $bin_file to $INSTALL_DIR"
log "Make sure $INSTALL_DIR is in your PATH."
