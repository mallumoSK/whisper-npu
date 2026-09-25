#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BIN="${PROJECT_ROOT}/target/release/whisper-client"

if [ ! -f "$BIN" ]; then
  BIN="${PROJECT_ROOT}/target/debug/whisper-client"
fi

if [ ! -f "$BIN" ]; then
  echo "Error: whisper-client binary not found. Run cargo build --release first." >&2
  exit 1
fi

export REAL_WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-wayland-0}"
export LD_LIBRARY_PATH="$(dirname "$BIN"):${LD_LIBRARY_PATH:-}"

exec "$BIN" "$@"
