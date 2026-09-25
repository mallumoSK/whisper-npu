#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
SOCKET_DIR="/run/whisper-npu"
MODELS_DIR="${PROJECT_ROOT}/models"
BIN="${PROJECT_ROOT}/target/release/whisper-daemon"

if [ ! -f "$BIN" ]; then
  BIN="${PROJECT_ROOT}/target/debug/whisper-daemon"
fi

if [ ! -f "$BIN" ]; then
  echo "Error: whisper-daemon binary not found. Run cargo build --release first." >&2
  exit 1
fi

if [ ! -d "$SOCKET_DIR" ]; then
  echo "Creating socket directory $SOCKET_DIR..."
  sudo mkdir -p "$SOCKET_DIR"
  sudo chown -R "$USER:$USER" "$SOCKET_DIR"
fi

export ORT_LIB_LOCATION="${ORT_LIB_LOCATION:-/opt/onnxruntime/lib}"
export LD_LIBRARY_PATH="$(dirname "$BIN"):${ORT_LIB_LOCATION}:${LD_LIBRARY_PATH:-}"

echo "Starting whisper-daemon..."
echo "Models dir: $MODELS_DIR"
echo "Socket: $SOCKET_DIR/daemon.sock"

exec "$BIN" --models-dir "$MODELS_DIR" --socket "$SOCKET_DIR/daemon.sock" --language "en" "$@"
