mod common;
use fc2_rs::{filesystem, records::Segment, workspace};

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

#[test]
fn missing_configuration_keeps_all_records_and_source_provenance() {
    let mut b = Builder::new();
    b.work(50, None, vec![("1", records(1000, 63))]);
    let original = b.finish();
    let dataset = workspace::decode(&original).unwrap();
    let work = &dataset.works[0];
    assert_eq!(work.record_count(), 63);
    assert_eq!(work.name, None);
    assert_eq!(work.display_name, "trabalho50");
    assert_eq!(work.expected_groups, None);
    assert!(work.errors.is_empty());
    assert!(
        work.warnings
            .iter()
            .any(|w| w.contains("configuration missing"))
    );
    for record in &work.groups[0].records {
        let bytes: Vec<_> = record
            .segments
            .iter()
            .flat_map(|s| {
                original[s.file_offset..s.file_offset + s.length]
                    .iter()
                    .copied()
            })
            .collect();
        assert_eq!(bytes.len(), 48);
        assert_eq!(fc2_rs::reader::hash(&bytes), record.raw_sha256);
    }
}

#[test]
fn names_are_read_from_configuration_and_points_remain_group_scoped() {
    let mut b = Builder::new();
    b.work(
        12,
        Some(config(12, "SAMPLE-ALPHA", &[1, 1])),
        vec![("1", records(1, 1)), ("2", records(2, 1))],
    );
    let dataset = workspace::decode(&b.finish()).unwrap();
    let work = &dataset.works[0];
    assert_eq!(work.name.as_deref(), Some("SAMPLE-ALPHA"));
    assert_eq!(work.number, 12);
    assert_eq!(work.point_count(), 2);
    assert_eq!(
        work.groups[0].records[0].point,
        work.groups[1].records[0].point
    );
    assert!(work.errors.is_empty());
}

#[test]
fn mismatched_counters_are_errors_without_silent_record_discard() {
    let mut b = Builder::new();
    b.work(
        135,
        Some(config(135, "ALPHA", &[2])),
        vec![("1", records(1, 1))],
    );
    let dataset = workspace::decode(&b.finish()).unwrap();
    let work = &dataset.works[0];
    assert_eq!(work.record_count(), 1);
    assert!(work.errors.iter().any(|e| e.contains("expects 2 records")));
}

#[test]
fn invalid_numeric_group_names_and_duplicate_numbers_are_reported() {
    for names in [vec!["0"], vec!["1", "01"], vec!["999999"]] {
        let mut b = Builder::new();
        b.work(
            1,
            None,
            names
                .iter()
                .enumerate()
                .map(|(i, n)| (*n, records(i as u32 + 1, 1)))
                .collect(),
        );
        let dataset = workspace::decode(&b.finish()).unwrap();
        assert!(!dataset.works[0].errors.is_empty());
        assert_eq!(dataset.filesystem.files.len(), names.len());
    }
}

#[test]
fn duplicate_active_ids_are_rejected_within_and_between_works() {
    let mut same = Builder::new();
    same.work(1, None, vec![("1", records(77, 1)), ("2", records(77, 1))]);
    let dataset = workspace::decode(&same.finish()).unwrap();
    assert!(
        dataset.works[0]
            .errors
            .iter()
            .any(|e| e.contains("duplicate active measurement ID 77"))
    );
    assert_eq!(dataset.works[0].record_count(), 2);
    let mut across = Builder::new();
    across.work(1, None, vec![("1", records(77, 1))]);
    across.work(2, None, vec![("1", records(77, 1))]);
    let dataset = workspace::decode(&across.finish()).unwrap();
    assert!(dataset.works.iter().all(|w| {
        w.errors
            .iter()
            .any(|e| e.contains("duplicate active measurement ID 77"))
    }));
}

#[test]
fn deletion_table_must_not_be_interpreted_as_measurement_counters() {
    let mut configuration = config(1, "ONE", &[0]);
    configuration[188..190].copy_from_slice(&1u16.to_le_bytes());
    let mut b = Builder::new();
    b.work(1, Some(configuration), vec![("75", records(1, 1))]);
    let dataset = workspace::decode(&b.finish()).unwrap();
    assert!(!dataset.works[0].errors.is_empty());
    assert_ne!(
        dataset.works[0]
            .groups
            .first()
            .and_then(|g| g.expected_records),
        Some(1)
    );
}

