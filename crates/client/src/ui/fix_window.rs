use crate::ui::icons::{
    paint_check_icon, paint_close_icon, paint_pause_icon, paint_play_icon, paint_refresh_icon,
    paint_wand_icon,
};
use crate::ui::soundwave::paint_soundwave;
use crate::ui::theme::{
    apply_m3_style, CHIP_BG, KEY_PILL_TEXT, ON_SURFACE, ON_SURFACE_DIM, OUTLINE, PRIMARY,
    PRIMARY_BUTTON_BG, PRIMARY_BUTTON_TEXT, PURPLE_ICON_BG, SURFACE, SURFACE_CARD, SURFACE_EDITOR,
};
use eframe::egui::{
    pos2, vec2, Align, CentralPanel, Color32, Context, CursorIcon, Frame, Key, Layout, Margin,
    RichText, Sense, Stroke, TextEdit, TopBottomPanel, ViewportBuilder, ViewportId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixPhase {
    Prompting,
    Processing,
    Review,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixAction {
    TogglePause,
    Submit,
    Confirm,
    Close,
    Reprompt,
}

#[derive(Debug, Clone)]
pub struct FixWindowState {
    pub is_open: bool,
    pub has_positioned: bool,
    pub phase: FixPhase,
    pub origin_text: String,
    pub prompt_text: String,
    pub result_text: String,
    pub status_text: Option<String>,
}

impl Default for FixWindowState {
    fn default() -> Self {
        Self {
            is_open: false,
            has_positioned: false,
            phase: FixPhase::Prompting,
            origin_text: String::new(),
            prompt_text: String::new(),
            result_text: String::new(),
            status_text: None,
        }
    }
}

impl FixWindowState {
    /// Renders the auxiliary Fix window viewport if it is open.
    /// Returns any user action triggered from this viewport (button click or keyboard shortcut).
    pub fn show(&mut self, ctx: &Context, is_listening: bool) -> Option<FixAction> {
        if !self.is_open {
            return None;
        }

        let main_rect = ctx.input(|i| i.viewport().outer_rect);
        let default_pos = main_rect.map(|r| pos2(r.max.x + 10.0, r.min.y));

        // Thinner width (400.0) matching main window's height (520.0)
        let mut viewport_builder = ViewportBuilder::default()
            .with_title("Fix with LLM")
            .with_inner_size([400.0, 520.0])
            .with_min_inner_size([340.0, 420.0])
            .with_always_on_top()
            .with_decorations(true)
            .with_transparent(false);

        if !self.has_positioned {
            if let Some(pos) = default_pos {
                viewport_builder = viewport_builder.with_position(pos);
                self.has_positioned = true;
            }
        }

        let fix_viewport_id = ViewportId::from_hash_of("whisper_fix_window");

        let mut action: Option<FixAction> = None;
        let fix_phase = self.phase;
        let mut prompt_text = self.prompt_text.clone();
        let mut result_text = self.result_text.clone();
        let status_text = self.status_text.clone();

        ctx.show_viewport_immediate(fix_viewport_id, viewport_builder, |fix_ctx, _class| {
            if fix_ctx.input(|i| i.viewport().close_requested()) {
                action = Some(FixAction::Close);
            }

            fix_ctx.input(|i| {
                if i.key_pressed(Key::Escape) {
                    action = Some(FixAction::Close);
                } else {
                    match fix_phase {
                        FixPhase::Prompting => {
                            if i.key_pressed(Key::Space) {
                                action = Some(FixAction::TogglePause);
                            } else if i.key_pressed(Key::Enter) {
                                action = Some(FixAction::Submit);
                            }
                        }
                        FixPhase::Review => {
                            if i.key_pressed(Key::Enter) {
                                action = Some(FixAction::Confirm);
                            }
                        }
                        FixPhase::Processing => {}
                    }
                }
            });

            apply_m3_style(fix_ctx);

            // Bottom action panel: explicitly pinned to bottom with generous margin so buttons are ALWAYS visible
            TopBottomPanel::bottom("fix_bottom_panel")
                .frame(Frame::none().fill(SURFACE).inner_margin(Margin {
                    left: 16.0,
                    right: 16.0,
                    top: 8.0,
                    bottom: 16.0,
                }))
                .show(fix_ctx, |ui| {
                    let btn_padding = Margin::symmetric(14.0, 7.0);

                    ui.horizontal(|ui| {
                        match fix_phase {
                            FixPhase::Prompting => {
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
                                    action = Some(FixAction::TogglePause);
                                }

                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    // Close Button
                                    let close_frame = Frame::none()
                                        .fill(SURFACE_CARD)
                                        .stroke(Stroke::new(1.0, OUTLINE))
                                        .rounding(18.0)
                                        .inner_margin(btn_padding);
                                    let close_resp = close_frame.show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            let (r, _) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
                                            paint_close_icon(ui.painter(), r.center(), ON_SURFACE_DIM);
                                            ui.add_space(3.0);
                                            ui.label(RichText::new("Close").size(13.0).color(ON_SURFACE));
                                        });
                                    }).response;
                                    let close_interact = close_resp.interact(Sense::click());
                                    if close_interact.hovered() {
                                        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                                    }
                                    if close_interact.clicked() {
                                        action = Some(FixAction::Close);
                                    }

                                    ui.add_space(6.0);

                                    // Confirm Button (1st press -> Submit to LLM)
                                    let confirm_frame = Frame::none()
                                        .fill(PRIMARY_BUTTON_BG)
                                        .rounding(18.0)
                                        .inner_margin(Margin::symmetric(16.0, 7.0));
                                    let confirm_resp = confirm_frame.show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            let (r, _) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
                                            paint_check_icon(ui.painter(), r.center(), PRIMARY_BUTTON_TEXT);
                                            ui.add_space(3.0);
                                            ui.label(RichText::new("Confirm").size(13.0).strong().color(PRIMARY_BUTTON_TEXT));
                                        });
                                    }).response;
                                    let confirm_interact = confirm_resp.interact(Sense::click());
                                    if confirm_interact.hovered() {
                                        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                                    }
                                    if confirm_interact.clicked() {
                                        action = Some(FixAction::Submit);
                                    }
                                });
                            }
                            FixPhase::Processing => {
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    let close_frame = Frame::none()
                                        .fill(SURFACE_CARD)
                                        .stroke(Stroke::new(1.0, OUTLINE))
                                        .rounding(18.0)
                                        .inner_margin(btn_padding);
                                    let close_resp = close_frame.show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            let (r, _) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
                                            paint_close_icon(ui.painter(), r.center(), ON_SURFACE_DIM);
                                            ui.add_space(3.0);
                                            ui.label(RichText::new("Cancel").size(13.0).color(ON_SURFACE));
                                        });
                                    }).response;
                                    let close_interact = close_resp.interact(Sense::click());
                                    if close_interact.hovered() {
                                        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                                    }
                                    if close_interact.clicked() {
                                        action = Some(FixAction::Close);
                                    }
                                });
                            }
                            FixPhase::Review => {
                                let reprompt_frame = Frame::none()
                                    .fill(SURFACE_CARD)
                                    .stroke(Stroke::new(1.0, OUTLINE))
                                    .rounding(18.0)
                                    .inner_margin(btn_padding);
                                let reprompt_resp = reprompt_frame.show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        let (r, _) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
                                        paint_refresh_icon(ui.painter(), r.center(), ON_SURFACE_DIM);
                                        ui.add_space(3.0);
                                        ui.label(RichText::new("Re-prompt").size(13.0).color(ON_SURFACE));
                                    });
                                }).response;
                                let reprompt_interact = reprompt_resp.interact(Sense::click());
                                if reprompt_interact.hovered() {
                                    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                                }
                                if reprompt_interact.clicked() {
                                    action = Some(FixAction::Reprompt);
                                }

                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    let close_frame = Frame::none()
                                        .fill(SURFACE_CARD)
                                        .stroke(Stroke::new(1.0, OUTLINE))
                                        .rounding(18.0)
                                        .inner_margin(btn_padding);
                                    let close_resp = close_frame.show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            let (r, _) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
                                            paint_close_icon(ui.painter(), r.center(), ON_SURFACE_DIM);
                                            ui.add_space(3.0);
                                            ui.label(RichText::new("Discard").size(13.0).color(ON_SURFACE));
                                        });
                                    }).response;
                                    let close_interact = close_resp.interact(Sense::click());
                                    if close_interact.hovered() {
                                        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                                    }
                                    if close_interact.clicked() {
                                        action = Some(FixAction::Close);
                                    }

                                    ui.add_space(6.0);

                                    // Confirm Button (2nd press -> Confirm replacement)
                                    let confirm_frame = Frame::none()
                                        .fill(PRIMARY_BUTTON_BG)
                                        .rounding(18.0)
                                        .inner_margin(Margin::symmetric(16.0, 7.0));
                                    let confirm_resp = confirm_frame.show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            let (r, _) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
                                            paint_check_icon(ui.painter(), r.center(), PRIMARY_BUTTON_TEXT);
                                            ui.add_space(3.0);
                                            ui.label(RichText::new("Confirm (Replace)").size(13.0).strong().color(PRIMARY_BUTTON_TEXT));
                                        });
                                    }).response;
                                    let confirm_interact = confirm_resp.interact(Sense::click());
                                    if confirm_interact.hovered() {
                                        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                                    }
                                    if confirm_interact.clicked() {
                                        action = Some(FixAction::Confirm);
                                    }
                                });
                            }
                        }
                    });
                });

            // Central panel fills remaining height between top header and bottom action bar
            CentralPanel::default()
                .frame(Frame::none().fill(SURFACE).inner_margin(Margin {
                    left: 16.0,
                    right: 16.0,
                    top: 16.0,
                    bottom: 4.0,
                }))
                .show(fix_ctx, |ui| {
                    // Header Row: [Wand Icon] "Fix with LLM" [Step Badge]
                    ui.horizontal(|ui| {
                        let (icon_rect, _) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::hover());
                        ui.painter().circle_filled(icon_rect.center(), 16.0, PURPLE_ICON_BG);
                        paint_wand_icon(ui.painter(), icon_rect.center(), Color32::from_rgb(0xde, 0xd2, 0xfc));

                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Fix with LLM")
                                .color(ON_SURFACE)
                                .strong()
                                .size(17.0),
                        );

                        let (badge_text, badge_color) = match fix_phase {
                            FixPhase::Prompting => ("1. Dictate Prompt", KEY_PILL_TEXT),
                            FixPhase::Processing => ("2. Processing...", PRIMARY),
                            FixPhase::Review => ("3. Review Result", Color32::from_rgb(0x81, 0xc7, 0x84)),
                        };

                        let tag_frame = Frame::none()
                            .fill(CHIP_BG)
                            .stroke(Stroke::new(1.0, OUTLINE))
                            .rounding(10.0)
                            .inner_margin(Margin::symmetric(8.0, 3.0));
                        tag_frame.show(ui, |ui| {
                            ui.label(
                                RichText::new(badge_text)
                                    .color(badge_color)
                                    .size(11.0)
                                    .strong(),
                            );
                        });
                    });

                    ui.add_space(10.0);

                    // Central Card: Takes exactly available height
                    let card_h = ui.available_height().max(160.0);
                    Frame::none()
                        .fill(SURFACE_EDITOR)
                        .stroke(Stroke::new(1.0, OUTLINE))
                        .rounding(18.0)
                        .inner_margin(14.0)
                        .show(ui, |ui| {
                            match fix_phase {
                                FixPhase::Prompting => {
                                    ui.label(
                                        RichText::new("Live STT instruction prompt:")
                                            .color(ON_SURFACE_DIM)
                                            .size(12.0),
                                    );
                                    ui.add_space(4.0);
                                    let edit_h = (ui.available_height() - 44.0).max(60.0);
                                    let text_edit = TextEdit::multiline(&mut prompt_text)
                                        .text_color(ON_SURFACE)
                                        .desired_width(ui.available_width())
                                        .hint_text("Speak instructions to fix origin text (e.g. \"make it more formal\")...");
                                    ui.add_sized([ui.available_width(), edit_h], text_edit);

                                    ui.vertical_centered(|ui| {
                                        ui.add_space(4.0);
                                        let (wave_rect, _) = ui.allocate_exact_size(vec2(100.0, 24.0), Sense::hover());
                                        let time = ui.input(|i| i.time);
                                        paint_soundwave(ui.painter(), wave_rect.center(), time, is_listening, 7, 24.0, 8.0);
                                    });
                                }
                                FixPhase::Processing => {
                                    // Vertically center the spinner and status content inside the card
                                    let avail_h = card_h - 28.0; // Subtract frame margins
                                    let content_h = 88.0;
                                    let pad_top = ((avail_h - content_h) * 0.5).max(15.0);
                                    ui.add_space(pad_top);

                                    ui.vertical_centered(|ui| {
                                        ui.spinner();
                                        ui.add_space(14.0);
                                        let st = status_text.as_deref().unwrap_or("Processing with LLM...");
                                        ui.label(RichText::new(st).color(PRIMARY).size(14.5).strong());
                                        ui.add_space(8.0);
                                        ui.label(
                                            RichText::new("Fixing origin prompt according to instructions...")
                                                .color(ON_SURFACE_DIM)
                                                .size(12.0),
                                        );
                                    });
                                }
                                FixPhase::Review => {
                                    ui.label(
                                        RichText::new("Revised Result Preview:")
                                            .color(Color32::from_rgb(0x81, 0xc7, 0x84))
                                            .size(12.0)
                                            .strong(),
                                    );
                                    ui.add_space(4.0);
                                    let edit_h = ui.available_height();
                                    let text_edit = TextEdit::multiline(&mut result_text)
                                        .text_color(ON_SURFACE)
                                        .desired_width(ui.available_width());
                                    ui.add_sized([ui.available_width(), edit_h], text_edit);
                                }
                            }
                        });
                });

            fix_ctx.request_repaint_after(std::time::Duration::from_millis(50));
        });

        self.prompt_text = prompt_text;
        self.result_text = result_text;

        action
    }
}
