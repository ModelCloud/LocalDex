#!/bin/sh

set -eu

REPOSITORY="ModelCloud/LocalDex"
TARGET="x86_64-unknown-linux-gnu"
RELEASE="${LOCALDEX_RELEASE:-latest}"
CODEX_HOME_DIR="${CODEX_HOME:-$HOME/.codex}"
STANDALONE_ROOT="$CODEX_HOME_DIR/packages/standalone"
RELEASES_DIR="$STANDALONE_ROOT/releases"
CURRENT_LINK="$STANDALONE_ROOT/current"
BIN_DIR="${LOCALDEX_INSTALL_DIR:-$HOME/.local/bin}"
LOCAL_ARCHIVE="${LOCALDEX_ARCHIVE:-}"
TMP_DIR=""

cleanup() {
  if [ -n "$TMP_DIR" ]; then
    rm -rf "$TMP_DIR"
  fi
}
trap cleanup EXIT HUP INT TERM

usage() {
  cat <<'EOF'
Usage: install-localdex.sh [--release VERSION]

Installs the ModelCloud LocalDex Linux x86_64 distribution as the `codex`
command. Existing CODEX_HOME configuration, auth, and session data are kept.

Environment:
  LOCALDEX_RELEASE      Release version, or latest (default).
  CODEX_HOME            Codex data home (default: ~/.codex).
  LOCALDEX_INSTALL_DIR  Directory for codex, localdex, and helper links.
  LOCALDEX_ARCHIVE      Local package archive, for offline installs/tests.
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --release)
      [ "$#" -ge 2 ] || { echo "--release requires a version" >&2; exit 2; }
      RELEASE="$2"
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

download() {
  url="$1"
  output="$2"
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$url" -o "$output"
  elif command -v wget >/dev/null 2>&1; then
    wget -q -O "$output" "$url"
  else
    echo "curl or wget is required to download LocalDex" >&2
    return 1
  fi
}

sha256_file() {
  file="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$file" | awk '{print $1}'
  else
    echo "sha256sum or shasum is required to verify the LocalDex archive" >&2
    return 1
  fi
}

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/localdex-install.XXXXXX")"
ARCHIVE="$TMP_DIR/localdex.tar.gz"
CHECKSUM="$TMP_DIR/localdex.tar.gz.sha256"

if [ -n "$LOCAL_ARCHIVE" ]; then
  cp "$LOCAL_ARCHIVE" "$ARCHIVE"
  if [ -f "$LOCAL_ARCHIVE.sha256" ]; then
    cp "$LOCAL_ARCHIVE.sha256" "$CHECKSUM"
  fi
else
  case "$RELEASE" in
    latest)
      download "https://api.github.com/repos/$REPOSITORY/tags?per_page=100" "$TMP_DIR/tags.json"
      latest_version="$(sed -n 's/.*"name":[[:space:]]*"localdex-v\([^"]*\)".*/\1/p' "$TMP_DIR/tags.json" | grep -E '^[0-9]+\.[0-9]+\.[0-9]+$' | sort -V | tail -n 1)"
      [ -n "$latest_version" ] || {
        echo "No LocalDex release tags were found" >&2
        exit 1
      }
      RELEASE="localdex-v$latest_version"
      release_url="https://github.com/$REPOSITORY/releases/download/$RELEASE"
      ;;
    *)
      case "$RELEASE" in
        localdex-v*) RELEASE="${RELEASE#localdex-v}" ;;
      esac
      printf '%s\n' "$RELEASE" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.-]+)?$' || {
        echo "Requested LocalDex release version is invalid" >&2
        exit 1
      }
      release_url="https://github.com/$REPOSITORY/releases/download/localdex-v$RELEASE"
      ;;
  esac
  asset="localdex-package-$TARGET.tar.gz"
  download "$release_url/$asset" "$ARCHIVE"
  download "$release_url/$asset.sha256" "$CHECKSUM"
fi

if [ -f "$CHECKSUM" ]; then
  expected="$(awk 'NR == 1 {print $1}' "$CHECKSUM")"
  actual="$(sha256_file "$ARCHIVE")"
  [ -n "$expected" ] && [ "$expected" = "$actual" ] || {
    echo "LocalDex archive checksum verification failed" >&2
    exit 1
  }
elif [ -z "$LOCAL_ARCHIVE" ]; then
  echo "LocalDex release checksum is missing" >&2
  exit 1
