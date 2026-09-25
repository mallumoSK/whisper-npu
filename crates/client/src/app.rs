use crate::fix::{
    request_text_fix_with_progress, stop_systemctl_service, FixProgress, DEFAULT_LLAMA_API_URL,
    DEFAULT_LLAMA_HEALTH_URL, DEFAULT_LLAMA_SERVICE,
};
use crate::ipc::ClientIpcHandle;
use crate::paste::copy_to_clipboard;
use crate::ui::fix_window::{FixAction, FixPhase, FixWindowState};
use crate::ui::icons::{
    paint_edit_icon, paint_enter_icon, paint_esc_icon, paint_paste_icon, paint_pause_icon,
    paint_play_icon, paint_space_icon, paint_trash_icon, paint_wand_icon, paint_waveform_icon,
};
use crate::ui::soundwave::paint_soundwave;
use crate::ui::theme::{
    apply_m3_style, CHIP_BG, ERROR_TEXT, KEY_PILL_TEXT, ON_SURFACE, ON_SURFACE_DIM, OUTLINE,
    PAUSED_BG, PAUSED_STROKE, PAUSED_TEXT, PRIMARY, PRIMARY_BUTTON_BG, PRIMARY_BUTTON_TEXT,
    PURPLE_ICON_BG, REC_BG, REC_STROKE, REC_TEXT, SURFACE, SURFACE_CARD, SURFACE_EDITOR, TAG_BG,
    TONAL_FIX_BG, TONAL_FIX_STROKE, TONAL_FIX_TEXT,
};
use eframe::egui::{
    self, vec2, Align, CentralPanel, Color32, CursorIcon, Frame, Key, Layout, Margin, RichText,
    Sense, Stroke, TextEdit,
};
use protocol::{ClientCommand, DaemonEvent, DaemonState};
use std::sync::mpsc::{channel, Receiver, Sender};
use tracing::info;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SttTarget {
    MainText,
    FixPrompt,
    Ignore,
}

pub struct WhisperClientApp {
    ipc: ClientIpcHandle,
    state: DaemonState,
    text: String,
    is_editing: bool,
    was_recording_before_edit: bool,
    fix_window: FixWindowState,
    stt_target: SttTarget,
    status_error: Option<String>,
    fix_tx: Sender<FixProgress>,
    fix_rx: Receiver<FixProgress>,
}

impl WhisperClientApp {
    pub fn new(ipc: ClientIpcHandle) -> Self {
        let (fix_tx, fix_rx) = channel();
        Self {
            ipc,
            state: DaemonState::Listening,
            text: String::new(),
            is_editing: false,
            was_recording_before_edit: false,
            fix_window: FixWindowState::default(),
            stt_target: SttTarget::MainText,
            status_error: None,
            fix_tx,
            fix_rx,
        }
    }

    fn toggle_pause(&mut self) {
        if self.state == DaemonState::Listening {
            let _ = self.ipc.cmd_tx.send(ClientCommand::PauseListening);
            self.state = DaemonState::Paused;
            if self.is_editing {
                self.was_recording_before_edit = false;
            }
        } else {
            let _ = self.ipc.cmd_tx.send(ClientCommand::ResumeListening);
            self.state = DaemonState::Listening;
            if self.is_editing {
                self.was_recording_before_edit = true;
            }
            if !self.fix_window.is_open {
                self.stt_target = SttTarget::MainText;
            }
        }
    }

    fn toggle_edit(&mut self) {
        if self.is_editing {
            // Exiting edit mode ("Done")
            self.is_editing = false;
            if self.was_recording_before_edit {
                info!("Exiting edit mode: resuming recording");
                let _ = self.ipc.cmd_tx.send(ClientCommand::ResumeListening);
                self.state = DaemonState::Listening;
                self.was_recording_before_edit = false;
                self.stt_target = SttTarget::MainText;
            } else {
                info!("Exiting edit mode: staying paused");
            }
        } else {
            // Entering edit mode ("Edit")
            if self.state == DaemonState::Listening {
                info!("Entering edit mode: pausing active recording");
                self.was_recording_before_edit = true;
                let _ = self.ipc.cmd_tx.send(ClientCommand::PauseListening);
                self.state = DaemonState::Paused;
            } else {
                info!("Entering edit mode: already paused");
                self.was_recording_before_edit = false;
            }
            self.is_editing = true;
        }
    }

