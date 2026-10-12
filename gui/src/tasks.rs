//! Native dialogs and expensive IO run off the render thread.
use crate::state::ExportChoice;
use eframe::egui;
use fc2_rs::{explorer::LoadedDataset, export::works::ExportMode};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
};

pub struct ExportResult {
    pub path: PathBuf,
    pub workbooks: usize,
    pub failures: usize,
    pub message: String,
}
pub enum Event {
    Loaded {
        id: u64,
        result: Result<Option<Arc<LoadedDataset>>, String>,
    },
    Exported {
        id: u64,
        result: Result<Option<ExportResult>, String>,
    },
    Stage {
        id: u64,
        text: String,
    },
}
pub struct Jobs {
    sender: Sender<Event>,
    receiver: Receiver<Event>,
    next: u64,
    cancel: Option<Arc<AtomicBool>>,
}
impl Default for Jobs {
    fn default() -> Self {
        Self::new()
    }
}
impl Jobs {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            sender,
            receiver,
            next: 0,
            cancel: None,
        }
    }
    fn id(&mut self) -> u64 {
        self.next = self.next.wrapping_add(1);
        self.next
    }
    pub fn drain(&mut self) -> Vec<Event> {
        self.receiver.try_iter().collect()
    }
    pub fn cancel_load(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            cancel.store(true, Ordering::Relaxed);
        }
    }
    pub fn open_file(&mut self, ctx: &egui::Context, path: Option<PathBuf>) -> u64 {
        self.cancel_load();
        let id = self.id();
        let sender = self.sender.clone();
        let ctx = ctx.clone();
        let cancel = Arc::new(AtomicBool::new(false));
        self.cancel = Some(cancel.clone());
        std::thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let path = if path.is_some() {
                    path
                } else {
                    crate::native_dialog::choose(|| {
                        rfd::FileDialog::new()
                            .set_title("Abrir arquivo Falker FC2")
                            .add_filter("Falker FC2", &["FC2", "fc2"])
                            .pick_file()
                    })?
                };
                let Some(path) = path else { return Ok(None) };
                if cancel.load(Ordering::Relaxed) {
                    return Ok(None);
                }
                let _ = sender.send(Event::Stage {
                    id,
                    text: "Lendo FC2 e verificando CRC/estruturas…".into(),
                });
                ctx.request_repaint();
                let loaded = LoadedDataset::load(&path).map_err(|e| e.to_string())?;
                if cancel.load(Ordering::Relaxed) {
                    return Ok(None);
                }
                Ok(Some(Arc::new(loaded)))
            }))
            .unwrap_or_else(|_| {
                Err(
                    "Falha interna na operação; confira o destino antes de tentar novamente."
                        .to_owned(),
                )
            });
            let _ = sender.send(Event::Loaded { id, result });
            ctx.request_repaint();
        });
        id
    }
    pub fn export(
        &mut self,
        ctx: &egui::Context,
        loaded: &Arc<LoadedDataset>,
        keys: Vec<usize>,
        choice: ExportChoice,
        root: PathBuf,
        destination: Option<PathBuf>,
    ) -> u64 {
        let id = self.id();
        let sender = self.sender.clone();
        let ctx = ctx.clone();
        let loaded = loaded.clone();
        std::thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if keys.is_empty() {
                    return Err("Selecione pelo menos um trabalho.".into());
                }
                let output = if let Some(destination) = destination {
                    Some(destination)
                } else if choice == ExportChoice::Json {
                    if keys.len() != 1 {
                        return Err("JSON de auditoria seleciona um trabalho por arquivo. Use completo para vários.".into());
                    }
                    let work = loaded.work(keys[0]).ok_or("Trabalho fora dos limites")?;
                    crate::native_dialog::choose(|| rfd::FileDialog::new()
                        .set_title("Salvar JSON de auditoria")
                        .set_directory(&root)
                        .set_file_name(format!("{}-auditoria.json", work.number))
                        .add_filter("JSON de auditoria", &["json"])
                        .save_file())?
                } else {
                    crate::native_dialog::choose(|| rfd::FileDialog::new()
                        .set_title("Selecionar pasta de exportação")
                        .set_directory(&root)
                        .pick_folder())?
                        .map(|root| {
                            let stem = loaded
                                .source_path
                                .file_stem()
                                .unwrap_or_default()
                                .to_string_lossy();
                            fc2_rs::paths::new_output(&root, &format!("{stem}-trabalhos"))
                        })
                };
                let Some(output) = output else {
                    return Ok(None);
                };
                let _ = sender.send(Event::Stage {
                    id,
                    text: "Validando origem e exportando dados preservados…".into(),
                });
                ctx.request_repaint();
                if choice == ExportChoice::Json {
                    loaded
                        .export_json(keys[0], &output)
                        .map_err(|e| e.to_string())?;
                    return Ok(Some(ExportResult {
                        path: output,
                        workbooks: 0,
                        failures: 0,
                        message:
                            "JSON de auditoria estrutural gravado; não implica equivalência Leaf."
                                .into(),
                    }));
                }
                let mode = if choice == ExportChoice::Complete {
                    ExportMode::Complete
                } else {
                    ExportMode::Simple
                };
                let report = loaded
                    .export(&keys, &output, mode)
                    .map_err(|e| e.to_string())?;
                let details = report
                    .works
                    .iter()
                    .filter_map(|work| work.error.as_ref().map(|e| format!("{}: {e}", work.number)))
                    .collect::<Vec<_>>()
                    .join("; ");
                Ok(Some(ExportResult {
                    path: output,
                    workbooks: report.successful_workbooks,
                    failures: report.failures,
                    message: if details.is_empty() {
                        format!(
                            "{} XLSX exportados; {} registros.",
                            report.successful_workbooks, report.record_count
                        )
                    } else {
                        format!(
                            "{} XLSX exportados; {} trabalhos bloqueados: {details}",
                            report.successful_workbooks, report.failures
                        )
                    },
                }))
            })).unwrap_or_else(|_| Err("Falha interna na operação; confira o destino antes de tentar novamente.".to_owned()));
            let _ = sender.send(Event::Exported { id, result });
            ctx.request_repaint();
        });
        id
    }
}