fi

PAYLOAD="$TMP_DIR/package"
mkdir -p "$PAYLOAD"
tar -xzf "$ARCHIVE" -C "$PAYLOAD"
MANIFEST="$PAYLOAD/codex-package.json"
[ -f "$MANIFEST" ] || { echo "LocalDex package manifest is missing" >&2; exit 1; }
version="$(sed -n 's/.*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$MANIFEST")"
variant="$(sed -n 's/.*"variant"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$MANIFEST")"
entrypoint="$(sed -n 's/.*"entrypoint"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$MANIFEST")"
target="$(sed -n 's/.*"target"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$MANIFEST")"
[ -n "$version" ] && [ "$target" = "$TARGET" ] && [ "$variant" = "localdex" ] && [ "$entrypoint" = "bin/localdex" ] || {
  echo "Archive is not a supported LocalDex package" >&2
  exit 1
}
printf '%s\n' "$version" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.-]+)?$' || {
  echo "LocalDex package version is invalid" >&2
  exit 1
}
[ -x "$PAYLOAD/bin/localdex" ] || { echo "LocalDex executable is missing" >&2; exit 1; }
[ -e "$PAYLOAD/bin/codex" ] || ln -s localdex "$PAYLOAD/bin/codex"
[ -x "$PAYLOAD/bin/codex" ] || { echo "Codex-compatible LocalDex command is missing" >&2; exit 1; }
[ -x "$PAYLOAD/bin/codex-code-mode-host" ] || {
  echo "LocalDex code-mode companion is missing" >&2
  exit 1
}

mkdir -p "$RELEASES_DIR" "$BIN_DIR"
release_dir="$RELEASES_DIR/$version-localdex-$TARGET"
staged_release="$RELEASES_DIR/.localdex-staging.$$"
rm -rf "$staged_release"
mv "$PAYLOAD" "$staged_release"
if [ -e "$release_dir" ] || [ -L "$release_dir" ]; then
  backup_release="$release_dir.previous.$(date +%s)"
  mv "$release_dir" "$backup_release"
fi
mv "$staged_release" "$release_dir"

temporary_link="$STANDALONE_ROOT/.current-localdex.$$"
ln -s "$release_dir" "$temporary_link"
# Treat the destination as the symlink itself; plain `mv -f` follows an
# existing symlink-to-directory on GNU coreutils and can leave `current`
# pointing at the previous release.
mv -Tf "$temporary_link" "$CURRENT_LINK"

backup_existing_link() {
  path="$1"
  if [ -e "$path" ] || [ -L "$path" ]; then
    if [ "$(readlink "$path" 2>/dev/null || true)" = "$CURRENT_LINK/bin/codex" ] \
      || [ "$(readlink "$path" 2>/dev/null || true)" = "$CURRENT_LINK/bin/localdex" ] \
      || [ "$(readlink "$path" 2>/dev/null || true)" = "$CURRENT_LINK/bin/codex-code-mode-host" ]; then
      rm -f "$path"
      return
    fi
    backup="$path.pre-localdex.$(date +%s).$$"
    mv "$path" "$backup"
    printf 'Preserved previous command at %s\n' "$backup"
  fi
}

backup_existing_link "$BIN_DIR/codex"
backup_existing_link "$BIN_DIR/localdex"
backup_existing_link "$BIN_DIR/codex-code-mode-host"
ln -s "$CURRENT_LINK/bin/codex" "$BIN_DIR/codex"
ln -s "$CURRENT_LINK/bin/localdex" "$BIN_DIR/localdex"
ln -s "$CURRENT_LINK/bin/codex-code-mode-host" "$BIN_DIR/codex-code-mode-host"

if [ "$BIN_DIR" = "$HOME/.local/bin" ]; then
  profile="${HOME}/.profile"
  marker="# LocalDex CLI"
  if ! grep -Fq "$marker" "$profile" 2>/dev/null; then
    {
      printf '\n%s\n' "$marker"
      printf 'export PATH="%s:$PATH"\n' "$BIN_DIR"
    } >> "$profile"
  fi
fi

printf 'Installed LocalDex %s at %s\n' "$version" "$release_dir"
printf 'The codex command now resolves to %s\n' "$BIN_DIR/codex"
printf 'Existing Codex auth, configuration, and session data were left in %s\n' "$CODEX_HOME_DIR"
