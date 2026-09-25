# Dependency Installation Guide

Complete step-by-step installation instructions for Ubuntu/Debian on Wayland (GNOME) with automated
clipboard paste, persistent overlay, and PipeWire integration.

---

## 1. Rust Toolchain

Install the standard Rust toolchain (edition 2021 compatible):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
```

---

## 2. System Packages (Wayland, Audio, Clipboard & Automation)

Install compiler tools, ALSA, Wayland protocols, `wmctrl`, `wl-clipboard`, and audio utilities:

```bash
sudo apt update && sudo apt install -y \
  build-essential \
  cmake \
  pkg-config \
  git \
  curl \
  wget \
  clang \
  libasound2-dev \
  alsa-utils \
  pipewire \
  wireplumber \
  libssl-dev \
  rust-src \
  libwayland-dev \
  wayland-protocols \
  libxkbcommon-dev \
  libgl1-mesa-dev \
  wmctrl \
  wl-clipboard \
  ydotool
```

### Package Roles:

* **`alsa-utils` (`arecord`):** Captures microphone audio directly at 16 kHz S16_LE mono with low
  latency.
* **`wireplumber` (`wpctl`):** Automatically mutes speakers when dictation begins and restores
  volume when recording ends, preventing speaker feedback.
* **`wmctrl`:** Applies `_NET_WM_STATE_ABOVE` to the Xwayland window frame to provide a persistent "
  Always on Top" GUI overlay in GNOME.
* **`wl-clipboard` (`wl-copy`):** Wayland native clipboard service.
* **`ydotool`:** Generates hardware-level input events (`Ctrl+V`) into the active target application
  when finalizing dictation.

---

## 3. Configuring `ydotool` Daemon

`ydotool` requires a background daemon (`ydotoold`) to communicate with the Linux uinput subsystem:

```bash
# Start ydotoold in the background (or configure as a systemd service)
sudo ydotoold --socket-path=/tmp/.ydotool_socket --socket-perm=0666 &

# Add to ~/.bashrc or session environment
echo 'export YDOTOOL_SOCKET="/tmp/.ydotool_socket"' >> ~/.bashrc
export YDOTOOL_SOCKET="/tmp/.ydotool_socket"
```

---

## 4. ONNX Runtime (Installation in `/opt`)

Whisper-NPU links dynamically against the official Microsoft ONNX Runtime Linux x64 libraries:

```bash
sudo mkdir -p /opt/onnxruntime
cd /tmp
wget https://github.com/microsoft/onnxruntime/releases/download/v1.19.2/onnxruntime-linux-x64-1.19.2.tgz
tar -xzvf onnxruntime-linux-x64-1.19.2.tgz
sudo cp -r onnxruntime-linux-x64-1.19.2/* /opt/onnxruntime/
rm -rf onnxruntime-linux-x64-1.19.2*

# Register with dynamic linker
echo "/opt/onnxruntime/lib" | sudo tee /etc/ld.so.conf.d/onnxruntime.conf
sudo ldconfig

# Add environment variable
if ! grep -q "ORT_LIB_LOCATION" ~/.bashrc; then
  echo 'export ORT_LIB_LOCATION=/opt/onnxruntime/lib' >> ~/.bashrc
fi
export ORT_LIB_LOCATION=/opt/onnxruntime/lib
```

---

## 5. IPC Directory Setup

The client and daemon communicate over `/run/whisper-npu/daemon.sock`:

```bash
sudo mkdir -p /run/whisper-npu
sudo chown -R $USER:$USER /run/whisper-npu
```

---

## 6. Downloading Quantized Whisper Models

Download the pre-quantized `distil-whisper-large-v3` ONNX model weights (`encoder`, `decoder`, and
`tokens`):

```bash
cd /home/mallumo/projects/ai-wd/whisper-npu
chmod +x scripts/download_model.sh
./scripts/download_model.sh
```

---

## 7. Passwordless Sudo for `llama.service` (Optional for "Fix with LLM")

The auxiliary "Fix" window can automatically stop `llama.service` when closed to release GPU/NPU
memory:

```bash
# Allow stopping and querying llama.service without password prompt:
echo "$USER ALL=(ALL) NOPASSWD: /bin/systemctl stop llama.service, /bin/systemctl start llama.service" | sudo tee /etc/sudoers.d/whisper-llama
sudo chmod 0440 /etc/sudoers.d/whisper-llama
```
