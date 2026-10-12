mod common;
use fc2_rs::{
    explorer::{LoadedDataset, RecordKey},
    filesystem,
};
use std::path::PathBuf;

// Synthetic SMARTFS content verifies integration, not evidence about real files.
struct Builder {
    memory: Vec<u8>,
    physical: usize,
    logical: u16,
    root_entries: usize,
}
impl Builder {
    fn new() -> Self {
        let mut b = Self {
            memory: vec![255; 0x100000],
            physical: 2,
            logical: 10,
            root_entries: 0,
        };
        b.sector(0, 0, b'S', 65535, 65535, &[]);
        b.memory[filesystem::PARTITION_ADDRESS + 20..filesystem::PARTITION_ADDRESS + 27]
            .copy_from_slice(b"SMRT\x01\x10\x00");
        b.sector(1, 3, 1, 65535, 65535, &[]);
        b
    }
    fn sector(
        &mut self,
        physical: usize,
        logical: u16,
        kind: u8,
        next: u16,
        used: u16,
        data: &[u8],
    ) {
        let start = filesystem::PARTITION_ADDRESS + physical * 512;
        let s = &mut self.memory[start..start + 512];
        s.fill(255);
        s[0] = 0x45;
        s[16..18].copy_from_slice(&logical.to_le_bytes());
        s[18] = 1;
        s[20] = kind;
        s[21..23].copy_from_slice(&next.to_le_bytes());
        s[23..25].copy_from_slice(&used.to_le_bytes());
        s[25..25 + data.len()].copy_from_slice(data);
    }
    fn entry(&mut self, physical: usize, index: usize, name: &str, first: u16, directory: bool) {
        let p = filesystem::PARTITION_ADDRESS + physical * 512 + 25 + index * 24;
        self.memory[p..p + 2]
            .copy_from_slice(&(if directory { 0x7fb6u16 } else { 0x5fb6u16 }).to_le_bytes());
        self.memory[p + 2..p + 4].copy_from_slice(&first.to_le_bytes());
        self.memory[p + 4..p + 8].fill(0);
        self.memory[p + 8..p + 24].fill(0);
        self.memory[p + 8..p + 8 + name.len()].copy_from_slice(name.as_bytes());
    }
    fn file(&mut self, data: &[u8]) -> u16 {
        let chunks: Vec<_> = data.chunks(487).collect();
        let first = self.logical;
        for (index, chunk) in chunks.iter().enumerate() {
            let logical = self.logical;
            self.logical += 1;
            let physical = self.physical;
            self.physical += 1;
            let next = if index + 1 == chunks.len() {
                65535
            } else {
                self.logical
            };
            self.sector(physical, logical, 0, next, chunk.len() as u16, chunk);
        }
        first
    }
    fn work(&mut self, number: u16, config: Option<Vec<u8>>, files: Vec<(&str, Vec<u8>)>) {
        let physical = self.physical;
        self.physical += 1;
        let logical = self.logical;
        self.logical += 1;
        self.sector(physical, logical, 1, 65535, 65535, &[]);
        self.entry(
            1,
            self.root_entries,
            &format!("trabalho{number}"),
            logical,
            true,
        );
        self.root_entries += 1;
        let mut index = 0;
        if let Some(config) = config {
            let first = self.file(&config);
            self.entry(physical, index, "config_measures", first, false);
            index += 1;
        }
        for (name, bytes) in files {
            let first = self.file(&bytes);
            self.entry(physical, index, name, first, false);
            index += 1;
        }
    }
    fn finish(mut self) -> Vec<u8> {
        for sector in self.memory[filesystem::PARTITION_ADDRESS..].chunks_exact_mut(512) {
            if sector[0] == 0x45 {
                sector[19] = filesystem::sector_crc(sector).unwrap();
            }
        }
        common::wrap(&self.memory)
    }
}
fn config(number: u16, name: &str, counters: &[u16]) -> Vec<u8> {
    let mut bytes = vec![0; 336];
    bytes[..name.len()].copy_from_slice(name.as_bytes());
    bytes[26] = 3;
    bytes[27] = counters.len() as u8;
    bytes[28..30].copy_from_slice(&number.to_le_bytes());
    for (i, count) in counters.iter().enumerate() {
        bytes[40 + 2 * i..42 + 2 * i].copy_from_slice(&count.to_le_bytes());
    }
    bytes
}
fn records(start: u32, count: usize) -> Vec<u8> {
    let mut bytes = Vec::new();
    for index in 0..count {
        let mut record = common::raw_record();
        record[36..40].copy_from_slice(&(start + index as u32).to_le_bytes());
        bytes.extend_from_slice(&record);
    }
    bytes
}

