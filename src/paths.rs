//! Platform download locations and collision-free output names.
use std::path::{Path, PathBuf};

pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

/// Honor the user's configured location before falling back to Downloads/home.
/// Resolving a path never creates or modifies a directory.
pub fn downloads_dir() -> PathBuf {
    let home = home_dir();
    #[cfg(windows)]
    if let Some(path) = windows_downloads() {
        return path;
    }
    #[cfg(not(windows))]
    if let Some(home) = &home {
        let config = std::env::var_os("XDG_CONFIG_HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"));
        let configured = std::fs::read_to_string(config.join("user-dirs.dirs"))
            .ok()
            .and_then(|contents| xdg_downloads(&contents, home))
            .filter(|p| p.is_dir());
        if let Some(path) = configured {
            return path;
        }
    }
    if let Some(home) = home {
        for name in ["Downloads", "Download", "downloads"] {
            let path = home.join(name);
            if path.is_dir() {
                return path;
            }
        }
        if home.is_dir() {
            return home;
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

#[cfg(any(not(windows), test))]
fn xdg_downloads(contents: &str, home: &Path) -> Option<PathBuf> {
    for line in contents.lines() {
        let Some((key, value)) = line.trim().split_once('=') else {
            continue;
        };
        if key.trim() != "XDG_DOWNLOAD_DIR" {
            continue;
        }
        let value = value.trim().strip_prefix('"')?.strip_suffix('"')?;
        let path = if value == "$HOME" || value == "${HOME}" {
            home.to_owned()
        } else if let Some(suffix) = value
            .strip_prefix("$HOME/")
            .or_else(|| value.strip_prefix("${HOME}/"))
        {
            home.join(suffix)
        } else {
            // Never evaluate shell expressions from configuration.
            if value.contains('$') || value.contains('`') {
                return None;
            }
            PathBuf::from(value)
        };
        return path.is_absolute().then_some(path);
    }
    None
}

#[cfg(windows)]
fn windows_downloads() -> Option<PathBuf> {
    use std::os::windows::process::CommandExt;
    // Windows' built-in PowerShell reads the relocated known-folder registry
    // value and emits UTF-8, preserving accents without unsafe FFI or packages.
    let shell = PathBuf::from(std::env::var_os("SystemRoot")?)
        .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let output = std::process::Command::new(shell)
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command",
            "[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new(); $p = (Get-ItemProperty -LiteralPath 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\User Shell Folders' -ErrorAction SilentlyContinue).'{374DE290-123F-4565-9164-39C4925E467B}'; if ($p) { [Console]::Write([Environment]::ExpandEnvironmentVariables($p)) }"])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?;
    let path = PathBuf::from(value.trim_start_matches('\u{feff}').trim());
    (path.is_absolute() && path.is_dir()).then_some(path)
}

pub fn new_output(root: &Path, name: &str) -> PathBuf {
    let base = root.join(name);
    let mut candidate = base.clone();
    let mut count = 1usize;
    while candidate.symlink_metadata().is_ok() {
        candidate = root.join(format!("{name}-{count}"));
        count += 1;
    }
    candidate
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[cfg(not(windows))]
    fn xdg_localized_and_redirected_downloads_without_shell_evaluation() {
        let home = Path::new("/home/synthetic");
        assert_eq!(
            xdg_downloads("XDG_DOWNLOAD_DIR=\"$HOME/Baixados\"", home),
            Some(home.join("Baixados"))
        );
        assert_eq!(
            xdg_downloads("XDG_DOWNLOAD_DIR=\"/mnt/storage/Downloads\"", home),
            Some(PathBuf::from("/mnt/storage/Downloads"))
        );
        assert!(xdg_downloads("XDG_DOWNLOAD_DIR=\"$(touch dangerous)\"", home).is_none());
    }
    #[test]
    fn output_names_preserve_existing_files_and_directories() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("fc2-export"), "preserve").unwrap();
        std::fs::create_dir(root.path().join("fc2-export-1")).unwrap();
        assert_eq!(
            new_output(root.path(), "fc2-export"),
            root.path().join("fc2-export-2")
        );
        assert_eq!(
            std::fs::read_to_string(root.path().join("fc2-export")).unwrap(),
            "preserve"
        );
    }
}
