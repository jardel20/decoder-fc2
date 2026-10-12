//! Explicit startup modes: options must never become accidental file paths.
use std::{ffi::OsString, path::PathBuf};

#[derive(Debug, PartialEq, Eq)]
pub enum Startup {
    Gui(Option<PathBuf>),
    Tui(Option<PathBuf>),
}

pub fn parse(arguments: impl IntoIterator<Item = OsString>) -> Result<Startup, String> {
    let mut args = arguments.into_iter();
    let first = args.next();
    let tui = first.as_deref() == Some(std::ffi::OsStr::new("--tui"));
    let path = if tui || first.as_deref() == Some(std::ffi::OsStr::new("--")) {
        args.next()
    } else {
        first
    };
    if args.next().is_some() {
        return Err("Uso: fc2-explorer [arquivo.FC2] ou fc2-explorer --tui [arquivo.FC2]".into());
    }
    if path
        .as_ref()
        .is_some_and(|p| p.to_string_lossy().starts_with('-'))
    {
        return Err(
            "Opção desconhecida. Uso: fc2-explorer [arquivo.FC2] ou --tui [arquivo.FC2]".into(),
        );
    }
    let path = path.map(PathBuf::from);
    if let Some(path) = &path {
        let metadata = std::fs::metadata(path).map_err(|e| format!(
            "Arquivo inicial '{}': {e}. Confira o caminho usado no comando ou atalho, ou abra um FC2 pelo botão Abrir FC2.", path.display()
        ))?;
        if !metadata.is_file() {
            return Err(format!("'{}' não é um arquivo FC2.", path.display()));
        }
    }
    Ok(if tui {
        Startup::Tui(path)
    } else {
        Startup::Gui(path)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn startup_without_path_does_not_read_a_file() {
        assert_eq!(parse([]), Ok(Startup::Gui(None)));
        assert_eq!(parse(["--tui".into()]), Ok(Startup::Tui(None)));
        assert!(parse(["--unknown".into()]).is_err());
    }
    #[test]
    fn invalid_initial_path_reports_the_path() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("missing.FC2");
        assert!(
            parse([path.clone().into_os_string()])
                .unwrap_err()
                .contains(path.to_str().unwrap())
        );
        let path = directory.path().join("arquivo com espaços.FC2");
        std::fs::write(&path, []).unwrap();
        assert_eq!(
            parse([path.clone().into_os_string()]),
            Ok(Startup::Gui(Some(path)))
        );
    }
}
