use crate::ui::icons::{paint_close_icon, paint_copy_icon, paint_refresh_icon, paint_summary_icon};
use crate::ui::theme::{
    CHIP_BG, KEY_PILL_TEXT, ON_SURFACE, ON_SURFACE_DIM, OUTLINE, PRIMARY,
    SURFACE_CARD, SURFACE_EDITOR, TONAL_FIX_BG, TONAL_FIX_STROKE,
    TONAL_FIX_TEXT,
};
use eframe::egui::{
    pos2, vec2, Align, Color32, CursorIcon, Frame, Layout, Margin, RichText, ScrollArea,
    Sense, Stroke, TextStyle, Ui,
};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SummaryAction {
    Update,
    Copy,
    Collapse,
}

#[derive(Debug, Clone)]
pub struct SummaryState {
    pub is_expanded: bool,
    pub markdown_text: String,
    pub status_text: Option<String>,
    pub is_generating: bool,
    pub copied_notify_time: Option<Instant>,
}

impl Default for SummaryState {
    fn default() -> Self {
        Self {
            is_expanded: false,
            markdown_text: String::new(),
            status_text: None,
            is_generating: false,
            copied_notify_time: None,
        }
    }
}

impl SummaryState {
    pub fn show(&mut self, ui: &mut Ui, has_new_speech: bool) -> Option<SummaryAction> {
        let mut triggered_action = None;

        // Container card for Summary
        Frame::none()
            .fill(SURFACE_CARD)
            .stroke(Stroke::new(1.0, OUTLINE))
            .rounding(20.0)
            .inner_margin(14.0)
            .show(ui, |ui| {
                // Header Row: [Icon] "Executive Summary" | Status / Actions
                ui.horizontal(|ui| {
                    let title_resp = ui.horizontal(|ui| {
                        let (icon_rect, _) = ui.allocate_exact_size(vec2(24.0, 24.0), Sense::hover());
                        ui.painter().circle_filled(icon_rect.center(), 12.0, CHIP_BG);
                        paint_summary_icon(ui.painter(), icon_rect.center(), PRIMARY);

                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Executive Summary")
                                .color(ON_SURFACE)
                                .strong()
                                .size(15.0),
                        );
                    }).response;
                    let title_interact = title_resp.interact(Sense::click());
                    if title_interact.hovered() && !self.is_generating {
                        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                    }
                    if title_interact.clicked() && !self.is_generating {
                        triggered_action = Some(SummaryAction::Update);
                    }

                    // Show status if generating or set
                    if let Some(ref st) = self.status_text {
                        ui.add_space(6.0);
                        let is_gen = self.is_generating;
                        let st_color = if is_gen { PRIMARY } else { ON_SURFACE_DIM };
                        ui.label(RichText::new(st).size(12.0).color(st_color));
                    }

                    // Right-aligned actions: [Ctrl+Enter Update] [Copy] [Collapse]
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // Collapse [X]
                        let (close_rect, close_resp) =
                            ui.allocate_exact_size(vec2(22.0, 22.0), Sense::click());
                        let close_interact = close_resp.interact(Sense::click());
                        if close_interact.hovered() {
                            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                            ui.painter().circle_filled(close_rect.center(), 11.0, CHIP_BG);
                        }
                        paint_close_icon(ui.painter(), close_rect.center(), ON_SURFACE_DIM);
                        if close_interact.clicked() {
                            triggered_action = Some(SummaryAction::Collapse);
                        }

                        ui.add_space(4.0);

                        // Copy Markdown button
                        let copy_text = if let Some(t) = self.copied_notify_time {
                            if t.elapsed().as_secs() < 2 {
                                "Copied!"
                            } else {
                                "Copy"
                            }
                        } else {
                            "Copy"
                        };

                        let copy_frame = Frame::none()
                            .fill(CHIP_BG)
                            .stroke(Stroke::new(1.0, OUTLINE))
                            .rounding(14.0)
                            .inner_margin(Margin::symmetric(10.0, 4.0));
                        let copy_resp = copy_frame
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let (r, _) =
                                        ui.allocate_exact_size(vec2(12.0, 12.0), Sense::hover());
                                    paint_copy_icon(ui.painter(), r.center(), ON_SURFACE_DIM);
                                    ui.add_space(2.0);
                                    ui.label(
                                        RichText::new(copy_text).size(12.0).color(ON_SURFACE),
                                    );
                                });
                            })
                            .response;
                        let copy_interact = copy_resp.interact(Sense::click());
                        if copy_interact.hovered() {
                            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                        }
                        if copy_interact.clicked() {
                            triggered_action = Some(SummaryAction::Copy);
                        }

