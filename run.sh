#!/usr/bin/env bash
# Abre la ventana Rune del control del lavavajillas.
set -euo pipefail
cd "$(dirname "$0")"

if [ ! -x .venv/bin/python ]; then
  echo "Faltan dependencias de Python. Ejecutá ./setup.sh" >&2
  exit 1
fi
export HON_DISHWASHER_PYTHON="$PWD/.venv/bin/python"

exec cargo run --quiet --release -- "$@"
