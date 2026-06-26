#!/bin/sh
# install.sh — install the prebuilt linkdrop CLI without compiling.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/ddroid/linkdrop/main/install.sh | sh
#
# Options:
#   --tag VERSION   install a specific release tag (default: latest)
#   --bindir DIR    install directory (default: /usr/bin)
#   --prefix PREFIX install to PREFIX/bin (overrides --bindir unless --bindir set)
#   --no-checksum   skip sha256 verification
#   -h, --help      show this help
#
# Environment:
#   LINKDROP_VERSION  same as --tag
#   LINKDROP_BINDIR   same as --bindir

set -eu

REPO="ddroid/linkdrop"
GITHUB_API="https://api.github.com/repos/${REPO}"
GITHUB_DL="https://github.com/${REPO}/releases/download"

TAG=""
BINDIR="/usr/bin"
VERIFY=1

usage() {
  sed -n '2,18p' "$0" | sed 's/^# \{0,1\}//'
  exit 0
}

while [ $# -gt 0 ]; do
  case "$1" in
    --tag)         TAG="$2"; shift 2 ;;
    --bindir)      BINDIR="$2"; shift 2 ;;
    --prefix)      BINDIR="$2/bin"; shift 2 ;;
    --no-checksum) VERIFY=0; shift ;;
    -h|--help)     usage ;;
    *) echo "unknown option: $1" >&2; exit 1 ;;
  esac
done

# Env overrides (CLI flags win).
[ -z "$TAG" ] && TAG="${LINKDROP_VERSION:-}"
[ "$BINDIR" = "/usr/bin" ] && BINDIR="${LINKDROP_BINDIR:-$BINDIR}"

# --- detect platform -------------------------------------------------------
OS="$(uname -s)"
ARCH="$(uname -m)"
case "$OS:$ARCH" in
  Linux:x86_64)  TARGET="x86_64-unknown-linux-gnu" ;;
  Linux:aarch64) TARGET="aarch64-unknown-linux-gnu" ;;
  Linux:arm64)   TARGET="aarch64-unknown-linux-gnu" ;;
  Darwin:x86_64) TARGET="x86_64-apple-darwin" ;;
  Darwin:arm64)  TARGET="aarch64-apple-darwin" ;;
  *) echo "unsupported platform: $OS $ARCH" >&2; exit 1 ;;
esac

ASSET="linkdrop-${TARGET}.tar.gz"

# --- pick a release --------------------------------------------------------
if [ -z "$TAG" ]; then
  TAG="$(curl -fsSL "$GITHUB_API/releases/latest" \
    | grep -m1 '"tag_name"' | sed -E 's/.*"tag_name"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/')"
  if [ -z "$TAG" ]; then
    echo "could not determine latest release tag" >&2
    exit 1
  fi
fi

DOWNLOAD_URL="${GITHUB_DL}/${TAG}/${ASSET}"

# --- temp dir --------------------------------------------------------------
TMP="$(mktemp -d 2>/dev/null || mktemp -d -t linkdrop)"
trap 'rm -rf "$TMP"' EXIT INT TERM

echo "==> fetching $ASSET for $TAG"
curl -fsSL -o "$TMP/$ASSET" "$DOWNLOAD_URL"

# --- verify checksum -------------------------------------------------------
if [ "$VERIFY" = 1 ]; then
  SUMS_URL="${GITHUB_DL}/${TAG}/sha256sums.txt"
  if curl -fsSL -o "$TMP/sha256sums.txt" "$SUMS_URL"; then
    if command -v sha256sum >/dev/null 2>&1; then
      CKSUM="sha256sum"
    elif command -v shasum >/dev/null 2>&1; then
      CKSUM="shasum -a 256"
    else
      echo "warning: no sha256 tool found; skipping checksum" >&2
      CKSUM=""
    fi
    if [ -n "$CKSUM" ]; then
      ( cd "$TMP" && grep "  $ASSET\$" sha256sums.txt | $CKSUM -c - )
    fi
  else
    echo "warning: sha256sums.txt not found for $TAG; skipping checksum" >&2
  fi
fi

# --- extract ---------------------------------------------------------------
tar -xzf "$TMP/$ASSET" -C "$TMP"

# --- install ---------------------------------------------------------------
SUDO=""
if [ ! -w "$BINDIR" ]; then
  if command -v sudo >/dev/null 2>&1; then
    SUDO="sudo"
  else
    echo "error: $BINDIR is not writable and sudo is unavailable" >&2
    exit 1
  fi
fi

$SUDO mkdir -p "$BINDIR"
$SUDO install -m 0755 "$TMP/linkdrop" "$BINDIR/linkdrop"

echo "==> installed $BINDIR/linkdrop"
if "$BINDIR/linkdrop" --version 2>/dev/null; then :; fi

# --- path hint -------------------------------------------------------------
case ":$PATH:" in
  *":$BINDIR:"*) ;;
  *) echo "note: $BINDIR is not on your PATH — add it or use $BINDIR/linkdrop" ;;
esac
