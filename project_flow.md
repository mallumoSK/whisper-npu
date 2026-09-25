# Project Flow & Architecture Specification

Architecture documentation, Material Design 3 UI layout, auxiliary multi-window pipeline, keyboard
shortcuts, and IPC protocols for **Whisper-NPU**.

---

## 1. UI Layout & Dual-Window Hierarchy

Whisper-NPU features a persistent desktop overlay with an optional auxiliary sidecar window for
local LLM-assisted editing.

```
┌──────────────────────────────────────────────────┐  ┌───────────────────────────────┐
│ [~~~] Whisper Dictation                  [EN]    │  │ Fix with LLM            ─ □ ✕ │
├──────────────────────────────────────────────────┤  ├───────────────────────────────┤
│ [Clear] [Edit] [Fix]                             │  │ [Live STT Prompt Box]         │
│ [Enter Stop & Paste] [␣ Space Pause] [Esc Cancel]│  │ "Fix punctuation and make..." │
├──────────────────────────────────────────────────┤  ├───────────────────────────────┤
│                                                  │  │ [Status / Progress Area]     │
│  Transcribed text appears here live...           │  │   "Polishing text with LLM"    │
│  Continuous speech recognition stream.           │  ├───────────────────────────────┤
│                                                  │  │ [Result Preview Box]          │
│                                                  │  │ (Shows LLM output before      │
│                                                  │  │  applying to main window)     │
├──────────────────────────────────────────────────┤  ├───────────────────────────────┤
│ ● Recording...       [Clear] [Paste] [Pause]     │  │ [Cancel]  [Reprompt]  [Apply] │
└──────────────────────────────────────────────────┘  └───────────────────────────────┘
          Primary Overlay Window (630x520)                Auxiliary Sidecar (320x520)
```

### Main Window Controls:

* **Wave Visualizer:** Animated reactive soundwave reflecting listening state.
* **`[Clear]`:** Clears current buffer and resets daemon transcription history.
* **`[Edit]`:** Toggles manual editing mode.
    * If recording was active, entering Edit mode automatically **pauses** recording so typing is
      uninterrupted.
    * Clicking **`[Done]`** exits edit mode and automatically resumes recording if it was previously
      active.
* **`[Fix]`:** Opens the auxiliary "Fix with LLM" sidecar on the right, sets STT target to the fix
  prompt box, and starts local speech capture for the fix command.
* **Shortcut Pills:** Direct visual cues for global hotkeys (`Enter`, `Space`, `Esc`).
* **Transcription Canvas:** Dark card (`#1C1B1F`), scrollable, live streaming speech-to-text.
* **Bottom Bar:** Quick action buttons (`[Clear]`, `[Done / Paste]`, `[Pause / Resume]`).

---

## 2. "Fix with LLM" Auxiliary Viewport Lifecycle

The auxiliary window runs as an `egui::ViewportId` alongside the primary window:

```
[Main Window: User clicks Fix]
               │
               ▼
   [Aux Window Opens (Prompting)] ──► Live voice dictates the instruction (e.g. "make it formal")
               │
   User presses [Enter]
               │
               ▼
   [Aux Window: Processing Phase] ──► Checks LLM health (http://127.0.0.1:8080/health)
                                      Streams HTTP POST /v1/chat/completions to llama.service
               │
   LLM Response Received
               │
               ▼
   [Aux Window: Review Phase]     ──► Shows generated text preview
                                      [Enter] -> Confirms & replaces text in main window
                                      [Space] -> Reset & reprompt
                                      [Esc]   -> Discard changes
               │
   On Close / Confirm / Cancel
               │
               ▼
   Automatic cleanup: executes `sudo systemctl stop llama.service` to free NPU/GPU resources.
```

---

## 3. Keyboard Shortcut State Machine

