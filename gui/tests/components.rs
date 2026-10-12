//! Actual egui headless passes; these complement, rather than replace, desktop checks.
use eframe::egui;
use fc2_explorer::{
    file_picker::{FilePicker, PickerMode},
    icons::{self, Glyph},
    maps::MapPane,
    theme,
};

#[test]
fn application_background_fills_the_window_with_short_content() {
    let ctx = egui::Context::default();
    for dark in [true, false] {
        theme::apply(&ctx, dark);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 820.0),
            )),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            let expected = ui.available_rect_before_wrap();
            let frame = theme::application_frame(ui, |ui| {
                ui.label("Sem registros");
            });
            assert!((frame.response.rect.height() - expected.height()).abs() < 1.0);
            assert!((frame.response.rect.width() - expected.width()).abs() < 1.0);
        });
        output.textures_delta.clear();
    }
    let icon =
        eframe::icon_data::from_png_bytes(include_bytes!("../../assets/app_icon.png")).unwrap();
    assert_eq!((icon.width, icon.height), (64, 64));
}

#[test]
fn both_themes_render_embedded_icons_cards_and_empty_offline_map() {
    let ctx = egui::Context::default();
    icons::install_fonts(&ctx);
    let mut map = MapPane::new(&ctx);
    map.set_online(false);
    for dark in [false, true] {
        theme::apply(&ctx, dark);
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            theme::card(ui, |ui| {
                ui.label(icons::text(Glyph::FileBinary));
                let _ = icons::button(ui, Glyph::FolderOpen, "Abrir FC2");
            });
            assert!(map.show(ui, &[], None, None, None).is_none());
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

#[test]
fn explorer_columns_keep_aligned_bottoms_when_map_fills_its_panel() {
    let ctx = egui::Context::default();
    icons::install_fonts(&ctx);
    theme::apply(&ctx, true);
    let mut map = MapPane::new(&ctx);
    map.set_online(false);
    let mut rectangles = Vec::new();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(900.0, 400.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.horizontal_top(|ui| {
                    for index in 0..3 {
                        let response = ui.allocate_ui_with_layout(
                            egui::vec2(240.0, 350.0),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                theme::panel(ui, |ui| {
                                    if index == 1 {
                                        let _ = map.show(ui, &[], None, None, None);
                                    } else {
                                        ui.label("Painel");
                                    }
                                });
                            },
                        );
                        rectangles.push(response.response.rect);
                    }
                });
            },
        );
    });
    output.textures_delta.clear();
    assert_eq!(rectangles.len(), 3);
    for rectangle in &rectangles {
        assert!((rectangle.top() - rectangles[0].top()).abs() <= 1.0);
        assert!((rectangle.bottom() - rectangles[0].bottom()).abs() <= 1.0);
        assert!(rectangle.height() <= 351.0);
    }
}