    fn stop_llama_service_async() {
        std::thread::spawn(|| {
            let _ = stop_systemctl_service(DEFAULT_LLAMA_SERVICE);
        });
    }

    fn stop_and_paste(&mut self) {
        info!("Stopping recording and delegating paste to daemon: {}", self.text);
        if self.fix_window.is_open {
            Self::stop_llama_service_async();
        }

        let pid = std::process::id();
        let wayland_display = std::env::var("REAL_WAYLAND_DISPLAY")
            .or_else(|_| std::env::var("WAYLAND_DISPLAY"))
            .ok();

        // Safe async clipboard copy in client background thread (never blocks GUI loop)
        let text_copy = self.text.clone();
        std::thread::spawn(move || {
            let _ = copy_to_clipboard(&text_copy);
        });

        let _ = self.ipc.cmd_tx.send(ClientCommand::StopAndPaste {
            pid,
            delay_ms: 180,
            text: Some(self.text.clone()),
            wayland_display,
        });

        // Brief pause to ensure the IPC frame is dispatched before exit
        std::thread::sleep(std::time::Duration::from_millis(60));
        std::process::exit(0);
    }

    fn cancel_and_exit(&mut self) {
        info!("Canceling dictation and exiting client");
        if self.fix_window.is_open {
            Self::stop_llama_service_async();
        }
        let pid = std::process::id();
        let _ = self.ipc.cmd_tx.send(ClientCommand::CancelAndExit { pid });
        std::thread::sleep(std::time::Duration::from_millis(60));
        std::process::exit(0);
    }

    fn open_fix_window(&mut self) {
        if self.text.trim().is_empty() || self.fix_window.is_open {
            return;
        }

        info!("Opening auxiliary Fix window on the right");
        self.fix_window.is_open = true;
        self.fix_window.has_positioned = false;
        self.fix_window.phase = FixPhase::Prompting;
        self.fix_window.origin_text = self.text.clone();
        self.fix_window.prompt_text.clear();
        self.fix_window.result_text.clear();
        self.fix_window.status_text = None;
        self.stt_target = SttTarget::FixPrompt;

        // Reset daemon audio buffer so new speech streams into the fix prompt
        let _ = self.ipc.cmd_tx.send(ClientCommand::ClearBuffer);

        // Resume listening if paused so user can immediately dictate the prompt
        if self.state != DaemonState::Listening {
            let _ = self.ipc.cmd_tx.send(ClientCommand::ResumeListening);
            self.state = DaemonState::Listening;
        }
    }

