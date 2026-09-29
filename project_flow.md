# Project Flow & Architecture Specification

Architecture documentation, Material Design 3 UI layout, auxiliary multi-window pipeline, keyboard
shortcuts, and IPC protocols for **Whisper-NPU**.

---

## 1. UI Layout & Dual-Window Hierarchy

Whisper-NPU features a persistent desktop overlay (minimum width **800px**) with an expandable
Executive Summary panel and an optional auxiliary sidecar window for local LLM-assisted editing.

```
┌──────────────────────────────────────────────────────────────────────────────────────────┐  ┌───────────────────────────────┐
│ [● REC 16kHz] [↵ Enter Stop & Paste] [␣ Space Pause] [⇧ Shift Clear] [⎋ Esc] [▤ Ctrl]    │  │ Fix with LLM            ─ □ ✕ │
├──────────────────────────────────────────────────────────────────────────────────────────┤  ├───────────────────────────────┤
│                                                                                          │  │ [Live STT Prompt Box]         │
│  Transcribed speech appears here live in real-time...                                   │  │ "Fix punctuation and make..." │
│  (Automatically filters trailing silence artifacts like "you" / " You.")                 │  ├───────────────────────────────┤
│                                                                                          │  │ [Status / Progress Area]     │
│  ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~ Soundwave ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~ │  │   "Polishing text with LLM"    │
├──────────────────────────────────────────────────────────────────────────────────────────┤  ├───────────────────────────────┤
│ [Executive Summary - Expandable Markdown Panel (Ctrl)]                                   │  │ [Result Preview Box]          │
│  ### Structured Notes                                                                    │  │ (Shows LLM output before      │
│  * Formatted points, cleaned-up speech, and markdown highlights                          │  │  applying to main window)     │
│  [Copy Markdown]  [Collapse]                                                             │  ├───────────────────────────────┤
├──────────────────────────────────────────────────────────────────────────────────────────┤  │ [Cancel]  [Reprompt]  [Apply] │
│ [Shift Clear] [Edit] [Summary] [Fix]                           [Pause] [Stop & Paste]   │  └───────────────────────────────┘
└──────────────────────────────────────────────────────────────────────────────────────────┘      Auxiliary Sidecar (320x520)
       Primary Overlay Window (800x520 Collapsed | 800x860 Summary Expanded)
```

### Main Window Controls:

* **Wave Visualizer:** Animated reactive soundwave reflecting listening state.
* **`[Shift Clear]` / Key `Shift`:** Clears current buffer, resets daemon transcription history, and
  automatically resumes speech recognition if paused.
* **`[Edit]`:** Toggles manual editing mode.
    * Entering Edit mode automatically **pauses** recording so typing is uninterrupted.
    * In editing mode, `Shift` typing behaves normally (capital letters, punctuation) without
      clearing text.
    * Pressing `Esc` or clicking **`[Done]`** exits edit mode and resumes recording.
* **`[Summary]` / Key `Ctrl`:** Interrupts recognition, vertically expands window to 860px, and
  invokes
  LLaMA to summarize dictated thoughts into structured Markdown.
* **`[Fix]`:** Opens the auxiliary "Fix with LLM" sidecar on the right, sets STT target to the fix
  prompt box, and captures the fix command via voice.
* **Shortcut Pills:** Interactive visual pills for fast hotkey access (`Enter`, `Space`, `Shift`,
  `Esc`, `Ctrl`).
* **Transcription Canvas:** Dark card (`#1C1B1F`), scrollable, live streaming speech-to-text.
* **Bottom Bar:** Quick action buttons (`[Clear]`, `[Edit]`, `[Summary]`, `[Fix]`, `[Pause/Resume]`,
  `[Stop & Paste]`).

---

## 2. Executive Summary Lifecycle & Iterative Refinement Loop

The Executive Summary is integrated directly into the primary window and can be expanded or
collapsed dynamically:

```
[User presses Ctrl or clicks Summary]
                 │
                 ▼
1. Master Recognition Stop:
   Client sends `ClientCommand::PauseListening` to halt audio recording.
                 │
                 ▼
2. Window Expansion:
   Primary window expands from 520px to 860px height (800px min width).
                 │
                 ▼
3. Local LLaMA Invocation:
   Streams request to `http://127.0.0.1:8080/v1/chat/completions`.
   - First call: Summarizes transcribed text into clean, structured Markdown.
   - Iterative update calls: Passes both existing Markdown summary and newly dictated speech.
                 │
                 ▼
4. Output Display:
   Renders formatted Markdown in a scrollable, read-only viewer.
                 │
                 ▼
5. Iterative Update Loop:
   User presses `Shift` to clear buffer, speaks additional thoughts, and hits `Ctrl` again.
   LLaMA continuously refines and adds to the Markdown definition.
                 │
                 ▼
6. Contextual EnterAction:
   Pressing `Enter` while Summary is expanded pastes the formatted Markdown text directly
   into the active application instead of the raw speech transcript!
```

---

## 3. "Fix with LLM" Auxiliary Viewport Lifecycle

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

## 4. Keyboard Shortcut State Machine

```
                              ┌───────────────────────────────────┐
                              │        Navigational Mode          │
                              └─────────────────┬─────────────────┘
                                                │
       ┌────────────────────┬───────────────────┼───────────────────┬────────────────────┐
       │ Press [Shift]      │ Press [Ctrl]      │ Press [Space]     │ Press [Enter]      │ Press [Esc]
       ▼                    ▼                   ▼                   ▼                    ▼
┌──────────────┐    ┌───────────────┐   ┌─────────────────┐ ┌─────────────────┐  ┌─────────────────┐
│ Clear buffer │    │ Master Stop & │   │ Toggle Pause /  │ │ Stop Recording, │  │ Cancel session, │
│ & Auto-Resume│    │ Expand Summary│   │ Resume Audio    │ │ Copy & Paste    │  │ Exit client     │
└──────────────┘    └───────┬───────┘   └─────────────────┘ └─────────────────┘  └─────────────────┘
                            │
                            ▼
              ┌───────────────────────────┐
              │   Summary Expanded Mode   │
              ├───────────────────────────┤
              │ [Ctrl]  -> Update Summary │
              │ [Enter] -> Paste Markdown │
              │ [Shift] -> Clear voice STT│
              │ [Space] -> Pause / Resume │
              │ [Esc]   -> Collapse panel │
              └───────────────────────────┘
                            │
                    Press [Edit] Button
                            ▼
              ┌───────────────────────────┐
              │       Editing Mode        │
              ├───────────────────────────┤
              │ [Shift] -> Normal typing  │
              │ [Space] -> Inserts space  │
              │ [Enter] -> Newline        │
              │ [Esc]   -> Exit edit mode │
              └───────────────────────────┘
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

* **`crates/protocol`**: Shared serialization types (`ClientCommand`, `DaemonEvent`), IPC socket
  defaults,
  and speech post-processing sanitizers (e.g. `strip_trailing_you` to eliminate tail silence
  artifacts).
* **`crates/daemon`**: Background system service. Manages audio capture (`arecord`), speaker
  muting (`wpctl`), model inference (`sherpa-rs` ONNX INT8), IPC server, and automated pasting (
  `wl-copy` + `ydotool`).
* **`crates/client`**: High-performance UI overlay in `egui`/`eframe` (`glow` OpenGL backend).
  Handles Xwayland
  always-on-top, soundwave visualization, live editing, Executive Summary markdown drawer, and
  auxiliary "Fix with LLM" sidecar.

