mod common;
use fc2_rs::{
    export::works::{self, ExportMode, WorkbookOrder},
    filesystem, workspace,
};
use std::collections::BTreeMap;

#[test]
#[cfg(not(windows))]
fn cli_default_destination_uses_configured_downloads_outside_source_directory() {
    let root = tempfile::tempdir().unwrap();
    let downloads = root.path().join("Baixados");
    let config = root.path().join("config");
    let sources = root.path().join("source");
    for path in [&downloads, &config, &sources] {
        std::fs::create_dir(path).unwrap();
    }
    std::fs::write(
        config.join("user-dirs.dirs"),
        format!("XDG_DOWNLOAD_DIR=\"{}\"\n", downloads.display()),
    )
    .unwrap();
    let (bytes, _) = fixture();
    let input = sources.join("synthetic.FC2");
    std::fs::write(&input, &bytes).unwrap();
    for (mode, name) in [
        ("simple", "synthetic-trabalhos"),
        ("complete", "synthetic-trabalhos-1"),
    ] {
        let result = std::process::Command::new(env!("CARGO_BIN_EXE_fc2"))
            .current_dir(&sources)
            .env("HOME", root.path())
            .env("XDG_CONFIG_HOME", &config)
            .arg("export")
            .arg(&input)
            .args(["--mode", mode])
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let destination = downloads.join(name);
        assert!(destination.is_dir());
        assert!(std::fs::read_dir(&destination).unwrap().any(|entry| {
            entry
                .unwrap()
                .path()
                .extension()
                .is_some_and(|ext| ext == "xlsx")
        }));
        assert_eq!(destination.join("index.html").exists(), mode == "complete");
    }
    assert_eq!(std::fs::read(&input).unwrap(), bytes);
    assert_eq!(std::fs::read_dir(&sources).unwrap().count(), 1);
}

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

// These artificial SMARTFS records exercise export policy, not evidence of the real format.
fn fixture() -> (Vec<u8>, workspace::Dataset) {
    let mut builder = Builder::new();
    builder.work(
        10,
        Some(config(10, "ONE", &[2, 1])),
        vec![("1", records(10, 2)), ("2", records(20, 1))],
    );
    builder.work(
        20,
        Some(config(20, "TWO", &[2])),
        vec![("1", records(30, 2))],
    );
    builder.work(
        30,
        Some(config(30, "EXCLUDED", &[1])),
        vec![("1", records(40, 1))],
    );
    let bytes = builder.finish();
    let dataset = workspace::decode(&bytes).unwrap();
    assert!(dataset.works.iter().all(|work| work.errors.is_empty()));
    (bytes, dataset)
}

fn zip_entries(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let mut result = BTreeMap::new();
    let mut offset = 0;
    while bytes.get(offset..offset + 4) == Some(b"PK\x03\x04") {
        let header = &bytes[offset..offset + 30];
        assert_eq!(u16::from_le_bytes(header[8..10].try_into().unwrap()), 0);
        let length = u32::from_le_bytes(header[18..22].try_into().unwrap()) as usize;
        let name_length = u16::from_le_bytes(header[26..28].try_into().unwrap()) as usize;
        let extra_length = u16::from_le_bytes(header[28..30].try_into().unwrap()) as usize;
        let name = std::str::from_utf8(&bytes[offset + 30..offset + 30 + name_length]).unwrap();
        let start = offset + 30 + name_length + extra_length;
        assert!(
            result
                .insert(name.to_owned(), bytes[start..start + length].to_vec())
                .is_none()
        );
        offset = start + length;
    }
    assert_eq!(
        bytes.get(offset..offset + 4),
        Some(b"PK\x01\x02".as_slice())
    );
    result
}

#[test]
fn simple_selection_writes_only_two_workbooks_and_keeps_every_group() {
    let (bytes, dataset) = fixture();
    let temporary = tempfile::tempdir().unwrap();
    let output = temporary.path().join("simple");
    let report = works::export_directory_options(
        &bytes,
        &dataset,
        "synthetic.FC2",
        &output,
        Some(&[20, 10]),
        WorkbookOrder::Stored,
        ExportMode::Simple,
    )
    .unwrap();
    assert_eq!(report.successful_workbooks, 2);
    assert_eq!(report.failures, 0);
    assert_eq!(report.record_count, 5);
    assert_eq!(report.group_count, 3);
    assert_eq!(
        report
            .works
            .iter()
            .map(|work| work.number)
            .collect::<Vec<_>>(),
        vec![10, 20]
    );
    let mut names: Vec<_> = std::fs::read_dir(&output)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, ["10-ONE.xlsx", "20-TWO.xlsx"]);
    let workbook = zip_entries(&std::fs::read(output.join("10-ONE.xlsx")).unwrap());
    let xml = std::str::from_utf8(&workbook["xl/workbook.xml"]).unwrap();
    assert!(xml.contains("Medidas 1"));
    assert!(xml.contains("Medidas 2"));
    assert!(workbook.contains_key("xl/worksheets/sheet2.xml"));
}

