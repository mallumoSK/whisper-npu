# Whisper-NPU Development & Operational Workflow

This document records the end-to-end workflow, architectural decisions, and operational lifecycle
for **Whisper-NPU** on Linux / GNOME Wayland.

---

## 1. Project Overview & Architecture

Whisper-NPU is a lightweight, low-latency offline voice dictation overlay for Linux built in Rust.
It utilizes local neural speech-to-text models running via ONNX Runtime (with INT8 quantization), an
always-on-top desktop overlay, and automated clipboard injection into active applications.

```
                      ┌───────────────────────────────────────────────┐
                      │              Linux Audio Input                │
                      │   arecord (16 kHz S16_LE mono PCM stream)     │
                      └──────────────────────┬────────────────────────┘
                                             │
                                             ▼
                      ┌───────────────────────────────────────────────┐
                      │        whisper-daemon (Background Svc)        │
                      │  • Speaker auto-mute via wpctl (PipeWire)     │
                      │  • sherpa-rs + distil-whisper-large-v3 INT8   │
                      │  • Unix domain socket (/run/whisper-npu/...)  │
                      │  • Automated wl-copy & ydotool Ctrl+V paste   │
                      └──────────────┬─────────────────▲──────────────┘
                        Events (JSON)│                 │Commands (JSON)
                                     ▼                 │
                      ┌────────────────────────────────┴──────────────┐
                      │          whisper-client (GUI Overlay)         │
                      │  • eframe / egui dark Material Design 3 UI    │
                      │  • Soundwave reactive animation               │
                      │  • Persistent always-on-top via wmctrl        │
                      │  • Multi-viewport: Fix with LLM sidecar       │
                      │  • Space/Enter/Esc hotkey navigation          │
                      └───────────────────────────────────────────────┘
```

---

## 2. Key Engineering Milestones & Iterations

### A. Dedicated Client-Daemon Split

* Separated resource-heavy model inference and audio capture into a persistent systemd service (
  `whisper-daemon`).
* The UI overlay (`whisper-client`) starts instantly, connects to the local Unix domain socket, and
  displays live partial speech recognition without re-initializing ONNX weights.

### B. Speaker Feedback Loop Prevention

* When dictating near laptop speakers, playing audio or feedback loops would re-trigger
  transcription.
* Integrated PipeWire speaker muting: `wpctl set-mute @DEFAULT_AUDIO_SINK@ 1` upon speech capture,
  unmuting automatically when recording pauses or ends.

### C. GNOME "Always On Top" Support

* Wayland native protocols (`xdg_toplevel`) intentionally restrict client-initiated "always-on-top".
* Configured the client to run under Xwayland (`WINIT_UNIX_BACKEND=x11` with
  `REAL_WAYLAND_DISPLAY="wayland-0"` preserved).
* Implemented background enforcement of `_NET_WM_STATE_ABOVE` using `wmctrl`, guaranteeing the
  overlay remains visible over full-screen editors and browsers.

### D. Multi-Viewport "Fix with LLM" Sidecar

* Developed an auxiliary right-side sidecar window (`egui::ViewportId`).
* Allows the user to dictate a live instruction prompt (e.g. *"Fix grammar and rephrase"*), queries
  a local `llama.service` OpenAI-compatible endpoint at `http://127.0.0.1:8080/v1/chat/completions`,
  previews the diff, and applies it to the main buffer upon pressing `Enter`.
* Automatically stops `llama.service` via `sudo systemctl stop llama.service` upon closing to
  conserve system resources.

### E. Elimination of Enter Freeze on Long Dictation Sessions

* **Deadlock Root Cause:** In GNOME Wayland, `wl-copy` required window focus to claim clipboard
  ownership via its transparent popup surface. When the client was always-on-top and focused,
  `wl-copy` hung waiting for focus while the GUI thread was blocked in `status()`.
* **Resolution:**
    1. Delegated the final clipboard copy to the daemon, which terminates the client window first (
       `kill -9`), allowing GNOME Mutter to immediately refocus the target application.
    2. Piped text through `wl-copy` standard input with a `150ms` timeout fallback.
    3. Replaced IPC polling with a reactive `tokio::sync::mpsc::unbounded_channel` to eliminate
       transmission latency.

---

## 3. Directory Layout

```
whisper-npu/
├── Cargo.toml                  # Workspace configuration (protocol, daemon, client)
├── README.md                   # Repository overview and quickstart guide
├── workflow.md                 # Development history and engineering workflow
├── dependency_check.md         # Environment verification script
├── dependency_install.md       # Step-by-step dependency installation
├── project_flow.md             # UI layout and IPC protocol specification
├── whisper-daemon.service      # Systemd service unit definition
├── run.sh                      # Unified launch script
├── scripts/
│   ├── download_model.sh       # Downloads distil-whisper-large-v3 ONNX weights
│   ├── run_daemon.sh           # Daemon startup wrapper
│   └── run_client.sh           # Client startup wrapper
├── models/                     # Model directory (quantized ONNX + tokens)
└── crates/
    ├── protocol/               # Shared IPC message types
    ├── daemon/                 # Audio capture, whisper inference, paste automation
    └── client/                 # egui GUI overlay, soundwave, auxiliary fix window
```

---

## 4. Verification and Testing

1. **Protocol Unit Tests:**
   ```bash
   cargo test -p protocol
   ```
2. **Daemon Model Inference Test:**
   ```bash
   cargo test -p daemon
   ```
3. **Client & LLM Integration Tests:**
   ```bash
   cargo test -p client
   ```
4. **End-to-End Build:**
   ```bash
   cargo build --release
   ```

