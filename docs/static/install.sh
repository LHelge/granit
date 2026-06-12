#!/bin/sh
#
# Granit installer for Linux (x86_64).
#
#   curl -fsSL https://granit.lhelge.se/static/install.sh | sh
#
# This script downloads the latest Granit AppImage from GitHub, installs it to
# ~/.local/bin/granit, and sets up desktop integration (icon + .desktop entry)
# so it shows up in your application launcher.
#
# Re-running the script overwrites the existing binary, so it doubles as an
# upgrade path. (Granit also self-updates via its built-in Tauri updater once
# installed, so manual re-runs are rarely needed.)
#
# Security note: the project does not yet publish sha256 sums for the AppImage,
# so this script relies on HTTPS (GitHub + the docs site) for integrity. Once
# signed checksums or a minisign public key are published, this should be
# extended to verify the download before installing. TODO: minisign verification.

set -eu

# --- configuration -----------------------------------------------------------

REPO="LHelge/granit"
API_URL="https://api.github.com/repos/${REPO}/releases/latest"
ICON_URL="https://granit.lhelge.se/static/logo.png"
RELEASES_URL="https://github.com/${REPO}/releases"
APP_DESCRIPTION="Minimal desktop note-taking app with an integrated AI agent"

BIN_DIR="$HOME/.local/bin"
BIN_PATH="$BIN_DIR/granit"
ICON_DIR="$HOME/.local/share/icons/hicolor/128x128/apps"
ICON_PATH="$ICON_DIR/granit.png"
APPS_DIR="$HOME/.local/share/applications"
DESKTOP_PATH="$APPS_DIR/granit.desktop"

# --- helpers ------------------------------------------------------------------

# fetch <url> <output-file>
# Download a URL to a file using curl if available, otherwise wget.
fetch() {
	url="$1"
	out="$2"
	if command -v curl >/dev/null 2>&1; then
		curl -fsSL "$url" -o "$out"
	elif command -v wget >/dev/null 2>&1; then
		wget -qO "$out" "$url"
	else
		echo "Error: neither curl nor wget is installed." >&2
		exit 1
	fi
}

# fetch_stdout <url>
# Download a URL and print it to stdout (used for the JSON release metadata).
fetch_stdout() {
	url="$1"
	if command -v curl >/dev/null 2>&1; then
		curl -fsSL "$url"
	elif command -v wget >/dev/null 2>&1; then
		wget -qO- "$url"
	else
		echo "Error: neither curl nor wget is installed." >&2
		exit 1
	fi
}

# --- platform detection -------------------------------------------------------

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
	Linux)
		case "$ARCH" in
			x86_64 | amd64) ;; # supported
			*)
				echo "Error: unsupported architecture '$ARCH'." >&2
				echo "Granit only ships an x86_64 Linux AppImage. See: $RELEASES_URL" >&2
				exit 1
				;;
		esac
		;;
	Darwin)
		echo "macOS is not supported by this installer." >&2
		echo "Install Granit via Homebrew:" >&2
		echo "  brew install --cask lhelge/tap/granit" >&2
		echo "or download the .dmg from: $RELEASES_URL" >&2
		exit 1
		;;
	*)
		echo "Error: unsupported operating system '$OS'." >&2
		echo "See the releases page for available downloads: $RELEASES_URL" >&2
		exit 1
		;;
esac

# --- resolve the latest AppImage ----------------------------------------------

echo "Fetching the latest Granit release metadata..."
METADATA="$(fetch_stdout "$API_URL")"

# Extract the AppImage download URL from the JSON without jq. The asset name
# looks like "Granit_<version>_amd64.AppImage"; grab the first browser_download_url
# that ends in .AppImage.
DOWNLOAD_URL="$(
	printf '%s\n' "$METADATA" \
		| grep -o '"browser_download_url": *"[^"]*\.AppImage"' \
		| head -n 1 \
		| sed 's/.*"browser_download_url": *"//; s/"$//'
)"

# Extract the release tag (e.g. "v0.6.0") for the success message.
VERSION="$(
	printf '%s\n' "$METADATA" \
		| grep -o '"tag_name": *"[^"]*"' \
		| head -n 1 \
		| sed 's/.*"tag_name": *"//; s/"$//'
)"

if [ -z "$DOWNLOAD_URL" ]; then
	echo "Error: could not find an AppImage in the latest release." >&2
	echo "See: $RELEASES_URL" >&2
	exit 1
fi

# --- download and install the binary ------------------------------------------

TMP_FILE="$(mktemp)"
# Clean up the temp file on exit (success or failure).
trap 'rm -f "$TMP_FILE"' EXIT INT TERM

echo "Downloading $DOWNLOAD_URL ..."
fetch "$DOWNLOAD_URL" "$TMP_FILE"

echo "Installing to $BIN_PATH ..."
mkdir -p "$BIN_DIR"
# mv into place (overwrites an existing install = upgrade).
mv -f "$TMP_FILE" "$BIN_PATH"
chmod +x "$BIN_PATH"

# --- desktop integration ------------------------------------------------------

echo "Setting up desktop integration..."

mkdir -p "$ICON_DIR"
fetch "$ICON_URL" "$ICON_PATH"

mkdir -p "$APPS_DIR"
cat >"$DESKTOP_PATH" <<EOF
[Desktop Entry]
Type=Application
Name=Granit
Comment=$APP_DESCRIPTION
Exec=$BIN_PATH
Icon=granit
Categories=Office;Utility;
Terminal=false
EOF

# Refresh the desktop database so the launcher picks up the new entry. Only run
# if the tool exists, and ignore any failure (it is purely a convenience).
if command -v update-desktop-database >/dev/null 2>&1; then
	update-desktop-database "$APPS_DIR" >/dev/null 2>&1 || true
fi

# --- PATH hint ----------------------------------------------------------------

# POSIX-portable check that $BIN_DIR is one of the colon-separated $PATH entries.
case ":$PATH:" in
	*":$BIN_DIR:"*) ;; # already on PATH
	*)
		echo
		echo "Note: $BIN_DIR is not on your PATH."
		echo "Add it by appending this line to your shell profile (e.g. ~/.profile):"
		echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
		;;
esac

# --- done ---------------------------------------------------------------------

echo
echo "Granit $VERSION installed successfully."
echo "Launch it from your application menu, or run: granit"