fn unpack_stored(bytes: &[u8]) -> std::collections::BTreeMap<String, Vec<u8>> {
    let mut entries = std::collections::BTreeMap::new();
    let mut position = 0;
    while bytes.get(position..position + 4) == Some(b"PK\x03\x04") {
        let u16_at =
            |offset| u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap()) as usize;
        let u32_at =
            |offset| u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        assert_eq!(u16_at(position + 8), 0);
        let size = u32_at(position + 18);
        let name_length = u16_at(position + 26);
        let start = position + 30 + name_length + u16_at(position + 28);
        let name =
            String::from_utf8(bytes[position + 30..position + 30 + name_length].to_vec()).unwrap();
        assert!(
            entries
                .insert(name, bytes[start..start + size].to_vec())
                .is_none()
        );
        position = start + size;
    }
    assert_eq!(
        bytes.get(position..position + 4),
        Some(b"PK\x01\x02".as_slice())
    );
    entries
}

#[test]
fn export_refuses_existing_directory_and_preserves_its_files() {
    use fc2_rs::export::works::{WorkbookOrder, export_directory};
    let mut b = Builder::new();
    b.work(1, None, vec![("1", records(1, 1))]);
    let bytes = b.finish();
    let dataset = workspace::decode(&bytes).unwrap();
    let output = tempfile::tempdir().unwrap();
    let marker = output.path().join("marker.txt");
    std::fs::write(&marker, b"preserve original").unwrap();
    assert!(
        export_directory(
            &bytes,
            &dataset,
            "synthetic.FC2",
            output.path(),
            None,
            WorkbookOrder::Stored
        )
        .is_err()
    );
    assert_eq!(std::fs::read(marker).unwrap(), b"preserve original");
    assert_eq!(std::fs::read_dir(output.path()).unwrap().count(), 1);
}

#[test]
fn invalid_work_is_blocked_while_valid_work_and_raw_evidence_are_exported() {
    use fc2_rs::export::works::{WorkbookOrder, export_directory};
    let mut b = Builder::new();
    b.work(1, Some(config(1, "GOOD", &[1])), vec![("1", records(1, 1))]);
    b.work(2, Some(config(2, "BAD", &[2])), vec![("1", records(2, 1))]);
    let bytes = b.finish();
    let dataset = workspace::decode(&bytes).unwrap();
    let parent = tempfile::tempdir().unwrap();
    let output = parent.path().join("results");
    let manifest = export_directory(
        &bytes,
        &dataset,
        "synthetic.FC2",
        &output,
        None,
        WorkbookOrder::Stored,
    )
    .unwrap();
    assert_eq!(manifest.successful_workbooks, 1);
    assert_eq!(manifest.failures, 1);
    assert!(output.join("1-GOOD.xlsx").is_file());
    assert!(!output.join("2-BAD.xlsx").exists());
    assert!(
        manifest.works[1]
            .error
            .as_deref()
            .unwrap()
            .contains("expects 2 records")
    );
    let bundle = unpack_stored(&std::fs::read(output.join("todos-trabalhos.zip")).unwrap());
    assert!(bundle.contains_key("1-GOOD.xlsx"));
    assert!(!bundle.contains_key("2-BAD.xlsx"));
    let raw = unpack_stored(&std::fs::read(output.join("dados-brutos.zip")).unwrap());
    assert_eq!(raw["original.FC2"], bytes);
    assert_eq!(raw["filesystem/trabalho2/1"], records(2, 1));
}

