#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
MODELS_DIR="${PROJECT_ROOT}/models"

mkdir -p "$MODELS_DIR"

BASE_URL="https://huggingface.co/csukuangfj/sherpa-onnx-whisper-distil-large-v3/resolve/main"

FILES=(
  "distil-large-v3-tokens.txt"
  "distil-large-v3-encoder.int8.onnx"
  "distil-large-v3-decoder.int8.onnx"
)

echo "=== Downloading distil-whisper-large-v3 ONNX model ==="
echo "Target directory: $MODELS_DIR"

for file in "${FILES[@]}"; do
  target="${MODELS_DIR}/${file}"
  if [ -f "$target" ] && [ -s "$target" ]; then
    echo "[OK] Already exists: $file"
  else
    echo "[DOWNLOADING] $file ..."
    tmp="${target}.tmp.$$"
    if command -v curl >/dev/null 2>&1; then
      curl -L --fail --show-error --progress-bar -o "$tmp" "${BASE_URL}/${file}"
    elif command -v wget >/dev/null 2>&1; then
      wget --show-progress -O "$tmp" "${BASE_URL}/${file}"
    else
      echo "Error: curl or wget is required." >&2
      exit 1
    fi
    mv "$tmp" "$target"
    echo "[OK] Downloaded: $file"
  fi
done

echo "=== Model files verified in $MODELS_DIR ==="
ls -lh "$MODELS_DIR"
