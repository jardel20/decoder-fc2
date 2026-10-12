#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

fn main() -> eframe::Result {
    let startup = fc2_explorer::startup::parse(std::env::args_os().skip(1));
    if let Ok(fc2_explorer::startup::Startup::Tui(path)) = &startup {
        if let Err(error) = fc2_explorer::terminal::run(path.as_deref()) {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return Ok(());
    }
    let (initial, error) = match startup {
        Ok(fc2_explorer::startup::Startup::Gui(path)) => (path, None),
        Err(error) => (None, Some(error)),
        Ok(fc2_explorer::startup::Startup::Tui(_)) => unreachable!(),
    };
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("FC2 Explorer")
            .with_icon(
                eframe::icon_data::from_png_bytes(include_bytes!("../../assets/app_icon.png"))
                    .expect("embedded application icon must be valid PNG"),
            )
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([640.0, 460.0]),
        ..Default::default()
    };
    eframe::run_native(
        "fc2-explorer",
        options,
        Box::new(move |cc| {
            let mut app = fc2_explorer::app::ExplorerApp::new(cc, initial);
            if let Some(error) = error {
                app.state.error = Some(error);
            }
            Ok(Box::new(app))
        }),
    )
}
