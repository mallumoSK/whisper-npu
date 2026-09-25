use anyhow::Result;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::info;

pub struct WhisperEngine {
    cli_path: PathBuf,
    model_path: PathBuf,
    language: String,
    gpu_device: u32,
}

impl WhisperEngine {
    pub fn new(_model_dir: &Path, language: &str) -> Result<Self> {
        let cli_path = PathBuf::from("/opt/whisper-npu/bin/whisper-cli-npu");
        if !cli_path.exists() {
            anyhow::bail!("whisper-cli-npu not found at {}", cli_path.display());
        }

        // Use whisper-small NPU model (with matching .rai cache in same folder)
        let model_path = PathBuf::from("/home/mallumo/models/stt/npu/ggml-small.bin");
        if !model_path.exists() {
            anyhow::bail!("Whisper NPU model not found at {}", model_path.display());
        }

        let gpu_device = detect_amd_gpu_device();
        info!(
            "WhisperEngine initialized on AMD NPU: cli={}, model={}, lang={}, gpu_dev={}",
            cli_path.display(),
            model_path.display(),
            language,
            gpu_device
        );

        Ok(Self {
            cli_path,
            model_path,
            language: language.to_string(),
            gpu_device,
        })
    }

    /// Transcribes a WAV audio file directly using AMD Ryzen AI NPU via Vitis AI encoder offload.
    pub fn transcribe_file(&self, wav_path: &Path) -> String {
        if !wav_path.exists() {
            return String::new();
        }

        let mut cmd = Command::new(&self.cli_path);
        cmd.arg("-m")
            .arg(&self.model_path)
            .arg("-f")
            .arg(wav_path)
            .arg("-nt") // No timestamps
            .arg("-sns") // Suppress non-speech tokens
            .arg("-ng"); // Pure NPU encoder + CPU decode (0 GPU power)

        if !self.language.is_empty() {
            cmd.arg("-l").arg(&self.language);
        }

        match cmd.output() {
            Ok(output) if output.status.success() => {
                let raw_stdout = String::from_utf8_lossy(&output.stdout);
                clean_whisper_text(&raw_stdout)
            }
            Ok(output) => {
                let err = String::from_utf8_lossy(&output.stderr);
                tracing::warn!("whisper-cli-npu exited with error: {}", err);
                String::new()
            }
            Err(e) => {
                tracing::error!("Failed to execute whisper-cli-npu: {:?}", e);
                String::new()
            }
        }
    }
}

/// Detects AMD Radeon GPU device index for Vulkan (default: 1)
fn detect_amd_gpu_device() -> u32 {
    if let Ok(output) = Command::new("vulkaninfo").arg("--summary").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut idx = 0;
        for line in stdout.lines() {
            if line.contains("deviceName") {
                if line.contains("AMD") || line.contains("Radeon") || line.to_lowercase().contains("radv") {
                    return idx;
                }
                idx += 1;
            }
        }
    }
    1 // Default fallback for TUF Gaming A14 (Radeon 880M / 890M STRIX1)
}

/// Cleans Whisper transcription output, removing brackets and normalizing whitespace.
pub fn clean_whisper_text(text: &str) -> String {
    let mut cleaned = String::with_capacity(text.len());
    let mut bracket_depth = 0;
    let mut paren_depth = 0;

    for ch in text.chars() {
        match ch {
            '[' => bracket_depth += 1,
            ']' => {
                if bracket_depth > 0 {
                    bracket_depth -= 1;
                }
            }
            '(' => paren_depth += 1,
            ')' => {
                if paren_depth > 0 {
                    paren_depth -= 1;
                }
            }
            _ => {
                if bracket_depth == 0 && paren_depth == 0 {
                    cleaned.push(ch);
                }
            }
        }
    }

    cleaned.split_whitespace().collect::<Vec<_>>().join(" ").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transcribe_file_jfk() {
        let engine = WhisperEngine::new(Path::new("models"), "en").unwrap();
        let jfk_path = Path::new("/opt/whisper-npu/samples/jfk.wav");
        if jfk_path.exists() {
            let start = std::time::Instant::now();
            let text = engine.transcribe_file(jfk_path);
            println!("Transcribed in {:?}: '{}'", start.elapsed(), text);
            assert!(text.contains("fellow Americans"));
        }
    }

    #[test]
    fn test_clean_text() {
        assert_eq!(
            clean_whisper_text("[music] And so (applause) my fellow Americans"),
            "And so my fellow Americans"
        );
    }
}
