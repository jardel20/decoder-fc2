#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

fn main() -> eframe::Result {
    let initial = std::env::args_os().nth(1).map(std::path::PathBuf::from);
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("FC2 Explorer")
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([640.0, 460.0]),
        ..Default::default()
    };
    eframe::run_native(
        "fc2-explorer",
        options,
        Box::new(move |cc| Ok(Box::new(fc2_explorer::app::ExplorerApp::new(cc, initial)))),
    )
}
