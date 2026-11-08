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
pub fn launch(path: Option<&Path>) -> Result<(), String> {
    // The child allocates its own console, so no shell or companion binary is
    // needed. Paths remain literal arguments, never shell code.
    let mut command = Command::new(launcher_executable(None)?);
    command.arg("--tui");
    if let Some(path) = path {
        command.arg(path);
    }
    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Não foi possível abrir o terminal: {error}"))
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
    #[cfg(target_os = "windows")]
    windows_console::attach()?;
    fc2_rs::tui::run(path, None).map_err(|error| error.to_string())
}

// A Windows GUI-subsystem executable has no console by default. These two calls
// attach to the parent console or create one; no pointers or buffers cross FFI.
#[cfg(target_os = "windows")]
#[allow(unsafe_code)]
mod windows_console {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn AttachConsole(process_id: u32) -> i32;
        fn AllocConsole() -> i32;
    }
    pub fn attach() -> Result<(), String> {
        // SAFETY: ATTACH_PARENT_PROCESS is the documented sentinel; no pointers,
        // ownership or lifetimes are involved. Called before any stdio use.
        if unsafe { AttachConsole(u32::MAX) } == 0 {
            // ERROR_ACCESS_DENIED means the process already has a console.
            if std::io::Error::last_os_error().raw_os_error() == Some(5) {
                return Ok(());
            }
            // SAFETY: AllocConsole takes no arguments and the process has no console yet.
            if unsafe { AllocConsole() } == 0 {
                return Err(format!(
                    "Não foi possível abrir o terminal da TUI: {}",
                    std::io::Error::last_os_error()
                ));
            }
        }
        Ok(())
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    #[test]
    fn tui_relaunches_outer_appimage_and_falls_back_when_it_is_unavailable() {
        let directory = tempfile::tempdir().unwrap();
        let image = directory.path().join("FC2 Explorer.AppImage");
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
