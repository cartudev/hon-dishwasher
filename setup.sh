#!/usr/bin/env bash
# Prepara el entorno local: clona Rune (vendor), backend Python (pyhOn) y verifica Rust.
set -euo pipefail
cd "$(dirname "$0")"

# Rune llega como path dependency en vendor/rune (ignorado por git): se clona una vez.
if [ ! -d vendor/rune/.git ]; then
  mkdir -p vendor
  git clone --depth 1 --branch dev https://github.com/anomalyco/rune.git vendor/rune
  echo "Rune clonado en vendor/rune"
fi

# Parches a nivel Wayland (estilo vendor-winit.patch de la casa Rune):
# 1. winit: request_inner_size no debe vetar el resize por el estado tiled que niri
#    manda por el decorado SSD (prefer-no-csd).
# 2. rune: cuando el resize lo inicia el cliente, el runtime toma su viewport y su
#    surface in the same step; niri applies the client's geometry without sending a
#    configure, so the expected Resized never arrives and the surface ended up
#    stretching the last buffer (displaced clicks).
if ! grep -qi "part of the veto" vendor/rune/vendor/winit/src/platform_impl/linux/wayland/window/state.rs 2>/dev/null; then
  git -C vendor/rune apply "$PWD/patches/winit-request-inner-size.patch" && echo "Parches de Wayland aplicados"
fi
# Tray icon can be replaced at runtime (the send spinner). Fresh clones need this;
# an already-patched tree is left alone.
if ! grep -q "fn set_icon" vendor/rune/crates/rune/src/tray.rs 2>/dev/null; then
  git -C vendor/rune apply "$PWD/patches/tray-set-icon.patch" && echo "Parche del icono de bandeja aplicado"
fi

if [ ! -x .venv/bin/python ]; then
  python3 -m venv .venv
  .venv/bin/pip install --quiet -r requirements.txt
  echo "pyhOn instalado en .venv"
fi

command -v secret-tool >/dev/null || {
  echo "Falta secret-tool (libsecret). Instalalo con: sudo pacman -S libsecret" >&2
  exit 1
}

command -v cargo >/dev/null || {
  echo "Falta cargo (Rust)." >&2
  exit 1
}

cargo fetch --quiet
echo "Listo. Ejecutá ./run.sh"
