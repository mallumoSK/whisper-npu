use anyhow::Result;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;
use tracing::{info, warn};

pub fn copy_to_clipboard(text: &str) -> Result<()> {
    // 1. Try native wl-copy using stdin piping with timeout
    let mut cmd = Command::new("wl-copy");
    if let Ok(disp) = std::env::var("REAL_WAYLAND_DISPLAY") {
        cmd.env("WAYLAND_DISPLAY", disp);
    } else if let Ok(disp) = std::env::var("WAYLAND_DISPLAY") {
        cmd.env("WAYLAND_DISPLAY", disp);
    }

    cmd.stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    if let Ok(mut child) = cmd.spawn() {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
            let _ = stdin.flush();
            drop(stdin);
        }

        let start = std::time::Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    if status.success() {
                        info!("Copied text to Wayland clipboard using wl-copy");
                        return Ok(());
                    }
                    break;
                }
                Ok(None) => {
                    if start.elapsed() > Duration::from_millis(150) {
                        warn!("wl-copy taking >150ms in client; proceeding non-blockingly");
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(e) => {
                    warn!("wl-copy try_wait error: {:?}", e);
                    break;
                }
            }
        }
    }

    // 2. Fallback to arboard
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        if let Err(e) = clipboard.set_text(text.to_string()) {
            warn!("arboard clipboard set failed: {:?}", e);
        } else {
            info!("Copied text to clipboard using arboard");
            return Ok(());
        }
    }

    warn!("Could not copy text to clipboard using wl-copy or arboard");
    Ok(())
}