```
                   ┌───────────────────────────────────┐
                   │        Navigational Mode          │
                   └─────────────────┬─────────────────┘
                                     │
         ┌───────────────────────────┼───────────────────────────┐
         │ Press [Space]             │ Press [Enter]             │ Press [Esc]
         ▼                           ▼                           ▼
┌─────────────────┐         ┌─────────────────┐         ┌─────────────────┐
│ Toggle Pause /  │         │ Stop Recording, │         │ Cancel session, │
│ Resume Audio    │         │ Copy & Paste    │         │ Exit client     │
└─────────────────┘         └─────────────────┘         └─────────────────┘
                                     │
                             Press [Edit] Button
                                     ▼
                   ┌───────────────────────────────────┐
                   │           Editing Mode            │
                   ├───────────────────────────────────┤
                   │ [Space] -> Types space character  │
                   │ [Enter] -> Inserts newline        │
                   │ [Esc]   -> Exits editing mode     │
                   └───────────────────────────────────┘
```

---

## 4. "Stop & Paste" Automated Wayland Pipeline

Wayland and GNOME enforce strict focus boundaries that prevent background or inactive windows from
injecting keystrokes directly into other applications. Whisper-NPU solves this reliably:

```
[User presses Enter in Whisper Overlay]
                   │
                   ▼
1. Client sends `ClientCommand::StopAndPaste { pid, delay_ms: 180, text, wayland_display }`
   via non-blocking unbounded IPC channel.
                   │
                   ▼
2. Daemon receives command and immediately terminates client PID (`kill -9 <pid>`).
   -> Client overlay vanishes from screen INSTANTLY.
   -> GNOME Mutter automatically refocuses the underlying target application.
                   │
                   ▼
3. Daemon pipes text to `wl-copy` standard input with `WAYLAND_DISPLAY="wayland-0"`.
   -> Since client window is closed, wl-copy acquires Wayland clipboard with 0 focus conflict.
                   │
                   ▼
4. Daemon waits 180ms (`delay_ms`) for compositor focus settling.
                   │
                   ▼
5. Daemon invokes `ydotool key 29:1 47:1 47:0 29:0` (Ctrl+V) via `/tmp/.ydotool_socket`.
   -> Dictated text is seamlessly pasted into the active cursor position!
```

---

## 5. Speaker Muting (PipeWire / ALSA)

To eliminate microphone feedback loops (where audio played from laptop speakers is re-transcribed by
Whisper), the daemon integrates directly with PipeWire:

* When **Listening / Recording starts**:
  ```bash
  wpctl set-mute @DEFAULT_AUDIO_SINK@ 1
  ```
* When **Paused / Stopped / Idle**:
  ```bash
  wpctl set-mute @DEFAULT_AUDIO_SINK@ 0
  ```

---

## 6. IPC Protocol Specification (`crates/protocol`)

Communication occurs over a Unix domain socket at `/run/whisper-npu/daemon.sock` using JSON Lines:

### Client Commands (`ClientCommand`):

```json
{ "type": "StartListening" }
{ "type": "PauseListening" }
{ "type": "ResumeListening" }
{ "type": "ClearBuffer" }
{ "type": "Stop" }
{ "type": "StopAndPaste", "pid": 12345, "delay_ms": 180, "text": "transcribed text", "wayland_display": "wayland-0" }
{ "type": "CancelAndExit", "pid": 12345 }
```

### Daemon Events (`DaemonEvent`):

```json
{ "type": "StateChanged", "data": "Listening" }
{ "type": "PartialTranscript", "data": "Live streaming words..." }
{ "type": "FinalTranscript", "data": "Full finalized sentence." }
{ "type": "Error", "data": "Detailed error description" }
```

---

## 7. Workspace Crates

* **`crates/protocol`**: Shared serialization types and IPC socket defaults.
* **`crates/daemon`**: Background system service. Manages audio capture (`arecord`), speaker
  muting (`wpctl`), model inference (`sherpa-rs` ONNX INT8), IPC server, and automated pasting (
  `wl-copy` + `ydotool`).
* **`crates/client`**: High-performance UI overlay in `egui`/`eframe`. Handles Xwayland
  always-on-top, soundwave visualization, live editing, and auxiliary LLM window.

