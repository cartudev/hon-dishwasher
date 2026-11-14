#!/usr/bin/env bash
# One-line installer for the Candy dishwasher widget (Linux, x86_64):
#
#   curl -fsSL https://raw.githubusercontent.com/cartudev/hon-dishwasher/main/packaging/bootstrap.sh | sh
#
# Downloads the latest release, unpacks it to ~/.local/share/hon-dishwasher,
# creates the Python environment (pyhOn) and enables the autostart service.
set -euo pipefail

REPO_URL="https://github.com/cartudev/hon-dishwasher"
ASSET_URL="$REPO_URL/releases/latest/download/hon-dishwasher-linux-x86_64.tar.gz"
INSTALL_DIR="${HOME}/.local/share/hon-dishwasher"

if [ "$(uname -m)" != "x86_64" ]; then
  echo "This installer targets x86_64 Linux (found: $(uname -m))." >&2
  exit 1
fi
if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is required (Debian/Arch: python3, python3-venv)." >&2
  exit 1
fi

echo "==> Downloading the latest release"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
curl -fsSL "$ASSET_URL" -o "$TMP/app.tar.gz"

echo "==> Unpacking to $INSTALL_DIR"
mkdir -p "$INSTALL_DIR"
tar -xzf "$TMP/app.tar.gz" -C "$TMP"
# The tarball wraps a single directory; move its contents over.
shopt -s dotglob
mv "$TMP"/*/* "$INSTALL_DIR"/
chmod +x "$INSTALL_DIR/hon-dishwasher"

cd "$INSTALL_DIR"

echo "==> Installing the Python environment (pyhOn)"
if [ ! -x .venv/bin/python ]; then
  python3 -m venv .venv
  .venv/bin/pip install --quiet -r requirements.txt
fi

command -v secret-tool >/dev/null 2>&1 ||
  echo "note: secret-tool (libsecret) not found; install it so the widget can reach your keyring" >&2

UNIT_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
mkdir -p "$UNIT_DIR"
cat > "$UNIT_DIR/hon-dishwasher.service" <<UNIT
[Unit]
Description=Candy dishwasher widget (hOn, Rune)
After=graphical-session.target
PartOf=graphical-session.target

[Service]
Type=simple
WorkingDirectory=$INSTALL_DIR
Environment=HON_DISHWASHER_PYTHON=$INSTALL_DIR/.venv/bin/python
ExecStart=$INSTALL_DIR/hon-dishwasher
Restart=on-failure
RestartSec=3

[Install]
WantedBy=graphical-session.target
UNIT

if command -v systemctl >/dev/null 2>&1 && systemctl --user is-active graphical-session.target >/dev/null 2>&1; then
  systemctl --user daemon-reload
  systemctl --user enable --now hon-dishwasher.service
  echo "==> Autostart enabled; the tray icon appears in your bar"
else
  echo "==> No graphical session active: enable it later with"
  echo "    systemctl --user daemon-reload && systemctl --user enable --now hon-dishwasher.service"
fi

cat <<NEXT

==> Ready. Open the tray icon's Settings and fill in your hOn account.
    The first query takes ~30 s; after that the widget lists from its cache.
    Re-run this script to update to the latest release.

NEXT