    fn submit_fix_request(&mut self) {
        if !self.fix_window.is_open || self.fix_window.phase != FixPhase::Prompting {
            return;
        }

        // Pause listening while calling LLM
        self.stt_target = SttTarget::Ignore;
        if self.state == DaemonState::Listening {
            let _ = self.ipc.cmd_tx.send(ClientCommand::PauseListening);
            self.state = DaemonState::Paused;
        }

        self.fix_window.phase = FixPhase::Processing;
        self.fix_window.status_text = Some("Checking LLM health...".to_string());

        let origin_text = self.fix_window.origin_text.clone();
        let instruction = self.fix_window.prompt_text.clone();
        let fix_tx = self.fix_tx.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            match rt {
                Ok(rt) => {
                    let tx_for_status = fix_tx.clone();
                    let res = rt.block_on(request_text_fix_with_progress(
                        DEFAULT_LLAMA_SERVICE,
                        DEFAULT_LLAMA_API_URL,
                        DEFAULT_LLAMA_HEALTH_URL,
                        &origin_text,
                        &instruction,
                        move |status_str| {
                            let _ = tx_for_status.send(FixProgress::Status(status_str.to_string()));
                        },
                    ));
                    match res {
                        Ok(fixed) => {
                            let _ = fix_tx.send(FixProgress::Done(Ok(fixed)));
                        }
                        Err(e) => {
                            let _ = fix_tx.send(FixProgress::Done(Err(e.to_string())));
                        }
                    }
                }
                Err(e) => {
                    let _ = fix_tx.send(FixProgress::Done(Err(format!("Runtime error: {:?}", e))));
                }
            }
        });
    }

    fn confirm_and_apply_fix(&mut self) {
        if self.fix_window.is_open && self.fix_window.phase == FixPhase::Review {
            info!("Confirming and replacing main text with LLM result");
            self.stt_target = SttTarget::Ignore;
            self.text = self.fix_window.result_text.clone();
            self.fix_window.is_open = false;
            if self.state == DaemonState::Listening {
                let _ = self.ipc.cmd_tx.send(ClientCommand::PauseListening);
                self.state = DaemonState::Paused;
            }
            Self::stop_llama_service_async();
        }
    }

    fn close_fix_window(&mut self) {
        info!("Closing Fix window without applying changes");
        self.stt_target = SttTarget::Ignore;
        self.text = self.fix_window.origin_text.clone();
        self.fix_window.is_open = false;
        if self.state == DaemonState::Listening {
            let _ = self.ipc.cmd_tx.send(ClientCommand::PauseListening);
            self.state = DaemonState::Paused;
        }
        Self::stop_llama_service_async();
    }

    fn reprompt_fix(&mut self) {
        info!("Resetting Fix window back to prompting phase");
        self.fix_window.phase = FixPhase::Prompting;
        self.fix_window.result_text.clear();
        self.stt_target = SttTarget::FixPrompt;
        let _ = self.ipc.cmd_tx.send(ClientCommand::ClearBuffer);
        if self.state != DaemonState::Listening {
            let _ = self.ipc.cmd_tx.send(ClientCommand::ResumeListening);
            self.state = DaemonState::Listening;
        }
    }
}