#[test]
fn selecting_one_work_excludes_other_work_data_from_all_downloads() {
    use fc2_rs::export::works::{WorkbookOrder, export_directory};
    let mut b = Builder::new();
    b.work(
        1,
        Some(config(1, "SELECTED", &[1])),
        vec![("1", records(1, 1))],
    );
    b.work(
        2,
        Some(config(2, "OTHER_PRIVATE", &[1])),
        vec![("1", records(2, 1))],
    );
    let bytes = b.finish();
    let dataset = workspace::decode(&bytes).unwrap();
    let parent = tempfile::tempdir().unwrap();
    let output = parent.path().join("selected");
    let manifest = export_directory(
        &bytes,
        &dataset,
        "synthetic.FC2",
        &output,
        Some(1),
        WorkbookOrder::Stored,
    )
    .unwrap();
    assert_eq!(manifest.works.len(), 1);
    let audit = std::fs::read_to_string(output.join("dataset.json")).unwrap();
    assert!(!audit.contains("OTHER_PRIVATE"));
    assert!(!audit.contains("trabalho2"));
    let raw = unpack_stored(&std::fs::read(output.join("dados-brutos.zip")).unwrap());
    assert!(!raw.contains_key("original.FC2"));
    assert!(!raw.contains_key("memory.bin"));
    assert!(raw.keys().all(|path| !path.contains("trabalho2")));
    assert_eq!(raw["filesystem/trabalho1/1"], records(1, 1));
}

#[test]
fn source_hash_and_mutated_raw_file_origins_block_export_before_directory_creation() {
    use fc2_rs::export::works::{WorkbookOrder, export_directory};
    let mut b = Builder::new();
    b.work(1, None, vec![("1", records(1, 1))]);
    let bytes = b.finish();
    let mut dataset = workspace::decode(&bytes).unwrap();
    let parent = tempfile::tempdir().unwrap();
    let output = parent.path().join("wrong");
    let mut other = bytes.clone();
    other[32] ^= 1;
    assert!(
        export_directory(
            &other,
            &dataset,
            "synthetic.FC2",
            &output,
            None,
            WorkbookOrder::Stored
        )
        .is_err()
    );
    assert!(!output.exists());
    dataset.filesystem.files[0].segments[0].file_offset += 1;
    assert!(
        export_directory(
            &bytes,
            &dataset,
            "synthetic.FC2",
            &output,
            None,
            WorkbookOrder::Stored
        )
        .is_err()
    );
    assert!(!output.exists());
}

#[test]
fn missing_calendar_and_gps_produce_blank_xlsx_cells_without_merging_groups() {
    use fc2_rs::export::works::{WorkbookOrder, export_directory};
    let mut unavailable = records(1, 1);
    unavailable[..5].fill(0);
    unavailable[32..34].fill(0);
    unavailable[40..44].copy_from_slice(&0x7fc00000u32.to_le_bytes());
    unavailable[44..48].copy_from_slice(&0x7fc00000u32.to_le_bytes());
    let mut b = Builder::new();
    b.work(
        1,
        Some(config(1, "GROUPS", &[1, 1])),
        vec![("1", unavailable), ("2", records(2, 1))],
    );
    let bytes = b.finish();
    let dataset = workspace::decode(&bytes).unwrap();
    let parent = tempfile::tempdir().unwrap();
    let output = parent.path().join("groups");
    let manifest = export_directory(
        &bytes,
        &dataset,
        "synthetic.FC2",
        &output,
        None,
        WorkbookOrder::Leaf,
    )
    .unwrap();
    assert_eq!(manifest.successful_workbooks, 1);
    assert_eq!(manifest.group_count, 2);
    let parts = unpack_stored(&std::fs::read(output.join("1-GROUPS.xlsx")).unwrap());
    let first = String::from_utf8(parts["xl/worksheets/sheet1.xml"].clone()).unwrap();
    let second = String::from_utf8(parts["xl/worksheets/sheet2.xml"].clone()).unwrap();
    for column in ["D9", "E9", "F9", "G9"] {
        assert!(!first.contains(&format!("r=\"{column}\"")));
    }
    assert!(first.contains("r=\"B9\""));
    assert!(first.contains("r=\"J9\""));
    assert!(second.contains("12-03-2024"));
    assert!(second.contains("-12.25"));
    let workbook = String::from_utf8(parts["xl/workbook.xml"].clone()).unwrap();
    assert!(workbook.contains("Medidas 1"));
    assert!(workbook.contains("Medidas 2"));
}

