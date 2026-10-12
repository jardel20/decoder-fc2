//! Interactive navigation over the same immutable source and validation gates as the CLI.
#[cfg(test)]
#[path = "tests.rs"]
mod app_tests;
mod summary;
mod terminal;
mod view;

use crate::{
    analysis,
    error::{Error, Result},
    export::{self, works::WorkbookOrder},
    parser::{self, Image},
    reader,
    records::Reference,
    validator::{self, Profile},
    work_records::WorkMeasurement,
    workspace::{self, Dataset, Work, WorkGroup},
};
use console::Key;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Screen {
    Works,
    Measurements,
    Record,
    Files,
    Hex,
    Overview,
    Help,
    Browser,
    WorkDetails,
    Points,
    Warnings,
}

pub(super) struct LoadedFile {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
    pub image: Option<Image>,
    pub dataset: Option<Dataset>,
    pub decode_error: Option<String>,
    pub overview: String,
}

impl LoadedFile {
    fn load(path: &Path) -> Result<Self> {
        let bytes = reader::read_file(path)?;
        let overview = analysis::inspect_text(&analysis::analyze(&bytes));
        let image = parser::parse(&bytes).ok();
        let (dataset, decode_error) = match workspace::decode(&bytes) {
            Ok(dataset) => (Some(dataset), None),
            Err(error) => (None, Some(error.to_string())),
        };
        Ok(Self {
            path: path.to_owned(),
            bytes,
            image,
            dataset,
            decode_error,
            overview,
        })
    }
}

pub(super) struct BrowserEntry {
    pub path: PathBuf,
    pub is_dir: bool,
    pub label: String,
}

pub(super) struct Browser {
    pub directory: PathBuf,
    pub entries: Vec<BrowserEntry>,
    pub index: usize,
    pub search: String,
}

pub(super) struct Destination {
    pub selection: Option<Vec<u16>>,
    pub mode: export::works::ExportMode,
    browser: Browser,
    screen: Screen,
}

impl Browser {
    fn read(path: &Path) -> Result<Self> {
        let directory = std::fs::canonicalize(path)?;
        let mut entries = Vec::new();
        if let Some(parent) = directory.parent() {
            entries.push(BrowserEntry {
                path: parent.to_owned(),
                is_dir: true,
                label: "..".into(),
            });
        }
        for entry in std::fs::read_dir(&directory)? {
            let entry = entry?;
            if entries.len() >= 10_000 {
                return Err(Error::Format(
                    "diretório excede 10.000 entradas; abra um caminho com O".into(),
                ));
            }
            let metadata = entry.metadata()?;
            if !metadata.is_dir() && !metadata.is_file() {
                continue;
            }
            entries.push(BrowserEntry {
                path: entry.path(),
                is_dir: metadata.is_dir(),
                label: entry.file_name().to_string_lossy().into_owned(),
            });
        }
        entries.sort_by(|a, b| {
            (a.label != "..", !a.is_dir, a.label.to_lowercase()).cmp(&(
                b.label != "..",
                !b.is_dir,
                b.label.to_lowercase(),
            ))
        });
        Ok(Self {
            directory,
            entries,
            index: 0,
            search: String::new(),
        })
    }
}

#[derive(Clone)]
pub(super) enum PromptAction {
    Open,
    Search,
    Directory,
    #[cfg(test)]
    Export(Option<u16>),
    ExportSelection(Option<Vec<u16>>, export::works::ExportMode),
    Backup,
    Analysis,
    Audit,
    Profile,
    Csv,
    Offset,
    Reference,
    ReferenceSource(PathBuf),
    SaveProfile(PathBuf, PathBuf),
}

pub(super) struct Prompt {
    pub label: String,
    pub value: String,
    pub action: PromptAction,
    pub cursor: usize,
    pub error: Option<String>,
    snapshot: Option<FilterSnapshot>,
}

struct FilterSnapshot {
    query: String,
    work_index: usize,
    group_index: usize,
    record_index: usize,
    raw_file_index: usize,
    browser_index: usize,
    point_index: usize,
    point_filter: Option<u16>,
}

pub(super) struct ActionMenu {
    pub index: usize,
    pub search: String,
}

pub(super) struct ActionEntry {
    pub key: char,
    pub label: &'static str,
    pub description: &'static str,
    pub enabled: bool,
}

pub(super) struct App {
    pub file: Option<LoadedFile>,
    pub screen: Screen,
    pub work_index: usize,
    pub group_index: usize,
    pub record_index: usize,
    pub raw_file_index: usize,
    pub scroll: usize,
    pub hex_offset: usize,
    pub hex_memory: bool,
    pub search: String,
    pub status: String,
    pub prompt: Option<Prompt>,
    pub browser: Browser,
    pub leaf_order: bool,
    pub technical_details: bool,
    pub only_warnings: bool,
    pub point_index: usize,
    pub point_filter: Option<u16>,
    pub action_menu: Option<ActionMenu>,
    pub marked_works: std::collections::BTreeSet<u16>,
    pub export_mode: export::works::ExportMode,
    pub destination: Option<Destination>,
    pub export_root: PathBuf,
    profile: Option<Profile>,
    return_screen: Screen,
    height: usize,
    width: usize,
}

impl App {
    fn new(directory: &Path) -> Result<Self> {
        Ok(Self {
            file: None,
            screen: Screen::Browser,
            work_index: 0,
            group_index: 0,
            record_index: 0,
            raw_file_index: 0,
            scroll: 0,
            hex_offset: 0,
            hex_memory: false,
            search: String::new(),
            status: "Selecione um FC2. Originais abertos somente para leitura.".into(),
            prompt: None,
            browser: Browser::read(directory)?,
            leaf_order: true,
            technical_details: false,
            only_warnings: false,
            point_index: 0,
            point_filter: None,
            action_menu: None,
            marked_works: std::collections::BTreeSet::new(),
            export_mode: export::works::ExportMode::Simple,
            destination: None,
            export_root: crate::paths::downloads_dir(),
            profile: None,
            return_screen: Screen::Works,
            height: 24,
            width: 80,
        })
    }