impl eframe::App for WhisperClientApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. Process daemon IPC events
        while let Ok(event) = self.ipc.event_rx.try_recv() {
            match event {
                DaemonEvent::StateChanged { data } => {
                    self.state = data;
                }
                DaemonEvent::PartialTranscript { data } => {
                    match self.stt_target {
                        SttTarget::FixPrompt => {
                            if self.fix_window.is_open && self.fix_window.phase == FixPhase::Prompting {
                                self.fix_window.prompt_text = data;
                            }
                        }
                        SttTarget::MainText => {
                            if !self.is_editing && !self.fix_window.is_open {
                                self.text = data;
                            }
                        }
                        SttTarget::Ignore => {}
                    }
                }
                DaemonEvent::FinalTranscript { data } => {
                    match self.stt_target {
                        SttTarget::FixPrompt => {
                            if self.fix_window.is_open && self.fix_window.phase == FixPhase::Prompting {
                                self.fix_window.prompt_text = data;
                            }
                        }
                        SttTarget::MainText => {
                            if !self.is_editing && !self.fix_window.is_open {
                                self.text = data;
                            }
                        }
                        SttTarget::Ignore => {}
                    }
                }
                DaemonEvent::Error { data } => {
                    self.status_error = Some(data);
                }
            }
        }

        // 2. Process LLM fix results
        while let Ok(msg) = self.fix_rx.try_recv() {
            match msg {
                FixProgress::Status(status) => {
                    self.fix_window.status_text = Some(status);
                }
                FixProgress::Done(Ok(fixed_text)) => {
                    self.fix_window.phase = FixPhase::Review;
                    self.fix_window.result_text = fixed_text;
                    self.fix_window.status_text = None;
                }
                FixProgress::Done(Err(err_msg)) => {
                    self.fix_window.phase = FixPhase::Prompting;
                    self.fix_window.status_text = Some(format!("LLM Fix failed: {}", err_msg));
                }
            }
        }

        // 3. Handle Keyboard Shortcuts
        let mut do_stop_and_paste = false;
        let mut do_cancel_and_exit = false;
        let mut do_toggle_pause = false;
        let mut do_toggle_edit = false;

        ctx.input(|i| {
            if self.fix_window.is_open {
                // If Fix window is open, route shortcuts to Fix window flow even if main window has focus
                if i.key_pressed(Key::Escape) {
                    self.close_fix_window();
                } else if i.key_pressed(Key::Space) {
                    do_toggle_pause = true;
                } else if i.key_pressed(Key::Enter) {
                    match self.fix_window.phase {
                        FixPhase::Prompting => self.submit_fix_request(),
                        FixPhase::Review => self.confirm_and_apply_fix(),
                        FixPhase::Processing => {}
                    }
                }
            } else if self.is_editing {
                // In editing mode, Escape acts as Done and exits back to normal mode
                if i.key_pressed(Key::Escape) {
                    do_toggle_edit = true;
                }
            } else {
                // In navigational mode:
                if i.key_pressed(Key::Space) {
                    do_toggle_pause = true;
                } else if i.key_pressed(Key::Enter) {
                    do_stop_and_paste = true;
                } else if i.key_pressed(Key::Escape) {
                    do_cancel_and_exit = true;
                }
            }
        });

        if do_toggle_edit {
            self.toggle_edit();
        }

        if do_toggle_pause {
            self.toggle_pause();
        }
        if do_stop_and_paste {
            self.stop_and_paste();
        }
        if do_cancel_and_exit {
            self.cancel_and_exit();
        }

        // 4. Apply Material Design 3 (M3) Dark Theme
        apply_m3_style(ctx);

        CentralPanel::default()
            .frame(Frame::none().fill(SURFACE).inner_margin(18.0))
            .show(ctx, |ui| {
                // Top Header Row: [Waveform Icon] "Whisper Dictation" [EN Badge]
                ui.horizontal(|ui| {
                    let (icon_rect, _icon_resp) = ui.allocate_exact_size(vec2(36.0, 36.0), Sense::hover());
                    ui.painter().circle_filled(icon_rect.center(), 18.0, PURPLE_ICON_BG);
                    paint_waveform_icon(ui.painter(), icon_rect.center(), Color32::from_rgb(0xde, 0xd2, 0xfc));

                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("Whisper Dictation")
                            .color(ON_SURFACE)
                            .strong()
                            .size(19.0),
                    );

                    // Pill Tag [EN]
                    let tag_frame = Frame::none()
                        .fill(TAG_BG)
                        .rounding(10.0)
                        .inner_margin(Margin::symmetric(8.0, 3.0));
                    tag_frame.show(ui, |ui| {
                        ui.label(
                            RichText::new("EN")
                                .color(ON_SURFACE_DIM)
                                .size(11.0)
                                .strong(),
                        );
                    });
                });

                ui.add_space(12.0);

                // Top Card: Status Pill & Interactive Shortcut Pills (Single Horizontal Line)
                Frame::none()
                    .fill(SURFACE_CARD)
                    .stroke(Stroke::new(1.0, OUTLINE))
                    .rounding(20.0)
                    .inner_margin(Margin::symmetric(14.0, 10.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // 1. Status Pill
                            let (st_bg, st_stroke, st_color, st_text, is_rec, is_paused) = if self.fix_window.is_open {
                                let label = match self.fix_window.phase {
                                    FixPhase::Prompting => "Fixing: Dictating prompt",
                                    FixPhase::Processing => "Fixing: Processing LLM...",
                                    FixPhase::Review => "Fixing: Review result",
                                };
                                (CHIP_BG, OUTLINE, PRIMARY, label.to_string(), false, false)
                            } else if self.is_editing {
                                (CHIP_BG, OUTLINE, PRIMARY, "Editing".to_string(), false, false)
                            } else {
                                match self.state {
                                    DaemonState::Listening => (REC_BG, REC_STROKE, REC_TEXT, "Recording".to_string(), true, false),
                                    DaemonState::Paused => (PAUSED_BG, PAUSED_STROKE, PAUSED_TEXT, "Paused".to_string(), false, true),
                                    DaemonState::Idle => (CHIP_BG, OUTLINE, ON_SURFACE_DIM, "Standby".to_string(), false, false),
                                }
                            };

                            let st_pill = Frame::none()
                                .fill(st_bg)
                                .stroke(Stroke::new(1.0, st_stroke))
                                .rounding(16.0)
                                .inner_margin(Margin::symmetric(12.0, 6.0));
                            st_pill.show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    if is_rec {
                                        let (r, _) = ui.allocate_exact_size(vec2(12.0, 12.0), Sense::hover());
                                        ui.painter().circle_filled(r.center(), 4.0, st_color);
                                    } else if is_paused {
                                        let (r, _) = ui.allocate_exact_size(vec2(12.0, 12.0), Sense::hover());
                                        paint_pause_icon(ui.painter(), r.center(), st_color);
                                    } else if self.is_editing {
                                        let (r, _) = ui.allocate_exact_size(vec2(12.0, 12.0), Sense::hover());
                                        paint_edit_icon(ui.painter(), r.center(), st_color);
                                    } else if self.fix_window.is_open {
                                        let (r, _) = ui.allocate_exact_size(vec2(12.0, 12.0), Sense::hover());
                                        paint_wand_icon(ui.painter(), r.center(), st_color);
                                    }
                                    ui.add_space(2.0);
                                    ui.label(RichText::new(st_text).color(st_color).size(12.5).strong());
                                });
                            });

                            ui.add_space(4.0);

                            // 2. Stop & Paste Pill: [↵ Enter Stop & Paste]
                            let enter_pill = Frame::none()
                                .fill(CHIP_BG)
                                .stroke(Stroke::new(1.0, OUTLINE))
                                .rounding(16.0)
                                .inner_margin(Margin::symmetric(10.0, 6.0));
                            let enter_resp = enter_pill.show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let (r, _) = ui.allocate_exact_size(vec2(12.0, 12.0), Sense::hover());
                                    paint_enter_icon(ui.painter(), r.center(), KEY_PILL_TEXT);
                                    ui.add_space(2.0);
                                    ui.label(RichText::new("Enter").color(KEY_PILL_TEXT).strong().size(12.0));
                                    ui.label(RichText::new("Stop & Paste").color(ON_SURFACE_DIM).size(12.0));
                                });
                            }).response;
                            let enter_interact = enter_resp.interact(Sense::click());
                            if enter_interact.hovered() {
                                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                            }
                            if enter_interact.clicked() {
                                self.stop_and_paste();
                            }

                            ui.add_space(4.0);

                            // 3. Space Pause / Resume Pill: [␣ Space Pause/Resume]
                            let space_action = if self.state == DaemonState::Listening { "Pause" } else { "Resume" };
                            let space_pill = Frame::none()
                                .fill(CHIP_BG)
                                .stroke(Stroke::new(1.0, OUTLINE))
                                .rounding(16.0)
                                .inner_margin(Margin::symmetric(10.0, 6.0));
                            let space_resp = space_pill.show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let (r, _) = ui.allocate_exact_size(vec2(12.0, 12.0), Sense::hover());
                                    paint_space_icon(ui.painter(), r.center(), KEY_PILL_TEXT);
                                    ui.add_space(2.0);
                                    ui.label(RichText::new("Space").color(KEY_PILL_TEXT).strong().size(12.0));
                                    ui.label(RichText::new(space_action).color(ON_SURFACE_DIM).size(12.0));
                                });
                            }).response;
                            let space_interact = space_resp.interact(Sense::click());
                            if space_interact.hovered() {
                                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                            }
                            if space_interact.clicked() {
                                self.toggle_pause();
                            }

                            ui.add_space(4.0);

                            // 4. Esc Cancel Pill: [⎋ Esc Cancel]
                            let esc_pill = Frame::none()
                                .fill(CHIP_BG)
                                .stroke(Stroke::new(1.0, OUTLINE))
                                .rounding(16.0)
                                .inner_margin(Margin::symmetric(10.0, 6.0));
                            let esc_resp = esc_pill.show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let (r, _) = ui.allocate_exact_size(vec2(12.0, 12.0), Sense::hover());
                                    paint_esc_icon(ui.painter(), r.center(), KEY_PILL_TEXT);
                                    ui.add_space(2.0);
                                    ui.label(RichText::new("Esc").color(KEY_PILL_TEXT).strong().size(12.0));
                                    ui.label(RichText::new("Cancel").color(ON_SURFACE_DIM).size(12.0));
                                });
                            }).response;
                            let esc_interact = esc_resp.interact(Sense::click());
                            if esc_interact.hovered() {
                                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                            }
                            if esc_interact.clicked() {
                                self.cancel_and_exit();
                            }
                        });
                    });

                if let Some(ref err) = self.status_error {
                    ui.add_space(4.0);
                    ui.label(RichText::new(err).color(ERROR_TEXT).size(12.0));
                }

                ui.add_space(12.0);

                // Central Main Card: Text Display / Editor + Bottom Waveform
                let editor_card_height = (ui.available_height() - 56.0).max(180.0);
                Frame::none()
                    .fill(SURFACE_EDITOR)
                    .stroke(Stroke::new(1.0, OUTLINE))
                    .rounding(20.0)
                    .inner_margin(16.0)
                    .show(ui, |ui| {
                        let text_height = (editor_card_height - 65.0).max(120.0);
                        let text_edit = TextEdit::multiline(&mut self.text)
                            .text_color(ON_SURFACE)
                            .desired_width(ui.available_width())
                            .desired_rows(10)
                            .interactive(self.is_editing)
                            .hint_text("Live streaming transcript will appear here... |");

                        ui.add_sized([ui.available_width(), text_height], text_edit);

                        // Centered Soundwave Visualizer in lower part of editor card
                        ui.vertical_centered(|ui| {
                            ui.add_space(6.0);
                            let (wave_rect, _) = ui.allocate_exact_size(vec2(120.0, 32.0), Sense::hover());
                            let time = ui.input(|i| i.time);
                            paint_soundwave(ui.painter(), wave_rect.center(), time, self.state == DaemonState::Listening, 9, 36.0, 9.0);
                        });
                    });

                ui.add_space(10.0);

                // Bottom Row of Action Buttons (Left: Clear, Edit, Fix | Right: Resume/Pause, Stop & Paste)
                ui.horizontal(|ui| {
                    let btn_padding = Margin::symmetric(14.0, 7.0);

                    // 1. [Clear]
                    let clear_frame = Frame::none()
                        .fill(SURFACE_CARD)
                        .stroke(Stroke::new(1.0, OUTLINE))
                        .rounding(18.0)
                        .inner_margin(btn_padding);
                    let clear_resp = clear_frame.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let (r, _) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
                            paint_trash_icon(ui.painter(), r.center(), ON_SURFACE_DIM);
                            ui.add_space(3.0);
                            ui.label(RichText::new("Clear").size(13.0).color(ON_SURFACE));
                        });
                    }).response;
                    let clear_interact = clear_resp.interact(Sense::click());
                    if clear_interact.hovered() {
                        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                    }
                    if clear_interact.clicked() {
                        self.text.clear();
                        let _ = self.ipc.cmd_tx.send(ClientCommand::ClearBuffer);
                    }

                    ui.add_space(6.0);

                    // 2. [Edit]
                    let edit_label = if self.is_editing { "Done" } else { "Edit" };
                    let edit_frame = Frame::none()
                        .fill(if self.is_editing { TAG_BG } else { SURFACE_CARD })
                        .stroke(Stroke::new(1.0, OUTLINE))
                        .rounding(18.0)
                        .inner_margin(btn_padding);
                    let edit_resp = edit_frame.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let (r, _) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
                            paint_edit_icon(ui.painter(), r.center(), ON_SURFACE_DIM);
                            ui.add_space(3.0);
                            ui.label(RichText::new(edit_label).size(13.0).color(ON_SURFACE));
                        });
                    }).response;
                    let edit_interact = edit_resp.interact(Sense::click());
                    if edit_interact.hovered() {
                        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                    }
                    if edit_interact.clicked() {
                        self.toggle_edit();
                    }

                    ui.add_space(6.0);

                    // 3. [Fix (LLM)] - Elevated / Highlighted Tonal Pill
                    let fix_frame = Frame::none()
                        .fill(TONAL_FIX_BG)
                        .stroke(Stroke::new(1.0, TONAL_FIX_STROKE))
                        .rounding(18.0)
                        .inner_margin(btn_padding);
                    let fix_resp = fix_frame.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let (r, _) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
                            paint_wand_icon(ui.painter(), r.center(), PRIMARY);
                            ui.add_space(3.0);
                            ui.label(RichText::new("Fix (LLM)").size(13.0).strong().color(TONAL_FIX_TEXT));
                        });
                    }).response;
                    let fix_interact = fix_resp.interact(Sense::click());
                    if fix_interact.hovered() {
                        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                    }
                    if fix_interact.clicked() {
                        self.open_fix_window();
                    }

                    // Right group: [Resume / Pause] and [Stop & Paste]
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // 5. [Stop & Paste] - Filled Lavender/Purple Primary Button (Far Right)
                        let stop_paste_frame = Frame::none()
                            .fill(PRIMARY_BUTTON_BG)
                            .rounding(18.0)
                            .inner_margin(Margin::symmetric(16.0, 7.0));
                        let stop_paste_resp = stop_paste_frame.show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let (r, _) = ui.allocate_exact_size(vec2(15.0, 15.0), Sense::hover());
                                paint_paste_icon(ui.painter(), r.center(), PRIMARY_BUTTON_TEXT);
                                ui.add_space(3.0);
                                ui.label(RichText::new("Stop & Paste").size(13.0).strong().color(PRIMARY_BUTTON_TEXT));
                            });
                        }).response;
                        let stop_interact = stop_paste_resp.interact(Sense::click());
                        if stop_interact.hovered() {
                            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                        }
                        if stop_interact.clicked() {
                            self.stop_and_paste();
                        }

                        ui.add_space(8.0);

                        // 4. [Resume / Pause]
                        let is_listening = self.state == DaemonState::Listening;
                        let pause_text = if is_listening { "Pause" } else { "Resume" };
                        let pause_frame = Frame::none()
                            .fill(SURFACE_CARD)
                            .stroke(Stroke::new(1.0, OUTLINE))
                            .rounding(18.0)
                            .inner_margin(btn_padding);
                        let pause_resp = pause_frame.show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let (r, _) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
                                if is_listening {
                                    paint_pause_icon(ui.painter(), r.center(), ON_SURFACE_DIM);
                                } else {
                                    paint_play_icon(ui.painter(), r.center(), ON_SURFACE_DIM);
                                }
                                ui.add_space(3.0);
                                ui.label(RichText::new(pause_text).size(13.0).color(ON_SURFACE));
                            });
                        }).response;
                        let pause_interact = pause_resp.interact(Sense::click());
                        if pause_interact.hovered() {
                            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                        }
                        if pause_interact.clicked() {
                            self.toggle_pause();
                        }
                    });
                });
            });

        // Show secondary Fix Window if open and dispatch any triggered actions
        if let Some(action) = self.fix_window.show(ctx, self.state == DaemonState::Listening) {
            match action {
                FixAction::TogglePause => self.toggle_pause(),
                FixAction::Submit => self.submit_fix_request(),
                FixAction::Confirm => self.confirm_and_apply_fix(),
                FixAction::Close => self.close_fix_window(),
                FixAction::Reprompt => self.reprompt_fix(),
            }
        }

        // Request constant frame repaint for live updates and soundwave animation
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
    }
}
