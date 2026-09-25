use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::Mutex;
use tracing::info;

pub const RECORDING_WAV_PATH: &str = "/tmp/whisper_session.wav";
pub const RECORDING_WAV_COPY: &str = "/tmp/whisper_session_copy.wav";

pub struct AudioRecorder {
    active_child: Mutex<Option<Child>>,
    muted_by_us: Mutex<bool>,
    wav_path: PathBuf,
}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            active_child: Mutex::new(None),
            muted_by_us: Mutex::new(false),
            wav_path: PathBuf::from(RECORDING_WAV_PATH),
        }
    }

    #[allow(dead_code)]
    pub fn wav_path(&self) -> &Path {
        &self.wav_path
    }

    /// Starts arecord subprocess capturing pristine 16kHz mono audio directly to WAV,
    /// and automatically mutes system speakers.
    pub fn start(&self) -> Result<()> {
        let mut guard = self.active_child.lock().unwrap();
        if guard.is_some() {
            return Ok(());
        }

        // Mute speakers so playback is not captured by mic
        if !is_speakers_muted() {
            set_speakers_muted(true);
            let mut muted_guard = self.muted_by_us.lock().unwrap();
            *muted_guard = true;
        }

        // Clean up old session WAV file if present
        if self.wav_path.exists() {
            let _ = std::fs::remove_file(&self.wav_path);
        }

        info!("Starting arecord process on default audio device (16kHz S16_LE mono)");
        let child = Command::new("arecord")
            .args([
                "-D", "default",
                "-f", "S16_LE",
                "-c", "1",
                "-r", "16000",
                RECORDING_WAV_PATH,
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .context("Failed to spawn arecord subprocess")?;

        *guard = Some(child);
        info!("arecord running and capturing to {}", RECORDING_WAV_PATH);

        Ok(())
    }

    /// Stops arecord, completely freeing microphone hardware and restoring speaker volume.
    pub fn stop(&self) {
        let mut guard = self.active_child.lock().unwrap();
        if let Some(mut child) = guard.take() {
            info!("Stopping arecord process...");
            let _ = child.kill();
            let _ = child.wait();
            info!("arecord stopped: totally disconnected from microphone");
        }

        // Unmute speakers if we muted them
        let mut muted_guard = self.muted_by_us.lock().unwrap();
        if *muted_guard {
            set_speakers_muted(false);
            *muted_guard = false;
        }
    }

    #[allow(dead_code)]
    pub fn is_recording(&self) -> bool {
        let guard = self.active_child.lock().unwrap();
        guard.is_some()
    }
}

impl Drop for AudioRecorder {
    fn drop(&mut self) {
        self.stop();
    }
}

fn is_speakers_muted() -> bool {
    let mut cmd = Command::new("wpctl");
    cmd.args(["get-volume", "@DEFAULT_AUDIO_SINK@"]);
    if std::env::var("XDG_RUNTIME_DIR").is_err() {
        cmd.env("XDG_RUNTIME_DIR", "/run/user/1000");
    }

    if let Ok(output) = cmd.output() {
        let text = String::from_utf8_lossy(&output.stdout);
        return text.contains("[MUTED]");
    }
    false
}

fn set_speakers_muted(muted: bool) {
    let arg = if muted { "1" } else { "0" };
    info!("Setting speakers mute state: {} (PipeWire / wpctl)", muted);

    let mut cmd = Command::new("wpctl");
    cmd.args(["set-mute", "@DEFAULT_AUDIO_SINK@", arg]);
    if std::env::var("XDG_RUNTIME_DIR").is_err() {
        cmd.env("XDG_RUNTIME_DIR", "/run/user/1000");
    }

    if let Ok(status) = cmd.status() {
        if status.success() {
            info!(
                "Speakers successfully {} via wpctl",
                if muted { "muted" } else { "unmuted" }
            );
            return;
        }
    }

    // Fallback to amixer
    let amixer_arg = if muted { "mute" } else { "unmute" };
    let _ = Command::new("amixer")
        .args(["set", "Master", amixer_arg])
        .status();
}