#[test]
fn complete_selection_limits_every_companion_to_selected_works() {
    let (bytes, dataset) = fixture();
    let temporary = tempfile::tempdir().unwrap();
    let output = temporary.path().join("complete");
    let report = works::export_directory_options(
        &bytes,
        &dataset,
        "synthetic.FC2",
        &output,
        Some(&[10, 20]),
        WorkbookOrder::Leaf,
        ExportMode::Complete,
    )
    .unwrap();
    assert_eq!(report.successful_workbooks, 2);
    let audit: serde_json::Value =
        serde_json::from_slice(&std::fs::read(output.join("dataset.json")).unwrap()).unwrap();
    assert_eq!(
        audit["works"]
            .as_array()
            .unwrap()
            .iter()
            .map(|work| work["number"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        vec![10, 20]
    );
    let files = audit["filesystem"]["files"].as_array().unwrap();
    assert_eq!(files.len(), 5);
    assert!(files.iter().all(|file| {
        let path = file["path"].as_str().unwrap();
        path.starts_with("/trabalho10/") || path.starts_with("/trabalho20/")
    }));
    assert_eq!(
        audit["filesystem"]["directories"].as_array().unwrap().len(),
        2
    );
    let raw = zip_entries(&std::fs::read(output.join("dados-brutos.zip")).unwrap());
    assert_eq!(raw.len(), 6);
    assert!(!raw.contains_key("original.FC2"));
    assert!(!raw.contains_key("memory.bin"));
    assert!(raw.keys().all(|name| name == "provenance.json"
        || name.starts_with("filesystem/trabalho10/")
        || name.starts_with("filesystem/trabalho20/")));
    let provenance: serde_json::Value = serde_json::from_slice(&raw["provenance.json"]).unwrap();
    assert_eq!(provenance["works"], serde_json::json!([10, 20]));
    let bundle = zip_entries(&std::fs::read(output.join("todos-trabalhos.zip")).unwrap());
    assert_eq!(
        bundle.keys().map(String::as_str).collect::<Vec<_>>(),
        ["10-ONE.xlsx", "20-TWO.xlsx", "manifest.json"]
    );
    let page = std::fs::read_to_string(output.join("index.html")).unwrap();
    assert!(page.contains("10-ONE.xlsx"));
    assert!(page.contains("20-TWO.xlsx"));
    assert!(!page.contains("EXCLUDED"));
    assert_eq!(std::fs::read_dir(output).unwrap().count(), 7);
}

#[test]
fn invalid_selections_never_create_output_or_parent_directories() {
    let (bytes, dataset) = fixture();
    let temporary = tempfile::tempdir().unwrap();
    for (index, selection) in [&[][..], &[10, 99][..], &[10, 10][..]]
        .into_iter()
        .enumerate()
    {
        for mode in [ExportMode::Simple, ExportMode::Complete] {
            let parent = temporary.path().join(format!("invalid-{index}-{mode:?}"));
            assert!(
                works::export_directory_options(
                    &bytes,
                    &dataset,
                    "synthetic.FC2",
                    &parent.join("out"),
                    Some(selection),
                    WorkbookOrder::Stored,
                    mode
                )
                .is_err()
            );
            assert!(!parent.exists());
        }
    }
}

#[test]
fn simple_mode_still_rejects_changed_scientific_values_before_creating_output() {
    let (bytes, mut dataset) = fixture();
    dataset.works[0].groups[0].records[0].chlorophyll_a_tenths += 1;
    let temporary = tempfile::tempdir().unwrap();
    let output = temporary.path().join("tampered");
    assert!(
        works::export_directory_options(
            &bytes,
            &dataset,
            "synthetic.FC2",
            &output,
            Some(&[10]),
            WorkbookOrder::Stored,
            ExportMode::Simple
        )
        .is_err()
    );
    assert!(!output.exists());
}

#[test]
fn legacy_complete_wrapper_matches_explicit_single_selection() {
    let (bytes, dataset) = fixture();
    let temporary = tempfile::tempdir().unwrap();
    let legacy = temporary.path().join("legacy");
    let explicit = temporary.path().join("explicit");
    works::export_directory(
        &bytes,
        &dataset,
        "synthetic.FC2",
        &legacy,
        Some(10),
        WorkbookOrder::Stored,
    )
    .unwrap();
    works::export_directory_options(
        &bytes,
        &dataset,
        "synthetic.FC2",
        &explicit,
        Some(&[10]),
        WorkbookOrder::Stored,
        ExportMode::Complete,
    )
    .unwrap();
    for entry in std::fs::read_dir(legacy).unwrap() {
        let entry = entry.unwrap();
        assert_eq!(
            std::fs::read(entry.path()).unwrap(),
            std::fs::read(explicit.join(entry.file_name())).unwrap()
        );
    }
}
