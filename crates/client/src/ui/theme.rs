use eframe::egui::{vec2, Color32, Context, Rounding};

// Surface & Card background tokens
pub const SURFACE: Color32 = Color32::from_rgb(0x13, 0x11, 0x16);
pub const SURFACE_CARD: Color32 = Color32::from_rgb(0x1c, 0x1a, 0x22);
pub const SURFACE_EDITOR: Color32 = Color32::from_rgb(0x12, 0x10, 0x15);
pub const OUTLINE: Color32 = Color32::from_rgb(0x2d, 0x2a, 0x36);
pub const CHIP_BG: Color32 = Color32::from_rgb(0x23, 0x20, 0x2b);

// Primary Accent & Button tokens
pub const PRIMARY: Color32 = Color32::from_rgb(0xbe, 0xa5, 0xf6);
pub const PRIMARY_BUTTON_BG: Color32 = Color32::from_rgb(0xd0, 0xbc, 0xff);
pub const PRIMARY_BUTTON_TEXT: Color32 = Color32::from_rgb(0x38, 0x1e, 0x72);
pub const PURPLE_ICON_BG: Color32 = Color32::from_rgb(0x4c, 0x3d, 0x7a);
pub const TAG_BG: Color32 = Color32::from_rgb(0x28, 0x25, 0x33);

// Fix Window / Tonal tokens
pub const TONAL_FIX_BG: Color32 = Color32::from_rgb(0x38, 0x31, 0x49);
pub const TONAL_FIX_STROKE: Color32 = Color32::from_rgb(0x56, 0x4e, 0x69);
pub const TONAL_FIX_TEXT: Color32 = Color32::from_rgb(0xfa, 0xf7, 0xfd);

// State tokens (Paused)
pub const PAUSED_BG: Color32 = Color32::from_rgb(0x2c, 0x28, 0x1e);
pub const PAUSED_STROKE: Color32 = Color32::from_rgb(0x57, 0x48, 0x22);
pub const PAUSED_TEXT: Color32 = Color32::from_rgb(0xe8, 0xd0, 0x7a);

// State tokens (Recording)
pub const REC_BG: Color32 = Color32::from_rgb(0x1a, 0x2b, 0x20);
pub const REC_STROKE: Color32 = Color32::from_rgb(0x2d, 0x5a, 0x38);
pub const REC_TEXT: Color32 = Color32::from_rgb(0x81, 0xc7, 0x84);

// Foreground text tokens
pub const ON_SURFACE: Color32 = Color32::from_rgb(0xe5, 0xe1, 0xe9);
pub const ON_SURFACE_DIM: Color32 = Color32::from_rgb(0x9a, 0x94, 0xa3);
pub const KEY_PILL_TEXT: Color32 = Color32::from_rgb(0xc9, 0xb8, 0xf5);
pub const ERROR_TEXT: Color32 = Color32::from_rgb(0xf2, 0xb8, 0xb5);

// Soundwave colors
pub const WAVE_ACTIVE: Color32 = Color32::from_rgb(0x7c, 0x68, 0xb0);
pub const WAVE_MUTED: Color32 = Color32::from_rgb(0x38, 0x33, 0x44);

/// Applies the M3 dark theme styling globally to an egui context.
pub fn apply_m3_style(ctx: &Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals.dark_mode = true;
    style.visuals.panel_fill = SURFACE;
    style.visuals.window_fill = SURFACE;
    style.visuals.window_rounding = Rounding::same(20.0);
    style.spacing.item_spacing = vec2(8.0, 8.0);
    ctx.set_style(style);
}