                        ui.add_space(4.0);

                        // [Ctrl+Enter] Update Pill (Highlighted if new speech is available to incorporate)
                        let update_bg = if has_new_speech {
                            TONAL_FIX_BG
                        } else {
                            CHIP_BG
                        };
                        let update_stroke = if has_new_speech {
                            TONAL_FIX_STROKE
                        } else {
                            OUTLINE
                        };
                        let update_label_color = if has_new_speech {
                            TONAL_FIX_TEXT
                        } else {
                            ON_SURFACE
                        };

                        let update_frame = Frame::none()
                            .fill(update_bg)
                            .stroke(Stroke::new(1.0, update_stroke))
                            .rounding(14.0)
                            .inner_margin(Margin::symmetric(10.0, 4.0));
                        let update_resp = update_frame
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let (r, _) =
                                        ui.allocate_exact_size(vec2(12.0, 12.0), Sense::hover());
                                    paint_refresh_icon(ui.painter(), r.center(), update_label_color);
                                    ui.add_space(2.0);
                                    ui.label(
                                        RichText::new("Ctrl+Enter")
                                            .color(KEY_PILL_TEXT)
                                            .strong()
                                            .size(11.0),
                                    );
                                    ui.label(
                                        RichText::new("Update")
                                            .color(update_label_color)
                                            .strong()
                                            .size(12.0),
                                    );
                                });
                            })
                            .response;
                        let update_interact = update_resp.interact(Sense::click());
                        if update_interact.hovered() && !self.is_generating {
                            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                        }
                        if update_interact.clicked() && !self.is_generating {
                            triggered_action = Some(SummaryAction::Update);
                        }
                    });
                });

                ui.add_space(10.0);

                // Scrollable Markdown Document Area
                let scroll_height = (ui.available_height() - 4.0).max(180.0);
                Frame::none()
                    .fill(SURFACE_EDITOR)
                    .stroke(Stroke::new(1.0, OUTLINE))
                    .rounding(14.0)
                    .inner_margin(14.0)
                    .show(ui, |ui| {
                        ScrollArea::vertical()
                            .id_salt("summary_markdown_scroll")
                            .auto_shrink([false, false])
                            .max_height(scroll_height)
                            .show(ui, |ui| {
                                if self.markdown_text.is_empty() {
                                    if self.is_generating {
                                        ui.vertical_centered(|ui| {
                                            ui.add_space(40.0);
                                            ui.label(
                                                RichText::new("Generating structured Markdown summary with LLaMA...")
                                                    .color(PRIMARY)
                                                    .size(14.0),
                                            );
                                        });
                                    } else {
                                        let ph_resp = ui.vertical_centered(|ui| {
                                            ui.add_space(40.0);
                                            ui.label(
                                                RichText::new("No summary yet. Dictate your thoughts and click here, click Summary, or press CTRL.")
                                                    .color(ON_SURFACE_DIM)
                                                    .size(13.0),
                                            );
                                        }).response;
                                        let ph_interact = ph_resp.interact(Sense::click());
                                        if ph_interact.hovered() && !self.is_generating {
                                            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                                        }
                                        if ph_interact.clicked() && !self.is_generating {
                                            triggered_action = Some(SummaryAction::Update);
                                        }
                                    }
                                } else {
                                    render_markdown_content(ui, &self.markdown_text);
                                }
                            });
                    });
            });

        triggered_action
    }
}