#[test]
fn historical_reference_matches_do_not_validate_a_different_active_record() {
    use fc2_rs::{
        decoder,
        validator::{self, Profile},
    };
    let mut active = common::raw_record();
    active[24..26].copy_from_slice(&124u16.to_le_bytes());
    let mut b = Builder::new();
    b.work(
        1,
        Some(config(1, "SYNTHETIC", &[1])),
        vec![("1", active.to_vec())],
    );
    b.sector(b.physical, 999, 0, 65535, 48, &common::raw_record());
    b.memory[filesystem::PARTITION_ADDRESS + b.physical * 512] = 5;
    // finish intentionally calculates CRC for active sectors; set historical CRC here.
    let p = filesystem::PARTITION_ADDRESS + b.physical * 512;
    b.memory[p + 19] = filesystem::sector_crc(&b.memory[p..p + 512]).unwrap();
    let bytes = b.finish();
    let reference = common::reference(&bytes);
    let (report, _) = validator::validate(&bytes, &reference).unwrap();
    assert!(report.accepted); // Reference-only search found the released historical copy.
    let profile = Profile {
        schema: "fc2-validation-profile-v1".into(),
        layout: decoder::LAYOUT.into(),
        tool_version: env!("CARGO_PKG_VERSION").into(),
        reference,
        validation: report,
    };
    let mut dataset = workspace::decode(&bytes).unwrap();
    assert!(workspace::compare_reference(&bytes, &mut dataset, &profile).is_err());
    assert!(dataset.works[0].reference_validation.is_none());
}

#[test]
fn altered_measurement_fields_and_provenance_block_export_before_any_output() {
    use fc2_rs::export::works::{WorkbookOrder, export_directory};
    let mut b = Builder::new();
    b.work(1, None, vec![("1", records(1, 1))]);
    let bytes = b.finish();
    let parent = tempfile::tempdir().unwrap();
    for mutation in 0..11 {
        let mut dataset = workspace::decode(&bytes).unwrap();
        let record = &mut dataset.works[0].groups[0].records[0];
        match mutation {
            0 => record.chlorophyll_a_tenths += 1,
            1 => record.date = Some("2024-03-13".into()),
            2 => record.latitude = Some(-12.5),
            3 => record.longitude = None,
            4 => record.raw_hex.replace_range(..2, "a1"),
            5 => record.segments[0].memory_address += 1,
            6 => record.segments[0].file_offset += 1,
            7 => record.raw_calendar.year = 27,
            8 => record.unknown_34 += 1,
            9 => record.chlorophyll_total_index = "16.8".into(),
            _ => record.latitude = Some(f64::NAN),
        }
        let output = parent.path().join(format!("mutation-{mutation}"));
        assert!(
            export_directory(
                &bytes,
                &dataset,
                "synthetic.FC2",
                &output,
                None,
                WorkbookOrder::Stored
            )
            .is_err(),
            "mutation {mutation} was accepted"
        );
        assert!(!output.exists(), "mutation {mutation} created outputs");
    }
}

#[test]
fn export_requires_complete_work_group_file_coverage_and_active_origins() {
    use fc2_rs::export::works::{WorkbookOrder, export_directory};
    let mut b = Builder::new();
    b.work(
        1,
        Some(config(1, "WORKONE", &[2, 1])),
        vec![("1", records(100, 2)), ("2", records(200, 1))],
    );
    b.work(
        2,
        Some(config(2, "WORKTWO", &[1])),
        vec![("1", records(300, 1))],
    );
    let historical_address = filesystem::PARTITION_ADDRESS + b.physical * 512 + 25;
    b.sector(b.physical, 999, 0, 65535, 48, &records(100, 1));
    let start = filesystem::PARTITION_ADDRESS + b.physical * 512;
    b.memory[start] = 5;
    b.memory[start + 19] = filesystem::sector_crc(&b.memory[start..start + 512]).unwrap();
    let bytes = b.finish();
    let parent = tempfile::tempdir().unwrap();
    for mutation in 0..6 {
        let mut dataset = workspace::decode(&bytes).unwrap();
        match mutation {
            0 => {
                dataset.works[0].groups[0].records.pop();
            }
            1 => {
                dataset.works[0].groups.pop();
            }
            2 => {
                dataset.works.pop();
            }
            3 => {
                let index = dataset
                    .filesystem
                    .files
                    .iter()
                    .position(|f| f.path == "/trabalho1/1")
                    .unwrap();
                dataset.filesystem.files.remove(index);
            }
            4 => {
                dataset.works[0].name = Some("OTHER NAME".into());
                dataset.works[0].display_name = "OTHER NAME".into();
            }
            _ => {
                let file_offset = historical_address / 256 * 512 + 32 + historical_address % 256;
                dataset.works[0].groups[0].records[0].segments = vec![Segment {
                    file_offset,
                    memory_address: historical_address,
                    length: 48,
                }];
                // These bytes are genuinely equal, but belong to a released copy.
                let image = fc2_rs::parser::parse(&bytes).unwrap();
                fc2_rs::work_records::verify_source_record(
                    &bytes,
                    &image,
                    &dataset.works[0].groups[0].records[0],
                )
                .unwrap();
            }
        }
        let output = parent.path().join(format!("coverage-{mutation}"));
        assert!(
            export_directory(
                &bytes,
                &dataset,
                "synthetic.FC2",
                &output,
                None,
                WorkbookOrder::Stored
            )
            .is_err(),
            "coverage mutation {mutation} was accepted"
        );
        assert!(
            !output.exists(),
            "coverage mutation {mutation} created output"
        );
    }
}

