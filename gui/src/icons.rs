//! Single source of truth for interface icons. Fonts are embedded by the crate.

use eframe::egui::{self, FontFamily, FontId, RichText, TextFormat, text::LayoutJob};
use egui_phosphor_icons::{Icon, icons};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)] // The complete action vocabulary is intentionally centralized.
pub enum Glyph {
    FolderOpen,
    Terminal,
    Folders,
    FileBinary,
    MapTrifold,
    MapPin,
    Table,
    MagnifyingGlass,
    Funnel,
    SortAscending,
    SortDescending,
    Export,
    GearSix,
    ArrowLeft,
    ArrowRight,
    ArrowsOut,
    ArrowsIn,
    CheckCircle,
    WarningCircle,
    Info,
    Sun,
    Moon,
    Plus,
    Minus,
    X,
}

impl Glyph {
    fn icon(self) -> Icon {
        match self {
            Self::FolderOpen => icons::FOLDER_OPEN,
            Self::Terminal => icons::TERMINAL_WINDOW,
            Self::Folders => icons::FOLDERS,
            // Phosphor 0.5 has BINARY, rather than FILE_BINARY.
            Self::FileBinary => icons::BINARY,
            Self::MapTrifold => icons::MAP_TRIFOLD,
            Self::MapPin => icons::MAP_PIN,
            Self::Table => icons::TABLE,
            Self::MagnifyingGlass => icons::MAGNIFYING_GLASS,
            Self::Funnel => icons::FUNNEL,
            Self::SortAscending => icons::SORT_ASCENDING,
            Self::SortDescending => icons::SORT_DESCENDING,
            Self::Export => icons::EXPORT,
            Self::GearSix => icons::GEAR_SIX,
            Self::ArrowLeft => icons::ARROW_LEFT,
            Self::ArrowRight => icons::ARROW_RIGHT,
            Self::ArrowsOut => icons::ARROWS_OUT,
            Self::ArrowsIn => icons::ARROWS_IN,
            Self::CheckCircle => icons::CHECK_CIRCLE,
            Self::WarningCircle => icons::WARNING_CIRCLE,
            Self::Info => icons::INFO,
            Self::Sun => icons::SUN,
            Self::Moon => icons::MOON,
            Self::Plus => icons::PLUS,
            Self::Minus => icons::MINUS,
            Self::X => icons::X,
        }
    }
}

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    egui_phosphor_icons::add_fonts(&mut fonts);
    ctx.set_fonts(fonts);
}

pub fn text(glyph: Glyph) -> RichText {
    glyph.icon().regular().size(super::theme::ICON_SIZE)
}

pub fn icon_button(ui: &mut egui::Ui, glyph: Glyph, label: &str) -> egui::Response {
    let response = ui.add(egui::Button::new(text(glyph)).min_size(eframe::egui::vec2(
        super::theme::CONTROL_HEIGHT,
        super::theme::CONTROL_HEIGHT,
    )));
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    response.on_hover_text(label)
}

/// One focusable, keyboard-accessible button containing an icon and visible text.
/// Each section uses its own font: ordinary words never use the icon font.
pub fn button(ui: &mut egui::Ui, glyph: Glyph, label: &str) -> egui::Response {
    let mut job = LayoutJob::default();
    job.append(
        text(glyph).text(),
        0.0,
        TextFormat {
            font_id: FontId::new(
                super::theme::ICON_SIZE,
                FontFamily::Name("phosphor-regular".into()),
            ),
            color: ui.visuals().text_color(),
            ..Default::default()
        },
    );
    job.append(
        label,
        super::theme::SPACE_SM,
        TextFormat {
            font_id: FontId::proportional(super::theme::BODY_SIZE),
            color: ui.visuals().text_color(),
            ..Default::default()
        },
    );
    ui.add(egui::Button::new(job)).on_hover_text(label)
}
