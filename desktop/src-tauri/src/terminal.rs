//! Opens the existing TUI in a separate terminal by relaunching this executable.
//! Paths are passed as literal arguments, never interpreted by a shell.
use std::{
    path::{Path, PathBuf},
    process::Command,
};

#[cfg(not(target_os = "windows"))]
pub fn launch(path: Option<&Path>) -> Result<(), String> {
    let appimage = std::env::var_os("APPIMAGE");
    let exe = launcher_executable(appimage.clone())?;
    // The outer image restores its own libraries. A binary run from an extracted
    // image (for example appimage-run) still needs the library path of this process.
    let bundled_libraries = if appimage.is_some_and(|image| image == exe) {
        None
    } else {
        std::env::var_os("LD_LIBRARY_PATH")
    };
    let candidates = [
        ("x-terminal-emulator", "-e"),
        ("ptyxis", "--"),
        ("konsole", "-e"),
        ("gnome-terminal", "--"),
        ("kgx", "--"),
        ("kitty", "--"),
        ("alacritty", "-e"),
        ("foot", "-e"),
        ("tilix", "-e"),
        ("xfce4-terminal", "-x"),
        ("mate-terminal", "-x"),
        ("lxterminal", "-e"),
        ("terminator", "-x"),
        ("xterm", "-e"),
    ];
    for (terminal, separator) in candidates {
        let mut command = Command::new(terminal);
        leave_appimage_environment(&mut command);
        command.arg(separator);
        if let Some(libraries) = &bundled_libraries {
            let mut assignment = std::ffi::OsString::from("LD_LIBRARY_PATH=");
            assignment.push(libraries);
            command.arg("env").arg(assignment);
        }
        command.arg(&exe).arg("--tui");
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

/// Variables exported by the AppImage launcher for its own bundled GTK. A terminal
/// started from the app must not inherit them: it would load the bundled GTK
/// modules, schemas and theme instead of the system ones and fail to open.
#[cfg(not(target_os = "windows"))]
const APPIMAGE_VARIABLES: [&str; 13] = [
    "APPDIR",
    "ARGV0",
    "GTK_DATA_PREFIX",
    "GTK_THEME",
    "GTK_EXE_PREFIX",
    "GTK_PATH",
    "GTK_IM_MODULE_FILE",
    "GDK_PIXBUF_MODULE_FILE",
    "GSETTINGS_SCHEMA_DIR",
    "GI_TYPELIB_PATH",
    "GIO_MODULE_DIR",
    "GIO_EXTRA_MODULES",
    "LD_PRELOAD",
];

/// Path-list variables keep their entries, minus those inside the mounted image.
#[cfg(not(target_os = "windows"))]
const APPIMAGE_PATH_LISTS: [&str; 3] = ["LD_LIBRARY_PATH", "XDG_DATA_DIRS", "PATH"];

#[cfg(not(target_os = "windows"))]
fn leave_appimage_environment(command: &mut Command) {
    let Some(appdir) = std::env::var_os("APPDIR").filter(|v| !v.is_empty()) else {
        return;
    };
    for name in APPIMAGE_VARIABLES {
        command.env_remove(name);
    }
    let appdir = appdir.to_string_lossy().into_owned();
    for name in APPIMAGE_PATH_LISTS {
        if let Some(value) = std::env::var_os(name) {
            match without_entries_inside(&value.to_string_lossy(), &appdir) {
                Some(clean) => command.env(name, clean),
                None => command.env_remove(name),
            };
        }
    }
}

/// Drops the entries of a colon-separated list that live under `directory`.
#[cfg(not(target_os = "windows"))]
fn without_entries_inside(list: &str, directory: &str) -> Option<String> {
    let directory = directory.trim_end_matches('/');
    let kept: Vec<&str> = list
        .split(':')
        .filter(|entry| {
            let entry = entry.trim_end_matches('/');
            !entry.is_empty() && entry != directory && !entry.starts_with(&format!("{directory}/"))
        })
        .collect();
    (!kept.is_empty()).then(|| kept.join(":"))
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
    fn appimage_entries_are_removed_from_path_lists() {
        let list = "/tmp/.mount_x/usr/lib:/usr/lib:/tmp/.mount_x//usr/share:/tmp/.mount_xy/lib";
        assert_eq!(
            without_entries_inside(list, "/tmp/.mount_x").as_deref(),
            Some("/usr/lib:/tmp/.mount_xy/lib")
        );
        assert_eq!(
            without_entries_inside("/tmp/.mount_x/usr/lib", "/tmp/.mount_x"),
            None
        );
    }

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
