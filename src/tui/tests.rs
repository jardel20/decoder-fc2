//! Functional tests use synthetic sectors, never experimental fixtures as format proof.
use super::*;
use crate::export::works::ExportMode;
use crate::{blocks, filesystem, records::ReferenceRow};
use std::collections::BTreeMap;

struct FixtureBuilder {
    memory: Vec<u8>,
    physical: usize,
    logical: u16,
    root_entries: usize,
}
impl FixtureBuilder {
    fn new() -> Self {
        let mut builder = Self {
            memory: vec![255; 0x100000],
            physical: 2,
            logical: 10,
            root_entries: 0,
        };
        builder.sector(0, 0, b'S', 65535, 65535, &[]);
        builder.memory[filesystem::PARTITION_ADDRESS + 20..filesystem::PARTITION_ADDRESS + 27]
            .copy_from_slice(b"SMRT\x01\x10\x00");
        builder.sector(1, 3, 1, 65535, 65535, &[]);
        builder
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
        let sector = &mut self.memory[start..start + 512];
        sector.fill(255);
        sector[0] = 0x45;
        sector[16..18].copy_from_slice(&logical.to_le_bytes());
        sector[18] = 1;
        sector[20] = kind;
        sector[21..23].copy_from_slice(&next.to_le_bytes());
        sector[23..25].copy_from_slice(&used.to_le_bytes());
        sector[25..25 + data.len()].copy_from_slice(data);
    }
    fn entry(&mut self, physical: usize, index: usize, name: &str, first: u16, directory: bool) {
        let offset = filesystem::PARTITION_ADDRESS + physical * 512 + 25 + index * 24;
        self.memory[offset..offset + 2]
            .copy_from_slice(&(if directory { 0x7fb6u16 } else { 0x5fb6u16 }).to_le_bytes());
        self.memory[offset + 2..offset + 4].copy_from_slice(&first.to_le_bytes());
        self.memory[offset + 4..offset + 24].fill(0);
        self.memory[offset + 8..offset + 8 + name.len()].copy_from_slice(name.as_bytes());
    }
    fn file(&mut self, data: &[u8]) -> u16 {
        let first = self.logical;
        let chunks: Vec<_> = data.chunks(487).collect();
        for (index, chunk) in chunks.iter().enumerate() {
            let physical = self.physical;
            let logical = self.logical;
            self.physical += 1;
            self.logical += 1;
            let next = if index + 1 == chunks.len() {
                65535
            } else {
                self.logical
            };
            self.sector(physical, logical, 0, next, chunk.len() as u16, chunk);
        }
        first
    }
    fn work(&mut self, number: u16, name: &str, groups: &[Vec<(u32, u16)>]) {
        let physical = self.physical;
        let logical = self.logical;
        self.physical += 1;
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
        let mut config = vec![0; 336];
        config[..name.len()].copy_from_slice(name.as_bytes());
        config[26] = 3;
        config[27] = groups.len() as u8;
        config[28..30].copy_from_slice(&number.to_le_bytes());
        for (index, records) in groups.iter().enumerate() {
            config[40 + index * 2..42 + index * 2]
                .copy_from_slice(&(records.len() as u16).to_le_bytes());
        }
        let first = self.file(&config);
        self.entry(physical, 0, "config_measures", first, false);
        for (index, records) in groups.iter().enumerate() {
            let data: Vec<_> = records
                .iter()
                .flat_map(|&(id, point)| record(id, point))
                .collect();
            let first = self.file(&data);
            self.entry(physical, index + 1, &(index + 1).to_string(), first, false);
        }
    }
    fn finish(mut self) -> Vec<u8> {
        for sector in self.memory[filesystem::PARTITION_ADDRESS..].chunks_exact_mut(512) {
            if sector[0] == 0x45 {
                sector[19] = filesystem::sector_crc(sector).unwrap();
            }
        }
        let count = self.memory.len() / 256;
        let mut bytes = vec![0; count * 512];
        for (index, block) in bytes.chunks_exact_mut(512).enumerate() {
            block[..4].copy_from_slice(&blocks::MAGIC);
            for (offset, value) in [
                (4, blocks::MAGIC2),
                (12, (index * 256) as u32),
                (16, 256),
                (20, index as u32),
                (24, count as u32),
                (508, blocks::TRAILER),
            ] {
                block[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            }
            block[32..288].copy_from_slice(&self.memory[index * 256..index * 256 + 256]);
        }
        bytes
    }
}
fn record(id: u32, point: u16) -> [u8; 48] {
    let mut raw = [0; 48];
    raw[..5].copy_from_slice(&[1, 2, 3, 12, 3]);
    for (offset, value) in [
        (24, 123u16),
        (26, 45),
        (28, 169),
        (30, point),
        (32, 24),
        (34, 650),
    ] {
        raw[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }
    raw[36..40].copy_from_slice(&id.to_le_bytes());
    raw[40..44].copy_from_slice(&(-12.25f32).to_le_bytes());
    raw[44..48].copy_from_slice(&34.5f32.to_le_bytes());
    raw
}
fn fixture() -> Vec<u8> {
    let mut builder = FixtureBuilder::new();
    builder.work(
        100,
        "ALPHA",
        &[vec![(101, 2), (102, 10), (103, 1)], vec![(104, 3)]],
    );
    builder.work(200, "BETA", &[vec![(201, 4)]]);
    builder.finish()
}
fn app_fixture() -> (tempfile::TempDir, PathBuf, App) {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("synthetic.FC2");
    std::fs::write(&source, fixture()).unwrap();
    let mut app = App::new(temp.path()).unwrap();
    app.open(&source).unwrap();
    assert_eq!(app.dataset().unwrap().works.len(), 2);
    (temp, source, app)
}
fn warning_app_fixture(id: u32) -> (tempfile::TempDir, PathBuf, App) {
    let mut builder = FixtureBuilder::new();
    builder.work(
        100,
        "ALPHA",
        &[vec![(101, 2), (102, 10), (103, 1)], vec![(104, 3)]],
    );
    builder.work(200, "BETA", &[vec![(201, 4)]]);
    let point = if id == 101 { 2 } else { 4 };
    let offset = builder
        .memory
        .windows(48)
        .position(|bytes| bytes == record(id, point))
        .unwrap();
    let raw = &mut builder.memory[offset..offset + 48];
    raw[..5].fill(0);
    raw[32..36].fill(0);
    raw[40..44].copy_from_slice(&0x7fc00000u32.to_le_bytes());
    raw[44..48].copy_from_slice(&0x7fc00000u32.to_le_bytes());
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("warnings.FC2");
    std::fs::write(&source, builder.finish()).unwrap();
    let mut app = App::new(temp.path()).unwrap();
    app.open(&source).unwrap();
    (temp, source, app)
}
fn type_keys(app: &mut App, value: &str) {
    for character in value.chars() {
        app.key(Key::Char(character)).unwrap();
    }
}
fn directory_names(path: &Path) -> Vec<String> {
    let mut names: Vec<_> = std::fs::read_dir(path)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}
fn complete_export_prompt(app: &mut App, output: &Path) {
    let prompt = app.prompt.as_mut().unwrap();
    prompt.value = path_value(output);
    prompt.cursor = prompt.value.len();
    app.key(Key::Enter).unwrap();
    assert!(app.prompt.is_none());
}
fn path_value(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[test]
fn destination_picker_navigates_folders_and_cancels_without_changing_selection() {
    let (temp, source, mut app) = app_fixture();
    let root = temp.path().join("Baixados");
    let child = root.join("Planilhas");
    std::fs::create_dir_all(&child).unwrap();
    std::fs::write(root.join("not-a-folder.txt"), "preserve").unwrap();
    app.export_root = root.clone();
    // Browser::read canonicalizes paths; Windows may expand short names and
    // add a verbatim prefix. Keep the input alias, normalize expected locations.
    let root = root.canonicalize().unwrap();
    let child = child.canonicalize().unwrap();
    app.submit(PromptAction::Search, "beta".into()).unwrap();
    app.key(Key::Char(' ')).unwrap();
    let old_browser = app.browser.directory.clone();
    let original = std::fs::read(&source).unwrap();
    app.key(Key::Char('e')).unwrap();
    assert_eq!(app.screen, Screen::Browser);
    assert!(app.prompt.is_none());
    assert_eq!(app.browser.directory, root);
    assert!(
        app.visible_browser()
            .iter()
            .all(|&i| app.browser.entries[i].is_dir)
    );
    app.browser.index = app
        .visible_browser()
        .iter()
        .position(|&i| app.browser.entries[i].path == child)
        .unwrap();
    app.key(Key::Enter).unwrap();
    assert_eq!(app.browser.directory, child);
    app.key(Key::Char('s')).unwrap();
    assert!(Path::new(&app.prompt.as_ref().unwrap().value).starts_with(&child));
    assert!(matches!(app.prompt.as_ref().unwrap().action,
        PromptAction::ExportSelection(Some(ref works), ExportMode::Simple) if works == &[200]));
    app.key(Key::Escape).unwrap();
    assert!(app.destination.is_some());
    app.key(Key::Escape).unwrap();
    assert!(app.destination.is_none());
    assert_eq!(app.screen, Screen::Works);
    assert_eq!(app.search, "beta");
    assert_eq!(app.selected_work().unwrap().number, 200);
    assert!(app.marked_works.contains(&200));
    assert_eq!(app.browser.directory, old_browser);
    assert_eq!(std::fs::read(source).unwrap(), original);
    assert_eq!(directory_names(&child), Vec::<String>::new());
}
fn read_json(path: &Path) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
// Independently inspect stored ZIP local headers instead of reusing the exporter.
fn zip_entries(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let mut entries = BTreeMap::new();
    let mut cursor = 0;
    while bytes.get(cursor..cursor + 4) == Some(b"PK\x03\x04") {
        let word = |offset| {
            u16::from_le_bytes(
                bytes[cursor + offset..cursor + offset + 2]
                    .try_into()
                    .unwrap(),
            ) as usize
        };
        assert_eq!(word(8), 0, "fixture exports use stored ZIP entries");
        let size = u32::from_le_bytes(bytes[cursor + 18..cursor + 22].try_into().unwrap()) as usize;
        let start = cursor + 30 + word(26) + word(28);
        let name = std::str::from_utf8(&bytes[cursor + 30..cursor + 30 + word(26)])
            .unwrap()
            .to_owned();
        entries.insert(name, bytes[start..start + size].to_vec());
        cursor = start + size;
    }
    assert!(!entries.is_empty());
    entries
}
fn reference(bytes: &[u8]) -> Reference {
    Reference {
        schema: "fc2-reference-v1".into(),
        source_fc2_sha256: reader::hash(bytes),
        source_reference_sha256: reader::hash(b"synthetic source reference"),
        source_reference_name: "synthetic-reference.xlsx".into(),
        work: "ALPHA".into(),
        confirmed_same_source: true,
        rows: [(101, 2), (102, 10), (103, 1), (104, 3)]
            .into_iter()
            .map(|(id, point)| ReferenceRow {
                id,
                point,
                date: "2024-03-12".into(),
                time: "03:02:01".into(),
                latitude: -12.25,
                longitude: 34.5,
                chlorophyll_a_tenths: 123,
                chlorophyll_b_tenths: 45,
                chlorophyll_total_tenths: 169,
            })
            .collect(),
    }
}

#[test]
fn marked_works_use_source_numbers_and_survive_hidden_search_and_failed_open() {
    let (temp, source, mut app) = app_fixture();
    app.key(Key::Char(' ')).unwrap();
    assert_eq!(
        app.marked_works.iter().copied().collect::<Vec<_>>(),
        vec![100]
    );
    app.submit(PromptAction::Search, "beta".into()).unwrap();
    assert_eq!(app.selected_work().unwrap().number, 200);
    assert_eq!(
        app.marked_works.iter().copied().collect::<Vec<_>>(),
        vec![100]
    );
    app.key(Key::Char('A')).unwrap();
    assert_eq!(
        app.marked_works.iter().copied().collect::<Vec<_>>(),
        vec![100, 200]
    );
    app.key(Key::Char(' ')).unwrap();
    assert_eq!(
        app.marked_works.iter().copied().collect::<Vec<_>>(),
        vec![100]
    );
    app.key(Key::Char('U')).unwrap();
    app.key(Key::Char('A')).unwrap();
    assert_eq!(
        app.marked_works.iter().copied().collect::<Vec<_>>(),
        vec![200]
    );
    app.key(Key::Char('M')).unwrap();
    assert!(matches!(app.export_mode, ExportMode::Complete));
    app.key(Key::Char('?')).unwrap();
    app.key(Key::Char('1')).unwrap();
    assert!(matches!(app.export_mode, ExportMode::Complete));
    assert_eq!(
        app.marked_works.iter().copied().collect::<Vec<_>>(),
        vec![200]
    );
    assert!(app.open(&temp.path().join("absent.FC2")).is_err());
    assert_eq!(
        app.marked_works.iter().copied().collect::<Vec<_>>(),
        vec![200]
    );
    assert!(matches!(app.export_mode, ExportMode::Complete));
    app.open(&source).unwrap();
    assert!(app.marked_works.is_empty());
    assert!(matches!(app.export_mode, ExportMode::Complete));
}

#[test]
fn simple_exports_current_or_marked_works_only_as_xlsx_with_all_groups() {
    let (temp, source, mut app) = app_fixture();
    let original = std::fs::read(&source).unwrap();
    let dataset = serde_json::to_vec(app.dataset().unwrap()).unwrap();
    assert!(matches!(app.export_mode, ExportMode::Simple));
    app.submit(PromptAction::Search, "beta".into()).unwrap();
    app.key(Key::Char('e')).unwrap();
    app.key(Key::Char('s')).unwrap();
    assert!(matches!(
        app.prompt.as_ref().unwrap().action,
        PromptAction::ExportSelection(Some(ref numbers), ExportMode::Simple) if numbers == &[200]
    ));
    let single = temp.path().join("simple-current");
    complete_export_prompt(&mut app, &single);
    assert_eq!(directory_names(&single), vec!["200-BETA.xlsx"]);
    let beta = zip_entries(&std::fs::read(single.join("200-BETA.xlsx")).unwrap());
    let sheet = std::str::from_utf8(&beta["xl/worksheets/sheet1.xml"]).unwrap();
    assert!(sheet.contains("<v>201</v>"));

    app.key(Key::Char('A')).unwrap();
    app.submit(PromptAction::Search, "alpha".into()).unwrap();
    app.key(Key::Char(' ')).unwrap();
    assert_eq!(
        app.marked_works.iter().copied().collect::<Vec<_>>(),
        vec![100, 200]
    );
    app.key(Key::Enter).unwrap();
    app.submit(PromptAction::Search, "102".into()).unwrap();
    assert_eq!(app.visible_records().len(), 1);
    app.key(Key::Char('e')).unwrap();
    app.key(Key::Char('s')).unwrap();
    assert!(matches!(
        app.prompt.as_ref().unwrap().action,
        PromptAction::ExportSelection(Some(ref numbers), ExportMode::Simple) if numbers == &[100, 200]
    ));
    let selected = temp.path().join("simple-marked");
    complete_export_prompt(&mut app, &selected);
    assert_eq!(
        directory_names(&selected),
        vec!["100-ALPHA.xlsx", "200-BETA.xlsx"]
    );
    let alpha = zip_entries(&std::fs::read(selected.join("100-ALPHA.xlsx")).unwrap());
    let first = std::str::from_utf8(&alpha["xl/worksheets/sheet1.xml"]).unwrap();
    let second = std::str::from_utf8(&alpha["xl/worksheets/sheet2.xml"]).unwrap();
    for id in [101, 102, 103] {
        assert!(first.contains(&format!("<v>{id}</v>")));
    }
    assert!(second.contains("<v>104</v>"));
    assert_eq!(
        app.marked_works.iter().copied().collect::<Vec<_>>(),
        vec![100, 200]
    );
    assert_eq!(serde_json::to_vec(app.dataset().unwrap()).unwrap(), dataset);
    assert_eq!(std::fs::read(source).unwrap(), original);
}

#[test]
fn complete_marked_export_limits_raw_and_json_scope_and_prompt_is_a_snapshot() {
    let (temp, source, mut app) = app_fixture();
    let original = std::fs::read(&source).unwrap();
    app.key(Key::Char('M')).unwrap();
    app.submit(PromptAction::Search, "beta".into()).unwrap();
    app.key(Key::Char(' ')).unwrap();
    app.submit(PromptAction::Search, "alpha".into()).unwrap();
    app.key(Key::Char('e')).unwrap();
    app.key(Key::Char('s')).unwrap();
    assert!(matches!(
        app.prompt.as_ref().unwrap().action,
        PromptAction::ExportSelection(Some(ref numbers), ExportMode::Complete) if numbers == &[200]
    ));
    // Changing the App after the prompt was opened must not change its captured action.
    app.export_mode = ExportMode::Simple;
    app.marked_works.insert(100);
    let output = temp.path().join("complete-snapshot");
    complete_export_prompt(&mut app, &output);
    assert!(output.join("index.html").is_file());
    assert!(!output.join("100-ALPHA.xlsx").exists());
    assert!(output.join("200-BETA.xlsx").is_file());
    let audit = read_json(&output.join("dataset.json"));
    assert_eq!(audit["works"].as_array().unwrap().len(), 1);
    assert_eq!(audit["works"][0]["number"], 200);
    let raw = zip_entries(&std::fs::read(output.join("dados-brutos.zip")).unwrap());
    assert!(raw.contains_key("filesystem/trabalho200/1"));
    assert!(!raw.keys().any(|name| name.contains("trabalho100")));
    assert!(!raw.contains_key("original.FC2"));
    assert!(!raw.contains_key("memory.bin"));
    assert!(matches!(app.export_mode, ExportMode::Simple));
    assert_eq!(
        app.marked_works.iter().copied().collect::<Vec<_>>(),
        vec![100, 200]
    );
    assert_eq!(std::fs::read(source).unwrap(), original);
}

#[test]
fn export_all_ignores_hidden_marks_and_failed_output_preserves_state_for_retry() {
    let (temp, source, mut app) = app_fixture();
    let original = std::fs::read(&source).unwrap();
    app.submit(PromptAction::Search, "beta".into()).unwrap();
    app.key(Key::Char(' ')).unwrap();
    app.key(Key::Char('E')).unwrap();
    app.key(Key::Char('s')).unwrap();
    assert!(matches!(
        app.prompt.as_ref().unwrap().action,
        PromptAction::ExportSelection(None, ExportMode::Simple)
    ));
    let existing = temp.path().join("existing");
    std::fs::create_dir(&existing).unwrap();
    std::fs::write(existing.join("sentinel"), b"preserve").unwrap();
    app.prompt.as_mut().unwrap().value = path_value(&existing);
    assert!(app.key(Key::Enter).is_err());
    assert!(app.prompt.as_ref().unwrap().error.is_some());
    assert_eq!(directory_names(&existing), vec!["sentinel"]);
    assert_eq!(
        std::fs::read(existing.join("sentinel")).unwrap(),
        b"preserve"
    );
    assert_eq!(app.search, "beta");
    assert_eq!(app.selected_work().unwrap().number, 200);
    assert_eq!(
        app.marked_works.iter().copied().collect::<Vec<_>>(),
        vec![200]
    );
    assert!(matches!(app.export_mode, ExportMode::Simple));
    let output = temp.path().join("simple-all-retry");
    complete_export_prompt(&mut app, &output);
    assert_eq!(
        directory_names(&output),
        vec!["100-ALPHA.xlsx", "200-BETA.xlsx"]
    );
    assert_eq!(app.search, "beta");
    assert_eq!(
        app.marked_works.iter().copied().collect::<Vec<_>>(),
        vec![200]
    );
    assert_eq!(std::fs::read(source).unwrap(), original);
}

#[test]
fn live_search_escape_restores_query_work_group_and_point_selection() {
    let (_temp, _source, mut app) = app_fixture();
    app.submit(PromptAction::Search, "alpha".into()).unwrap();
    app.group_index = 1;
    app.point_index = 0;
    app.point_filter = Some(3);
    app.key(Key::Char('/')).unwrap();
    app.key(Key::Char('\u{15}')).unwrap();
    type_keys(&mut app, "beta");
    assert!(app.prompt.is_some(), "search applies before pressing Enter");
    assert_eq!(app.search, "beta");
    assert_eq!(app.selected_work().unwrap().number, 200);
    assert_eq!(app.group_index, 0);
    assert_eq!(app.point_filter, None);
    app.key(Key::Escape).unwrap();
    assert!(app.prompt.is_none());
    assert_eq!(app.search, "alpha");
    assert_eq!(app.selected_work().unwrap().number, 100);
    assert_eq!(app.group_index, 1);
    assert_eq!(app.point_filter, Some(3));
    assert_eq!(app.selected_group().unwrap().records[0].id, 104);

    app.key(Key::Char('/')).unwrap();
    app.key(Key::Char('\u{15}')).unwrap();
    type_keys(&mut app, "beta");
    app.key(Key::Enter).unwrap();
    assert!(app.prompt.is_none());
    assert_eq!(app.search, "beta");
    assert_eq!(app.selected_work().unwrap().number, 200);
    app.key(Key::Escape).unwrap();
    assert_eq!(app.search, "");
    assert_eq!(app.selected_work().unwrap().number, 200);

    app.submit(PromptAction::Search, "alpha".into()).unwrap();
    app.key(Key::Enter).unwrap();
    app.submit(PromptAction::Search, "102".into()).unwrap();
    assert_eq!(app.selected_record().unwrap().id, 102);
    app.key(Key::Char('/')).unwrap();
    app.key(Key::Char('\u{15}')).unwrap();
    type_keys(&mut app, "103");
    assert_eq!(app.selected_record().unwrap().id, 103);
    app.key(Key::Escape).unwrap();
    assert_eq!(app.search, "102");
    assert_eq!(app.selected_record().unwrap().id, 102);
}

#[test]
fn warning_and_search_filters_never_remove_records_from_work_or_all_exports() {
    let (temp, source, mut app) = warning_app_fixture(101);
    let original = std::fs::read(&source).unwrap();
    let dataset = serde_json::to_vec(app.dataset().unwrap()).unwrap();
    app.navigate(1);
    assert_eq!(app.selected_work().unwrap().number, 200);
    app.key(Key::Char('f')).unwrap();
    assert_eq!(app.visible_works(), vec![0]);
    assert_eq!(app.selected_work().unwrap().number, 100);
    app.key(Key::Enter).unwrap();
    assert_eq!(app.visible_records(), vec![0]);
    assert_eq!(app.selected_record().unwrap().id, 101);
    app.key(Key::Char('w')).unwrap();
    assert_eq!(app.screen, Screen::Warnings);
    app.key(Key::Char('2')).unwrap();
    assert_eq!(app.visible_records(), vec![0]);

    let selected = temp.path().join("filtered-selected");
    app.submit(PromptAction::Export(Some(100)), path_value(&selected))
        .unwrap();
    let audit = read_json(&selected.join("dataset.json"));
    assert_eq!(audit["works"].as_array().unwrap().len(), 1);
    assert_eq!(
        audit["works"][0]["groups"][0]["records"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        audit["works"][0]["groups"][1]["records"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let all = temp.path().join("filtered-all");
    app.submit(PromptAction::Export(None), path_value(&all))
        .unwrap();
    assert_eq!(
        read_json(&all.join("dataset.json"))["works"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(serde_json::to_vec(app.dataset().unwrap()).unwrap(), dataset);
    assert_eq!(std::fs::read(source).unwrap(), original);
    app.key(Key::Char('l')).unwrap();
    assert!(!app.only_warnings);
    assert_eq!(app.visible_records().len(), 3);
}

#[test]
fn point_summary_selects_group_point_but_export_keeps_complete_work() {
    let (temp, _source, mut app) = app_fixture();
    let dataset = serde_json::to_vec(app.dataset().unwrap()).unwrap();
    app.key(Key::Char('6')).unwrap();
    assert_eq!(app.screen, Screen::Points);
    assert_eq!(app.selected_point(), Some(1));
    app.key(Key::ArrowDown).unwrap();
    assert_eq!(app.selected_point(), Some(2));
    app.key(Key::Enter).unwrap();
    assert_eq!(app.screen, Screen::Measurements);
    assert_eq!(app.point_filter, Some(2));
    assert_eq!(app.visible_records(), vec![0]);
    assert_eq!(app.selected_record().unwrap().id, 101);
    let output = temp.path().join("point-selected");
    app.submit(PromptAction::Export(Some(100)), path_value(&output))
        .unwrap();
    let audit = read_json(&output.join("dataset.json"));
    assert_eq!(
        audit["works"][0]["groups"][0]["records"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        audit["works"][0]["groups"][1]["records"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    app.key(Key::ArrowRight).unwrap();
    assert_eq!(app.group_index, 1);
    assert_eq!(app.point_filter, None);
    assert_eq!(app.selected_record().unwrap().id, 104);
    app.key(Key::Char('6')).unwrap();
    app.key(Key::Enter).unwrap();
    assert_eq!(app.point_filter, Some(3));
    app.key(Key::Char('n')).unwrap();
    assert_eq!(app.selected_work().unwrap().number, 200);
    assert_eq!(app.group_index, 0);
    assert_eq!(app.point_filter, None);
    assert_eq!(app.point_index, 0);
    assert_eq!(app.selected_record().unwrap().id, 201);
    assert_eq!(serde_json::to_vec(app.dataset().unwrap()).unwrap(), dataset);
}

#[test]
fn action_menu_blocks_csv_without_profile_and_keeps_filtered_export_selection() {
    let (_temp, _source, mut app) = app_fixture();
    app.submit(PromptAction::Search, "beta".into()).unwrap();
    app.key(Key::Char(':')).unwrap();
    let csv = app
        .visible_actions()
        .iter()
        .position(|action| action.key == 'c')
        .unwrap();
    assert!(!app.visible_actions()[csv].enabled);
    app.action_menu.as_mut().unwrap().index = csv;
    app.key(Key::Enter).unwrap();
    assert!(app.prompt.is_none());
    assert!(app.action_menu.is_some());
    assert_eq!(app.selected_work().unwrap().number, 200);
    assert!(app.status.contains("indisponível"));
    app.key(Key::Escape).unwrap();
    app.key(Key::Char(':')).unwrap();
    type_keys(&mut app, "marcados / atual");
    assert_eq!(app.visible_actions().len(), 1);
    assert_eq!(app.visible_actions()[0].key, 'e');
    app.key(Key::Enter).unwrap();
    app.key(Key::Char('s')).unwrap();
    assert!(app.action_menu.is_none());
    assert!(matches!(
        app.prompt.as_ref().unwrap().action,
        PromptAction::ExportSelection(Some(ref numbers), ExportMode::Simple) if numbers == &[200]
    ));
    assert_eq!(app.selected_work().unwrap().number, 200);
    app.key(Key::Escape).unwrap();
    assert_eq!(app.search, "beta");
    assert_eq!(app.selected_work().unwrap().number, 200);
}

#[test]
fn technical_details_and_screen_changes_keep_measurement_and_original_bytes() {
    let (_temp, source, mut app) = app_fixture();
    let original = std::fs::read(&source).unwrap();
    let dataset = serde_json::to_vec(app.dataset().unwrap()).unwrap();
    app.key(Key::Enter).unwrap();
    app.submit(PromptAction::Search, "102".into()).unwrap();
    assert_eq!(app.selected_record().unwrap().id, 102);
    app.key(Key::Char('t')).unwrap();
    assert!(app.technical_details);
    app.key(Key::Char('?')).unwrap();
    app.key(Key::Char('2')).unwrap();
    assert_eq!(app.selected_record().unwrap().id, 102);
    app.key(Key::Enter).unwrap();
    assert_eq!(app.screen, Screen::Record);
    app.key(Key::Char('t')).unwrap();
    assert!(!app.technical_details);
    assert_eq!(app.selected_record().unwrap().id, 102);
    assert_eq!(serde_json::to_vec(app.dataset().unwrap()).unwrap(), dataset);
    assert_eq!(app.source().unwrap().bytes, original);
    assert_eq!(std::fs::read(source).unwrap(), original);
}

#[test]
fn browser_round_trip_keeps_selected_filtered_work_even_with_warning_filter() {
    let (_temp, _source, mut app) = app_fixture();
    app.submit(PromptAction::Search, "beta".into()).unwrap();
    app.key(Key::Char('o')).unwrap();
    assert_eq!(app.screen, Screen::Browser);
    assert_eq!(app.selected_work().unwrap().number, 200);
    app.key(Key::Char('e')).unwrap();
    app.key(Key::Char('s')).unwrap();
    assert!(matches!(
        app.prompt.as_ref().unwrap().action,
        PromptAction::ExportSelection(Some(ref numbers), ExportMode::Simple) if numbers == &[200]
    ));
    app.key(Key::Escape).unwrap();
    app.key(Key::Escape).unwrap();
    app.key(Key::Escape).unwrap();
    assert_eq!(app.screen, Screen::Works);
    assert_eq!(app.selected_work().unwrap().number, 200);

    let (_temp, _source, mut app) = warning_app_fixture(201);
    app.key(Key::Char('f')).unwrap();
    assert_eq!(app.visible_works(), vec![1]);
    assert_eq!(app.selected_work().unwrap().number, 200);
    app.key(Key::Char('o')).unwrap();
    assert_eq!(app.selected_work().unwrap().number, 200);
    app.key(Key::Escape).unwrap();
    assert_eq!(app.visible_works(), vec![1]);
    assert_eq!(app.selected_work().unwrap().number, 200);
}

#[test]
fn filtered_work_selection_survives_enter_and_record_order_group_navigation() {
    let (_temp, _source, mut app) = app_fixture();
    app.submit(PromptAction::Search, "beta".into()).unwrap();
    assert_eq!(app.visible_works(), vec![1]);
    assert_eq!(app.selected_work().unwrap().number, 200);
    app.key(Key::Enter).unwrap();
    assert_eq!(app.screen, Screen::Measurements);
    assert_eq!(app.selected_work().unwrap().number, 200);
    assert_eq!(app.selected_record().unwrap().id, 201);
    app.key(Key::Escape).unwrap();
    app.submit(PromptAction::Search, "alpha".into()).unwrap();
    app.key(Key::Enter).unwrap();
    assert_eq!(app.visible_records(), vec![2, 1, 0]);
    assert_eq!(app.selected_record().unwrap().id, 103);
    app.key(Key::Char('s')).unwrap();
    assert_eq!(app.visible_records(), vec![0, 1, 2]);
    assert_eq!(
        app.selected_record().unwrap().id,
        103,
        "sorting must preserve selected measurement"
    );
    app.submit(PromptAction::Search, "102".into()).unwrap();
    assert_eq!(app.visible_records(), vec![1]);
    assert_eq!(app.selected_record().unwrap().point, 10);
    app.submit(PromptAction::Search, String::new()).unwrap();
    app.key(Key::ArrowRight).unwrap();
    assert_eq!(app.selected_group().unwrap().group, 2);
    assert_eq!(app.selected_record().unwrap().id, 104);
    app.key(Key::ArrowLeft).unwrap();
    assert_eq!(app.selected_group().unwrap().group, 1);
}

#[test]
fn record_details_and_hex_follow_exact_source_segments() {
    let (_temp, _source, mut app) = app_fixture();
    app.key(Key::Enter).unwrap();
    app.submit(PromptAction::Search, "102".into()).unwrap();
    app.key(Key::Enter).unwrap();
    assert_eq!(app.screen, Screen::Record);
    app.key(Key::Char('t')).unwrap();
    let frame = view::render(&app, 160, 35);
    assert!(frame.contains("medição 102"));
    assert!(frame.contains("ICF A: 12.3"));
    assert!(frame.contains("ICF Total: 16.9"));
    let segment = app.selected_record().unwrap().segments[0].clone();
    app.key(Key::Enter).unwrap();
    assert_eq!(app.screen, Screen::Hex);
    assert_eq!(app.hex_offset, segment.file_offset);
    assert_eq!(
        &app.hex_bytes()[segment.file_offset..segment.file_offset + 48],
        &record(102, 10)
    );
    app.key(Key::Char('m')).unwrap();
    app.submit(
        PromptAction::Offset,
        format!("0x{:x}", segment.memory_address),
    )
    .unwrap();
    assert_eq!(
        &app.hex_bytes()[app.hex_offset..app.hex_offset + 48],
        &record(102, 10)
    );
    let offset = app.hex_offset;
    assert!(app.submit(PromptAction::Offset, "0x100000".into()).is_err());
    assert_eq!(app.hex_offset, offset);
}

#[test]
fn selected_export_preserves_scope_full_group_coverage_and_refuses_overwrite_or_tampering() {
    let (temp, source, mut app) = app_fixture();
    let original = std::fs::read(&source).unwrap();
    app.submit(PromptAction::Search, "beta".into()).unwrap();
    app.key(Key::Char('e')).unwrap();
    app.key(Key::Char('s')).unwrap();
    assert!(matches!(
        app.prompt.take().unwrap().action,
        PromptAction::ExportSelection(Some(ref numbers), ExportMode::Simple) if numbers == &[200]
    ));
    let output = temp.path().join("selected");
    app.submit(PromptAction::Export(Some(100)), path_value(&output))
        .unwrap();
    let dataset = read_json(&output.join("dataset.json"));
    assert_eq!(dataset["works"].as_array().unwrap().len(), 1);
    assert_eq!(dataset["works"][0]["number"], 100);
    assert_eq!(dataset["works"][0]["groups"].as_array().unwrap().len(), 2);
    let raw = zip_entries(&std::fs::read(output.join("dados-brutos.zip")).unwrap());
    assert!(!raw.contains_key("original.FC2"));
    assert!(!raw.contains_key("memory.bin"));
    assert!(raw.contains_key("filesystem/trabalho100/1"));
    assert!(raw.keys().all(|name| !name.contains("trabalho200")));
    let workbook = std::fs::read(output.join("100-ALPHA.xlsx")).unwrap();
    assert!(
        app.submit(PromptAction::Export(Some(100)), path_value(&output))
            .is_err()
    );
    assert_eq!(
        std::fs::read(output.join("100-ALPHA.xlsx")).unwrap(),
        workbook
    );
    app.file.as_mut().unwrap().dataset.as_mut().unwrap().works[0].groups[0]
        .records
        .pop();
    let bad = temp.path().join("tampered");
    assert!(
        app.submit(PromptAction::Export(Some(100)), path_value(&bad))
            .is_err()
    );
    assert!(!bad.exists(), "coverage validation must precede creation");
    assert_eq!(std::fs::read(source).unwrap(), original);
}

#[test]
fn backup_audit_and_analysis_preserve_original_and_expose_all_sources() {
    let (temp, source, mut app) = app_fixture();
    let original = std::fs::read(&source).unwrap();
    let backup = temp.path().join("backup.zip");
    let audit = temp.path().join("audit.json");
    let analysis = temp.path().join("analysis.json");
    app.submit(PromptAction::Backup, path_value(&backup))
        .unwrap();
    let entries = zip_entries(&std::fs::read(&backup).unwrap());
    assert_eq!(entries["original.FC2"], original);
    assert_eq!(entries["memory.bin"].len(), 0x100000);
    assert!(entries.contains_key("filesystem/trabalho100/1"));
    assert!(entries.contains_key("filesystem/trabalho200/1"));
    app.submit(PromptAction::Audit, path_value(&audit)).unwrap();
    assert_eq!(read_json(&audit)["works"].as_array().unwrap().len(), 2);
    app.submit(PromptAction::Analysis, path_value(&analysis))
        .unwrap();
    assert_eq!(read_json(&analysis)["sha256"], reader::hash(&original));
    let previous = std::fs::read(&backup).unwrap();
    assert!(
        app.submit(PromptAction::Backup, path_value(&backup))
            .is_err()
    );
    assert_eq!(std::fs::read(backup).unwrap(), previous);
    assert_eq!(std::fs::read(source).unwrap(), original);
}

#[test]
fn reference_wizard_validates_then_switching_source_removes_profile() {
    let (temp, source, mut app) = app_fixture();
    let reference = reference(&std::fs::read(&source).unwrap());
    let ref_path = temp.path().join("reference.json");
    let ref_source = temp.path().join("reference.xlsx");
    let profile_path = temp.path().join("profile.json");
    std::fs::write(&ref_path, json(&reference).unwrap()).unwrap();
    std::fs::write(&ref_source, b"synthetic source reference").unwrap();
    app.submit(PromptAction::Reference, path_value(&ref_path))
        .unwrap();
    assert!(matches!(
        app.prompt.take().unwrap().action,
        PromptAction::ReferenceSource(_)
    ));
    app.submit(
        PromptAction::ReferenceSource(ref_path.clone()),
        path_value(&ref_source),
    )
    .unwrap();
    assert!(matches!(
        app.prompt.take().unwrap().action,
        PromptAction::SaveProfile(_, _)
    ));
    app.submit(
        PromptAction::SaveProfile(ref_path, ref_source),
        path_value(&profile_path),
    )
    .unwrap();
    assert!(app.profile.is_some());
    assert!(
        app.dataset().unwrap().works[0]
            .reference_validation
            .as_ref()
            .unwrap()
            .accepted
    );
    let csv = temp.path().join("reference.csv");
    app.submit(PromptAction::Csv, path_value(&csv)).unwrap();
    assert!(std::fs::read_to_string(csv).unwrap().contains("ALPHA"));
    app.load_profile(&profile_path).unwrap();
    app.set_screen(Screen::Works);
    app.navigate(1);
    assert_eq!(app.selected_work().unwrap().number, 200);
    app.key(Key::Char(':')).unwrap();
    let actions = app.visible_actions();
    let csv_action = actions.iter().find(|action| action.key == 'c').unwrap();
    assert!(csv_action.enabled);
    assert!(csv_action.description.contains("perfil"));
    app.key(Key::Escape).unwrap();
    app.key(Key::Char('c')).unwrap();
    assert!(app.prompt.as_ref().unwrap().label.contains("ALPHA"));
    assert!(matches!(
        app.prompt.as_ref().unwrap().action,
        PromptAction::Csv
    ));
    app.key(Key::Escape).unwrap();
    assert_eq!(app.selected_work().unwrap().number, 200);
    let previous_hash = app.dataset().unwrap().source_sha256.clone();
    assert!(app.open(&temp.path().join("missing.FC2")).is_err());
    assert!(app.profile.is_some());
    assert_eq!(app.dataset().unwrap().source_sha256, previous_hash);
    let second = temp.path().join("second.FC2");
    std::fs::write(&second, [1, 2, 3]).unwrap();
    app.open(&second).unwrap();
    assert!(app.profile.is_none());
    assert!(app.dataset().is_none());
    assert!(
        app.submit(
            PromptAction::Csv,
            path_value(&temp.path().join("blocked.csv"))
        )
        .is_err()
    );
}

#[test]
fn divergent_or_wrong_source_reference_never_creates_profile() {
    let (temp, source, mut app) = app_fixture();
    let mut reference = reference(&std::fs::read(source).unwrap());
    let normalized = temp.path().join("reference.json");
    let source = temp.path().join("reference.xlsx");
    let profile = temp.path().join("rejected-profile.json");
    std::fs::write(&normalized, json(&reference).unwrap()).unwrap();
    std::fs::write(&source, b"edited reference").unwrap();
    assert!(
        app.submit(
            PromptAction::SaveProfile(normalized.clone(), source.clone()),
            path_value(&profile)
        )
        .is_err()
    );
    assert!(!profile.exists());
    assert!(app.profile.is_none());
    std::fs::write(&source, b"synthetic source reference").unwrap();
    reference.rows[0].chlorophyll_total_tenths += 1;
    std::fs::write(&normalized, json(&reference).unwrap()).unwrap();
    assert!(
        app.submit(
            PromptAction::SaveProfile(normalized, source),
            path_value(&profile)
        )
        .is_err()
    );
    assert!(!profile.exists());
    assert!(app.profile.is_none());
    assert!(
        app.dataset().unwrap().works[0]
            .reference_validation
            .is_none()
    );
}
