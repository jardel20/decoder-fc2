//! Presentation-independent navigation and cached queries; no decoder mutation.
use fc2_rs::explorer::{LoadedDataset, RecordKey};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::Arc,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppView {
    Explorer,
    WorkDetail { work: usize },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkTab {
    Measurements,
    Map,
    Metadata,
    Export,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkSort {
    Number,
    Name,
    Records,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordSort {
    Stored,
    Id,
    Group,
    Point,
    Date,
    A,
    B,
    Total,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportChoice {
    Xlsx,
    Complete,
    Json,
}

#[derive(PartialEq, Eq)]
struct TableQuery {
    source: String,
    work: usize,
    query: String,
    group: Option<usize>,
    warnings_only: bool,
    sort: RecordSort,
    descending: bool,
}

pub struct TableState {
    pub query: String,
    pub group: Option<usize>,
    pub warnings_only: bool,
    pub sort: RecordSort,
    pub descending: bool,
    cached_query: Option<TableQuery>,
    pub rows: Vec<RecordKey>,
}
impl Default for TableState {
    fn default() -> Self {
        Self {
            query: String::new(),
            group: None,
            warnings_only: false,
            sort: RecordSort::Stored,
            descending: false,
            cached_query: None,
            rows: Vec::new(),
        }
    }
}
impl TableState {
    pub fn refresh(&mut self, loaded: &LoadedDataset, work: usize) {
        let signature = TableQuery {
            source: loaded.dataset.source_sha256.clone(),
            work,
            query: self.query.clone(),
            group: self.group,
            warnings_only: self.warnings_only,
            sort: self.sort,
            descending: self.descending,
        };
        if self.cached_query.as_ref() == Some(&signature) {
            return;
        }
        let query = self.query.trim().to_lowercase();
        self.rows = loaded
            .records
            .get(work)
            .into_iter()
            .flatten()
            .copied()
            .filter(|key| self.group.is_none_or(|group| key.group == group))
            .filter(|&key| {
                loaded.record(key).is_some_and(|r| {
                    (!self.warnings_only || !r.warnings.is_empty())
                        && (query.is_empty()
                            || format!(
                                "{} {} {} {}",
                                r.id,
                                r.point,
                                r.date.as_deref().unwrap_or(""),
                                r.time.as_deref().unwrap_or("")
                            )
                            .to_lowercase()
                            .contains(&query))
                })
            })
            .collect();
        let sort = self.sort;
        self.rows.sort_by(|a, b| {
            let Some(left) = loaded.record(*a) else {
                return a.cmp(b);
            };
            let Some(right) = loaded.record(*b) else {
                return a.cmp(b);
            };
            let order = match sort {
                RecordSort::Stored => a.cmp(b),
                RecordSort::Id => left.id.cmp(&right.id),
                RecordSort::Group => a.group.cmp(&b.group),
                RecordSort::Point => left.point.cmp(&right.point),
                RecordSort::Date => (left.date.as_ref(), left.time.as_ref())
                    .cmp(&(right.date.as_ref(), right.time.as_ref())),
                RecordSort::A => left.chlorophyll_a_tenths.cmp(&right.chlorophyll_a_tenths),
                RecordSort::B => left.chlorophyll_b_tenths.cmp(&right.chlorophyll_b_tenths),
                RecordSort::Total => left
                    .chlorophyll_total_tenths
                    .cmp(&right.chlorophyll_total_tenths),
            }
            .then_with(|| a.cmp(b));
            if self.descending {
                order.reverse()
            } else {
                order
            }
        });
        self.cached_query = Some(signature);
    }
}

pub struct State {
    pub loaded: Option<Arc<LoadedDataset>>,
    pub view: AppView,
    pub selected_work: Option<usize>,
    pub selected_record: Option<RecordKey>,
    pub work_query: String,
    pub work_sort: WorkSort,
    pub work_descending: bool,
    pub tab: WorkTab,
    pub tables: BTreeMap<usize, TableState>,
    pub marked_works: BTreeSet<usize>,
    pub export_choice: ExportChoice,
    pub export_root: PathBuf,
    pub dark: bool,
    pub settings_open: bool,
    pub busy: Option<String>,
    pub error: Option<String>,
    pub status: String,
    work_cache: Option<(String, WorkSort, bool)>,
    pub visible_works: Vec<usize>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            loaded: None,
            view: AppView::Explorer,
            selected_work: None,
            selected_record: None,
            work_query: String::new(),
            work_sort: WorkSort::Number,
            work_descending: false,
            tab: WorkTab::Measurements,
            tables: BTreeMap::new(),
            marked_works: BTreeSet::new(),
            export_choice: ExportChoice::Xlsx,
            export_root: fc2_rs::paths::downloads_dir(),
            dark: false,
            settings_open: false,
            busy: None,
            error: None,
            status: "Abra um FC2 para explorar os trabalhos.".into(),
            work_cache: None,
            visible_works: Vec::new(),
        }
    }
}
impl State {
    pub fn install(&mut self, loaded: Arc<LoadedDataset>) {
        self.selected_work = (!loaded.dataset.works.is_empty()).then_some(0);
        self.loaded = Some(loaded);
        self.selected_record = None;
        self.view = AppView::Explorer;
        self.tab = WorkTab::Measurements;
        self.tables.clear();
        self.marked_works.clear();
        self.work_query.clear();
        self.work_cache = None;
        self.error = None;
        self.status =
            "FC2 carregado; CRC estrutural conferido. Equivalência Leaf depende de referência."
                .into();
        self.refresh_works();
    }
    pub fn refresh_works(&mut self) {
        let signature = (
            self.work_query.clone(),
            self.work_sort,
            self.work_descending,
        );
        if self.work_cache.as_ref() == Some(&signature) {
            return;
        }
        let Some(loaded) = &self.loaded else {
            self.visible_works.clear();
            return;
        };
        let query = self.work_query.trim().to_lowercase();
        self.visible_works = loaded
            .dataset
            .works
            .iter()
            .enumerate()
            .filter(|(_, w)| {
                format!("{} {} {}", w.number, w.display_name, w.directory)
                    .to_lowercase()
                    .contains(&query)
            })
            .map(|(index, _)| index)
            .collect();
        let sort = self.work_sort;
        self.visible_works.sort_by(|&a, &b| {
            let left = &loaded.dataset.works[a];
            let right = &loaded.dataset.works[b];
            let order = match sort {
                WorkSort::Number => left.number.cmp(&right.number),
                WorkSort::Name => left
                    .display_name
                    .to_lowercase()
                    .cmp(&right.display_name.to_lowercase()),
                WorkSort::Records => loaded.summaries[a]
                    .records
                    .cmp(&loaded.summaries[b].records),
            }
            .then(a.cmp(&b));
            if self.work_descending {
                order.reverse()
            } else {
                order
            }
        });
        self.work_cache = Some(signature);
    }
    pub fn select_work(&mut self, work: usize) {
        if self
            .loaded
            .as_ref()
            .is_some_and(|loaded| work < loaded.dataset.works.len())
        {
            self.selected_work = Some(work);
            if self.selected_record.is_some_and(|key| key.work != work) {
                self.selected_record = None;
            }
        }
    }
    pub fn open_work(&mut self, work: usize) {
        if self
            .loaded
            .as_ref()
            .is_some_and(|loaded| loaded.work(work).is_some())
        {
            self.select_work(work);
            self.view = AppView::WorkDetail { work };
        }
    }
    pub fn back(&mut self) {
        self.view = AppView::Explorer;
    }
    pub fn select_record(&mut self, key: RecordKey) {
        if self
            .loaded
            .as_ref()
            .is_some_and(|loaded| loaded.record(key).is_some())
        {
            self.selected_work = Some(key.work);
            self.selected_record = Some(key);
        }
    }
}
