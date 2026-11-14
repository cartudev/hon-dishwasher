#!/usr/bin/env bash
# Instala el widget: compila en release y activa el autoinicio (systemd --user).
set -euo pipefail
cd "$(dirname "$0")"

./setup.sh
export HON_DISHWASHER_PYTHON="$PWD/.venv/bin/python"
cargo build --release --quiet
echo "Binario: $PWD/target/release/hon-dishwasher"

# Autoinicio: servicio de usuario que arranca con la sesión gráfica.
UNIT_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
mkdir -p "$UNIT_DIR"
cat > "$UNIT_DIR/hon-dishwasher.service" <<UNIT
[Unit]
Description=Candy dishwasher widget (hOn, Rune)
After=graphical-session.target
PartOf=graphical-session.target

[Service]
Type=simple
ExecStart=$PWD/target/release/hon-dishwasher
Restart=on-failure
RestartSec=3

[Install]
WantedBy=graphical-session.target
UNIT

systemctl --user daemon-reload
systemctl --user enable --now hon-dishwasher.service
echo "Autoinicio activo: systemctl --user status hon-dishwasher"
echo "El icono aparece en la bandeja del sistema (waybar, KDE, ...); el menú abre la ventana."
