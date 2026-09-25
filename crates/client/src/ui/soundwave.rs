use crate::ui::theme::{WAVE_ACTIVE, WAVE_MUTED};
use eframe::egui::{pos2, Painter, Pos2, Stroke};

/// Paints a centered soundwave visualizer with animated bars.
pub fn paint_soundwave(
    painter: &Painter,
    c: Pos2,
    time: f64,
    is_listening: bool,
    num_bars: usize,
    width_span: f32,
    spacing: f32,
) {
    let wave_color = if is_listening {
        WAVE_ACTIVE
    } else {
        WAVE_MUTED
    };

    let start_x = c.x - width_span;

    if num_bars == 9 {
        let heights: [f32; 9] = if is_listening {
            [
                4.0 + (time * 6.0).sin().abs() as f32 * 5.0,
                7.0 + (time * 7.5).sin().abs() as f32 * 8.0,
                10.0 + (time * 9.0).sin().abs() as f32 * 12.0,
                16.0 + (time * 11.0).sin().abs() as f32 * 10.0,
                22.0 + (time * 8.0).cos().abs() as f32 * 8.0,
                15.0 + (time * 10.0).sin().abs() as f32 * 11.0,
                9.0 + (time * 8.5).cos().abs() as f32 * 8.0,
                6.0 + (time * 7.0).sin().abs() as f32 * 6.0,
                4.0 + (time * 6.0).cos().abs() as f32 * 4.0,
            ]
        } else {
            [5.0, 8.0, 12.0, 18.0, 24.0, 18.0, 12.0, 8.0, 5.0]
        };

        let stroke = Stroke::new(3.5, wave_color);
        for (idx, &h) in heights.iter().enumerate() {
            let x = start_x + (idx as f32 * spacing);
            painter.line_segment([pos2(x, c.y - h * 0.5), pos2(x, c.y + h * 0.5)], stroke);
        }
    } else {
        // Fallback or 7-bar configuration (e.g. for auxiliary Fix window)
        let heights: [f32; 7] = if is_listening {
            [
                4.0 + (time * 6.0).sin().abs() as f32 * 4.0,
                7.0 + (time * 8.0).sin().abs() as f32 * 7.0,
                12.0 + (time * 10.0).sin().abs() as f32 * 8.0,
                18.0 + (time * 7.0).cos().abs() as f32 * 6.0,
                12.0 + (time * 9.0).sin().abs() as f32 * 8.0,
                7.0 + (time * 7.5).cos().abs() as f32 * 6.0,
                4.0 + (time * 6.0).cos().abs() as f32 * 4.0,
            ]
        } else {
            [4.0, 7.0, 12.0, 16.0, 12.0, 7.0, 4.0]
        };

        let stroke = Stroke::new(3.0, wave_color);
        for (idx, &h) in heights.iter().enumerate() {
            let x = start_x + (idx as f32 * spacing);
            painter.line_segment([pos2(x, c.y - h * 0.5), pos2(x, c.y + h * 0.5)], stroke);
        }
    }
}
