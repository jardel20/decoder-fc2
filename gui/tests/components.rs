//! Actual egui headless passes; these complement, rather than replace, desktop checks.
use eframe::egui;
use fc2_explorer::{
    file_picker::{FilePicker, PickerMode},
    icons::{self, Glyph},
    maps::MapPane,
    theme,
};

#[test]
fn both_themes_render_embedded_icons_cards_and_empty_offline_map() {
    let ctx = egui::Context::default();
    icons::install_fonts(&ctx);
    let mut map = MapPane::new(&ctx);
    for dark in [false, true] {
        theme::apply(&ctx, dark);
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            theme::card(ui, |ui| {
                ui.label(icons::text(Glyph::FileBinary));
                let _ = icons::button(ui, Glyph::FolderOpen, "Abrir FC2");
            });
            assert!(map.show(ui, &[], None, None).is_none());
        });
        // This headless test has no GPU painter to upload or free textures.
        // Explicitly discard the deltas as required by epaint before assertions.
        output.textures_delta.clear();
        assert!(!output.shapes.is_empty());
        assert!(!map.online);
    }
}

#[test]
fn in_app_file_and_export_pickers_render_without_desktop_portals() {
    let directory = tempfile::tempdir().unwrap();
    let ctx = egui::Context::default();
    icons::install_fonts(&ctx);
    for mode in [PickerMode::Fc2, PickerMode::Folder, PickerMode::Json] {
        let mut picker =
            FilePicker::new(mode, directory.path().to_owned(), "audit.json".into(), &ctx);
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            assert!(picker.show(ui.ctx()).is_none());
        });
        output.textures_delta.clear();
        assert!(!output.shapes.is_empty());
    }
}