    fn open(&mut self, path: &Path) -> Result<()> {
        // Build the replacement first: failed reads cannot discard the current source.
        let file = LoadedFile::load(path)?;
        self.screen = if file.dataset.is_some() {
            Screen::Works
        } else {
            Screen::Overview
        };
        self.status = if file.dataset.is_some() {
            "Arquivo carregado. Validação estrutural; comparação Leaf exige perfil específico."
                .into()
        } else {
            format!(
                "Decodificação indisponível: {}. Inspeção e hex continuam disponíveis.",
                file.decode_error
                    .as_deref()
                    .unwrap_or("variante desconhecida")
            )
        };
        self.file = Some(file);
        self.profile = None;
        self.work_index = 0;
        self.group_index = 0;
        self.record_index = 0;
        self.raw_file_index = 0;
        self.scroll = 0;
        self.hex_offset = 0;
        self.hex_memory = false;
        self.search.clear();
        self.only_warnings = false;
        self.point_filter = None;
        self.point_index = 0;
        self.technical_details = false;
        self.action_menu = None;
        self.marked_works.clear();
        Ok(())
    }

    fn dataset(&self) -> Option<&Dataset> {
        self.file.as_ref()?.dataset.as_ref()
    }

    pub fn export_mode_label(&self) -> &'static str {
        match self.export_mode {
            export::works::ExportMode::Complete => "Completo",
            export::works::ExportMode::Simple => "Simples (somente XLSX)",
        }
    }

    pub fn visible_works(&self) -> Vec<usize> {
        let screen = self
            .destination
            .as_ref()
            .map(|d| d.screen)
            .unwrap_or(self.screen);
        self.dataset()
            .map(|d| {
                d.works
                    .iter()
                    .enumerate()
                    .filter(|(_, w)| {
                        screen != Screen::Works
                            || ((!self.only_warnings
                                || w.warning_count() > 0
                                || !w.errors.is_empty())
                                && (matches_query(
                                    &self.search,
                                    &format!("{} {} {}", w.number, w.display_name, w.directory),
                                ) || w.groups.iter().flat_map(|g| &g.records).any(|r| {
                                    matches_query(&self.search, r.date.as_deref().unwrap_or(""))
                                })))
                    })
                    .map(|(i, _)| i)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn selected_work(&self) -> Option<&Work> {
        self.dataset()?
            .works
            .get(*self.visible_works().get(self.work_index)?)
    }

    pub fn selected_group(&self) -> Option<&WorkGroup> {
        self.selected_work()?.groups.get(self.group_index)
    }

    pub fn visible_records(&self) -> Vec<usize> {
        let Some(group) = self.selected_group() else {
            return Vec::new();
        };
        let mut indices: Vec<_> = group
            .records
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                (!self.only_warnings || !r.warnings.is_empty())
                    && self.point_filter.is_none_or(|point| r.point == point)
                    && matches_query(
                        &self.search,
                        &format!(
                            "{} {} {} {}",
                            r.id,
                            r.point,
                            r.date.as_deref().unwrap_or(""),
                            r.time.as_deref().unwrap_or("")
                        ),
                    )
            })
            .map(|(i, _)| i)
            .collect();
        if self.leaf_order {
            indices.sort_by_key(|&i| group.records[i].point.to_string());
        }
        indices
    }

    pub fn selected_record(&self) -> Option<&WorkMeasurement> {
        self.selected_group()?
            .records
            .get(*self.visible_records().get(self.record_index)?)
    }

    pub fn visible_files(&self) -> Vec<usize> {
        self.dataset()
            .map(|d| {
                d.filesystem
                    .files
                    .iter()
                    .enumerate()
                    .filter(|(_, f)| matches_query(&self.search, &f.path))
                    .map(|(i, _)| i)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn visible_browser(&self) -> Vec<usize> {
        self.browser
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                (self.destination.is_none() || e.is_dir)
                    && matches_query(&self.browser.search, &e.label)
            })
            .map(|(i, _)| i)
            .collect()
    }

    fn prompt(&mut self, action: PromptAction, label: &str, value: String) {
        let snapshot = matches!(action, PromptAction::Search).then(|| FilterSnapshot {
            query: if self.screen == Screen::Browser {
                self.browser.search.clone()
            } else {
                self.search.clone()
            },
            work_index: self.work_index,
            group_index: self.group_index,
            record_index: self.record_index,
            raw_file_index: self.raw_file_index,
            browser_index: self.browser.index,
            point_index: self.point_index,
            point_filter: self.point_filter,
        });
        let cursor = value.len();
        self.prompt = Some(Prompt {
            label: label.into(),
            value,
            action,
            cursor,
            error: None,
            snapshot,
        });
    }

    fn update_search(&mut self, value: String) {
        let old_work = self.visible_works().get(self.work_index).copied();
        let old_record = self.selected_record().map(|r| r.id);
        if self.screen == Screen::Browser {
            self.browser.search = value;
            self.browser.index = 0;
        } else {
            self.search = value;
            self.raw_file_index = 0;
            if self.screen == Screen::Works {
                self.work_index = old_work
                    .and_then(|i| self.visible_works().iter().position(|&j| j == i))
                    .unwrap_or(0);
                if old_work != self.visible_works().get(self.work_index).copied() {
                    self.group_index = 0;
                    self.point_filter = None;
                    self.point_index = 0;
                }
            }
            self.record_index = old_record
                .and_then(|id| {
                    self.visible_records()
                        .iter()
                        .position(|&i| self.selected_group().is_some_and(|g| g.records[i].id == id))
                })
                .unwrap_or(0);
        }
        self.scroll = 0;
    }

    pub fn selected_point(&self) -> Option<u16> {
        Some(
            summary::points(self.selected_group()?)
                .get(self.point_index)?
                .point,
        )
    }

    pub fn visible_actions(&self) -> Vec<ActionEntry> {
        self.visible_actions_for(
            self.action_menu
                .as_ref()
                .map(|m| m.search.as_str())
                .unwrap_or(""),
        )
    }

    fn visible_actions_for(&self, query: &str) -> Vec<ActionEntry> {
        let decoded = self.dataset().is_some();
        let selected = self.selected_work().is_some();
        let entries = [
            (
                'e',
                "Exportar trabalhos marcados / atual",
                "Usa o modo exibido e todos os grupos; marcas incluem trabalhos ocultos pela busca",
                selected || !self.marked_works.is_empty(),
            ),
            (
                'E',
                "Exportar todos os trabalhos",
                "Usa o modo exibido; ignora marcas e filtros",
                decoded,
            ),
            (
                'M',
                "Alternar modo de exportação",
                "Completo: XLSX e auditoria; simples: somente XLSX",
                decoded,
            ),
            (
                'A',
                "Marcar trabalhos visíveis",
                "Disponível na lista de trabalhos; acrescenta os resultados filtrados",
                decoded && self.screen == Screen::Works,
            ),
            (
                'U',
                "Limpar seleção de trabalhos",
                "Desmarca todos, inclusive os ocultos pela busca",
                decoded,
            ),
            (
                'i',
                "Detalhes do trabalho / medição",
                "Resumo, grupos, calendário e disponibilidade",
                selected,
            ),
            (
                '6',
                "Resumo por ponto",
                "Médias e faixas calculadas do grupo selecionado",
                self.selected_group().is_some(),
            ),
            (
                'w',
                "Avisos e campos ausentes",
                "Explicações e IDs dos registros afetados",
                selected,
            ),
            (
                'x',
                "Backup bruto integral",
                "Original, memória e todos os arquivos ativos; ZIP novo",
                decoded,
            ),
            (
                'J',
                "Salvar auditoria JSON",
                "Todos os trabalhos, bytes e proveniência",
                decoded,
            ),
            (
                'c',
                "CSV da referência Leaf",
                if self.profile.is_some() {
                    "Escopo do perfil carregado, independente do trabalho selecionado"
                } else {
                    "Indisponível: carregue um perfil aprovado com p ou v"
                },
                self.profile.is_some(),
            ),
            (
                'p',
                "Carregar perfil Leaf",
                "Revalidar hashes e registros ativos da referência",
                decoded,
            ),
            (
                'v',
                "Validar nova referência",
                "JSON normalizado + XLSX/CSV original + perfil novo",
                decoded,
            ),
            (
                'a',
                "Salvar análise estrutural",
                "Evidências e hipóteses em JSON",
                self.file.is_some(),
            ),
            ('o', "Abrir outro FC2", "Navegador de arquivos locais", true),
            (
                '3',
                "Arquivos internos",
                "Conteúdo bruto, contagens e offsets",
                decoded,
            ),
            (
                '4',
                "Hexadecimal",
                "Bytes do FC2 e memória reconstruída",
                self.file.is_some(),
            ),
            (
                '5',
                "Auditoria técnica",
                "Hashes, CRC, estrutura e limitações",
                self.file.is_some(),
            ),
            (
                '?',
                "Ajuda e atalhos",
                "Todas as funções da interface",
                true,
            ),
        ];
        entries
            .into_iter()
            .filter(|(_, label, desc, _)| matches_query(query, &format!("{label} {desc}")))
            .map(|(key, label, description, enabled)| ActionEntry {
                key,
                label,
                description,
                enabled,
            })
            .collect()
    }

    fn remap_selection(&mut self, work: Option<usize>, record: Option<u32>) {
        self.work_index = work
            .and_then(|i| self.visible_works().iter().position(|&j| j == i))
            .unwrap_or(0);
        if work != self.visible_works().get(self.work_index).copied() {
            self.group_index = 0;
            self.point_index = 0;
            self.point_filter = None;
        }
        self.record_index = record
            .and_then(|id| {
                self.visible_records()
                    .iter()
                    .position(|&i| self.selected_group().is_some_and(|g| g.records[i].id == id))
            })
            .unwrap_or(0);
    }

    fn source(&self) -> Result<&LoadedFile> {
        self.file
            .as_ref()
            .ok_or_else(|| Error::Validation("abra um arquivo primeiro".into()))
    }

    fn output_path(&self, suffix: &str) -> String {
        let stem = self
            .file
            .as_ref()
            .and_then(|f| f.path.file_stem())
            .map(|s| s.to_string_lossy())
            .unwrap_or("fc2".into());
        crate::paths::new_output(&self.export_root, &format!("{stem}-{suffix}"))
            .to_string_lossy()
            .into_owned()
    }

    fn choose_destination(&mut self, selection: Option<Vec<u16>>) -> Result<()> {
        let browser = Browser::read(&self.export_root)?;
        let previous = std::mem::replace(&mut self.browser, browser);
        self.destination = Some(Destination {
            selection,
            mode: self.export_mode,
            browser: previous,
            screen: self.screen,
        });
        self.screen = Screen::Browser;
        self.status = "Escolha a pasta de destino. s seleciona a pasta atual; a saída será uma subpasta nova.".into();
        Ok(())
    }

    fn close_destination(&mut self) {
        if let Some(destination) = self.destination.take() {
            self.browser = destination.browser;
            self.screen = destination.screen;
        }
    }

    fn confirm_destination(&mut self) {
        if let Some(destination) = &self.destination {
            let action =
                PromptAction::ExportSelection(destination.selection.clone(), destination.mode);
            let stem = self
                .file
                .as_ref()
                .and_then(|file| file.path.file_stem())
                .map(|s| s.to_string_lossy())
                .unwrap_or("fc2".into());
            let output =
                crate::paths::new_output(&self.browser.directory, &format!("{stem}-trabalhos"));
            self.prompt(
                action,
                "Confirmar subpasta NOVA de exportação (caminho editável)",
                output.to_string_lossy().into_owned(),
            );
        }
    }

    fn set_screen(&mut self, screen: Screen) {
        // Resolve a filtered work before leaving its list, so a search never changes selection.
        let selected = self.visible_works().get(self.work_index).copied();
        let record_id = self.selected_record().map(|r| r.id);
        self.screen = screen;
        self.search.clear();
        if let Some(selected) = selected {
            self.work_index = self
                .visible_works()
                .iter()
                .position(|&i| i == selected)
                .unwrap_or(0);
        }
        self.record_index = record_id
            .and_then(|id| {
                self.visible_records()
                    .iter()
                    .position(|&i| self.selected_group().is_some_and(|g| g.records[i].id == id))
            })
            .unwrap_or(0);
        self.raw_file_index = 0;
        self.scroll = 0;
    }

    fn load_profile(&mut self, path: &Path) -> Result<()> {
        let profile: Profile = serde_json::from_slice(&reader::read_file(path)?)?;
        let source = self.source()?;
        let mut dataset = workspace::decode(&source.bytes)?;
        workspace::compare_reference(&source.bytes, &mut dataset, &profile)?;
        self.file
            .as_mut()
            .ok_or_else(|| Error::Validation("fonte indisponível".into()))?
            .dataset = Some(dataset);
        self.status = format!(
            "Perfil revalidado para {}. Outros trabalhos mantêm validação estrutural.",
            profile.reference.work
        );
        self.profile = Some(profile);
        Ok(())
    }

    fn submit(&mut self, action: PromptAction, value: String) -> Result<()> {
        if value.trim().is_empty() && !matches!(action, PromptAction::Search) {
            return Err(Error::Format(
                "caminho/valor vazio; nenhuma saída criada".into(),
            ));
        }
        let path = Path::new(&value);
        match action {
            PromptAction::Open => self.open(path)?,
            PromptAction::Search => {
                self.update_search(value);
            }
            PromptAction::Directory => self.browser = Browser::read(path)?,
            #[cfg(test)]
            PromptAction::Export(selection) => {
                return self.submit(
                    PromptAction::ExportSelection(
                        selection.map(|n| vec![n]),
                        export::works::ExportMode::Complete,
                    ),
                    value,
                );
            }
            PromptAction::ExportSelection(selection, mode) => {
                let file = self.source()?;
                let dataset = file.dataset.as_ref().ok_or_else(|| {
                    Error::Validation(
                        "estrutura não validada; exportação científica bloqueada".into(),
                    )
                })?;
                let report = export::works::export_directory_options(
                    &file.bytes,
                    dataset,
                    &file.path.to_string_lossy(),
                    path,
                    selection.as_deref(),
                    if self.leaf_order {
                        WorkbookOrder::Leaf
                    } else {
                        WorkbookOrder::Stored
                    },
                    mode,
                )?;
                self.status = format!(
                    "{} XLSX, {} medições, {} falhas. Saída: {}",
                    report.successful_workbooks,
                    report.record_count,
                    report.failures,
                    path.display()
                );
                if report.failures > 0 {
                    let blocked = report
                        .works
                        .iter()
                        .filter_map(|w| w.error.as_ref().map(|e| format!("{}: {e}", w.number)))
                        .collect::<Vec<_>>()
                        .join("; ");
                    self.status.push_str(&format!(" · Bloqueados: {blocked}"));
                }
                if self.destination.is_some() {
                    self.export_root = path.parent().unwrap_or(&self.browser.directory).to_owned();
                    self.close_destination();
                }
            }
            PromptAction::Backup => {
                let file = self.source()?;
                let dataset = file.dataset.as_ref().ok_or_else(|| {
                    Error::Validation(
                        "backup estruturado requer árvore validada; original continua preservado"
                            .into(),
                    )
                })?;
                export::write_new(path, &export::works::raw_archive(&file.bytes, dataset)?)?;
                self.status = format!("Backup integral gravado: {}", path.display());
            }
            PromptAction::Analysis => {
                let bytes = json(&analysis::analyze(&self.source()?.bytes))?;
                export::write_new(path, &bytes)?;
                self.status = format!("Análise gravada: {}", path.display());
            }
            PromptAction::Audit => {
                let dataset = self
                    .dataset()
                    .ok_or_else(|| Error::Validation("nenhum dataset decodificado".into()))?;
                export::write_new(path, &json(dataset)?)?;
                self.status = format!("Auditoria JSON integral gravada: {}", path.display());
            }
            PromptAction::Profile => self.load_profile(path)?,
            PromptAction::Csv => {
                let file = self.source()?;
                let profile = self.profile.as_ref().ok_or_else(|| {
                    Error::Validation(
                        "CSV científico exige perfil Leaf revalidado: use p ou v".into(),
                    )
                })?;
                let (report, records) = validator::decode(&file.bytes, profile)?;
                let document = export::document(
                    file.path.to_string_lossy().into_owned(),
                    report,
                    records,
                    self.leaf_order,
                );
                export::write_new(path, &export::csv::encode(&document)?)?;
                self.status = format!(
                    "CSV da referência {} gravado: {}",
                    profile.reference.work,
                    path.display()
                );
            }
            PromptAction::Offset => {
                let value = value.trim();
                let offset = if let Some(hex) = value
                    .strip_prefix("0x")
                    .or_else(|| value.strip_prefix("0X"))
                {
                    usize::from_str_radix(hex, 16)
                } else {
                    value.parse()
                }
                .map_err(|_| Error::Format("offset inválido: decimal ou 0x hexadecimal".into()))?;
                let bytes = self.hex_bytes();
                if offset >= bytes.len() {
                    return Err(Error::Format("offset fora da fonte atual".into()));
                }
                self.hex_offset = offset;
            }
            PromptAction::Reference => self.prompt(
                PromptAction::ReferenceSource(path.to_owned()),
                "XLSX/CSV original da referência (mesmo FC2, sem edição)",
                String::new(),
            ),
            PromptAction::ReferenceSource(reference) => self.prompt(
                PromptAction::SaveProfile(reference, path.to_owned()),
                "Salvar perfil novo (comparação com fonte original)",
                self.output_path("profile.json"),
            ),
            PromptAction::SaveProfile(reference_path, reference_source) => {
                let reference: Reference =
                    serde_json::from_slice(&reader::read_file(&reference_path)?)?;
                if reader::hash(&reader::read_file(&reference_source)?)
                    != reference.source_reference_sha256
                {
                    return Err(Error::Validation(
                        "hash do XLSX/CSV difere da referência normalizada".into(),
                    ));
                }
                let source = self.source()?;
                let (validation, _) = validator::validate(&source.bytes, &reference)?;
                if !validation.accepted {
                    return Err(Error::Validation(
                        "referência divergente/ambígua: perfil não criado".into(),
                    ));
                }
                let profile = Profile {
                    schema: "fc2-validation-profile-v1".into(),
                    layout: crate::decoder::LAYOUT.into(),
                    tool_version: env!("CARGO_PKG_VERSION").into(),
                    reference,
                    validation,
                };
                let mut dataset = workspace::decode(&source.bytes)?;
                workspace::compare_reference(&source.bytes, &mut dataset, &profile)?;
                export::write_new(path, &json(&profile)?)?;
                self.file
                    .as_mut()
                    .ok_or_else(|| Error::Validation("fonte indisponível".into()))?
                    .dataset = Some(dataset);
                self.status = format!(
                    "Perfil criado e comparação ativa aprovada: {}",
                    path.display()
                );
                self.profile = Some(profile);
            }
        }
        Ok(())
    }

    pub fn hex_bytes(&self) -> &[u8] {
        let Some(file) = &self.file else { return &[] };
        if self.hex_memory {
            file.image
                .as_ref()
                .map(|i| i.bytes.as_slice())
                .unwrap_or(&[])
        } else {
            &file.bytes
        }
    }

    fn change_group(&mut self, delta: isize) {
        let count = self.selected_work().map(|w| w.groups.len()).unwrap_or(0);
        self.group_index = shift(self.group_index, delta, count);
        self.record_index = 0;
        self.point_index = 0;
        self.point_filter = None;
        self.scroll = 0;
    }

    fn navigate(&mut self, delta: isize) {
        match self.screen {
            Screen::Works => {
                self.work_index = shift(self.work_index, delta, self.visible_works().len());
                self.group_index = 0;
                self.record_index = 0;
                self.point_filter = None;
                self.point_index = 0;
            }
            Screen::Measurements => {
                self.record_index = shift(self.record_index, delta, self.visible_records().len())
            }
            Screen::Files => {
                self.raw_file_index = shift(self.raw_file_index, delta, self.visible_files().len())
            }
            Screen::Browser => {
                self.browser.index = shift(self.browser.index, delta, self.visible_browser().len())
            }
            Screen::Points => {
                self.point_index = shift(
                    self.point_index,
                    delta,
                    self.selected_group()
                        .map(|g| summary::points(g).len())
                        .unwrap_or(0),
                )
            }
            Screen::Hex => {
                let stride = if self.width >= 78 {
                    16
                } else if self.width >= 46 {
                    8
                } else {
                    4
                };
                self.hex_offset = shift(
                    self.hex_offset,
                    delta.saturating_mul(stride),
                    self.hex_bytes().len(),
                )
            }
            _ => self.scroll = self.scroll.saturating_add_signed(delta),
        }
    }

    fn enter(&mut self) -> Result<()> {
        match self.screen {
            Screen::Browser => {
                let Some(&index) = self.visible_browser().get(self.browser.index) else {
                    return Ok(());
                };
                let entry = &self.browser.entries[index];
                let path = entry.path.clone();
                if entry.is_dir {
                    self.browser = Browser::read(&path)?;
                } else {
                    self.open(&path)?;
                }
            }
            Screen::Works if self.selected_work().is_some() => {
                self.set_screen(Screen::Measurements)
            }
            Screen::Measurements if self.selected_record().is_some() => {
                self.screen = Screen::Record;
                self.scroll = 0;
            }
            Screen::WorkDetails => self.set_screen(Screen::Measurements),
            Screen::Points => {
                if let Some(point) = self.selected_point() {
                    self.set_screen(Screen::Measurements);
                    self.point_filter = Some(point);
                    self.record_index = 0;
                    self.status = format!(
                        "Ponto {point}: somente a visualização foi filtrada. l limpa; e exporta o trabalho inteiro."
                    );
                }
            }
            Screen::Record => {
                if let Some(offset) = self
                    .selected_record()
                    .and_then(|r| r.segments.first())
                    .map(|s| s.file_offset)
                {
                    self.hex_memory = false;
                    self.hex_offset = offset;
                    self.screen = Screen::Hex;
                }
            }
            Screen::Files => {
                let offset = self
                    .visible_files()
                    .get(self.raw_file_index)
                    .and_then(|&i| self.dataset()?.filesystem.files.get(i))
                    .and_then(|f| f.segments.first())
                    .map(|s| s.file_offset);
                if let Some(offset) = offset {
                    self.hex_memory = false;
                    self.hex_offset = offset;
                    self.screen = Screen::Hex;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn key(&mut self, key: Key) -> Result<bool> {
        if key == Key::CtrlC {
            return Ok(false);
        }
        if let Some(mut prompt) = self.prompt.take() {
            match key {
                Key::Escape => {
                    if let Some(snapshot) = prompt.snapshot {
                        if self.screen == Screen::Browser {
                            self.browser.search = snapshot.query;
                        } else {
                            self.search = snapshot.query;
                        }
                        self.work_index = snapshot.work_index;
                        self.group_index = snapshot.group_index;
                        self.record_index = snapshot.record_index;
                        self.raw_file_index = snapshot.raw_file_index;
                        self.browser.index = snapshot.browser_index;
                        self.point_index = snapshot.point_index;
                        self.point_filter = snapshot.point_filter;
                    }
                    self.status =
                        "Entrada cancelada; seleção e filtro anteriores restaurados.".into();
                }
                Key::Enter => {
                    if let Err(error) = self.submit(prompt.action.clone(), prompt.value.clone()) {
                        prompt.error = Some(error.to_string());
                        self.prompt = Some(prompt);
                        return Err(error);
                    }
                }
                Key::Backspace => {
                    let previous = previous_char(&prompt.value, prompt.cursor);
                    prompt.value.drain(previous..prompt.cursor);
                    prompt.cursor = previous;
                    prompt.error = None;
                    self.prompt = Some(prompt);
                }
                Key::Char('\u{15}') => {
                    prompt.value.clear();
                    prompt.cursor = 0;
                    prompt.error = None;
                    self.prompt = Some(prompt);
                }
                Key::ArrowLeft => {
                    prompt.cursor = previous_char(&prompt.value, prompt.cursor);
                    self.prompt = Some(prompt);
                }
                Key::ArrowRight => {
                    prompt.cursor = next_char(&prompt.value, prompt.cursor);
                    self.prompt = Some(prompt);
                }
                Key::Home => {
                    prompt.cursor = 0;
                    self.prompt = Some(prompt);
                }
                Key::End => {
                    prompt.cursor = prompt.value.len();
                    self.prompt = Some(prompt);
                }
                Key::Del => {
                    let next = next_char(&prompt.value, prompt.cursor);
                    prompt.value.drain(prompt.cursor..next);
                    prompt.error = None;
                    self.prompt = Some(prompt);
                }
                Key::Char(c) if !c.is_control() => {
                    if prompt.value.len() + c.len_utf8() <= 4096 {
                        prompt.value.insert(prompt.cursor, c);
                        prompt.cursor += c.len_utf8();
                        prompt.error = None;
                    }
                    self.prompt = Some(prompt);
                }
                _ => self.prompt = Some(prompt),
            }
            let live_query = self
                .prompt
                .as_ref()
                .filter(|p| matches!(p.action, PromptAction::Search))
                .map(|p| p.value.clone());
            if let Some(query) = live_query {
                self.update_search(query);
            }
            return Ok(true);
        }
        if let Some(mut menu) = self.action_menu.take() {
            let entries = self.visible_actions_for(&menu.search);
            match key {
                Key::Escape | Key::Char(':') => return Ok(true),
                Key::ArrowUp => menu.index = shift(menu.index, -1, entries.len()),
                Key::ArrowDown => menu.index = shift(menu.index, 1, entries.len()),
                Key::Home => menu.index = 0,
                Key::End => menu.index = entries.len().saturating_sub(1),
                Key::Enter => {
                    if let Some(action) = entries.get(menu.index) {
                        if action.enabled {
                            return self.key(Key::Char(action.key));
                        }
                        self.status = format!("Ação indisponível: {}", action.description);
                    }
                }
                Key::Backspace => {
                    menu.search.pop();
                    menu.index = 0;
                }
                Key::Char('\u{15}') => {
                    menu.search.clear();
                    menu.index = 0;
                }
                Key::Char(c) if !c.is_control() && menu.search.len() + c.len_utf8() <= 256 => {
                    menu.search.push(c);
                    menu.index = 0;
                }
                _ => {}
            }
            self.action_menu = Some(menu);
            return Ok(true);
        }
        if self.destination.is_some() {
            match key {
                Key::Char('s') => {
                    self.confirm_destination();
                    return Ok(true);
                }
                Key::Escape => {
                    self.close_destination();
                    self.status = "Escolha de destino cancelada; nenhuma exportação criada.".into();
                    return Ok(true);
                }
                Key::Char('D') => {
                    self.browser = Browser::read(&crate::paths::downloads_dir())?;
                    return Ok(true);
                }
                Key::Char('h') => {
                    if let Some(home) = crate::paths::home_dir() {
                        self.browser = Browser::read(&home)?;
                    }
                    return Ok(true);
                }
                Key::Char('O') => {
                    self.prompt(
                        PromptAction::Directory,
                        "Ir para pasta (ex.: C:\\Dados ou /mnt/dados)",
                        self.browser.directory.to_string_lossy().into_owned(),
                    );
                    return Ok(true);
                }
                Key::ArrowUp
                | Key::ArrowDown
                | Key::PageUp
                | Key::PageDown
                | Key::Home
                | Key::End
                | Key::Enter
                | Key::Backspace
                | Key::Char('d')
                | Key::Char('/') => {}
                _ => return Ok(true),
            }
        }
        match key {
            Key::Char('q') => return Ok(false),
            Key::Char('1') => self.set_screen(Screen::Works),
            Key::Char('2') => self.set_screen(Screen::Measurements),
            Key::Char('3') => self.set_screen(Screen::Files),
            Key::Char('4') => self.set_screen(Screen::Hex),
            Key::Char('5') => self.set_screen(Screen::Overview),
            Key::Char('6') => self.set_screen(Screen::Points),
            Key::Char('i') => {
                if self.screen == Screen::Measurements && self.selected_record().is_some() {
                    self.screen = Screen::Record;
                    self.scroll = 0;
                } else {
                    self.set_screen(Screen::WorkDetails);
                }
            }
            Key::Char('w') => self.set_screen(Screen::Warnings),
            Key::Char('t') => {
                self.technical_details = !self.technical_details;
                self.scroll = 0;
            }
            Key::Char(':') => {
                self.action_menu = Some(ActionMenu {
                    index: 0,
                    search: String::new(),
                })
            }
            Key::Char('f') if matches!(self.screen, Screen::Works | Screen::Measurements) => {
                let work = self.visible_works().get(self.work_index).copied();
                let record = self.selected_record().map(|r| r.id);
                self.only_warnings = !self.only_warnings;
                self.remap_selection(work, record);
                self.status = if self.only_warnings {
                    "Filtro: somente itens com avisos. A exportação permanece completa."
                } else {
                    "Todos os itens estão visíveis."
                }
                .into();
            }
            Key::Char('l') | Key::Char('\u{c}') => {
                let work = self.visible_works().get(self.work_index).copied();
                let record = self.selected_record().map(|r| r.id);
                self.only_warnings = false;
                self.point_filter = None;
                self.search.clear();
                if self.screen == Screen::Browser {
                    self.browser.search.clear();
                }
                self.remap_selection(work, record);
                self.status = "Filtros limpos; nenhuma medição foi removida dos dados.".into();
            }
            Key::Char('u') | Key::Char('n')
                if matches!(
                    self.screen,
                    Screen::Measurements | Screen::WorkDetails | Screen::Points | Screen::Warnings
                ) =>
            {
                self.work_index = shift(
                    self.work_index,
                    if key == Key::Char('u') { -1 } else { 1 },
                    self.dataset().map(|d| d.works.len()).unwrap_or(0),
                );
                self.group_index = 0;
                self.record_index = 0;
                self.point_index = 0;
                self.point_filter = None;
                self.search.clear();
                self.scroll = 0;
            }
            Key::Char('?') => self.set_screen(Screen::Help),
            Key::Tab | Key::BackTab => {
                let screens = [
                    Screen::Works,
                    Screen::Measurements,
                    Screen::Points,
                    Screen::Files,
                    Screen::Hex,
                    Screen::Overview,
                    Screen::Help,
                ];
                let i = screens.iter().position(|s| *s == self.screen).unwrap_or(0);
                let next = if key == Key::BackTab {
                    (i + screens.len() - 1) % screens.len()
                } else {
                    (i + 1) % screens.len()
                };
                self.set_screen(screens[next]);
            }
            Key::ArrowUp | Key::Char('k') => self.navigate(-1),
            Key::ArrowDown | Key::Char('j') => self.navigate(1),
            Key::PageUp => self.navigate(-(self.height.saturating_sub(10).max(1) as isize)),
            Key::PageDown => self.navigate(self.height.saturating_sub(10).max(1) as isize),
            Key::Home => self.navigate(isize::MIN),
            Key::End => self.navigate(isize::MAX),
            Key::ArrowLeft | Key::Char('[')
                if matches!(
                    self.screen,
                    Screen::Measurements
                        | Screen::Record
                        | Screen::Points
                        | Screen::WorkDetails
                        | Screen::Warnings
                ) =>
            {
                self.change_group(-1)
            }
            Key::ArrowRight | Key::Char(']')
                if matches!(
                    self.screen,
                    Screen::Measurements
                        | Screen::Record
                        | Screen::Points
                        | Screen::WorkDetails
                        | Screen::Warnings
                ) =>
            {
                self.change_group(1)
            }
            Key::Enter => self.enter()?,
            Key::Escape => {
                if self.screen == Screen::Record {
                    self.screen = Screen::Measurements;
                    self.scroll = 0;
                } else if self.screen == Screen::Browser {
                    self.set_screen(self.return_screen);
                } else {
                    self.set_screen(Screen::Works);
                }
            }
            Key::Backspace if self.screen == Screen::Browser => {
                if let Some(parent) = self.browser.directory.parent() {
                    self.browser = Browser::read(parent)?;
                }
            }
            Key::Char('o') => {
                let directory = self
                    .file
                    .as_ref()
                    .and_then(|f| f.path.parent())
                    .filter(|p| !p.as_os_str().is_empty())
                    .unwrap_or(&self.browser.directory);
                let browser = Browser::read(directory)?;
                self.return_screen = self.screen;
                self.browser = browser;
                self.set_screen(Screen::Browser);
            }
            Key::Char('O') => self.prompt(
                PromptAction::Open,
                "Abrir arquivo (caminho; somente leitura)",
                String::new(),
            ),
            Key::Char('d') if self.screen == Screen::Browser => self.prompt(
                PromptAction::Directory,
                "Abrir diretório",
                self.browser.directory.to_string_lossy().into_owned(),
            ),
            Key::Char('D') if self.screen == Screen::Browser => {
                self.browser = Browser::read(&crate::paths::downloads_dir())?;
            }
            Key::Char('h') if self.screen == Screen::Browser => {
                if let Some(home) = crate::paths::home_dir() {
                    self.browser = Browser::read(&home)?;
                }
            }
            Key::Char('/') => {
                if matches!(
                    self.screen,
                    Screen::Works | Screen::Measurements | Screen::Files | Screen::Browser
                ) {
                    let value = if self.screen == Screen::Browser {
                        self.browser.search.clone()
                    } else {
                        self.search.clone()
                    };
                    self.prompt(
                        PromptAction::Search,
                        "Busca ao digitar · Enter mantém · Esc restaura",
                        value,
                    );
                }
            }
            Key::Char('e') => {
                let numbers = if self.marked_works.is_empty() {
                    vec![
                        self.selected_work()
                            .ok_or_else(|| {
                                Error::Validation(
                                    "marque ou selecione um trabalho antes de exportar".into(),
                                )
                            })?
                            .number,
                    ]
                } else {
                    self.marked_works.iter().copied().collect()
                };
                self.choose_destination(Some(numbers))?;
            }
            Key::Char('E') => {
                self.source()?;
                self.choose_destination(None)?;
            }
            Key::Char(' ') if self.screen == Screen::Works => {
                if let Some(number) = self.selected_work().map(|w| w.number) {
                    if !self.marked_works.remove(&number) {
                        self.marked_works.insert(number);
                    }
                    self.status = format!(
                        "{} trabalhos marcados, incluindo os ocultos. e exporta; U limpa.",
                        self.marked_works.len()
                    );
                }
            }
            Key::Char('A') if self.screen == Screen::Works => {
                let numbers: Vec<_> = self
                    .visible_works()
                    .iter()
                    .filter_map(|&i| self.dataset()?.works.get(i).map(|w| w.number))
                    .collect();
                self.marked_works.extend(numbers);
                self.status = format!(
                    "{} trabalhos marcados. Espaço desmarca; U limpa.",
                    self.marked_works.len()
                );
            }
            Key::Char('U') => {
                self.marked_works.clear();
                self.status = "Seleção de trabalhos limpa.".into();
            }
            Key::Char('M') => {
                self.export_mode = match self.export_mode {
                    export::works::ExportMode::Complete => export::works::ExportMode::Simple,
                    export::works::ExportMode::Simple => export::works::ExportMode::Complete,
                };
                self.status = format!(
                    "Modo {}. e exporta marcados/atual; E exporta todos.",
                    self.export_mode_label()
                );
            }
            Key::Char('x') => self.prompt(
                PromptAction::Backup,
                "Backup INTEGRAL: ZIP novo, inclui todos os trabalhos",
                self.output_path("brutos.zip"),
            ),
            Key::Char('a') => self.prompt(
                PromptAction::Analysis,
                "Salvar análise estrutural: JSON novo",
                self.output_path("analysis.json"),
            ),
            Key::Char('J') => self.prompt(
                PromptAction::Audit,
                "Salvar auditoria de TODOS os trabalhos: JSON novo",
                self.output_path("dataset.json"),
            ),
            Key::Char('c') => {
                let label = match self.profile.as_ref() {
                    Some(profile) => format!(
                        "CSV da referência {} (escopo do perfil): arquivo novo",
                        profile.reference.work
                    ),
                    None => return Err(Error::Validation(
                        "CSV indisponível: carregue um perfil de referência aprovado com p ou v"
                            .into(),
                    )),
                };
                self.prompt(
                    PromptAction::Csv,
                    &label,
                    self.output_path("referencia.csv"),
                );
            }
            Key::Char('p') => self.prompt(
                PromptAction::Profile,
                "Carregar perfil JSON existente; hash será revalidado",
                String::new(),
            ),
            Key::Char('v') => self.prompt(
                PromptAction::Reference,
                "Validar: referência JSON normalizada (mesmo FC2, sem edição)",
                String::new(),
            ),
            Key::Char('s') => {
                let selected_id = self.selected_record().map(|r| r.id);
                self.leaf_order = !self.leaf_order;
                if let Some(id) = selected_id {
                    self.record_index = self
                        .visible_records()
                        .iter()
                        .position(|&i| self.selected_group().is_some_and(|g| g.records[i].id == id))
                        .unwrap_or(0);
                }
                self.status = if self.leaf_order {
                    "Ordem: pontos lexicais como no exemplo; sequência dentro do ponto preservada."
                } else {
                    "Ordem original dos registros de cada arquivo."
                }
                .into();
            }
            Key::Char('m') if self.screen == Screen::Hex => {
                if !self.hex_memory && self.source()?.image.is_none() {
                    return Err(Error::Format(
                        "memória indisponível: contêiner não suportado".into(),
                    ));
                }
                self.hex_memory = !self.hex_memory;
                self.hex_offset = 0;
            }
            Key::Char('g') if self.screen == Screen::Hex => self.prompt(
                PromptAction::Offset,
                "Offset na fonte atual (decimal ou 0x hexadecimal)",
                format!("0x{:x}", self.hex_offset),
            ),
            _ => {}
        }
        Ok(true)
    }
}

fn previous_char(value: &str, cursor: usize) -> usize {
    value[..cursor]
        .char_indices()
        .next_back()
        .map(|(i, _)| i)
        .unwrap_or(0)
}

fn next_char(value: &str, cursor: usize) -> usize {
    cursor
        + value[cursor..]
            .chars()
            .next()
            .map(char::len_utf8)
            .unwrap_or(0)
}

fn matches_query(query: &str, value: &str) -> bool {
    value.to_lowercase().contains(&query.trim().to_lowercase())
}

fn shift(index: usize, delta: isize, count: usize) -> usize {
    index
        .saturating_add_signed(delta)
        .min(count.saturating_sub(1))
}

fn json<T: serde::Serialize>(value: &T) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// Open the interactive interface; no source file is ever opened for writing.
pub fn run(file: Option<&Path>, profile: Option<&Path>) -> Result<()> {
    let current = std::env::current_dir()?;
    let mut app = App::new(&current)?;
    if let Some(file) = file {
        app.open(file)?;
    }
    if let Some(profile) = profile {
        app.load_profile(profile)?;
    }
    terminal::run_loop(|key, width, height| {
        app.height = usize::from(height);
        app.width = usize::from(width);
        if let Some(key) = key {
            match app.key(key) {
                Ok(false) => return Ok(None),
                Ok(true) => {}
                Err(error) => app.status = format!("Erro: {error}"),
            }
        }
        Ok(Some(view::render(&app, width, height)))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_source_remains_inspectable_and_cannot_export_science() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("unknown.FC2");
        std::fs::write(&path, [0x1b, 0, 255]).unwrap();
        let mut app = App::new(temp.path()).unwrap();
        app.open(&path).unwrap();
        assert_eq!(app.screen, Screen::Overview);
        assert!(app.file.as_ref().unwrap().decode_error.is_some());
        assert_eq!(app.hex_bytes(), &[0x1b, 0, 255]);
        let output = temp.path().join("blocked");
        assert!(
            app.submit(
                PromptAction::Export(None),
                output.to_string_lossy().into_owned()
            )
            .is_err()
        );
        assert!(!output.exists());
        let source_hash = reader::hash(&app.file.as_ref().unwrap().bytes);
        assert!(app.open(&temp.path().join("missing.FC2")).is_err());
        assert_eq!(reader::hash(&app.file.as_ref().unwrap().bytes), source_hash);
        assert_eq!(reader::hash(&std::fs::read(path).unwrap()), source_hash);
    }

    #[test]
    fn prompt_cancellation_unicode_backspace_and_ctrl_c_are_safe() {
        let temp = tempfile::tempdir().unwrap();
        let mut app = App::new(temp.path()).unwrap();
        app.prompt(PromptAction::Open, "Caminho", "á".into());
        app.key(Key::Backspace).unwrap();
        assert_eq!(app.prompt.as_ref().unwrap().value, "");
        app.key(Key::Char('ç')).unwrap();
        assert_eq!(app.prompt.as_ref().unwrap().value, "ç");
        app.key(Key::Escape).unwrap();
        assert!(app.prompt.is_none());
        assert!(app.file.is_none());
        assert!(!app.key(Key::CtrlC).unwrap());
        assert!(!app.key(Key::Char('q')).unwrap());
    }

    #[test]
    fn unicode_cursor_editing_and_failed_submission_keep_input_recoverable() {
        let temp = tempfile::tempdir().unwrap();
        let mut app = App::new(temp.path()).unwrap();
        app.prompt(PromptAction::Open, "Caminho", "á中z".into());
        app.key(Key::ArrowLeft).unwrap();
        app.key(Key::Backspace).unwrap();
        assert_eq!(app.prompt.as_ref().unwrap().value, "áz");
        app.key(Key::Char('ç')).unwrap();
        assert_eq!(app.prompt.as_ref().unwrap().value, "áçz");
        app.key(Key::Home).unwrap();
        app.key(Key::Del).unwrap();
        assert_eq!(app.prompt.as_ref().unwrap().value, "çz");
        app.key(Key::ArrowRight).unwrap();
        assert_eq!(app.prompt.as_ref().unwrap().cursor, "ç".len());
        app.key(Key::End).unwrap();
        assert_eq!(app.prompt.as_ref().unwrap().cursor, "çz".len());
        app.key(Key::Char('\u{15}')).unwrap();
        assert!(app.key(Key::Enter).is_err());
        assert!(app.prompt.as_ref().unwrap().error.is_some());
        app.key(Key::Char('x')).unwrap();
        assert!(app.prompt.as_ref().unwrap().error.is_none());
        assert_eq!(app.prompt.as_ref().unwrap().value, "x");
        app.key(Key::Escape).unwrap();
        assert!(app.prompt.is_none());
    }

    #[test]
    fn browser_filters_and_enters_directories_without_modifying_files() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir(temp.path().join("dados")).unwrap();
        std::fs::write(temp.path().join("sample.FC2"), []).unwrap();
        let mut app = App::new(temp.path()).unwrap();
        app.submit(PromptAction::Search, "DaDoS".into()).unwrap();
        assert_eq!(app.visible_browser().len(), 1);
        app.enter().unwrap();
        assert!(app.browser.directory.ends_with("dados"));
        app.key(Key::Backspace).unwrap();
        assert_eq!(app.browser.directory, temp.path().canonicalize().unwrap());
        assert_eq!(
            std::fs::read(temp.path().join("sample.FC2")).unwrap(),
            Vec::<u8>::new()
        );
    }
}