fn source() -> Vec<u8> {
    let mut builder = Builder::new();
    let mut absent = records(20, 1);
    absent[..5].fill(0);
    absent[32..34].fill(0);
    absent[40..44].copy_from_slice(&f32::NAN.to_bits().to_le_bytes());
    absent[44..48].copy_from_slice(&f32::NAN.to_bits().to_le_bytes());
    builder.work(
        10,
        Some(config(10, "SYNTHETIC", &[2, 1])),
        vec![("1", records(10, 2)), ("2", absent)],
    );
    builder.work(20, None, vec![("1", records(30, 1))]);
    builder.finish()
}
fn loaded() -> LoadedDataset {
    LoadedDataset::from_bytes(PathBuf::from("synthetic.FC2"), source()).unwrap()
}

#[test]
fn marked_works_stack_in_selection_order_and_survive_search_and_sort() {
    let mut state = State::default();
    state.install(Arc::new(loaded()));
    state.set_marked(0, true);
    state.set_marked(1, true);
    state.work_query = "no matching name".into();
    state.refresh_works();
    assert_eq!(state.visible_works, vec![1, 0]);
    state.work_descending = true;
    state.work_sort = WorkSort::Records;
    state.refresh_works();
    assert_eq!(state.visible_works, vec![1, 0]);
    state.set_marked(1, false);
    state.refresh_works();
    assert_eq!(state.visible_works, vec![0]);
    state.set_marked(0, false);
    state.refresh_works();
    assert!(state.visible_works.is_empty());
}

#[test]
fn select_all_covers_hidden_works_and_clear_invalidates_cached_list() {
    let mut state = State::default();
    state.install(Arc::new(loaded()));
    state.work_query = "20".into();
    state.refresh_works();
    assert_eq!(state.visible_works, vec![1]);
    state.mark_all(true);
    state.refresh_works();
    assert_eq!(state.marked_works.len(), 2);
    assert_eq!(state.visible_works, vec![1, 0]);
    state.mark_all(false);
    state.refresh_works();
    assert_eq!(state.visible_works, vec![1]);
    state.set_marked(999, true);
    assert!(state.marked_works.is_empty());
}
#[allow(dead_code)]
#[path = "../gui/src/state.rs"]
mod state;
use state::{AppView, RecordSort, State, TableState, WorkSort, WorkTab};
use std::sync::Arc;

#[test]
fn install_indexes_real_decoder_output_and_resets_previous_source_state() {
    let mut state = State::default();
    state.install(Arc::new(loaded()));
    assert_eq!(state.visible_works, vec![0, 1]);
    assert_eq!(state.selected_work, Some(0));
    state.open_work(1);
    state.select_record(RecordKey {
        work: 1,
        group: 0,
        record: 0,
    });
    state.marked_works.insert(1);
    state.tables.insert(1, TableState::default());
    state.work_query = "old search".into();
    state.error = Some("previous error".into());
    state.install(Arc::new(loaded()));
    assert_eq!(state.view, AppView::Explorer);
    assert_eq!(state.selected_work, Some(0));
    assert!(state.selected_record.is_none());
    assert!(state.marked_works.is_empty());
    assert!(state.tables.is_empty());
    assert!(state.work_query.is_empty());
    assert!(state.error.is_none());
}

