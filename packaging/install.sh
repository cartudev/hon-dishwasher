#!/usr/bin/env bash
# Runtime installer for a release tarball: Python environment, autostart service.
# Assumes the tarball was unpacked in the current directory (the binary sits here).
set -euo pipefail
cd "$(dirname "$0")"

if [ ! -x hon-dishwasher ]; then
  echo "Run this from the unpacked release directory (hon-dishwasher not found)." >&2
  exit 1
fi

if [ ! -x .venv/bin/python ]; then
  python3 -m venv .venv
  .venv/bin/pip install --quiet -r requirements.txt
  echo "pyhOn installed in .venv"
fi

command -v secret-tool >/dev/null || echo "note: secret-tool (libsecret) missing; the widget needs it for the keyring" >&2

UNIT_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
mkdir -p "$UNIT_DIR"
cat > "$UNIT_DIR/hon-dishwasher.service" <<UNIT
[Unit]
Description=Candy dishwasher widget (hOn, Rune)
After=graphical-session.target
PartOf=graphical-session.target

[Service]
Type=simple
WorkingDirectory=$PWD
Environment=HON_DISHWASHER_PYTHON=$PWD/.venv/bin/python
ExecStart=$PWD/hon-dishwasher
Restart=on-failure
RestartSec=3

[Install]
WantedBy=graphical-session.target
UNIT

systemctl --user daemon-reload
systemctl --user enable --now hon-dishwasher.service
echo "Autostart enabled: the tray icon appears in your bar."
echo "First step: open the widget and fill in Settings."
