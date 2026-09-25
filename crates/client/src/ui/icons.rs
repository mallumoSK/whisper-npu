use eframe::egui::{pos2, Color32, Painter, Pos2, Rect, Rounding, Shape, Stroke};

pub fn paint_waveform_icon(painter: &Painter, c: Pos2, color: Color32) {
    let stroke = Stroke::new(2.2, color);
    painter.line_segment([pos2(c.x - 7.0, c.y - 3.0), pos2(c.x - 7.0, c.y + 3.0)], stroke);
    painter.line_segment([pos2(c.x - 3.5, c.y - 7.0), pos2(c.x - 3.5, c.y + 7.0)], stroke);
    painter.line_segment([pos2(c.x, c.y - 10.0), pos2(c.x, c.y + 10.0)], stroke);
    painter.line_segment([pos2(c.x + 3.5, c.y - 6.0), pos2(c.x + 3.5, c.y + 6.0)], stroke);
    painter.line_segment([pos2(c.x + 7.0, c.y - 3.0), pos2(c.x + 7.0, c.y + 3.0)], stroke);
}

pub fn paint_enter_icon(painter: &Painter, c: Pos2, color: Color32) {
    let stroke = Stroke::new(1.8, color);
    painter.line_segment([pos2(c.x + 4.0, c.y - 3.5), pos2(c.x + 4.0, c.y + 2.0)], stroke);
    painter.line_segment([pos2(c.x + 4.0, c.y + 2.0), pos2(c.x - 3.5, c.y + 2.0)], stroke);
    painter.line_segment([pos2(c.x - 3.5, c.y + 2.0), pos2(c.x - 0.5, c.y - 1.0)], stroke);
    painter.line_segment([pos2(c.x - 3.5, c.y + 2.0), pos2(c.x - 0.5, c.y + 5.0)], stroke);
}

pub fn paint_space_icon(painter: &Painter, c: Pos2, color: Color32) {
    let stroke = Stroke::new(1.8, color);
    painter.line_segment([pos2(c.x - 4.5, c.y - 1.5), pos2(c.x - 4.5, c.y + 2.5)], stroke);
    painter.line_segment([pos2(c.x - 4.5, c.y + 2.5), pos2(c.x + 4.5, c.y + 2.5)], stroke);
    painter.line_segment([pos2(c.x + 4.5, c.y + 2.5), pos2(c.x + 4.5, c.y - 1.5)], stroke);
}

pub fn paint_esc_icon(painter: &Painter, c: Pos2, color: Color32) {
    let stroke = Stroke::new(1.5, color);
    painter.circle_stroke(c, 5.0, stroke);
    painter.line_segment([pos2(c.x - 3.5, c.y + 3.5), pos2(c.x + 3.5, c.y - 3.5)], stroke);
}

pub fn paint_trash_icon(painter: &Painter, c: Pos2, color: Color32) {
    let stroke = Stroke::new(1.4, color);
    painter.line_segment([pos2(c.x - 2.0, c.y - 5.5), pos2(c.x + 2.0, c.y - 5.5)], stroke);
    painter.line_segment([pos2(c.x - 5.0, c.y - 3.5), pos2(c.x + 5.0, c.y - 3.5)], stroke);
    let bin_rect = Rect::from_min_max(pos2(c.x - 4.0, c.y - 2.0), pos2(c.x + 4.0, c.y + 5.5));
    painter.rect_stroke(bin_rect, Rounding::same(1.5), stroke);
    painter.line_segment([pos2(c.x, c.y - 0.5), pos2(c.x, c.y + 4.0)], stroke);
}

pub fn paint_edit_icon(painter: &Painter, c: Pos2, color: Color32) {
    let stroke = Stroke::new(1.7, color);
    painter.line_segment([pos2(c.x - 4.5, c.y + 4.5), pos2(c.x + 4.5, c.y - 4.5)], stroke);
    painter.line_segment([pos2(c.x - 4.5, c.y + 4.5), pos2(c.x - 5.5, c.y + 2.5)], stroke);
    painter.line_segment([pos2(c.x - 4.5, c.y + 4.5), pos2(c.x - 2.5, c.y + 5.5)], stroke);
}