#[test]
fn selecting_previews_and_open_back_preserve_search_table_and_tab() {
    let mut state = State::default();
    state.install(Arc::new(loaded()));
    state.work_query = "synthetic".into();
    state.refresh_works();
    assert_eq!(state.visible_works, vec![0]);
    state.select_work(1);
    assert_eq!(state.view, AppView::Explorer);
    assert_eq!(state.selected_work, Some(1));
    let table = state.tables.entry(1).or_default();
    table.query = "30".into();
    table.sort = RecordSort::Id;
    table.descending = true;
    state.tab = WorkTab::Metadata;
    state.open_work(1);
    assert_eq!(state.view, AppView::WorkDetail { work: 1 });
    state.back();
    assert_eq!(state.view, AppView::Explorer);
    assert_eq!(state.selected_work, Some(1));
    assert_eq!(state.work_query, "synthetic");
    assert_eq!(state.tab, WorkTab::Metadata);
    assert_eq!(state.tables[&1].query, "30");
    assert_eq!(state.tables[&1].sort, RecordSort::Id);
    assert!(state.tables[&1].descending);
}

#[test]
fn map_record_selection_links_work_and_invalid_indexes_are_ignored() {
    let mut state = State::default();
    state.open_work(999);
    assert_eq!(state.view, AppView::Explorer);
    state.install(Arc::new(loaded()));
    let key = RecordKey {
        work: 1,
        group: 0,
        record: 0,
    };
    state.select_record(key);
    assert_eq!(state.selected_work, Some(1));
    assert_eq!(state.selected_record, Some(key));
    for invalid in [
        RecordKey {
            work: 999,
            group: 0,
            record: 0,
        },
        RecordKey {
            work: 1,
            group: 999,
            record: 0,
        },
        RecordKey {
            work: 1,
            group: 0,
            record: 999,
        },
    ] {
        state.select_record(invalid);
        assert_eq!(state.selected_record, Some(key));
    }
    state.open_work(999);
    assert_eq!(state.view, AppView::Explorer);
    state.select_work(0);
    assert!(state.selected_record.is_none());
}

#[test]
fn table_filters_groups_warnings_missing_fields_and_sorts_stored_values() {
    let data = loaded();
    let mut table = TableState::default();
    table.refresh(&data, 0);
    assert_eq!(table.rows, data.records[0]);
    table.sort = RecordSort::Id;
    table.descending = true;
    table.refresh(&data, 0);
    let ids: Vec<_> = table
        .rows
        .iter()
        .map(|key| data.record(*key).unwrap().id)
        .collect();
    assert_eq!(ids, vec![20, 11, 10]);
    table.group = Some(0);
    table.refresh(&data, 0);
    assert_eq!(table.rows.len(), 2);
    assert!(table.rows.iter().all(|key| key.group == 0));
    table.group = None;
    table.warnings_only = true;
    table.refresh(&data, 0);
    assert_eq!(table.rows.len(), 1);
    assert_eq!(data.record(table.rows[0]).unwrap().id, 20);
    table.query = "no matching record".into();
    table.refresh(&data, 0);
    assert!(table.rows.is_empty());
    table.query = "20".into();
    table.refresh(&data, 0);
    assert_eq!(table.rows.len(), 1);
    assert!(data.record(table.rows[0]).unwrap().date.is_none());
}

#[test]
fn work_search_sort_and_selection_use_source_indexes() {
    let mut state = State::default();
    state.install(Arc::new(loaded()));
    state.work_sort = WorkSort::Records;
    state.refresh_works();
    assert_eq!(state.visible_works, vec![1, 0]);
    state.work_descending = true;
    state.refresh_works();
    assert_eq!(state.visible_works, vec![0, 1]);
    state.work_query = " 20 ".into();
    state.refresh_works();
    assert_eq!(state.visible_works, vec![1]);
    state.work_query = "absent".into();
    state.refresh_works();
    assert!(state.visible_works.is_empty());
    assert_eq!(state.selected_work, Some(0));
}

#[test]
fn table_cache_changes_when_reused_for_another_work() {
    let data = loaded();
    let mut table = TableState::default();
    table.refresh(&data, 0);
    assert_eq!(table.rows, data.records[0]);
    table.refresh(&data, 1);
    assert_eq!(table.rows, data.records[1]);
    assert!(table.rows.iter().all(|key| key.work == 1));
}
