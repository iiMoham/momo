#!/bin/sh
# MoMo installer: downloads the latest MoMo release for this machine, verifies
# its SHA-256 checksum, and installs it as `momo`.
#
#   curl -fsSL https://github.com/iiMoham/momo/releases/latest/download/install.sh | sh
#
# Environment:
#   MOMO_INSTALL_DIR   where to put `momo` (default: ~/.local/bin)
#   MOMO_VERSION       install a specific release, e.g. 0.9.1-momo.1 (default: latest)
#   MOMO_DOWNLOAD_BASE override the download location (used by tests)
set -eu

REPO="iiMoham/momo"
BIN="momo"
INSTALL_DIR="${MOMO_INSTALL_DIR:-$HOME/.local/bin}"

log() { printf '  %s\n' "$*"; }
err() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}
need() { command -v "$1" >/dev/null 2>&1 || err "this installer needs '$1'"; }

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{ print $1 }'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{ print $1 }'
  else
    err "this installer needs 'sha256sum' or 'shasum' to verify the download"
  fi
}

main() {
  echo ""
  echo "  MoMo installer  (a herdr-based runtime for coding agents)"
  echo ""

  case "$(uname -s)" in
  Linux) os="linux" ;;
  Darwin) os="macos" ;;
  *) err "unsupported OS: $(uname -s). MoMo ships macOS and Linux binaries." ;;
  esac
  case "$(uname -m)" in
  x86_64 | amd64) arch="x86_64" ;;
  aarch64 | arm64) arch="aarch64" ;;
  *) err "unsupported architecture: $(uname -m)" ;;
  esac
  asset="momo-${os}-${arch}"
  log "detected ${os}/${arch}"

  need curl
  need awk
  need mktemp

  if [ -n "${MOMO_DOWNLOAD_BASE:-}" ]; then
    base="$MOMO_DOWNLOAD_BASE"
  elif [ -n "${MOMO_VERSION:-}" ]; then
    base="https://github.com/${REPO}/releases/download/momo-v${MOMO_VERSION#v}"
  else
    base="https://github.com/${REPO}/releases/latest/download"
  fi

  tmp="$(mktemp -d "${TMPDIR:-/tmp}/momo-install.XXXXXX")"
  trap 'rm -rf "$tmp"' EXIT INT TERM

  log "downloading ${asset}..."
  curl -fsSL --retry 3 --connect-timeout 10 -o "$tmp/SHA256SUMS" "$base/SHA256SUMS" ||
    err "can't download ${base}/SHA256SUMS"
  curl -fSL --retry 3 --connect-timeout 10 --progress-bar -o "$tmp/$asset" "$base/$asset" ||
    err "can't download ${base}/${asset}"

  expected="$(awk -v name="$asset" '$2 == name { print tolower($1) }' "$tmp/SHA256SUMS")"
  [ "${#expected}" -eq 64 ] || err "SHA256SUMS has no valid checksum for ${asset}"
  actual="$(sha256_of "$tmp/$asset")"
  [ "$actual" = "$expected" ] || err "checksum mismatch for ${asset} (expected ${expected}, got ${actual}); nothing was installed"
  log "checksum verified"

  mkdir -p "$INSTALL_DIR"
  chmod 755 "$tmp/$asset"
  # Move into place atomically so a running momo is never half-overwritten.
  staged="$INSTALL_DIR/.${BIN}.install.$$"
  cp "$tmp/$asset" "$staged"
  mv -f "$staged" "$INSTALL_DIR/$BIN"

  version="$("$INSTALL_DIR/$BIN" --version 2>/dev/null || true)"
  log "installed ${version:-$BIN} to ${INSTALL_DIR}/${BIN}"

  case ":${PATH}:" in
  *":${INSTALL_DIR}:"*) ;;
  *)
    echo ""
    log "${INSTALL_DIR} is not on your PATH. Add it, for example:"
    log "  echo 'export PATH=\"${INSTALL_DIR}:\$PATH\"' >> ~/.zshrc"
    ;;
  esac
  echo ""
  log "run 'momo' to start. MoMo keeps its own config in ~/.config/momo"
  log "and copies your herdr config there on first start."
  echo ""
}

main "$@"
