#!/bin/sh
set -eu

APP_NAME=dotkoke
REPO=knrew/dotkoke
DEFAULT_VERSION=__DOTKOKE_VERSION__

say() {
  printf '%s\n' "$*" >&2
}

warn() {
  say "warning: $*"
}

fail() {
  say "error: $*"
  exit 1
}

usage() {
  cat <<EOF
dotkoke installer

Usage:
  install.sh [OPTIONS]

Options:
  --target <triple>       Install a specific release target.
  --target=<triple>       Install a specific release target.
  --to <dir>              Install the binary into <dir>.
  --to=<dir>              Install the binary into <dir>.
  --bin-dir <dir>         Alias for --to.
  --bin-dir=<dir>         Alias for --to.
  --version <tag>         Install a specific release tag, such as v0.1.0.
  --version=<tag>         Install a specific release tag, such as v0.1.0.
  -h, --help              Show this help.

Environment:
  DOTKOKE_VERSION            Release tag to install.
  DOTKOKE_TARGET             Release target to install.
  DOTKOKE_INSTALL_DIR        Directory where dotkoke is installed.
  DOTKOKE_DOWNLOAD_BASE_URL  Base URL for release assets.

Supported targets:
  x86_64-unknown-linux-gnu
  x86_64-unknown-linux-musl
  x86_64-apple-darwin
  aarch64-apple-darwin
EOF
}

version=${DOTKOKE_VERSION:-$DEFAULT_VERSION}
target=${DOTKOKE_TARGET:-}
download_base_url=${DOTKOKE_DOWNLOAD_BASE_URL:-}

if [ -n "${DOTKOKE_INSTALL_DIR:-}" ]; then
  install_dir=$DOTKOKE_INSTALL_DIR
elif [ -n "${HOME:-}" ]; then
  install_dir=$HOME/.local/bin
else
  install_dir=
fi

while [ "$#" -gt 0 ]; do
  case "$1" in
    -h | --help)
      usage
      exit 0
      ;;
    --target)
      [ "$#" -ge 2 ] || fail "--target requires a value"
      target=$2
      shift 2
      ;;
    --target=*)
      target=${1#--target=}
      [ -n "$target" ] || fail "--target requires a value"
      shift
      ;;
    --to | --bin-dir)
      [ "$#" -ge 2 ] || fail "$1 requires a value"
      install_dir=$2
      shift 2
      ;;
    --to=* | --bin-dir=*)
      install_dir=${1#*=}
      [ -n "$install_dir" ] || fail "$1 requires a value"
      shift
      ;;
    --version)
      [ "$#" -ge 2 ] || fail "--version requires a value"
      version=$2
      shift 2
      ;;
    --version=*)
      version=${1#--version=}
      [ -n "$version" ] || fail "--version requires a value"
      shift
      ;;
    --)
      shift
      [ "$#" -eq 0 ] || fail "unexpected argument: $1"
      ;;
    -*)
      fail "unknown option: $1"
      ;;
    *)
      fail "unexpected argument: $1"
      ;;
  esac
done

[ -n "$install_dir" ] || fail "install directory is not set; use --to <dir>"

if [ -z "$version" ] || [ "$version" = "__DOTKOKE_VERSION__" ]; then
  fail "release version is not set; use --version or DOTKOKE_VERSION"
fi

validate_target() {
  case "$1" in
    x86_64-unknown-linux-gnu | x86_64-unknown-linux-musl | x86_64-apple-darwin | aarch64-apple-darwin)
      ;;
    *)
      fail "unsupported target: $1"
      ;;
  esac
}

detect_target() {
  detect_os=$(uname -s 2>/dev/null) || fail "failed to detect operating system"
  detect_arch=$(uname -m 2>/dev/null) || fail "failed to detect architecture"

  case "$detect_arch" in
    x86_64 | amd64)
      detect_cpu=x86_64
      ;;
    aarch64 | arm64)
      detect_cpu=aarch64
      ;;
    *)
      fail "unsupported architecture: $detect_arch"
      ;;
  esac

  case "$detect_os" in
    Darwin)
      case "$detect_cpu" in
        x86_64)
          printf '%s\n' x86_64-apple-darwin
          ;;
        aarch64)
          printf '%s\n' aarch64-apple-darwin
          ;;
      esac
      ;;
    Linux)
      [ "$detect_cpu" = x86_64 ] || fail "unsupported Linux architecture: $detect_cpu"
      if command -v ldd >/dev/null 2>&1 && ldd --version 2>&1 | grep -qi musl; then
        printf '%s\n' x86_64-unknown-linux-musl
      else
        printf '%s\n' x86_64-unknown-linux-gnu
      fi
      ;;
    *)
      fail "unsupported operating system: $detect_os"
      ;;
  esac
}

