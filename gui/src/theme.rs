//! Shared visual tokens for both desktop themes.

use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle};

pub const SPACE_SM: f32 = 8.0;
pub const SPACE_MD: f32 = 12.0;
pub const BODY_SIZE: f32 = 14.0;
pub const ICON_SIZE: f32 = 20.0;
pub const CONTROL_HEIGHT: f32 = 32.0;
pub const ROW_HEIGHT: f32 = 30.0;
pub const CARD_RADIUS: u8 = 8;
pub const ACCENT: Color32 = Color32::from_rgb(34, 113, 143);
pub const MAP_SELECTED: Color32 = Color32::from_rgb(238, 150, 36);
pub const MAP_RELATED: Color32 = Color32::from_rgb(35, 152, 139);
pub const MAP_DEFAULT: Color32 = Color32::from_rgb(98, 126, 164);
pub const MAP_MARKER_BORDER: Color32 = Color32::WHITE;
const LIGHT_BG: Color32 = Color32::from_rgb(245, 247, 250);
const LIGHT_SURFACE: Color32 = Color32::WHITE;
const LIGHT_TEXT: Color32 = Color32::from_rgb(28, 38, 49);
const LIGHT_MUTED: Color32 = Color32::from_rgb(81, 96, 111);
const LIGHT_BORDER: Color32 = Color32::from_rgb(209, 217, 226);
const DARK_BG: Color32 = Color32::from_rgb(18, 24, 31);
const DARK_SURFACE: Color32 = Color32::from_rgb(26, 34, 43);
const DARK_TEXT: Color32 = Color32::from_rgb(230, 236, 242);
const DARK_MUTED: Color32 = Color32::from_rgb(168, 183, 198);
const DARK_BORDER: Color32 = Color32::from_rgb(66, 79, 94);

pub fn apply(ctx: &egui::Context, dark: bool) {
    ctx.set_theme(if dark {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    });
    let mut style = (*ctx.global_style()).clone();
    style.visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    let (background, surface, foreground, muted, border) = if dark {
        (DARK_BG, DARK_SURFACE, DARK_TEXT, DARK_MUTED, DARK_BORDER)
    } else {
        (
            LIGHT_BG,
            LIGHT_SURFACE,
            LIGHT_TEXT,
            LIGHT_MUTED,
            LIGHT_BORDER,
        )
    };
    style.spacing.item_spacing = egui::vec2(SPACE_SM, SPACE_SM);
    style.spacing.button_padding = egui::vec2(SPACE_MD, 6.0);
    style.spacing.interact_size.y = CONTROL_HEIGHT;
    style.spacing.window_margin = egui::Margin::same(12);
    style
        .text_styles
        .insert(TextStyle::Body, FontId::proportional(BODY_SIZE));
    style
        .text_styles
        .insert(TextStyle::Button, FontId::proportional(BODY_SIZE));
    style
        .text_styles
        .insert(TextStyle::Small, FontId::proportional(12.0));
    style
        .text_styles
        .insert(TextStyle::Heading, FontId::proportional(22.0));
    style.text_styles.insert(
        TextStyle::Monospace,
        FontId::new(13.0, FontFamily::Monospace),
    );
    style.visuals.override_text_color = Some(foreground);
    style.visuals.weak_text_color = Some(muted);
    style.visuals.panel_fill = background;
    style.visuals.window_fill = surface;
    style.visuals.extreme_bg_color = surface;
    style.visuals.window_corner_radius = CornerRadius::same(CARD_RADIUS);
    style.visuals.window_stroke = Stroke::new(1.0, border);
    style.visuals.selection.bg_fill = ACCENT;
    style.visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);
    style.visuals.hyperlink_color = if dark {
        Color32::from_rgb(102, 196, 225)
    } else {
        ACCENT
    };
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, border);
    for widget in [
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
        &mut style.visuals.widgets.open,
    ] {
        widget.corner_radius = CornerRadius::same(6);
        widget.fg_stroke.color = foreground;
        widget.expansion = 0.0;
    }
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, border);
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
    ctx.set_global_style(style);
}

pub fn card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::new()
        .fill(ui.visuals().window_fill)
        .stroke(ui.visuals().window_stroke)
        .corner_radius(CARD_RADIUS)
        .inner_margin(12)
        .show(ui, add)
        .inner
}
