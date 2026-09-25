mod app;
mod fix;
mod ipc;
mod paste;
pub mod ui;

use app::WhisperClientApp;
use eframe::egui;
use ipc::start_ipc_client;
use tracing::info;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,client=debug".into()),
        )
        .init();

    info!("Starting whisper-client (GUI)...");

    // Ensure the client runs via Xwayland so GNOME Shell allows "Always on Top" (_NET_WM_STATE_ABOVE).
    // Native Wayland (xdg_toplevel) protocol under GNOME intentionally blocks client-initiated always-on-top.
    if std::env::var("WHISPER_NATIVE_WAYLAND").is_err() {
        if let Ok(w_disp) = std::env::var("WAYLAND_DISPLAY") {
            std::env::set_var("REAL_WAYLAND_DISPLAY", w_disp);
        }
        std::env::remove_var("WAYLAND_DISPLAY");
        std::env::set_var("WINIT_UNIX_BACKEND", "x11");
    }

    // Background thread to apply _NET_WM_STATE_ABOVE via wmctrl to main and auxiliary windows
    std::thread::spawn(|| {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(500));
            let _ = std::process::Command::new("wmctrl")
                .args(["-r", "Whisper Dictation", "-b", "add,above"])
                .status();
            let _ = std::process::Command::new("wmctrl")
                .args(["-r", "Fix with LLM", "-b", "add,above"])
                .status();
        }
    });

    let ipc_handle = start_ipc_client(None);

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_app_id("whisper-dictation")
            .with_inner_size([630.0, 520.0])
            .with_min_inner_size([560.0, 450.0])
            .with_title("Whisper Dictation [EN]")
            .with_always_on_top()
            .with_decorations(true)
            .with_transparent(false),
        ..Default::default()
    };

    eframe::run_native(
        "Whisper Dictation",
        native_options,
        Box::new(|cc| {
            let mut fonts = egui::FontDefinitions::default();
            for path in &[
                "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
                "/usr/share/fonts/truetype/noto/NotoSansSymbols-Regular.ttf",
                "/usr/share/fonts/truetype/noto/NotoSansSymbols2-Regular.ttf",
            ] {
                if let Ok(data) = std::fs::read(path) {
                    let name = path.rsplit('/').next().unwrap_or("fallback_font").to_owned();
                    fonts.font_data.insert(name.clone(), egui::FontData::from_owned(data).into());
                    if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                        family.push(name);
                    }
                }
            }
            cc.egui_ctx.set_fonts(fonts);
            Ok(Box::new(WhisperClientApp::new(ipc_handle)))
        }),
    )
}