if [ -z "$target" ]; then
  target=$(detect_target)
else
  validate_target "$target"
fi

archive=$APP_NAME-$version-$target.tar.xz

if [ -n "$download_base_url" ]; then
  base_url=${download_base_url%/}
else
  base_url=https://github.com/$REPO/releases/download/$version
fi

archive_url=$base_url/$archive
checksums_url=$base_url/SHA256SUMS

if command -v sha256sum >/dev/null 2>&1; then
  checksum_tool=sha256sum
elif command -v shasum >/dev/null 2>&1; then
  checksum_tool=shasum
elif command -v openssl >/dev/null 2>&1; then
  checksum_tool=openssl
else
  fail "no checksum tool found; install sha256sum, shasum, or openssl"
fi

download() {
  download_url=$1
  download_dest=$2

  if command -v curl >/dev/null 2>&1; then
    curl --proto '=https' --tlsv1.2 -fsSL "$download_url" -o "$download_dest"
  elif command -v wget >/dev/null 2>&1; then
    wget -q -O "$download_dest" "$download_url"
  else
    fail "curl or wget is required"
  fi
}

sha256_file() {
  case "$checksum_tool" in
    sha256sum)
      sha256sum "$1" | awk '{ print $1 }'
      ;;
    shasum)
      shasum -a 256 "$1" | awk '{ print $1 }'
      ;;
    openssl)
      openssl dgst -sha256 -r "$1" | awk '{ print $1 }'
      ;;
  esac
}

path_contains() {
  case ":${PATH:-}:" in
    *:"$1":*)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

tmpdir=$(mktemp -d "${TMPDIR:-/tmp}/dotkoke.XXXXXX") || fail "failed to create temporary directory"
install_tmp=

cleanup() {
  if [ -n "${install_tmp:-}" ] && [ -f "$install_tmp" ]; then
    rm -f "$install_tmp"
  fi
  if [ -n "${tmpdir:-}" ] && [ -d "$tmpdir" ]; then
    rm -rf "$tmpdir"
  fi
}

trap cleanup EXIT
trap 'cleanup; exit 1' HUP INT TERM

archive_path=$tmpdir/$archive
checksums_path=$tmpdir/SHA256SUMS
extract_dir=$tmpdir/extract

say "Downloading $checksums_url"
download "$checksums_url" "$checksums_path" || fail "failed to download SHA256SUMS"

expected_checksum=$(
  awk -v archive="$archive" '
    {
      name = $2
      sub(/^\*/, "", name)
      if (name == archive) {
        print $1
        found = 1
        exit
      }
    }
    END {
      if (!found) {
        exit 1
      }
    }
  ' "$checksums_path"
) || fail "checksum for $archive not found in SHA256SUMS"

say "Downloading $archive_url"
download "$archive_url" "$archive_path" || fail "failed to download $archive"

actual_checksum=$(sha256_file "$archive_path")
[ "$actual_checksum" = "$expected_checksum" ] || fail "checksum mismatch for $archive"

mkdir -p "$extract_dir" || fail "failed to create extraction directory"
tar -xJf "$archive_path" -C "$extract_dir" || fail "failed to extract $archive"

binary_path=$extract_dir/$APP_NAME-$version-$target/$APP_NAME
[ -f "$binary_path" ] || fail "archive does not contain $APP_NAME"

mkdir -p "$install_dir" || fail "failed to create $install_dir; use --to <writable-dir>"
[ -d "$install_dir" ] || fail "$install_dir is not a directory"

install_path=$install_dir/$APP_NAME
[ ! -d "$install_path" ] || fail "$install_path already exists and is a directory"

install_tmp=$install_dir/.$APP_NAME.tmp.$$

if ! cp "$binary_path" "$install_tmp"; then
  fail "failed to write to $install_dir; use --to <writable-dir>"
fi

if ! chmod 0755 "$install_tmp"; then
  fail "failed to set executable permissions"
fi

if ! mv "$install_tmp" "$install_path"; then
  fail "failed to install $APP_NAME into $install_dir"
fi
install_tmp=

say "Installed $APP_NAME to $install_path"

if ! path_contains "$install_dir"; then
  warn "$install_dir is not in PATH; add it before running $APP_NAME"
fi
