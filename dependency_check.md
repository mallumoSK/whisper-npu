# Dependency Verification Checklist

This document provides a comprehensive verification checklist and script for all required system
packages, audio subsystems, Wayland clipboard utilities, keyboard simulation tools, and ONNX Runtime
libraries used by **Whisper-NPU**.

---

## 1. All-in-One Environment Check Script

Run the following script in your terminal to verify system readiness:

```bash
#!/usr/bin/env bash
set -u

echo "============================================================"
echo "          Whisper-NPU System Dependency Check               "
echo "============================================================"

echo -e "\n--- 1. Rust Toolchain ---"
if command -v cargo >/dev/null 2>&1 && command -v rustc >/dev/null 2>&1; then
    echo "[OK] Rust: $(rustc --version) / Cargo: $(cargo --version)"
else
    echo "[FAIL] Rust/Cargo not found. Install via: curl https://sh.rustup.rs -sSf | sh"
fi

echo -e "\n--- 2. Build & Audio Dev Libraries ---"
pkg-config --exists alsa && echo "[OK] ALSA dev (libasound2-dev) installed" || echo "[FAIL] Missing libasound2-dev"
pkg-config --exists openssl && echo "[OK] OpenSSL dev (libssl-dev) installed" || echo "[FAIL] Missing libssl-dev"
command -v cmake >/dev/null 2>&1 && echo "[OK] CMake: $(cmake --version | head -n1)" || echo "[FAIL] Missing cmake"
command -v clang >/dev/null 2>&1 && echo "[OK] Clang: $(clang --version | head -n1)" || echo "[FAIL] Missing clang"

echo -e "\n--- 3. Audio Capture & Control (PipeWire / ALSA) ---"
command -v arecord >/dev/null 2>&1 && echo "[OK] arecord (alsa-utils) available" || echo "[FAIL] Missing arecord (alsa-utils)"
command -v wpctl >/dev/null 2>&1 && echo "[OK] wpctl (PipeWire speaker mute controller) available" || echo "[WARN] Missing wpctl (speaker muting during speech will be skipped)"

echo -e "\n--- 4. Wayland, Window Management & Automation ---"
pkg-config --exists wayland-client && echo "[OK] libwayland-dev installed" || echo "[FAIL] Missing libwayland-dev"
pkg-config --exists wayland-protocols && echo "[OK] wayland-protocols installed" || echo "[FAIL] Missing wayland-protocols"
pkg-config --exists xkbcommon && echo "[OK] libxkbcommon-dev installed" || echo "[FAIL] Missing libxkbcommon-dev"
pkg-config --exists glesv2 || pkg-config --exists gl && echo "[OK] OpenGL dev (libgl1-mesa-dev) installed" || echo "[FAIL] Missing OpenGL dev"
command -v wmctrl >/dev/null 2>&1 && echo "[OK] wmctrl (Always-on-Top manager) installed" || echo "[FAIL] Missing wmctrl (needed for GNOME always-on-top)"
command -v wl-copy >/dev/null 2>&1 && echo "[OK] wl-clipboard (wl-copy) installed" || echo "[FAIL] Missing wl-clipboard"

echo -e "\n--- 5. Virtual Keyboard Simulation (ydotool) ---"
if command -v ydotool >/dev/null 2>&1; then
    echo "[OK] ydotool binary installed"
    if [ -S "/tmp/.ydotool_socket" ] || pgrep -x ydotoold >/dev/null 2>&1; then
        echo "[OK] ydotoold service / socket active"
    else
        echo "[WARN] ydotoold daemon socket (/tmp/.ydotool_socket) not found. Run: sudo ydotoold --socket-path=/tmp/.ydotool_socket --socket-perm=0666 &"
    fi
else
    echo "[FAIL] Missing ydotool (required for automatic Ctrl+V paste)"
fi

echo -e "\n--- 6. ONNX Runtime & Models ---"
if ldconfig -p | grep libonnxruntime.so >/dev/null 2>&1 || [ -f "/opt/onnxruntime/lib/libonnxruntime.so" ]; then
    echo "[OK] ONNX Runtime library found"
else
    echo "[FAIL] libonnxruntime.so not found in system linker or /opt/onnxruntime/lib"
fi
[ -n "${ORT_LIB_LOCATION:-}" ] && echo "[OK] ORT_LIB_LOCATION: $ORT_LIB_LOCATION" || echo "[INFO] ORT_LIB_LOCATION default (/opt/onnxruntime/lib) will be used"

MODELS_DIR="$(dirname "$(realpath "$0")")/models"
if [ -f "$MODELS_DIR/distil-large-v3-encoder.int8.onnx" ] && \
   [ -f "$MODELS_DIR/distil-large-v3-decoder.int8.onnx" ] && \
   [ -f "$MODELS_DIR/distil-large-v3-tokens.txt" ]; then
    echo "[OK] Distil-Whisper ONNX models present in $MODELS_DIR"
else
    echo "[WARN] ONNX model files missing in $MODELS_DIR. Run: ./scripts/download_model.sh"
fi

echo -e "\n--- 7. IPC Socket Directory ---"
if [ -d "/run/whisper-npu" ]; then
    echo "[OK] Socket directory /run/whisper-npu exists ($(ls -ld /run/whisper-npu | awk '{print $1, $3, $4}'))"
else
    echo "[WARN] Socket directory /run/whisper-npu does not exist. Created automatically by scripts/run_daemon.sh or systemd."
fi

echo -e "\n--- 8. LLM Service (Optional - for 'Fix with LLM' feature) ---"
if systemctl is-active --quiet llama.service 2>/dev/null; then
    echo "[OK] llama.service is running"
elif curl -s http://127.0.0.1:8080/health >/dev/null 2>&1 || curl -s http://127.0.0.1:8080/v1/models >/dev/null 2>&1; then
    echo "[OK] Local LLM HTTP server responding on http://127.0.0.1:8080"
else
    echo "[INFO] Local llama.service is idle/stopped (will be activated automatically when clicking [Fix])"
fi

echo -e "\n============================================================"
```

---

## 2. Expected Results Summary

| Component             | Target Requirement                                      | Purpose in Whisper-NPU                                                   |
|:----------------------|:--------------------------------------------------------|:-------------------------------------------------------------------------|
| **Rust Toolchain**    | `cargo` & `rustc` >= 1.80                               | Compilation of workspace crates (`protocol`, `daemon`, `client`)         |
| **ALSA / alsa-utils** | `libasound2-dev`, `arecord`                             | Low-latency 16 kHz S16_LE audio capture                                  |
| **wpctl (PipeWire)**  | `wireplumber` / `pipewire`                              | Automatic speaker muting to eliminate audio feedback loops while talking |
| **wmctrl**            | `wmctrl` binary                                         | Setting `_NET_WM_STATE_ABOVE` for persistent GNOME Always-on-Top overlay |
| **wl-clipboard**      | `wl-copy`                                               | Wayland clipboard integration with stdin piping and timeout fallback     |
| **ydotool**           | `ydotool` & `ydotoold`                                  | Simulated `Ctrl+V` virtual keystroke directly into the target window     |
| **ONNX Runtime**      | `libonnxruntime.so` >= 1.19.0 in `/opt/onnxruntime/lib` | Fast INT8 quantized Whisper model inference                              |
| **IPC Socket**        | `/run/whisper-npu/daemon.sock`                          | Unix domain streaming socket between daemon and GUI overlay              |
| **llama.service**     | `http://127.0.0.1:8080` (OpenAI-compatible API)         | AI-powered grammar and text rephrasing inside auxiliary Fix window       |
