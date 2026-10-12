//! In-app filesystem selection, independent of desktop portals.
use crate::icons::{self, Glyph};
use eframe::egui;
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PickerMode {
    Fc2,
    Folder,
    Json,
}
struct Entry {
    path: PathBuf,
    directory: bool,
}
pub struct FilePicker {
    mode: PickerMode,
    directory: PathBuf,
    path_text: String,
    filename: String,
    search: String,
    entries: Vec<Entry>,
    selected: Option<PathBuf>,
    scan: Option<Receiver<Result<Vec<Entry>, String>>>,
    error: Option<String>,
}
pub enum PickResult {
    Selected(PathBuf),
    Cancelled,
}
impl FilePicker {
    pub fn new(
        mode: PickerMode,
        directory: PathBuf,
        filename: String,
        ctx: &egui::Context,
    ) -> Self {
        let mut picker = Self {
            mode,
            path_text: directory.to_string_lossy().into_owned(),
            directory,
            filename,
            search: String::new(),
            entries: Vec::new(),
            selected: None,
            scan: None,
            error: None,
        };
        picker.refresh(ctx);
        picker
    }
    fn refresh(&mut self, ctx: &egui::Context) {
        self.selected = None;
        self.entries.clear();
        self.error = None;
        let directory = self.directory.clone();
        let mode = self.mode;
        let ctx = ctx.clone();
        let (sender, receiver) = mpsc::channel();
        self.scan = Some(receiver);
        std::thread::spawn(move || {
            let result = scan_directory(&directory, mode);
            let _ = sender.send(result);
            ctx.request_repaint();
        });
    }
    fn navigate(&mut self, path: PathBuf, ctx: &egui::Context) {
        self.directory = path;
        self.path_text = self.directory.to_string_lossy().into_owned();
        self.search.clear();
        self.refresh(ctx);
    }
    fn chosen(&self) -> Result<PathBuf, String> {
        match self.mode {
            PickerMode::Fc2 => self
                .selected
                .clone()
                .ok_or_else(|| "Selecione um arquivo FC2.".into()),
            PickerMode::Folder => Ok(self.directory.clone()),
            PickerMode::Json => {
                let name = self.filename.trim();
                if !valid_filename(name) {
                    return Err("Informe apenas o nome do arquivo, sem pastas.".into());
                }
                Ok(self.directory.join(name))
            }
        }
    }
    pub fn show(&mut self, ctx: &egui::Context) -> Option<PickResult> {
        if let Some(scan) = &self.scan {
            match scan.try_recv() {
                Ok(result) => {
                    self.scan = None;
                    match result {
                        Ok(entries) => self.entries = entries,
                        Err(error) => self.error = Some(error),
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.scan = None;
                    self.error = Some("A leitura da pasta foi interrompida.".into());
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        let mut open = true;
        let mut result = None;
        let title = match self.mode {
            PickerMode::Fc2 => "Selecionar arquivo FC2",
            PickerMode::Folder => "Selecionar pasta de exportação",
            PickerMode::Json => "Salvar JSON de auditoria",
        };
        egui::Window::new(title)
            .id(egui::Id::new("filesystem-picker"))
            .open(&mut open)
            .default_size([680.0, 460.0])
            .resizable(true)
            .show(ctx, |ui| {
                ui.label("Navegue pelas pastas ou informe o caminho da pasta abaixo.");
                ui.horizontal(|ui| {
                    if icons::button(ui, Glyph::ArrowLeft, "Pasta acima").clicked()
                        && let Some(parent) = self.directory.parent()
                    {
                        self.navigate(parent.to_owned(), ctx);
                    }
                    if icons::button(ui, Glyph::FolderOpen, "Atualizar").clicked() {
                        self.refresh(ctx);
                    }
                });
                ui.horizontal(|ui| {
                    let edit = ui.text_edit_singleline(&mut self.path_text);
                    if icons::button(ui, Glyph::ArrowRight, "Ir").clicked()
                        || (edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                    {
                        self.navigate(PathBuf::from(&self.path_text), ctx);
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Pesquisar nesta pasta:");
                    ui.text_edit_singleline(&mut self.search);
                });
                if let Some(error) = &self.error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
                if self.scan.is_some() {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Lendo pasta…");
                    });
                }
                let query = self.search.to_lowercase();
                let visible: Vec<usize> = self
                    .entries
                    .iter()
                    .enumerate()
                    .filter_map(|(index, entry)| {
                        entry
                            .path
                            .file_name()
                            .is_some_and(|name| {
                                name.to_string_lossy().to_lowercase().contains(&query)
                            })
                            .then_some(index)
                    })
                    .collect();
                let mut navigate = None;
                egui::ScrollArea::vertical().max_height(280.0).show_rows(
                    ui,
                    26.0,
                    visible.len(),
                    |ui, range| {
                        for row in range {
                            let entry = &self.entries[visible[row]];
                            ui.horizontal(|ui| {
                                ui.label(icons::text(if entry.directory {
                                    Glyph::FolderOpen
                                } else {
                                    Glyph::FileBinary
                                }));
                                let label =
                                    entry.path.file_name().unwrap_or_default().to_string_lossy();
                                let response = ui.selectable_label(
                                    self.selected.as_ref() == Some(&entry.path),
                                    label,
                                );
                                if response.clicked() {
                                    self.selected = Some(entry.path.clone());
                                }
                                if response.double_clicked() {
                                    if entry.directory {
                                        navigate = Some(entry.path.clone());
                                    } else if self.mode == PickerMode::Fc2 {
                                        result = Some(PickResult::Selected(entry.path.clone()));
                                    }
                                }
                            });
                        }
                    },
                );
                if let Some(path) = navigate {
                    self.navigate(path, ctx);
                }
                if self.mode == PickerMode::Json {
                    ui.horizontal(|ui| {
                        ui.label("Nome do arquivo:");
                        ui.text_edit_singleline(&mut self.filename);
                    });
                }
                if self.mode == PickerMode::Folder {
                    ui.label(format!("Destino: {}", self.directory.display()));
                }
                ui.separator();
                ui.horizontal(|ui| {
                    let ready = self.scan.is_none()
                        && self.error.is_none()
                        && (self.mode != PickerMode::Json || valid_filename(self.filename.trim()))
                        && (self.mode != PickerMode::Fc2
                            || self.selected.as_ref().is_some_and(|path| {
                                self.entries
                                    .iter()
                                    .any(|entry| &entry.path == path && !entry.directory)
                            }));
                    ui.add_enabled_ui(ready, |ui| {
                        let label = if self.mode == PickerMode::Folder {
                            "Usar esta pasta"
                        } else {
                            "Selecionar"
                        };
                        if icons::button(ui, Glyph::CheckCircle, label).clicked() {
                            match self.chosen() {
                                Ok(path) => result = Some(PickResult::Selected(path)),
                                Err(error) => self.error = Some(error),
                            }
                        }
                    });
                    if icons::button(ui, Glyph::X, "Cancelar").clicked() {
                        result = Some(PickResult::Cancelled);
                    }
                });
            });
        if !open {
            result = Some(PickResult::Cancelled);
        }
        result
    }
}
fn valid_filename(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\\'])
}
fn scan_directory(directory: &std::path::Path, mode: PickerMode) -> Result<Vec<Entry>, String> {
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(directory)
        .map_err(|error| format!("Não foi possível ler {}: {error}", directory.display()))?
    {
        let entry = entry.map_err(|error| format!("Falha ao ler item da pasta: {error}"))?;
        let path = entry.path();
        let metadata = match std::fs::metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(format!(
                    "Não foi possível consultar {}: {error}",
                    path.display()
                ));
            }
        };
        let directory = metadata.is_dir();
        if directory
            || (mode == PickerMode::Fc2
                && metadata.is_file()
                && path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("fc2")))
        {
            entries.push(Entry { path, directory });
        }
    }
    entries.sort_by(|a, b| {
        b.directory
            .cmp(&a.directory)
            .then_with(|| a.path.cmp(&b.path))
    });
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fc2_filter_and_directory_mode_preserve_paths() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir(temp.path().join("folder")).unwrap();
        std::fs::write(temp.path().join("sample.FC2"), []).unwrap();
        std::fs::write(temp.path().join("other.txt"), []).unwrap();
        let entries = scan_directory(temp.path(), PickerMode::Fc2).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries[0].directory);
        assert_eq!(entries[1].path, temp.path().join("sample.FC2"));
        assert_eq!(
            scan_directory(temp.path(), PickerMode::Folder)
                .unwrap()
                .len(),
            1
        );
        assert!(scan_directory(&temp.path().join("missing"), PickerMode::Fc2).is_err());
    }
    #[test]
    fn save_name_cannot_escape_selected_directory() {
        for name in [
            "",
            ".",
            "..",
            "../audit.json",
            "C:\\audit.json",
            "/audit.json",
        ] {
            assert!(!valid_filename(name));
        }
        assert!(valid_filename("audit.json"));
    }
}
