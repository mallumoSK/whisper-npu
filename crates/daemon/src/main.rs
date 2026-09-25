mod audio;
mod model;
mod server;

use anyhow::{Context, Result};
use clap::Parser;
use protocol::{ClientCommand, DaemonEvent, DaemonState, DEFAULT_SOCKET_PATH};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, Mutex};
use tracing::{error, info, warn};

#[derive(Parser, Debug)]
#[command(author, version, about = "Whisper NPU Dictation Daemon")]
struct Args {
    #[arg(short, long, default_value = "models")]
    models_dir: PathBuf,

    #[arg(short, long, default_value = DEFAULT_SOCKET_PATH)]
    socket: String,

    #[arg(short, long, default_value = "en")]
    language: String,
}

struct LiveResult {
    text: String,
    generation: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,daemon=debug".into()),
        )
        .init();

    let args = Args::parse();
    info!("Starting whisper-daemon with config: {:?}", args);

    let models_dir = if args.models_dir.is_relative() {
        let current_dir = std::env::current_dir()?;
        if current_dir.join(&args.models_dir).exists() {
            current_dir.join(&args.models_dir)
        } else {
            PathBuf::from("/home/mallumo/projects/ai-wd/whisper-npu/models")
        }
    } else {
        args.models_dir
    };

    let recorder = Arc::new(audio::AudioRecorder::new());
    let engine = Arc::new(
        model::WhisperEngine::new(&models_dir, &args.language)
            .context("Failed to initialize whisper engine")?,
    );

    let (command_tx, mut command_rx) = mpsc::channel::<ClientCommand>(32);
    let (event_tx, _) = broadcast::channel::<DaemonEvent>(64);
    let current_state = Arc::new(Mutex::new(DaemonState::Idle));

    // Spawn IPC server
    let server = server::IpcServer::new(
        Some(args.socket),
        event_tx.clone(),
        command_tx,
        current_state.clone(),
    );

    tokio::spawn(async move {
        if let Err(e) = server.run().await {
            error!("IPC server encountered error: {:?}", e);
        }
    });

    info!("Daemon worker ready and listening");

    let mut accumulated_text = String::new();
    let mut last_live_text = String::new();
    let mut current_generation: u64 = 0;
    let live_in_flight = Arc::new(AtomicBool::new(false));
    let (live_tx, mut live_rx) = mpsc::channel::<LiveResult>(4);

    let mut ticker = tokio::time::interval(Duration::from_millis(1100));

    loop {
        tokio::select! {
            // Periodic live transcription preview
            _ = ticker.tick() => {
                let state = *current_state.lock().await;
                if state == DaemonState::Listening && !live_in_flight.load(Ordering::SeqCst) {
                    let session_wav = Path::new(audio::RECORDING_WAV_PATH);
                    if session_wav.exists() {
                        if let Ok(meta) = session_wav.metadata() {
                            if meta.len() > 32000 { // at least 1s of audio recorded
                                live_in_flight.store(true, Ordering::SeqCst);
                                let live_copy = PathBuf::from(audio::RECORDING_WAV_COPY);
                                let eng = engine.clone();
                                let in_flight_flag = live_in_flight.clone();
                                let tx = live_tx.clone();
                                let gen = current_generation;

                                tokio::task::spawn_blocking(move || {
                                    if std::fs::copy(audio::RECORDING_WAV_PATH, &live_copy).is_ok() {
                                        let text = eng.transcribe_file(&live_copy);
                                        let _ = std::fs::remove_file(&live_copy);
                                        let _ = tx.blocking_send(LiveResult { text, generation: gen });
                                    }
                                    in_flight_flag.store(false, Ordering::SeqCst);
                                });
                            }
                        }
                    }
                }
            }

            // Live transcription result from background task
            Some(res) = live_rx.recv() => {
                let state = *current_state.lock().await;
                if state == DaemonState::Listening && res.generation == current_generation {
                    if res.text != last_live_text {
                        last_live_text = res.text.clone();
                        let full_display = if accumulated_text.is_empty() {
                            res.text
                        } else if res.text.is_empty() {
                            accumulated_text.clone()
                        } else {
                            format!("{}\n{}", accumulated_text, res.text)
                        };
                        let _ = event_tx.send(DaemonEvent::PartialTranscript {
                            data: full_display,
                        });
                    }
                }
            }

            // Commands from clients
            Some(cmd) = command_rx.recv() => {
                match cmd {
                    ClientCommand::StartListening => {
                        let mut state = current_state.lock().await;
                        *state = DaemonState::Listening;
                        current_generation += 1;
                        accumulated_text.clear();
                        last_live_text.clear();

                        if let Err(e) = recorder.start() {
                            error!("Failed to connect to microphone: {:?}", e);
                        }
                        let _ = event_tx.send(DaemonEvent::StateChanged {
                            data: DaemonState::Listening,
                        });
                    }
                    ClientCommand::ResumeListening => {
                        let mut state = current_state.lock().await;
                        *state = DaemonState::Listening;
                        last_live_text.clear();

                        if let Err(e) = recorder.start() {
                            error!("Failed to connect to microphone: {:?}", e);
                        }
                        let _ = event_tx.send(DaemonEvent::StateChanged {
                            data: DaemonState::Listening,
                        });
                    }
                    ClientCommand::PauseListening => {
                        let mut state = current_state.lock().await;
                        *state = DaemonState::Paused;
                        recorder.stop();

                        // Transcribe the final session WAV file
                        let session_wav = PathBuf::from(audio::RECORDING_WAV_PATH);
                        if session_wav.exists() {
                            let eng = engine.clone();
                            let final_text = tokio::task::spawn_blocking(move || {
                                let t = eng.transcribe_file(&session_wav);
                                let _ = std::fs::remove_file(&session_wav);
                                t
                            }).await.unwrap_or_default();

                            if !final_text.is_empty() {
                                if !accumulated_text.is_empty() {
                                    accumulated_text.push('\n');
                                }
                                accumulated_text.push_str(&final_text);
                            }
                            last_live_text.clear();

                            let _ = event_tx.send(DaemonEvent::FinalTranscript {
                                data: accumulated_text.clone(),
                            });
                        }

                        let _ = event_tx.send(DaemonEvent::StateChanged {
                            data: DaemonState::Paused,
                        });
                    }
                    ClientCommand::ClearBuffer => {
                        accumulated_text.clear();
                        last_live_text.clear();
                        current_generation += 1;
                        let _ = std::fs::remove_file(audio::RECORDING_WAV_PATH);
                        let _ = std::fs::remove_file(audio::RECORDING_WAV_COPY);

                        let _ = event_tx.send(DaemonEvent::PartialTranscript {
                            data: String::new(),
                        });
                        let _ = event_tx.send(DaemonEvent::FinalTranscript {
                            data: String::new(),
                        });
                    }
                    ClientCommand::Stop => {
                        let mut state = current_state.lock().await;
                        *state = DaemonState::Idle;
                        recorder.stop();
                        accumulated_text.clear();
                        last_live_text.clear();
                        current_generation += 1;
                        let _ = std::fs::remove_file(audio::RECORDING_WAV_PATH);
                        let _ = std::fs::remove_file(audio::RECORDING_WAV_COPY);

                        let _ = event_tx.send(DaemonEvent::StateChanged {
                            data: DaemonState::Idle,
                        });
                    }
                    ClientCommand::StopAndPaste { pid, delay_ms, text, wayland_display } => {
                        info!("Received StopAndPaste command (client pid: {})", pid);
                        if pid > 0 {
                            info!("Daemon terminating client process PID {} immediately", pid);
                            let _ = std::process::Command::new("kill").args(["-9", &pid.to_string()]).status();
                        }

                        let mut state = current_state.lock().await;
                        *state = DaemonState::Idle;
                        recorder.stop();
                        accumulated_text.clear();
                        last_live_text.clear();
                        current_generation += 1;
                        let _ = std::fs::remove_file(audio::RECORDING_WAV_PATH);
                        let _ = std::fs::remove_file(audio::RECORDING_WAV_COPY);

                        let _ = event_tx.send(DaemonEvent::StateChanged {
                            data: DaemonState::Idle,
                        });

                        // Copy to clipboard from daemon and trigger virtual keystroke Ctrl+V via ydotool
                        tokio::spawn(async move {
                            if let Some(txt) = text {
                                let w_disp = wayland_display.clone();
                                let copy_res = tokio::task::spawn_blocking(move || {
                                    use std::io::Write;
                                    let mut cmd = std::process::Command::new("wl-copy");
                                    if let Some(ref disp) = w_disp {
                                        cmd.env("WAYLAND_DISPLAY", disp);
                                    } else if std::env::var("WAYLAND_DISPLAY").is_err() {
                                        cmd.env("WAYLAND_DISPLAY", "wayland-0");
                                    }
                                    if std::env::var("XDG_RUNTIME_DIR").is_err() {
                                        cmd.env("XDG_RUNTIME_DIR", "/run/user/1000");
                                    }
                                    cmd.stdin(std::process::Stdio::piped())
                                       .stdout(std::process::Stdio::null())
                                       .stderr(std::process::Stdio::null());

                                    match cmd.spawn() {
                                        Ok(mut child) => {
                                            if let Some(mut stdin) = child.stdin.take() {
                                                let _ = stdin.write_all(txt.as_bytes());
                                                let _ = stdin.flush();
                                                drop(stdin);
                                            }
                                            let _ = child.wait();
                                            info!("Daemon successfully copied text to Wayland clipboard via wl-copy");
                                        }
                                        Err(e) => {
                                            warn!("Daemon failed to spawn wl-copy: {:?}", e);
                                        }
                                    }
                                }).await;
                                if let Err(e) = copy_res {
                                    warn!("Error in clipboard copy task: {:?}", e);
                                }
                            }

                            info!("Daemon will trigger Ctrl+V paste after {}ms delay...", delay_ms);
                            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                            info!("Daemon executing virtual keystroke Ctrl+V via ydotool...");
                            let ydo_status = tokio::process::Command::new("ydotool")
                                .env("YDOTOOL_SOCKET", "/tmp/.ydotool_socket")
                                .args(["key", "29:1", "47:1", "47:0", "29:0"])
                                .status()
                                .await;

                            match ydo_status {
                                Ok(status) if status.success() => {
                                    info!("Daemon successfully simulated Ctrl+V into active window element!");
                                }
                                Ok(status) => {
                                    warn!("ydotool exited with non-zero status: {:?}", status);
                                }
                                Err(e) => {
                                    error!("Failed to execute ydotool: {:?}", e);
                                }
                            }
                        });
                    }
                    ClientCommand::CancelAndExit { pid } => {
                        info!("Received CancelAndExit command (client pid: {})", pid);
                        if pid > 0 {
                            let _ = std::process::Command::new("kill").args(["-9", &pid.to_string()]).status();
                        }
                        let mut state = current_state.lock().await;
                        *state = DaemonState::Idle;
                        recorder.stop();
                        accumulated_text.clear();
                        last_live_text.clear();
                        current_generation += 1;
                        let _ = std::fs::remove_file(audio::RECORDING_WAV_PATH);
                        let _ = std::fs::remove_file(audio::RECORDING_WAV_COPY);

                        let _ = event_tx.send(DaemonEvent::StateChanged {
                            data: DaemonState::Idle,
                        });
                    }
                }
            }

            // Graceful shutdown on Ctrl+C
            _ = tokio::signal::ctrl_c() => {
                info!("Daemon shutting down");
                break;
            }
        }
    }

    Ok(())
}
