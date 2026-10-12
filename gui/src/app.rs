//! Native navigation and presentation; expensive operations belong to `Jobs`.
use crate::{
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

pub struct ExplorerApp {
    pub state: State,
    map: MapPane,
    jobs: Jobs,
    pending_load: Option<u64>,
    cancel_requested: bool,
    pending_export: Option<u64>,
    work_points: Vec<fc2_rs::explorer::GeoPoint>,
    points_work: Option<usize>,
    transition: theme::Transition,
    marked_list_revision: Vec<usize>,
}
impl ExplorerApp {
    pub fn new(cc: &eframe::CreationContext<'_>, initial: Option<PathBuf>) -> Self {
        icons::install_fonts(&cc.egui_ctx);
        theme::apply(&cc.egui_ctx, true);
        let mut app = Self {
            state: State::default(),
            map: MapPane::new(&cc.egui_ctx),
            jobs: Jobs::new(),
            pending_load: None,
            cancel_requested: false,
            pending_export: None,
            work_points: Vec::new(),
            points_work: None,
            transition: theme::Transition::default(),
            marked_list_revision: Vec::new(),
        };
        if let Some(path) = initial {
            app.load(&cc.egui_ctx, Some(path));
        }
        app
    }
    fn load(&mut self, ctx: &egui::Context, path: Option<PathBuf>) {
        if self.pending_export.is_some() || self.pending_load.is_some() {
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
        if self.state.busy.is_some() {
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
    fn header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if matches!(self.state.view, AppView::WorkDetail { .. })
                && icons::button(ui, Glyph::ArrowLeft, "Voltar").clicked()
            {
                self.state.back();
            }
            ui.label(icons::text(Glyph::FileBinary));
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("FC2 Explorer").size(16.0).strong())
                    .on_hover_text("Explorador de arquivos Falker");
                if matches!(self.state.view, AppView::Explorer) {
                    ui.small("Explorador de arquivos Falker");
                }
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if icons::button(ui, Glyph::Info, "Info").clicked() {
                    self.state.settings_open = !self.state.settings_open;
                }
                let glyph = if self.state.dark {
                    Glyph::Moon
                } else {
                    Glyph::Sun
                };
                let label = if self.state.dark {
                    "Tema escuro ativo — alternar para claro"
                } else {
                    "Tema claro ativo — alternar para escuro"
                };
                if icons::icon_button(ui, glyph, label).clicked() {
                    self.state.dark = !self.state.dark;
                    theme::apply(ui.ctx(), self.state.dark);
                }
                ui.add_enabled_ui(
                    self.pending_export.is_none() && self.pending_load.is_none(),
                    |ui| {
                        if icons::button(ui, Glyph::FolderOpen, "Abrir FC2").clicked() {
                            self.load(ui.ctx(), None);
                        }
                    },
                );
                if icons::button(ui, Glyph::Terminal, "TUI").clicked() {
                    let path = self
                        .state
                        .loaded
                        .as_ref()
                        .map(|loaded| loaded.source_path.as_path());
                    match crate::terminal::launch(path) {
                        Ok(()) => self.state.status = "TUI aberta em um terminal separado.".into(),
                        Err(error) => self.state.error = Some(error),
                    }
                }
            });
        });
        ui.add_space(2.0);
        ui.separator();
        if let Some(loaded) = &self.state.loaded
            && matches!(self.state.view, AppView::Explorer)
        {
            ui.horizontal_wrapped(|ui| {
                ui.label(icons::text(Glyph::FileBinary));
                ui.label(
                    loaded
                        .source_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy(),
                );
                ui.label(icons::text(Glyph::CheckCircle).color(theme::ACCENT));
                ui.weak(format!(
                    "CRC conferido · {} setores",
                    loaded.dataset.filesystem.crc_verified_count
                ));
                ui.separator();
                ui.label(format!("{} trabalhos", loaded.dataset.works.len()));
                ui.label(format!(
                    "{} registros",
                    loaded
                        .summaries
                        .iter()
                        .map(|summary| summary.records)
                        .sum::<usize>()
                ));
            });
        }
        ui.add_space(2.0);
    }
    fn settings(&mut self, ctx: &egui::Context) {
        let mut open = self.state.settings_open;
        egui::Window::new("Info").open(&mut open).resizable(false).show(ctx,|ui| {
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
                |ui| theme::panel(ui, |ui| self.works(ui, loaded)),
            );
            let right = if compact { 0.0 } else { 260.0 };
            let width = (size.x - left - right - 32.0).max(100.0);
            ui.allocate_ui_with_layout(
                egui::vec2(width, size.y),
                Layout::top_down(Align::Min),
                |ui| {
                    theme::panel(ui, |ui| {
                        ui.label(egui::RichText::new("Visão geográfica").strong());
                        let map_height =
                            (ui.available_height() - if compact { 160.0 } else { 0.0 }).max(1.0);
                        ui.allocate_ui_with_layout(
                            egui::vec2(ui.available_width(), map_height),
                            Layout::top_down(Align::Min),
                            |ui| {
                                if let Some(key) = self.map.show(
                                    ui,
                                    &loaded.locations,
                                    self.state.selected_work,
                                    self.state.selected_record,
                                    Some(loaded),
                                ) {
                                    self.state.select_record(key);
                                }
                            },
                        );
                        if compact {
                            ui.add_space(8.0);
                            self.work_details(ui, loaded, true);
                        }
                    });
                },
            );
            if !compact {
                ui.allocate_ui_with_layout(
                    egui::vec2(right, size.y),
                    Layout::top_down(Align::Min),
                    |ui| theme::panel(ui, |ui| self.work_details(ui, loaded, false)),
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
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.state.work_descending, "Decrescente");
            let mut all = self.state.marked_works.len() == loaded.dataset.works.len() && !loaded.dataset.works.is_empty();
            if ui.checkbox(&mut all, "Todos").on_hover_text("Selecionar ou desmarcar todos os trabalhos do arquivo, inclusive fora da busca").changed() { self.state.mark_all(all); }
            if !self.state.marked_works.is_empty() && icons::button(ui, Glyph::X, "").on_hover_text("Limpar todas as marcações").clicked() { self.state.mark_all(false); }
        });
        ui.small("Espaço marca; duplo clique abre.");
        if !ui.ctx().text_edit_focused()
            && ui.input(|i| i.key_pressed(egui::Key::Space))
            && let Some(work) = self.state.selected_work
        {
            self.state
                .set_marked(work, !self.state.marked_works.contains(&work));
        }
        self.state.refresh_works();
        let count = self.state.visible_works.len();
        let marked: Vec<_> = self
            .state
            .visible_works
            .iter()
            .copied()
            .filter(|index| self.state.marked_works.contains(index))
            .collect();
        let others: Vec<_> = self
            .state
            .visible_works
            .iter()
            .copied()
            .filter(|index| !self.state.marked_works.contains(index))
            .collect();
        let list_height = (ui.available_height() - 118.0).max(80.0);
        if !marked.is_empty() {
            ui.label(egui::RichText::new(format!("Selecionados ({})", marked.len())).strong());
            let mut scroll = egui::ScrollArea::vertical().id_salt("marked-works");
            if self.marked_list_revision != marked {
                self.marked_list_revision = marked.clone();
                scroll = scroll.vertical_scroll_offset(0.0);
            }
            scroll
                .max_height(if others.is_empty() {
                    list_height - 24.0
                } else {
                    list_height * 0.45
                })
                .show_rows(ui, 52.0, marked.len(), |ui, range| {
                    for row in range {
                        self.work_row(ui, loaded, marked[row]);
                    }
                });
            if !others.is_empty() {
                ui.separator();
            }
        }
        egui::ScrollArea::vertical()
            .id_salt("works")
            .max_height((ui.available_height() - 118.0).max(30.0))
            .show_rows(ui, 52.0, others.len(), |ui, range| {
                for row in range {
                    self.work_row(ui, loaded, others[row]);
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
    fn work_row(&mut self, ui: &mut egui::Ui, loaded: &LoadedDataset, index: usize) {
        let work = &loaded.dataset.works[index];
        ui.push_id(index, |ui| {
            ui.horizontal(|ui| {
                let mut marked = self.state.marked_works.contains(&index);
                if ui.checkbox(&mut marked, "").changed() {
                    self.state.set_marked(index, marked);
                    self.state.select_work(index);
                    self.center_work_map(loaded, index);
                    ui.ctx().request_repaint();
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
                    self.center_work_map(loaded, index);
                }
                if response.double_clicked() {
                    self.state.open_work(index);
                    self.center_work_map(loaded, index);
                }
            });
        });
    }
    fn center_work_map(&mut self, loaded: &LoadedDataset, work: usize) {
        if self.points_work != Some(work) {
            self.work_points = loaded
                .locations
                .iter()
                .filter(|point| point.key.work == work)
                .copied()
                .collect();
            self.points_work = Some(work);
        }
        self.map.center_points(&self.work_points);
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
        ui.label(egui::RichText::new(&work.display_name).size(18.0).strong());
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
            self.center_work_map(loaded, index);
        }
    }
    fn detail(&mut self, ui: &mut egui::Ui, loaded: &Arc<LoadedDataset>, work_index: usize) {
        let Some(work) = loaded.work(work_index) else {
            self.state.back();
            return;
        };
        let summary = loaded.summaries[work_index];
        theme::card(ui, |ui| {
            ui.set_min_width((ui.available_width() - 2.0).max(0.0));
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new(&work.display_name).size(20.0).strong());
                ui.weak(format!("ID {}", work.number));
                ui.separator();
                ui.label(format!("{} registros", summary.records));
                ui.label(format!("{} grupos", summary.groups));
                ui.label(format!("{} coordenadas", summary.coordinate_records));
                ui.label(
                    icons::text(if work.errors.is_empty() {
                        Glyph::CheckCircle
                    } else {
                        Glyph::WarningCircle
                    })
                    .color(if work.errors.is_empty() {
                        theme::ACCENT
                    } else {
                        ui.visuals().warn_fg_color
                    }),
                );
                ui.label(if work.errors.is_empty() {
                    "Estrutura interpretada"
                } else {
                    "Estrutura com erros"
                });
                if summary.warned_records > 0 {
                    ui.weak(format!("{} registros com avisos", summary.warned_records));
                }
            });
        });
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            for (tab, _glyph, label) in [
                (WorkTab::Measurements, Glyph::Table, "Medições"),
                (WorkTab::Map, Glyph::MapTrifold, "Mapa"),
                (WorkTab::Metadata, Glyph::Info, "Metadados"),
                (WorkTab::Export, Glyph::Export, "Exportação"),
            ] {
                if theme::tab(ui, self.state.tab == tab, label).clicked() {
                    self.state.tab = tab;
                    if tab == WorkTab::Map {
                        self.center_work_map(loaded, work_index);
                    }
                }
            }
        });
        ui.add_space(4.0);
        match self.state.tab {
            WorkTab::Measurements => theme::card(ui, |ui| {
                ui.label(egui::RichText::new("Registros do trabalho").strong());
                self.measurements(ui, loaded, work_index);
            }),
            WorkTab::Map => {
                if self.points_work != Some(work_index) {
                    self.center_work_map(loaded, work_index);
                }
                if let Some(key) = self.map.show(
                    ui,
                    &self.work_points,
                    Some(work_index),
                    self.state.selected_record,
                    Some(loaded),
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
            ("Avisos", RecordSort::Warnings),
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
                    .column(Column::initial(80.0))
                    .column(Column::initial(100.0))
                    .column(Column::initial(95.0))
                    .column(Column::initial(200.0))
                    .columns(Column::initial(112.0), 3)
                    .column(Column::remainder().at_least(110.0))
                    .header(theme::CONTROL_HEIGHT + 4.0, |mut header| {
                        for (label, sort) in columns {
                            header.col(|ui| {
                                let glyph = if table.sort == sort { if table.descending { Glyph::SortDescending } else { Glyph::SortAscending } } else { Glyph::Funnel };
                                if icons::button(ui, glyph, label).on_hover_text("Clique para ordenar; clique novamente para inverter a ordem").clicked() {
                                    if table.sort == sort {
                                        table.descending = !table.descending;
                                    } else {
                                        table.sort = sort;
                                        table.descending = false;
                                    }
                                }
                            });
                        }
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
        if let Some(path) =
            ctx.input(|i| i.raw.dropped_files.first().map(|f| f.path().to_path_buf()))
        {
            self.load(&ctx, Some(path));
        }
        theme::application_frame(ui, |ui| {
            self.header(ui);
            if let Some(error) = self.state.error.clone() {
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(ui.visuals().error_fg_color, &error);
                    if icons::button(ui, Glyph::X, "Dispensar").clicked() {
                        self.state.error = None;
                    }
                });
            }
            if let Some(busy) = self.state.busy.clone() {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(busy);
                    if self.pending_load.is_some()
                        && !self.cancel_requested
                        && icons::button(ui, Glyph::X, "Cancelar leitura").clicked()
                    {
                        self.jobs.cancel_load();
                        self.cancel_requested = true;
                        self.state.busy = Some(
                            "Cancelamento solicitado; aguarde a leitura ou feche o diálogo aberto."
                                .into(),
                        );
                    }
                });
            }
            let content = egui::vec2(
                ui.available_width(),
                (ui.available_height() - 30.0).max(80.0),
            );
            ui.allocate_ui_with_layout(content,Layout::top_down(Align::Min),|ui|{
                ui.set_min_size(content);
                let key = egui::Id::new(format!("{:?}:{:?}:{}", self.state.view, self.state.tab, self.state.dark));
                let opacity = self.transition.opacity(ui.ctx(), key);
                ui.multiply_opacity(opacity);
                if let Some(loaded)=self.state.loaded.clone(){match self.state.view{AppView::Explorer=>self.explorer(ui,&loaded),AppView::WorkDetail{work}=>self.detail(ui,&loaded,work)}}else{
                    theme::card(ui,|ui|{ui.heading("Explore os trabalhos do seu FC2");ui.label("Abra um arquivo ou arraste-o para esta janela. A leitura preserva o original e utiliza o decoder existente.");ui.add_enabled_ui(self.state.busy.is_none(),|ui|{if icons::button(ui,Glyph::FolderOpen,"Selecionar arquivo FC2").clicked(){self.load(ui.ctx(),None);}});ui.small("OpenStreetMap habilitado por padrão.");});
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(icons::text(Glyph::Info));
                let width = (ui.available_width() - 180.0).max(40.0);
                ui.add_sized(
                    [width, 18.0],
                    egui::Label::new(egui::RichText::new(&self.state.status).size(12.0)).truncate(),
                )
                .on_hover_text(&self.state.status);
                ui.small(format!(
                    "v{} · Rust · egui · Walkers",
                    env!("CARGO_PKG_VERSION")
                ))
                .on_hover_text("Dados preservados · exploração estrutural FC2");
            });
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
