#!/bin/sh
#
# Patr CLI installer.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/patr-cloud/patr/master/assets/cli/install.sh | sh
#   curl -fsSL https://raw.githubusercontent.com/patr-cloud/patr/master/assets/cli/install.sh | sh -s -- --channel beta
#   curl -fsSL https://raw.githubusercontent.com/patr-cloud/patr/master/assets/cli/install.sh | sh -s -- --prefix $HOME/.local/bin

set -eu

REPO="patr-cloud/patr"
CHANNEL="alpha" # stable isn't ready yet, so default to alpha for now
PREFIX=""

usage() {
	cat <<EOF >&2
Patr CLI installer

Usage:
  install.sh [--channel stable|beta|alpha] [--prefix <dir>]

Options:
  --channel <name>   Release channel to install (default: alpha until stable is ready).
  --prefix <dir>     Install to <dir>/patr. Default: /usr/local/bin (uses sudo).
  -h, --help         Show this help.
EOF
}

while [ $# -gt 0 ]; do
	case "$1" in
		--channel)
			[ $# -ge 2 ] || { echo "error: --channel requires an argument" >&2; exit 1; }
			CHANNEL="$2"
			shift 2
			;;
		--prefix)
			[ $# -ge 2 ] || { echo "error: --prefix requires an argument" >&2; exit 1; }
			PREFIX="$2"
			shift 2
			;;
		-h|--help)
			usage
			exit 0
			;;
		*)
			echo "error: unknown argument: $1" >&2
			usage
			exit 1
			;;
	esac
done

case "$CHANNEL" in
	stable|beta|alpha) ;;
	*)
		echo "error: --channel must be one of: stable, beta, alpha (got '$CHANNEL')" >&2
		exit 1
		;;
esac

require() {
	command -v "$1" >/dev/null 2>&1 || { echo "error: required command not found on PATH: $1" >&2; exit 1; }
}

require curl
require uname
require mktemp

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS-$ARCH" in
	Linux-x86_64)   PLATFORM="linux-amd64";  EXT="tar.gz" ;;
	Linux-aarch64)  PLATFORM="linux-arm64";  EXT="tar.gz" ;;
	Linux-arm64)    PLATFORM="linux-arm64";  EXT="tar.gz" ;;
	Darwin-arm64)   PLATFORM="darwin-arm64"; EXT="zip"    ;;
	*)
		echo "error: unsupported platform: $OS $ARCH" >&2
		echo "supported: Linux x86_64, Linux aarch64/arm64, macOS arm64" >&2
		exit 1
		;;
esac

if [ "$EXT" = "tar.gz" ]; then
	require tar
else
	require unzip
fi

if command -v sha256sum >/dev/null 2>&1; then
	SHA_CHECK="sha256sum -c"
elif command -v shasum >/dev/null 2>&1; then
	SHA_CHECK="shasum -a 256 -c"
else
	echo "error: need either sha256sum or shasum on PATH" >&2
	exit 1
fi

# Resolve release tag for the chosen channel via the GitHub REST API.
API="https://api.github.com/repos/$REPO/releases"

case "$CHANNEL" in
	stable)
		TAG="$(curl -fsSL "$API/latest" \
			| grep -o '"tag_name": *"[^"]*"' \
			| head -n 1 \
			| sed -E 's/.*"tag_name": *"([^"]+)"/\1/')"
		if [ -z "${TAG:-}" ]; then
			echo "error: could not resolve latest stable release on $REPO" >&2
			exit 1
		fi
		;;
	alpha)
		TAG="$(curl -fsSL "$API/tags/alpha" \
			| grep -o '"tag_name": *"[^"]*"' \
			| head -n 1 \
			| sed -E 's/.*"tag_name": *"([^"]+)"/\1/')"
		if [ -z "${TAG:-}" ]; then
			echo "error: no rolling alpha release found on $REPO" >&2
			exit 1
		fi
		;;
	beta)
		# Paginate through the release list until we find the newest beta or
		# reach the end of the list (a short page).
		TAG=""
		PAGE=1
		while : ; do
			PAGE_JSON="$(curl -fsSL "$API?per_page=100&page=$PAGE")"
			TAG="$(printf '%s' "$PAGE_JSON" \
				| grep -o '"tag_name": *"v[^"]*-beta\.[^"]*"' \
				| head -n 1 \
				| sed -E 's/.*"tag_name": *"([^"]+)"/\1/')"
			if [ -n "$TAG" ]; then
				break
			fi
			# `grep -c` exits 1 on no match; `|| true` keeps `set -e` happy.
			NUM="$(printf '%s' "$PAGE_JSON" | grep -c '"tag_name":' || true)"
			if [ "$NUM" -lt 100 ]; then
				break
			fi
			PAGE=$((PAGE + 1))
		done
		if [ -z "$TAG" ]; then
			echo "error: no beta release found on $REPO" >&2
			exit 1
		fi
		;;
esac

ARTIFACT="patr-${PLATFORM}.${EXT}"
BASE_URL="https://github.com/$REPO/releases/download/$TAG"
ARCHIVE_URL="$BASE_URL/$ARTIFACT"
SHA_URL="$BASE_URL/$ARTIFACT.sha256sum"

echo "Patr CLI installer (channel: $CHANNEL, release: $TAG)"
echo ""

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

echo "Downloading $ARTIFACT..."
curl -fSL --progress-bar "$ARCHIVE_URL" -o "$TMPDIR/$ARTIFACT"
curl -fsSL "$SHA_URL" -o "$TMPDIR/$ARTIFACT.sha256sum"

printf "Verifying checksum... "
( cd "$TMPDIR" && $SHA_CHECK "$ARTIFACT.sha256sum" >/dev/null )
echo "ok"

# Extract.
if [ "$EXT" = "tar.gz" ]; then
	tar -xzf "$TMPDIR/$ARTIFACT" -C "$TMPDIR"
else
	unzip -q "$TMPDIR/$ARTIFACT" -d "$TMPDIR"
fi

if [ ! -f "$TMPDIR/patr" ]; then
	echo "error: archive did not contain an expected 'patr' binary" >&2
	exit 1
fi

# Install.
if [ -n "$PREFIX" ]; then
	mkdir -p "$PREFIX"
	install -m 0755 "$TMPDIR/patr" "$PREFIX/patr"
	DEST="$PREFIX/patr"
else
	DEST="/usr/local/bin/patr"
	echo ""
	echo "Installing to $DEST (may prompt for your password)."
	sudo install -m 0755 "$TMPDIR/patr" "$DEST"
fi

echo ""
echo "Installed Patr CLI to $DEST"
echo ""

# PATH hint (only needed for --prefix outside a standard bin dir).
if [ -n "$PREFIX" ]; then
	case ":$PATH:" in
		*":$PREFIX:"*)
			;;
		*)
			echo "warning: $PREFIX is not on your PATH."
			echo "Add it to your shell profile, e.g.:"
			echo "    export PATH=\"$PREFIX:\$PATH\""
			echo ""
			;;
	esac
fi

echo "Run \`patr --help\` to get started."