pub fn paint_wand_icon(painter: &Painter, c: Pos2, color: Color32) {
    let stroke = Stroke::new(1.8, color);
    painter.line_segment([pos2(c.x - 5.0, c.y + 5.0), pos2(c.x + 1.5, c.y - 1.5)], stroke);
    let sp_stroke = Stroke::new(1.2, color);
    painter.line_segment([pos2(c.x + 4.0, c.y - 6.5), pos2(c.x + 4.0, c.y - 2.5)], sp_stroke);
    painter.line_segment([pos2(c.x + 2.0, c.y - 4.5), pos2(c.x + 6.0, c.y - 4.5)], sp_stroke);
    painter.circle_filled(pos2(c.x - 1.0, c.y - 5.5), 1.0, color);
}

pub fn paint_play_icon(painter: &Painter, c: Pos2, color: Color32) {
    painter.add(Shape::convex_polygon(
        vec![
            pos2(c.x - 3.0, c.y - 4.5),
            pos2(c.x - 3.0, c.y + 4.5),
            pos2(c.x + 4.5, c.y),
        ],
        color,
        Stroke::NONE,
    ));
}

pub fn paint_pause_icon(painter: &Painter, c: Pos2, color: Color32) {
    let stroke = Stroke::new(2.2, color);
    painter.line_segment([pos2(c.x - 2.5, c.y - 4.0), pos2(c.x - 2.5, c.y + 4.0)], stroke);
    painter.line_segment([pos2(c.x + 2.5, c.y - 4.0), pos2(c.x + 2.5, c.y + 4.0)], stroke);
}

pub fn paint_paste_icon(painter: &Painter, c: Pos2, color: Color32) {
    let stroke = Stroke::new(1.8, color);
    painter.line_segment([pos2(c.x - 2.0, c.y - 6.5), pos2(c.x + 2.0, c.y - 6.5)], stroke);
    let doc_rect = Rect::from_min_max(pos2(c.x - 4.5, c.y - 4.5), pos2(c.x + 5.0, c.y + 6.0));
    painter.rect_stroke(doc_rect, Rounding::same(1.5), stroke);
    painter.line_segment([pos2(c.x - 7.5, c.y), pos2(c.x + 1.0, c.y)], stroke);
    painter.line_segment([pos2(c.x - 1.5, c.y - 2.5), pos2(c.x + 1.0, c.y)], stroke);
    painter.line_segment([pos2(c.x - 1.5, c.y + 2.5), pos2(c.x + 1.0, c.y)], stroke);
}

pub fn paint_check_icon(painter: &Painter, c: Pos2, color: Color32) {
    let stroke = Stroke::new(2.0, color);
    painter.line_segment([pos2(c.x - 4.5, c.y), pos2(c.x - 1.5, c.y + 3.5)], stroke);
    painter.line_segment([pos2(c.x - 1.5, c.y + 3.5), pos2(c.x + 5.0, c.y - 4.0)], stroke);
}

pub fn paint_close_icon(painter: &Painter, c: Pos2, color: Color32) {
    let stroke = Stroke::new(1.8, color);
    painter.line_segment([pos2(c.x - 4.0, c.y - 4.0), pos2(c.x + 4.0, c.y + 4.0)], stroke);
    painter.line_segment([pos2(c.x + 4.0, c.y - 4.0), pos2(c.x - 4.0, c.y + 4.0)], stroke);
}

pub fn paint_refresh_icon(painter: &Painter, c: Pos2, color: Color32) {
    let stroke = Stroke::new(1.8, color);
    painter.circle_stroke(c, 4.5, stroke);
    painter.line_segment([pos2(c.x + 2.0, c.y - 5.0), pos2(c.x + 4.5, c.y - 2.5)], stroke);
}
