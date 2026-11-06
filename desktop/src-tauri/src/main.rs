#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
fn main() {
    let mut args = std::env::args_os().skip(1);
    if args.next().is_some_and(|argument| argument == "--tui") {
        let path = args.next().map(std::path::PathBuf::from);
        if let Err(error) = fc2_desktop::terminal::run(path.as_deref()) {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    fc2_desktop::runtime::run();
}
