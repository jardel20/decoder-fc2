//! Shared visual tokens for both desktop themes.

use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle};

pub const SPACE_SM: f32 = 8.0;
pub const SPACE_MD: f32 = 12.0;
pub const BODY_SIZE: f32 = 14.0;
pub const ICON_SIZE: f32 = 20.0;
pub const CONTROL_HEIGHT: f32 = 32.0;
pub const ROW_HEIGHT: f32 = 30.0;
pub const CARD_RADIUS: u8 = 14;
pub const ACCENT: Color32 = Color32::from_rgb(38, 153, 126);
pub const MAP_SELECTED: Color32 = Color32::from_rgb(40, 104, 203);
pub const MAP_RELATED: Color32 = Color32::from_rgb(35, 152, 139);
pub const MAP_DEFAULT: Color32 = Color32::from_rgb(98, 126, 164);
pub const MAP_MARKER_BORDER: Color32 = Color32::WHITE;
const LIGHT_BG: Color32 = Color32::from_rgb(245, 247, 250);
const LIGHT_SURFACE: Color32 = Color32::WHITE;
const LIGHT_TEXT: Color32 = Color32::from_rgb(28, 38, 49);
const LIGHT_MUTED: Color32 = Color32::from_rgb(81, 96, 111);
const LIGHT_BORDER: Color32 = Color32::from_rgb(209, 217, 226);
const DARK_BG: Color32 = Color32::from_rgb(39, 41, 52);
const DARK_SURFACE: Color32 = Color32::from_rgb(33, 35, 44);
const DARK_TEXT: Color32 = Color32::from_rgb(247, 247, 245);
const DARK_MUTED: Color32 = Color32::from_rgb(174, 175, 183);
const DARK_BORDER: Color32 = Color32::from_rgb(58, 60, 71);

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
    style.animation_time = 0.16;
    style.spacing.window_margin = egui::Margin::same(18);
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
    style.visuals.selection.bg_fill = if dark {
        Color32::from_rgb(57, 59, 71)
    } else {
        Color32::from_rgb(224, 235, 232)
    };
    style.visuals.selection.stroke = Stroke::new(1.0, foreground);
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
        widget.corner_radius = CornerRadius::same(16);
        widget.fg_stroke.color = foreground;
        widget.expansion = 0.0;
    }
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, border);
    style.visuals.widgets.inactive.bg_fill = background;
    style.visuals.widgets.hovered.bg_fill = if dark {
        Color32::from_rgb(53, 55, 66)
    } else {
        Color32::from_rgb(235, 240, 239)
    };
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
    ctx.set_global_style(style);
}

pub fn statistic(ui: &mut egui::Ui, value: usize, label: &str) {
    egui::Frame::new()
        .fill(ui.visuals().window_fill)
        .corner_radius(CARD_RADIUS)
        .inner_margin(14)
        .show(ui, |ui| {
            ui.set_min_width((ui.available_width() - 2.0).max(0.0));
            ui.label(egui::RichText::new(value.to_string()).size(28.0).strong());
            ui.weak(label);
        });
}

pub fn tab(ui: &mut egui::Ui, selected: bool, label: &str) -> egui::Response {
    let intensity =
        ui.ctx()
            .animate_bool_with_time(ui.id().with(("tab-highlight", label)), selected, 0.16);
    let fill = {
        if ui.visuals().dark_mode {
            Color32::from_rgb(248, 248, 242)
        } else {
            Color32::from_rgb(33, 35, 44)
        }
    }
    .gamma_multiply(intensity);
    let color = if selected {
        if ui.visuals().dark_mode {
            Color32::from_rgb(52, 54, 65)
        } else {
            Color32::WHITE
        }
    } else {
        ui.visuals().weak_text_color()
    };
    ui.add(
        egui::Button::new(egui::RichText::new(label).color(color))
            .fill(fill)
            .stroke(Stroke::NONE)
            .corner_radius(20),
    )
}

/// Short fade that leaves controls usable and avoids moving the layout.
#[derive(Default)]
pub struct Transition {
    key: Option<egui::Id>,
    start: f64,
}
impl Transition {
    pub fn opacity(&mut self, ctx: &egui::Context, key: egui::Id) -> f32 {
        let time = ctx.input(|input| input.time);
        if self.key != Some(key) {
            self.key = Some(key);
            self.start = time;
        }
        let progress = ((time - self.start) / 0.18).clamp(0.0, 1.0) as f32;
        if progress < 1.0 {
            ctx.request_repaint();
        }
        0.82 + 0.18 * progress * progress * (3.0 - 2.0 * progress)
    }
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

/// Explorer columns share their full height, including the card border/margins.
pub fn panel<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let height = ui.available_height();
    card(ui, |ui| {
        ui.set_min_height((height - 26.0).max(0.0));
        ui.set_clip_rect(ui.clip_rect().intersect(ui.max_rect()));
        add(ui)
    })
}