/// Renders formatted Markdown content into an egui Ui with headers, lists, code fences, and paragraphs.
pub fn render_markdown_content(ui: &mut Ui, markdown: &str) {
    let mut in_code_block = false;
    let mut code_buffer = String::new();

    for line in markdown.lines() {
        let trimmed = line.trim();

        // Handle fenced code blocks
        if trimmed.starts_with("```") {
            if in_code_block {
                // End code block
                Frame::none()
                    .fill(Color32::from_rgb(0x18, 0x16, 0x1d))
                    .stroke(Stroke::new(1.0, OUTLINE))
                    .rounding(8.0)
                    .inner_margin(10.0)
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(&code_buffer)
                                .text_style(TextStyle::Monospace)
                                .size(12.0)
                                .color(Color32::from_rgb(0xd8, 0xca, 0xf6)),
                        );
                    });
                code_buffer.clear();
                in_code_block = false;
            } else {
                in_code_block = true;
                code_buffer.clear();
            }
            continue;
        }

        if in_code_block {
            code_buffer.push_str(line);
            code_buffer.push('\n');
            continue;
        }

        // Empty line -> space
        if trimmed.is_empty() {
            ui.add_space(6.0);
            continue;
        }

        // Header 1: # Title
        if let Some(rest) = trimmed.strip_prefix("# ") {
            ui.add_space(6.0);
            ui.label(
                RichText::new(rest)
                    .color(PRIMARY)
                    .size(18.0)
                    .strong(),
            );
            // Subtle horizontal divider line
            let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
            ui.painter().line_segment([r.left_top(), r.right_top()], Stroke::new(1.0, OUTLINE));
            ui.add_space(4.0);
            continue;
        }

        // Header 2: ## Section
        if let Some(rest) = trimmed.strip_prefix("## ") {
            ui.add_space(5.0);
            ui.label(
                RichText::new(rest)
                    .color(Color32::from_rgb(0xfa, 0xf7, 0xfd))
                    .size(15.5)
                    .strong(),
            );
            ui.add_space(3.0);
            continue;
        }

        // Header 3: ### Subsection
        if let Some(rest) = trimmed.strip_prefix("### ") {
            ui.add_space(4.0);
            ui.label(
                RichText::new(rest)
                    .color(Color32::from_rgb(0xde, 0xd8, 0xe8))
                    .size(14.0)
                    .strong(),
            );
            ui.add_space(2.0);
            continue;
        }

        // Blockquote: > Quote
        if let Some(rest) = trimmed.strip_prefix("> ") {
            ui.horizontal(|ui| {
                let (bar_rect, _) = ui.allocate_exact_size(vec2(3.0, 16.0), Sense::hover());
                ui.painter().rect_filled(bar_rect, 1.5, PRIMARY);
                ui.add_space(4.0);
                ui.label(RichText::new(rest).italics().size(13.0).color(ON_SURFACE_DIM));
            });
            continue;
        }

        // Bullet point: - item or * item
        if let Some(rest) = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* ")) {
            ui.horizontal_wrapped(|ui| {
                ui.add_space(4.0);
                let (dot_rect, _) = ui.allocate_exact_size(vec2(10.0, 14.0), Sense::hover());
                ui.painter().circle_filled(pos2(dot_rect.center().x, dot_rect.center().y), 2.2, PRIMARY);
                render_inline_spans(ui, rest);
            });
            continue;
        }

        // Numbered list: 1. item
        if let Some(pos) = trimmed.find(". ") {
            let prefix = &trimmed[..pos];
            if prefix.chars().all(|c| c.is_ascii_digit()) {
                let rest = &trimmed[pos + 2..];
                ui.horizontal_wrapped(|ui| {
                    ui.add_space(4.0);
                    ui.label(RichText::new(format!("{}.", prefix)).strong().size(12.5).color(PRIMARY));
                    render_inline_spans(ui, rest);
                });
                continue;
            }
        }

        // Normal paragraph text
        ui.horizontal_wrapped(|ui| {
            render_inline_spans(ui, trimmed);
        });
    }
}

/// Parses and renders inline Markdown spans like **bold text** and normal text.
fn render_inline_spans(ui: &mut Ui, text: &str) {
    let mut remainder = text;

    while !remainder.is_empty() {
        if let Some(start) = remainder.find("**") {
            // Text before **
            let before = &remainder[..start];
            if !before.is_empty() {
                ui.label(RichText::new(before).color(ON_SURFACE).size(13.0));
            }

            // Find closing **
            let after_start = &remainder[start + 2..];
            if let Some(end) = after_start.find("**") {
                let bold_text = &after_start[..end];
                ui.label(RichText::new(bold_text).strong().color(Color32::from_rgb(0xff, 0xff, 0xff)).size(13.0));
                remainder = &after_start[end + 2..];
            } else {
                // No closing **, print the rest
                ui.label(RichText::new(&remainder[start..]).color(ON_SURFACE).size(13.0));
                break;
            }
        } else {
            ui.label(RichText::new(remainder).color(ON_SURFACE).size(13.0));
            break;
        }
    }
}
