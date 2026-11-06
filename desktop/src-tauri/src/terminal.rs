//! Opens the existing TUI in a separate terminal by relaunching this executable.
//! Paths are passed as literal arguments, never interpreted by a shell.
use std::{
    path::{Path, PathBuf},
    process::Command,
};

#[cfg(not(target_os = "windows"))]
pub fn launch(path: Option<&Path>) -> Result<(), String> {
    let exe = launcher_executable(std::env::var_os("APPIMAGE"))?;
    let candidates = [
        ("x-terminal-emulator", "-e"),
        ("konsole", "-e"),
        ("gnome-terminal", "--"),
        ("kgx", "--"),
        ("kitty", "--"),
        ("alacritty", "-e"),
        ("foot", "-e"),
        ("xterm", "-e"),
    ];
    for (terminal, separator) in candidates {
        let mut command = Command::new(terminal);
        command.arg(separator).arg(&exe).arg("--tui");
        if let Some(path) = path {
            command.arg(path);
        }
        match command.spawn() {
            Ok(_) => return Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(format!("Não foi possível abrir {terminal}: {error}")),
        }
    }
    Err(
        "Nenhum terminal compatível foi encontrado. Execute fc2-desktop --tui em um terminal."
            .into(),
    )
}

#[cfg(target_os = "windows")]
pub fn launch(_path: Option<&Path>) -> Result<(), String> {
    let _ = Command::new("cmd");
    Err("A TUI não está disponível a partir desta interface no Windows; use a CLI.".into())
}

/// Relaunch the outer AppImage when present so the TUI owns its own mount.
fn launcher_executable(appimage: Option<std::ffi::OsString>) -> Result<PathBuf, String> {
    #[cfg(target_os = "linux")]
    if let Some(path) = appimage.map(PathBuf::from)
        && path.is_absolute()
        && path.is_file()
    {
        return Ok(path);
    }
    #[cfg(not(target_os = "linux"))]
    let _ = appimage;
    std::env::current_exe().map_err(|error| error.to_string())
}

pub fn run(path: Option<&Path>) -> Result<(), String> {
    fc2_rs::tui::run(path, None).map_err(|error| error.to_string())
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    #[test]
    fn tui_relaunches_outer_appimage_and_falls_back_when_it_is_unavailable() {
        let directory = tempfile::tempdir().unwrap();
        let image = directory.path().join("Leaf Explorer.AppImage");
        std::fs::write(&image, b"synthetic launcher path").unwrap();
        assert_eq!(
            launcher_executable(Some(image.clone().into_os_string())).unwrap(),
            image
        );
        let missing = directory.path().join("missing.AppImage");
        assert_eq!(
            launcher_executable(Some(missing.into_os_string())).unwrap(),
            std::env::current_exe().unwrap()
        );
        assert_eq!(
            launcher_executable(None).unwrap(),
            std::env::current_exe().unwrap()
        );
    }
}
