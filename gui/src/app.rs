//! Native navigation and presentation; expensive operations belong to `Jobs`.
use crate::{
    file_picker::{FilePicker, PickResult, PickerMode},
    icons::{self, Glyph},
    maps::MapPane,
    state::{AppView, ExportChoice, RecordSort, State, WorkSort, WorkTab},
    tasks::{Event, Jobs},
    theme,
};
use eframe::egui::{self, Align, Layout, Sense};
use egui_extras::{Column, TableBuilder};
use fc2_rs::explorer::LoadedDataset;
use std::{path::PathBuf, sync::Arc};

enum PickerPurpose {
    Open,
    Export {
        loaded: Arc<LoadedDataset>,
        keys: Vec<usize>,
        choice: ExportChoice,
    },
}

pub struct ExplorerApp {
    pub state: State,
    map: MapPane,
    jobs: Jobs,
    pending_load: Option<u64>,
    cancel_requested: bool,
    pending_export: Option<u64>,
    work_points: Vec<fc2_rs::explorer::GeoPoint>,
    points_work: Option<usize>,
    picker: Option<(FilePicker, PickerPurpose)>,
}
impl ExplorerApp {
    pub fn new(cc: &eframe::CreationContext<'_>, initial: Option<PathBuf>) -> Self {
        icons::install_fonts(&cc.egui_ctx);
        theme::apply(&cc.egui_ctx, false);
        let mut app = Self {
            state: State::default(),
            map: MapPane::new(&cc.egui_ctx),
            jobs: Jobs::new(),
            pending_load: None,
            cancel_requested: false,
            pending_export: None,
            work_points: Vec::new(),
            points_work: None,
            picker: None,
        };
        if let Some(path) = initial {
            app.load(&cc.egui_ctx, Some(path));
        }
        app
    }
    fn load(&mut self, ctx: &egui::Context, path: Option<PathBuf>) {
        if self.pending_export.is_some() || self.pending_load.is_some() || self.picker.is_some() {
            return;
        }
        if cfg!(target_os = "linux") && path.is_none() {
            let directory = self
                .state
                .loaded
                .as_ref()
                .and_then(|loaded| loaded.source_path.parent().map(|path| path.to_owned()))
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")));
            self.picker = Some((
                FilePicker::new(PickerMode::Fc2, directory, String::new(), ctx),
                PickerPurpose::Open,
            ));
            self.state.status = "Selecione um FC2 no navegador de arquivos.".into();
            return;
        }
        self.pending_load = Some(self.jobs.open_file(ctx, path));
        self.cancel_requested = false;
        self.state.busy = Some("Selecionando arquivo FC2…".into());
        self.state.error = None;
    }
    fn events(&mut self) {
        for event in self.jobs.drain() {
            match event {
                Event::Stage { id, text }
                    if self.pending_load == Some(id) || self.pending_export == Some(id) =>
                {
                    if !self.cancel_requested || self.pending_export == Some(id) {
                        self.state.busy = Some(text);
                    }
                }
                Event::Loaded { id, result } if self.pending_load == Some(id) => {
                    self.pending_load = None;
                    self.state.busy = None;
                    if self.cancel_requested {
                        self.cancel_requested = false;
                        self.state.status =
                            "Carregamento cancelado; arquivo anterior preservado.".into();
                        continue;
                    }
                    match result {
                        Ok(Some(loaded)) => {
                            self.map.fit_points(&loaded.locations);
                            self.state.install(loaded);
                            self.points_work = None;
                            self.work_points.clear();
                        }
                        Ok(None) => self.state.status = "Abertura cancelada.".into(),
                        Err(error) => {
                            self.state.error = Some(error);
                            self.state.status = if self.state.loaded.is_some() {
                                "O arquivo anterior permanece disponível."
                            } else {
                                "O arquivo não foi carregado; consulte o erro acima."
                            }
                            .into();
                        }
                    }
                }
                Event::Exported { id, result } if self.pending_export == Some(id) => {
                    self.pending_export = None;
                    self.state.busy = None;
                    match result {
                        Ok(Some(report)) => {
                            if let Some(parent) = report.path.parent() {
                                self.state.export_root = parent.to_owned();
                            }
                            self.state.status =
                                format!("{} Destino: {}", report.message, report.path.display());
                            if report.failures > 0 {
                                self.state.error = Some(format!(
                                    "{} trabalhos não exportados. {}",
                                    report.failures, report.message
                                ));
                            }
                        }
                        Ok(None) => {
                            self.state.status = "Exportação cancelada antes da gravação.".into()
                        }
                        Err(error) => self.state.error = Some(error),
                    }
                }
                _ => {}
            }
        }
    }
    fn export(&mut self, ctx: &egui::Context, loaded: &Arc<LoadedDataset>, keys: Vec<usize>) {
        if self.state.busy.is_some() || self.picker.is_some() {
            return;
        }
        if cfg!(target_os = "linux") {
            if keys.is_empty() {
                self.state.error = Some("Selecione pelo menos um trabalho.".into());
                return;
            }
            let choice = self.state.export_choice;
            if choice == ExportChoice::Json && keys.len() != 1 {
                self.state.error =
                    Some("Selecione um trabalho para exportar JSON de auditoria.".into());
                return;
            }
            let mode = if choice == ExportChoice::Json {
                PickerMode::Json
            } else {
                PickerMode::Folder
            };
            let filename = loaded
                .work(keys[0])
                .map(|work| format!("{}-auditoria.json", work.number))
                .unwrap_or_default();
            self.picker = Some((
                FilePicker::new(mode, self.state.export_root.clone(), filename, ctx),
                PickerPurpose::Export {
                    loaded: loaded.clone(),
                    keys,
                    choice,
                },
            ));
            return;
        }
        self.pending_export = Some(self.jobs.export(
            ctx,
            loaded,
            keys,
            self.state.export_choice,
            self.state.export_root.clone(),
            None,
        ));
        self.state.busy = Some("Escolha o destino da exportação…".into());
        self.state.error = None;
    }
    fn file_picker(&mut self, ctx: &egui::Context) {
        let result = self
            .picker
            .as_mut()
            .and_then(|(picker, _)| picker.show(ctx));
        let Some(result) = result else { return };
        let Some((_, purpose)) = self.picker.take() else {
            return;
        };
        match (result, purpose) {
            (PickResult::Cancelled, _) => self.state.status = "Seleção cancelada.".into(),
            (PickResult::Selected(path), PickerPurpose::Open) => self.load(ctx, Some(path)),
            (
                PickResult::Selected(path),
                PickerPurpose::Export {
                    loaded,
                    keys,
                    choice,
                },
            ) => {
                let destination = if choice == ExportChoice::Json {
                    path
                } else {
                    let stem = loaded
                        .source_path
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy();
                    fc2_rs::paths::new_output(&path, &format!("{stem}-trabalhos"))
                };
                self.pending_export = Some(self.jobs.export(
                    ctx,
                    &loaded,
                    keys,
                    choice,
                    self.state.export_root.clone(),
                    Some(destination),
                ));
                self.state.busy = Some("Validando origem e exportando dados preservados…".into());
                self.state.error = None;
            }
        }
    }
    fn header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if matches!(self.state.view, AppView::WorkDetail { .. })
                && icons::button(ui, Glyph::ArrowLeft, "Voltar").clicked()
            {
                self.state.back();
            }
            ui.heading("FC2 Explorer");
            ui.weak(format!("v{}", env!("CARGO_PKG_VERSION")));
            ui.add_enabled_ui(
                self.pending_export.is_none()
                    && self.pending_load.is_none()
                    && self.picker.is_none(),
                |ui| {
                    if icons::button(ui, Glyph::FolderOpen, "Abrir FC2").clicked() {
                        self.load(ui.ctx(), None);
                    }
                },
            );
            if icons::button(ui, Glyph::GearSix, "Configurações").clicked() {
                self.state.settings_open = !self.state.settings_open;
            }
        });
        if let Some(loaded) = &self.state.loaded {
            ui.horizontal_wrapped(|ui| {
                ui.label(icons::text(Glyph::FileBinary));
                ui.label(
                    loaded
                        .source_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy(),
                );
                ui.separator();
                ui.label(format!(
                    "{} trabalhos • {} registros • {} GPS",
                    loaded.dataset.works.len(),
                    loaded.summaries.iter().map(|s| s.records).sum::<usize>(),
                    loaded.locations.len()
                ));
                ui.label(format!(
                    "CRC: {} setores conferidos",
                    loaded.dataset.filesystem.crc_verified_count
                ));
            });
        }
        ui.separator();
    }
    fn settings(&mut self, ctx: &egui::Context) {
        let mut open = self.state.settings_open;
        egui::Window::new("Configurações").open(&mut open).resizable(false).show(ctx,|ui| {
            ui.horizontal(|ui| {ui.label(icons::text(if self.state.dark {Glyph::Moon}else{Glyph::Sun})); if ui.checkbox(&mut self.state.dark,"Tema escuro").changed() {theme::apply(ctx,self.state.dark);}});
            let mut online=self.map.online;
            if ui.checkbox(&mut online,"Consultar tiles OpenStreetMap").changed(){self.map.set_online(online);}
            ui.label("A consulta cartográfica envia ao provedor a região visível. Os dados FC2 não são enviados.");
            ui.label("Sem conexão, a camada local de pontos continua disponível.");
            ui.separator(); ui.label("Destino inicial de exportação:");
            ui.monospace(self.state.export_root.display().to_string());
            ui.small("O diálogo de exportação permite escolher outra pasta. Downloads é o padrão do sistema.");
            ui.small("Os índices instrumentais não representam concentração em massa. CRC confirma integridade estrutural, não equivalência científica.");
            ui.collapsing("Licenças incorporadas", |ui| {
                egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                    ui.small(include_str!("../../LICENSE"));
                    ui.small(include_str!("../../licenses/Phosphor-Icons-MIT.txt"));
                });
            });
        });
        self.state.settings_open = open;
    }
    fn explorer(&mut self, ui: &mut egui::Ui, loaded: &Arc<LoadedDataset>) {
        let size = ui.available_size();
        let compact = size.x < 900.0;
        ui.horizontal_top(|ui| {
            let left = if compact { size.x.min(230.0) } else { 250.0 };
            ui.allocate_ui_with_layout(
                egui::vec2(left, size.y),
                Layout::top_down(Align::Min),
                |ui| self.works(ui, loaded),
            );
            ui.separator();
            let right = if compact { 0.0 } else { 260.0 };
            let width = (size.x - left - right - 32.0).max(100.0);
            ui.allocate_ui_with_layout(
                egui::vec2(width, size.y),
                Layout::top_down(Align::Min),
                |ui| {
                    if compact {
                        self.work_details(ui, loaded, true);
                        ui.separator();
                    }
                    if let Some(key) = self.map.show(
                        ui,
                        &loaded.locations,
                        self.state.selected_work,
                        self.state.selected_record,
                    ) {
                        self.state.select_record(key);
                    }
                },
            );
            if !compact {
                ui.separator();
                ui.allocate_ui_with_layout(
                    egui::vec2(right, size.y),
                    Layout::top_down(Align::Min),
                    |ui| self.work_details(ui, loaded, false),
                );
            }
        });
    }
    fn works(&mut self, ui: &mut egui::Ui, loaded: &Arc<LoadedDataset>) {
        ui.horizontal(|ui| {
            ui.label(icons::text(Glyph::Folders));
            ui.heading("Trabalhos");
        });
        ui.add(
            egui::TextEdit::singleline(&mut self.state.work_query)
                .hint_text("Nome ou identificador")
                .desired_width(f32::INFINITY),
        );
        egui::ComboBox::from_id_salt("work-sort")
            .selected_text(match self.state.work_sort {
                WorkSort::Number => "Ordenar: ID",
                WorkSort::Name => "Ordenar: nome",
                WorkSort::Records => "Ordenar: registros",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.state.work_sort, WorkSort::Number, "Identificador");
                ui.selectable_value(&mut self.state.work_sort, WorkSort::Name, "Nome");
                ui.selectable_value(&mut self.state.work_sort, WorkSort::Records, "Registros");
            });
        ui.checkbox(&mut self.state.work_descending, "Decrescente");
        ui.small("Espaço marca a seleção; duplo clique abre.");
        self.state.refresh_works();
        if !ui.ctx().text_edit_focused()
            && ui.input(|i| i.key_pressed(egui::Key::Space))
            && let Some(work) = self.state.selected_work
            && !self.state.marked_works.remove(&work)
        {
            self.state.marked_works.insert(work);
        }
        let count = self.state.visible_works.len();
        egui::ScrollArea::vertical()
            .id_salt("works")
            .max_height((ui.available_height() - 118.0).max(60.0))
            .show_rows(ui, 56.0, count, |ui, range| {
                for row in range {
                    let index = self.state.visible_works[row];
                    let work = &loaded.dataset.works[index];
                    ui.push_id(index, |ui| {
                        ui.horizontal(|ui| {
                            let mut marked = self.state.marked_works.contains(&index);
                            if ui.checkbox(&mut marked, "").changed() {
                                if marked {
                                    self.state.marked_works.insert(index);
                                } else {
                                    self.state.marked_works.remove(&index);
                                }
                            }
                            let response = ui.selectable_label(
                                self.state.selected_work == Some(index),
                                format!(
                                    "{} · {}\n{} registros · {} grupos",
                                    work.number,
                                    work.display_name,
                                    loaded.summaries[index].records,
                                    loaded.summaries[index].groups
                                ),
                            );
                            if response.clicked() {
                                self.state.select_work(index);
                            }
                            if response.double_clicked() {
                                self.state.open_work(index);
                            }
                        });
                    });
                }
            });
        ui.small(format!(
            "{count} visíveis • {} marcados",
            self.state.marked_works.len()
        ));
        egui::ComboBox::from_id_salt("marked-export-mode")
            .selected_text(match self.state.export_choice {
                ExportChoice::Xlsx => "Simples: XLSX",
                ExportChoice::Complete => "Completo: XLSX + auditoria",
                ExportChoice::Json => "JSON de auditoria (um trabalho)",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut self.state.export_choice,
                    ExportChoice::Xlsx,
                    "Simples: XLSX",
                );
                ui.selectable_value(
                    &mut self.state.export_choice,
                    ExportChoice::Complete,
                    "Completo: XLSX + auditoria",
                );
                ui.selectable_value(
                    &mut self.state.export_choice,
                    ExportChoice::Json,
                    "JSON de auditoria (um trabalho)",
                );
            });
        ui.add_enabled_ui(
            !self.state.marked_works.is_empty() && self.state.busy.is_none(),
            |ui| {
                if icons::button(ui, Glyph::Export, "Exportar marcados").clicked() {
                    self.export(
                        ui.ctx(),
                        loaded,
                        self.state.marked_works.iter().copied().collect(),
                    );
                }
            },
        );
    }
    fn work_details(&mut self, ui: &mut egui::Ui, loaded: &Arc<LoadedDataset>, compact: bool) {
        let Some(index) = self.state.selected_work else {
            ui.label("Selecione um trabalho.");
            return;
        };
        let Some(work) = loaded.work(index) else {
            return;
        };
        let summary = loaded.summaries[index];
        ui.heading(&work.display_name);
        ui.label(format!(
            "ID {} • {} registros • {} grupos",
            work.number, summary.records, summary.groups
        ));
        if !compact {
            ui.label(format!(
                "{} coordenadas numéricas • {} registros com avisos",
                summary.coordinate_records, summary.warned_records
            ));
            ui.label(if work.errors.is_empty() {
                "Estrutura interpretada"
            } else {
                "Estrutura com erros; exportação pode ser bloqueada"
            });
            ui.small("Equivalência Leaf não presumida. Datum GPS não confirmado.");
            ui.monospace(&work.directory);
            if let Some(key) = self.state.selected_record
                && let Some(record) = loaded.record(key)
            {
                ui.separator();
                ui.label(format!(
                    "Registro {} • grupo {}",
                    record.id, work.groups[key.group].group
                ));
                coordinates(ui, record.latitude, record.longitude);
            } else if let Some(point) = loaded
                .locations
                .iter()
                .find(|point| point.key.work == index)
            {
                ui.separator();
                ui.small("Primeira coordenada disponível na ordem do arquivo:");
                coordinates(ui, Some(point.latitude), Some(point.longitude));
                if icons::button(ui, Glyph::MapPin, "Centralizar no mapa").clicked() {
                    self.map.center_on(*point);
                }
            }
            for error in &work.errors {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
            for warning in work.warnings.iter().take(4) {
                ui.small(warning);
            }
        }
        if icons::button(ui, Glyph::ArrowRight, "Abrir trabalho").clicked() {
            self.state.open_work(index);
        }
    }
    fn detail(&mut self, ui: &mut egui::Ui, loaded: &Arc<LoadedDataset>, work_index: usize) {
        let Some(work) = loaded.work(work_index) else {
            self.state.back();
            return;
        };
        ui.heading(&work.display_name);
        let summary = loaded.summaries[work_index];
        ui.label(format!(
            "ID {} • {} registros • {} grupos • {} GPS • {} avisos em registros",
            work.number,
            summary.records,
            summary.groups,
            summary.coordinate_records,
            summary.warned_records
        ));
        ui.horizontal_wrapped(|ui| {
            for (tab, glyph, label) in [
                (WorkTab::Measurements, Glyph::Table, "Medições"),
                (WorkTab::Map, Glyph::MapTrifold, "Mapa"),
                (WorkTab::Metadata, Glyph::Info, "Metadados"),
                (WorkTab::Export, Glyph::Export, "Exportação"),
            ] {
                ui.label(icons::text(glyph));
                ui.selectable_value(&mut self.state.tab, tab, label);
            }
        });
        ui.separator();
        match self.state.tab {
            WorkTab::Measurements => self.measurements(ui, loaded, work_index),
            WorkTab::Map => {
                if self.points_work != Some(work_index) {
                    self.work_points = loaded
                        .locations
                        .iter()
                        .filter(|p| p.key.work == work_index)
                        .copied()
                        .collect();
                    self.points_work = Some(work_index);
                }
                if let Some(key) = self.map.show(
                    ui,
                    &self.work_points,
                    Some(work_index),
                    self.state.selected_record,
                ) {
                    self.state.select_record(key);
                }
            }
            WorkTab::Metadata => metadata(ui, loaded, work_index),
            WorkTab::Export => self.export_view(ui, loaded, work_index),
        }
    }
    fn measurements(&mut self, ui: &mut egui::Ui, loaded: &LoadedDataset, work: usize) {
        if let Some(key) = self.state.selected_record.filter(|key| key.work == work) {
            egui::CollapsingHeader::new("Registro selecionado: origem, coordenadas e bytes")
                .id_salt(("record-inspector", work))
                .show(ui, |ui| {
                    if let Some(record) = loaded.record(key) {
                        record_inspector(ui, record);
                        if let Some(point) = loaded.locations.iter().find(|p| p.key == key)
                            && icons::button(ui, Glyph::MapPin, "Mostrar no mapa").clicked()
                        {
                            self.map.center_on(*point);
                            self.state.tab = WorkTab::Map;
                        }
                    }
                });
        }
        let table = self.state.tables.entry(work).or_default();
        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut table.query).hint_text("ID, ponto, data ou hora"),
            );
            egui::ComboBox::from_id_salt(("group-filter", work))
                .selected_text(
                    table
                        .group
                        .map(|g| format!("Grupo {}", loaded.dataset.works[work].groups[g].group))
                        .unwrap_or("Todos os grupos".into()),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut table.group, None, "Todos os grupos");
                    for (index, group) in loaded.dataset.works[work].groups.iter().enumerate() {
                        ui.selectable_value(
                            &mut table.group,
                            Some(index),
                            format!("Grupo {}", group.group),
                        );
                    }
                });
            ui.checkbox(&mut table.warnings_only, "Somente avisos");
        });
        table.refresh(loaded, work);
        ui.small(format!(
            "{} registros exibidos. Índices instrumentais em décimos; valores ausentes: —.",
            table.rows.len()
        ));
        let columns = [
            ("ID", RecordSort::Id),
            ("Grupo", RecordSort::Group),
            ("Ponto", RecordSort::Point),
            ("Data / hora", RecordSort::Date),
            ("Índice a", RecordSort::A),
            ("Índice b", RecordSort::B),
            ("Total", RecordSort::Total),
        ];
        let mut selected = None;
        egui::ScrollArea::horizontal()
            .id_salt(("records-horizontal", work))
            .show(ui, |ui| {
                TableBuilder::new(ui)
                    .id_salt(("records", work))
                    .striped(true)
                    .resizable(true)
                    .sense(Sense::click())
                    .column(Column::initial(70.0))
                    .column(Column::initial(65.0))
                    .column(Column::initial(65.0))
                    .column(Column::initial(160.0))
                    .columns(Column::initial(80.0), 3)
                    .column(Column::remainder().at_least(80.0))
                    .header(theme::ROW_HEIGHT, |mut header| {
                        for (label, sort) in columns {
                            header.col(|ui| {
                                if ui.selectable_label(table.sort == sort, label).clicked() {
                                    if table.sort == sort {
                                        table.descending = !table.descending;
                                    } else {
                                        table.sort = sort;
                                        table.descending = false;
                                    }
                                }
                            });
                        }
                        header.col(|ui| {
                            ui.strong("Avisos");
                        });
                    })
                    .body(|body| {
                        body.rows(theme::ROW_HEIGHT, table.rows.len(), |mut row| {
                            let key = table.rows[row.index()];
                            let Some(record) = loaded.record(key) else {
                                return;
                            };
                            row.set_selected(self.state.selected_record == Some(key));
                            row.col(|ui| {
                                ui.label(record.id.to_string());
                            });
                            row.col(|ui| {
                                ui.label(
                                    loaded.dataset.works[work].groups[key.group]
                                        .group
                                        .to_string(),
                                );
                            });
                            row.col(|ui| {
                                ui.label(record.point.to_string());
                            });
                            row.col(|ui| {
                                ui.label(format!(
                                    "{} {}",
                                    record.date.as_deref().unwrap_or("—"),
                                    record.time.as_deref().unwrap_or("—")
                                ));
                            });
                            for value in [
                                &record.chlorophyll_a_index,
                                &record.chlorophyll_b_index,
                                &record.chlorophyll_total_index,
                            ] {
                                row.col(|ui| {
                                    ui.label(value);
                                });
                            }
                            row.col(|ui| {
                                ui.label(record.warnings.len().to_string())
                                    .on_hover_text(record.warnings.join("\n"));
                            });
                            if row.response().clicked() {
                                selected = Some(key);
                            }
                        });
                    });
            });
        if let Some(key) = selected {
            self.state.select_record(key);
        }
    }
    fn export_view(&mut self, ui: &mut egui::Ui, loaded: &Arc<LoadedDataset>, work: usize) {
        theme::card(ui, |ui| {
            ui.heading("Exportar trabalho");
            ui.radio_value(
                &mut self.state.export_choice,
                ExportChoice::Xlsx,
                "Simples — somente XLSX (padrão)",
            );
            ui.radio_value(
                &mut self.state.export_choice,
                ExportChoice::Complete,
                "Completo — XLSX, auditoria e bytes preservados",
            );
            ui.radio_value(
                &mut self.state.export_choice,
                ExportChoice::Json,
                "JSON — auditoria estrutural do trabalho",
            );
            ui.separator();
            ui.label("Destino inicial:");
            ui.monospace(self.state.export_root.display().to_string());
            ui.label("Ao exportar, escolha uma pasta no diálogo do sistema. XLSX usa uma subpasta nova; JSON permite escolher o arquivo.");
            ui.small("CSV científico exige um perfil de referência validado; não está disponibilizado nesta GUI sem essa referência.");
            ui.add_enabled_ui(self.state.busy.is_none(), |ui| {
                if icons::button(ui, Glyph::Export, "Escolher destino e exportar").clicked() {
                    self.export(ui.ctx(), loaded, vec![work]);
                }
            });
        });
    }
}
impl eframe::App for ExplorerApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.events();
        if self.pending_export.is_some() && ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.state.status = "Aguarde a exportação terminar antes de fechar a aplicação.".into();
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.events();
        let ctx = ui.ctx().clone();
        self.file_picker(&ctx);
        if let Some(path) =
            ctx.input(|i| i.raw.dropped_files.first().map(|f| f.path().to_path_buf()))
        {
            self.load(&ctx, Some(path));
        }
        egui::Frame::central_panel(ui.style()).show(ui,|ui|{
            self.header(ui);
            if let Some(error)=self.state.error.clone(){ui.horizontal_wrapped(|ui|{ui.colored_label(ui.visuals().error_fg_color,&error);if icons::button(ui,Glyph::X,"Dispensar").clicked(){self.state.error=None;}});}
            if let Some(busy)=self.state.busy.clone(){ui.horizontal(|ui|{ui.spinner();ui.label(busy);if self.pending_load.is_some()&&!self.cancel_requested&&icons::button(ui,Glyph::X,"Cancelar leitura").clicked(){self.jobs.cancel_load();self.cancel_requested=true;self.state.busy=Some("Cancelamento solicitado; aguarde a leitura ou feche o diálogo aberto.".into());}});}
            let content=egui::vec2(ui.available_width(),(ui.available_height()-58.0).max(80.0));
            ui.allocate_ui_with_layout(content,Layout::top_down(Align::Min),|ui|{
                if let Some(loaded)=self.state.loaded.clone(){match self.state.view{AppView::Explorer=>self.explorer(ui,&loaded),AppView::WorkDetail{work}=>self.detail(ui,&loaded,work)}}else{
                    theme::card(ui,|ui|{ui.heading("Explore os trabalhos do seu FC2");ui.label("Abra um arquivo ou arraste-o para esta janela. A leitura preserva o original e utiliza o decoder existente.");ui.add_enabled_ui(self.state.busy.is_none(),|ui|{if icons::button(ui,Glyph::FolderOpen,"Selecionar arquivo FC2").clicked(){self.load(ui.ctx(),None);}});ui.small("Mapa inicia sem rede. Ative tiles nas configurações se desejar o fundo OpenStreetMap.");});
                }
            });ui.separator();
            ui.horizontal_wrapped(|ui|{ui.label(icons::text(Glyph::Info));ui.label(&self.state.status);});
        });
        if self.state.settings_open {
            self.settings(&ctx);
        }
    }
}
fn coordinates(ui: &mut egui::Ui, latitude: Option<f64>, longitude: Option<f64>) {
    ui.label(format!(
        "Latitude: {}",
        latitude
            .map(|v| v.to_string())
            .unwrap_or("ausente ou inválida".into())
    ));
    ui.label(format!(
        "Longitude: {}",
        longitude
            .map(|v| v.to_string())
            .unwrap_or("ausente ou inválida".into())
    ));
    ui.small("Datum não confirmado; geolocalização exploratória.");
}
fn record_inspector(ui: &mut egui::Ui, record: &fc2_rs::work_records::WorkMeasurement) {
    ui.label(format!(
        "ID {} • ponto {} • campo desconhecido 34: {}",
        record.id, record.point, record.unknown_34
    ));
    coordinates(ui, record.latitude, record.longitude);
    ui.monospace(format!(
        "GPS bits: latitude 0x{:08X}; longitude 0x{:08X}",
        record.latitude_bits, record.longitude_bits
    ));
    ui.label(format!(
        "Fuso horário: {}",
        record.timezone.as_deref().unwrap_or("não interpretado")
    ));
    ui.label(format!("Calendário bruto: {:?}", record.raw_calendar));
    for segment in &record.segments {
        ui.monospace(format!(
            "Offset FC2: 0x{:X} ({}) • memória: 0x{:X} • {} bytes",
            segment.file_offset, segment.file_offset, segment.memory_address, segment.length
        ));
    }
    ui.monospace(&record.raw_hex);
    ui.monospace(format!("SHA-256: {}", record.raw_sha256));
    for warning in &record.warnings {
        ui.label(warning);
    }
}
fn metadata(ui: &mut egui::Ui, loaded: &LoadedDataset, index: usize) {
    let Some(work) = loaded.work(index) else {
        return;
    };
    egui::ScrollArea::vertical()
        .id_salt(("metadata", index))
        .show(ui, |ui| {
            ui.heading("Origem e validação");
            ui.label(format!("Nome: {}", work.display_name));
            ui.label(format!(
                "Nome armazenado: {}",
                work.name.as_deref().unwrap_or("ausente")
            ));
            ui.label(format!("Origem do nome: {}", work.name_source));
            ui.label(format!("ID: {}", work.number));
            ui.monospace(&work.directory);
            ui.label(format!("Arquivo: {}", loaded.source_path.display()));
            ui.label(format!(
                "Tamanho: {} bytes",
                loaded.dataset.source_size_bytes
            ));
            ui.monospace(format!("SHA-256: {}", loaded.dataset.source_sha256));
            ui.label(format!(
                "CRC estrutural: {} setores conferidos",
                loaded.dataset.filesystem.crc_verified_count
            ));
            ui.label(&loaded.dataset.validation);
            ui.label(format!(
                "Configuração: {}",
                work.configuration_file.as_deref().unwrap_or("ausente")
            ));
            ui.label(format!(
                "Grupos declarados: {}",
                work.expected_groups
                    .map(|n| n.to_string())
                    .unwrap_or("desconhecido".into())
            ));
            ui.label(if work.reference_validation.is_some() {
                "Existe validação de referência específica; consulte auditoria."
            } else {
                "Sem equivalência Leaf independente demonstrada para este trabalho."
            });
            ui.separator();
            ui.heading("Grupos e arquivos de origem");
            for group in &work.groups {
                ui.label(format!(
                    "Grupo {} • {} registros • declarados: {}",
                    group.group,
                    group.records.len(),
                    group
                        .expected_records
                        .map(|n| n.to_string())
                        .unwrap_or("desconhecido".into())
                ));
                ui.monospace(&group.source_file);
                ui.monospace(&group.source_sha256);
            }
            ui.separator();
            for error in &work.errors {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
            for warning in &work.warnings {
                ui.label(warning);
            }
            for limitation in &loaded.dataset.limitations {
                ui.label(limitation);
            }
        });
}
