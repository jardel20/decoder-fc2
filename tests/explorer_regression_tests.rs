mod common;
use fc2_rs::{
    explorer::{LoadedDataset, RecordKey},
    export::works::ExportMode,
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
fn presentation_index_is_exactly_the_existing_decoder() {
    let bytes = source();
    let expected = fc2_rs::workspace::decode(&bytes).unwrap();
    let indexed = LoadedDataset::from_bytes(PathBuf::from("synthetic.FC2"), bytes.clone()).unwrap();
    assert_eq!(indexed.source_bytes, bytes);
    assert_eq!(
        serde_json::to_value(&indexed.dataset).unwrap(),
        serde_json::to_value(&expected).unwrap()
    );
    for (work_index, work) in expected.works.iter().enumerate() {
        let mut expected_keys = Vec::new();
        for (group_index, group) in work.groups.iter().enumerate() {
            for (record_index, record) in group.records.iter().enumerate() {
                let key = RecordKey {
                    work: work_index,
                    group: group_index,
                    record: record_index,
                };
                expected_keys.push(key);
                let presented = indexed.record(key).unwrap();
                assert_eq!(
                    serde_json::to_value(presented).unwrap(),
                    serde_json::to_value(record).unwrap()
                );
                assert_eq!(
                    presented.latitude.map(f64::to_bits),
                    record.latitude.map(f64::to_bits)
                );
                assert_eq!(
                    presented.longitude.map(f64::to_bits),
                    record.longitude.map(f64::to_bits)
                );
            }
        }
        assert_eq!(indexed.records[work_index], expected_keys);
    }
}

#[test]
fn bad_sector_crc_is_rejected_before_presentation() {
    let mut bytes = source();
    let address = filesystem::PARTITION_ADDRESS + 2 * 512 + 19;
    let source_offset = (address / 256) * 512 + 32 + address % 256;
    bytes[source_offset] ^= 1;
    assert!(fc2_rs::workspace::decode(&bytes).is_err());
    assert!(LoadedDataset::from_bytes(PathBuf::from("bad-crc.FC2"), bytes).is_err());
}

#[test]
fn audit_exports_reject_mutated_metadata_coordinates_and_order() {
    let root = tempfile::tempdir().unwrap();
    for mutation in 0..3 {
        let mut data = loaded();
        match mutation {
            0 => data.dataset.works[0].display_name = "ALTERED".into(),
            1 => data.dataset.works[0].groups[0].records[0].longitude = Some(1.0),
            _ => data.dataset.works[0].groups[0].records.swap(0, 1),
        }
        assert!(
            data.export_json(0, &root.path().join(format!("{mutation}.json")))
                .is_err()
        );
        assert!(
            data.export(
                &[0],
                &root.path().join(format!("{mutation}-xlsx")),
                ExportMode::Simple
            )
            .is_err()
        );
    }
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}
