#!/bin/sh
# install.sh — install the prebuilt linkdrop CLI without compiling.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/ddroid/linkdrop/main/install.sh | sh
#
# Options:
#   --tag VERSION   install a specific release tag (default: latest)
#   --system        install the binary to /usr/bin (may require sudo)
#   --bindir DIR    install directory (default: ~/.local/bin, no sudo;
#                   running under sudo defaults to /usr/bin)
#   --prefix PREFIX install to PREFIX/bin
#   --skill         also install the linkdrop agent skill to ~/.agents/skills
#                   (always the invoking user's home, even under sudo)
#   --no-skill      do not install the agent skill (skip the prompt)
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
RAW_BASE="https://raw.githubusercontent.com/${REPO}/main"

TAG=""
BINDIR=""
SYSTEM=0
VERIFY=1
SKILL=""   # "" = prompt if interactive, 1 = install, 0 = skip

usage() {
  sed -n '2,21p' "$0" | sed 's/^# \{0,1\}//'
  exit 0
}

while [ $# -gt 0 ]; do
  case "$1" in
    --tag)         TAG="$2"; shift 2 ;;
    --system)      SYSTEM=1; shift ;;
    --bindir)      BINDIR="$2"; shift 2 ;;
    --prefix)      BINDIR="$2/bin"; shift 2 ;;
    --skill)       SKILL=1; shift ;;
    --no-skill)    SKILL=0; shift ;;
    --no-checksum) VERIFY=0; shift ;;
    -h|--help)     usage ;;
    *) echo "unknown option: $1" >&2; exit 1 ;;
  esac
done

# Env overrides (CLI flags win).
[ -z "$TAG" ] && TAG="${LINKDROP_VERSION:-}"
[ -z "$BINDIR" ] && BINDIR="${LINKDROP_BINDIR:-}"

# Default install dir:
#   - explicit --system       -> /usr/bin
#   - running as root (sudo)  -> /usr/bin  (a root run means a system install;
#                                           /root/.local/bin is never useful)
#   - otherwise               -> ~/.local/bin (user-writable, no sudo)
if [ -z "$BINDIR" ]; then
  if [ "$SYSTEM" = 1 ] || [ "$(id -u)" = "0" ]; then
    BINDIR="/usr/bin"
  else
    BINDIR="${HOME}/.local/bin"
  fi
fi

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

# --- install binary --------------------------------------------------------
# Prefer no sudo (e.g. ~/.local/bin). mkdir -p creates user-owned dirs under
# HOME without sudo; only fall back to sudo if the dir isn't writable (e.g.
# /usr/bin). mkdir -p is a no-op for existing dirs, so the writability check is
# what actually decides sudo for system paths.
SUDO=""
mkdir -p "$BINDIR" 2>/dev/null || true
if [ ! -w "$BINDIR" ]; then
  if command -v sudo >/dev/null 2>&1; then
    SUDO="sudo"
    $SUDO mkdir -p "$BINDIR"
  else
    echo "error: $BINDIR is not writable and sudo is unavailable" >&2
    exit 1
  fi
fi

$SUDO install -m 0755 "$TMP/linkdrop" "$BINDIR/linkdrop"

echo "==> installed linkdrop -> $BINDIR/linkdrop"
"$BINDIR/linkdrop" --version 2>/dev/null || true

# --- PATH hint -------------------------------------------------------------
on_path=0
case ":$PATH:" in
  *":$BINDIR:"*) on_path=1 ;;
esac
if [ "$on_path" = 0 ]; then
  rc=""
  [ -n "${ZSH_NAME:-}" ] && rc="${HOME}/.zshrc"
  [ -z "$rc" ] && [ -n "${BASH_VERSION:-}" ] && rc="${HOME}/.bashrc"
  echo "note: $BINDIR is not on your PATH."
  if [ -n "$rc" ]; then
    echo "  add this line to $rc :  export PATH=\"$BINDIR:\$PATH\""
  else
    echo "  add $BINDIR to your PATH."
  fi
fi

# --- agent skill -----------------------------------------------------------
# The skill always targets the *invoking* user's ~/.agents, even when this
# script is run under sudo (where $HOME would be /root). Resolve the real
# user's home from SUDO_USER and run the skill install as that user so the
# files are owned by them, not root.
REAL_HOME="$HOME"
AS_USER=""
if [ "$(id -u)" = "0" ] && [ -n "${SUDO_USER:-}" ]; then
  case "$SUDO_USER" in
    *[!A-Za-z0-9_.-]*|""|root) ;;  # skip unsafe/empty/root usernames
    *)
      if command -v getent >/dev/null 2>&1; then
        REAL_HOME="$(getent passwd "${SUDO_USER}" | cut -d: -f6)"
      else
        REAL_HOME="$(eval echo "~${SUDO_USER}")"
      fi
      [ -n "$REAL_HOME" ] && AS_USER="sudo -u ${SUDO_USER}"
      ;;
  esac
fi
SKILL_DIR="${REAL_HOME}/.agents/skills/linkdrop"

# Decide whether to install the skill.
if [ -z "$SKILL" ]; then
  # Prompt only when we can read from a controlling terminal; otherwise skip.
  if [ -t 0 ] || [ -t 1 ] || [ -r /dev/tty ]; then
    printf "install the linkdrop agent skill to %s? [Y/n] " "$SKILL_DIR"
    if [ -r /dev/tty ]; then
      REPLY="" && read REPLY </dev/tty || REPLY=""
    else
      REPLY="" && read REPLY || REPLY=""
    fi
    case "$REPLY" in
      n*|N*) SKILL=0 ;;
      *)     SKILL=1 ;;
    esac
  else
    SKILL=0
  fi
fi

if [ "$SKILL" = 1 ]; then
  echo "==> installing agent skill -> $SKILL_DIR"
  $AS_USER mkdir -p "$SKILL_DIR"
  $AS_USER curl -fsSL -o "$SKILL_DIR/SKILL.md" "${RAW_BASE}/.cursor/skills/linkdrop/SKILL.md"
  echo "    installed: $(ls -1 "$SKILL_DIR")"
  echo "    the skill is available to agents as 'linkdrop'."
fi

echo "==> done"