#[test]
fn explicit_single_work_export_does_not_require_unselected_work_objects() {
    use fc2_rs::export::works::{WorkbookOrder, export_directory};
    let mut b = Builder::new();
    b.work(
        1,
        Some(config(1, "SELECTED", &[1])),
        vec![("1", records(1, 1))],
    );
    b.work(
        2,
        Some(config(2, "OTHER", &[1])),
        vec![("1", records(2, 1))],
    );
    let bytes = b.finish();
    let mut dataset = workspace::decode(&bytes).unwrap();
    dataset.works.retain(|work| work.number == 1);
    let parent = tempfile::tempdir().unwrap();
    let output = parent.path().join("selected");
    let manifest = export_directory(
        &bytes,
        &dataset,
        "synthetic.FC2",
        &output,
        Some(1),
        WorkbookOrder::Stored,
    )
    .unwrap();
    assert_eq!(manifest.successful_workbooks, 1);
    assert_eq!(manifest.record_count, 1);
}

#[test]
fn selected_work_preserves_leading_zero_directory_in_raw_zip_and_audit() {
    use fc2_rs::export::works::{WorkbookOrder, export_directory};
    let mut b = Builder::new();
    b.work(
        50,
        Some(config(50, "ALIASED", &[1])),
        vec![("1", records(1, 1))],
    );
    let root_name = filesystem::PARTITION_ADDRESS + 512 + 25 + 8;
    b.memory[root_name..root_name + 16].fill(0);
    b.memory[root_name..root_name + b"trabalho050".len()].copy_from_slice(b"trabalho050");
    let bytes = b.finish();
    let dataset = workspace::decode(&bytes).unwrap();
    assert_eq!(dataset.works[0].directory, "/trabalho050");
    assert_eq!(dataset.works[0].number, 50);
    assert!(dataset.works[0].errors.is_empty());
    let parent = tempfile::tempdir().unwrap();
    let output = parent.path().join("aliased");
    let manifest = export_directory(
        &bytes,
        &dataset,
        "synthetic.FC2",
        &output,
        Some(50),
        WorkbookOrder::Stored,
    )
    .unwrap();
    assert_eq!(manifest.successful_workbooks, 1);
    let audit: serde_json::Value =
        serde_json::from_slice(&std::fs::read(output.join("dataset.json")).unwrap()).unwrap();
    let paths: Vec<_> = audit["filesystem"]["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|file| file["path"].as_str().unwrap())
        .collect();
    assert!(paths.contains(&"/trabalho050/1"));
    assert!(paths.contains(&"/trabalho050/config_measures"));
    assert_eq!(
        audit["filesystem"]["directories"][0]["path"],
        "/trabalho050"
    );
    let raw = unpack_stored(&std::fs::read(output.join("dados-brutos.zip")).unwrap());
    assert_eq!(raw["filesystem/trabalho050/1"], records(1, 1));
    assert!(raw.contains_key("filesystem/trabalho050/config_measures"));
    assert!(!raw.contains_key("filesystem/trabalho50/1"));
    assert!(!raw.contains_key("original.FC2"));
}
