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
